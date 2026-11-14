use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;
use crate::{db::{self, now_ms}, tasks::Task};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct InboxItem {
    pub id: String,
    pub text: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub processed_at: Option<i64>,
    pub revision: i64,
}

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("CREATE TABLE IF NOT EXISTS inbox_items (
        id TEXT PRIMARY KEY NOT NULL,
        text TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        processed_at INTEGER
    )").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_inbox_unprocessed ON inbox_items (processed_at, created_at DESC)")
        .execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS inbox_revisions (
        item_id TEXT PRIMARY KEY NOT NULL,
        revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0)
    )").execute(pool).await?;
    sqlx::query("INSERT OR IGNORE INTO inbox_revisions(item_id,revision) SELECT id,0 FROM inbox_items").execute(pool).await?;
    sqlx::query("CREATE TRIGGER IF NOT EXISTS inbox_revision_insert AFTER INSERT ON inbox_items
        BEGIN INSERT OR IGNORE INTO inbox_revisions(item_id,revision) VALUES(NEW.id,0); END").execute(pool).await?;
    sqlx::query("CREATE TRIGGER IF NOT EXISTS inbox_revision_edit AFTER UPDATE OF text,processed_at ON inbox_items
        BEGIN UPDATE inbox_revisions SET revision=revision+1 WHERE item_id=NEW.id; END").execute(pool).await?;
    Ok(())
}

const FIELDS: &str = "i.id,i.text,i.created_at,i.updated_at,i.processed_at,COALESCE(v.revision,0) AS revision";
const JOIN: &str = "FROM inbox_items i LEFT JOIN inbox_revisions v ON v.item_id=i.id";

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<InboxItem>, sqlx::Error> {
    sqlx::query_as::<_, InboxItem>(&format!("SELECT {FIELDS} {JOIN} WHERE i.id=? AND i.processed_at IS NULL"))
        .bind(id).fetch_optional(pool).await
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<InboxItem>, sqlx::Error> {
    sqlx::query_as::<_, InboxItem>(&format!("SELECT {FIELDS} {JOIN}
        WHERE i.processed_at IS NULL ORDER BY i.created_at DESC"))
        .fetch_all(pool).await
}

pub async fn capture(pool: &SqlitePool, text: &str) -> Result<InboxItem, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query("INSERT INTO inbox_items(id,text,created_at,updated_at) VALUES (?,?,?,?)")
        .bind(&id).bind(text.trim()).bind(now).bind(now).execute(pool).await?;
    get(pool, &id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn update(pool: &SqlitePool, id: &str, text: &str, expected_revision: i64) -> Result<Option<InboxItem>, sqlx::Error> {
    let result = sqlx::query("UPDATE inbox_items SET text=?, updated_at=? WHERE id=? AND processed_at IS NULL
        AND EXISTS(SELECT 1 FROM inbox_revisions WHERE item_id=inbox_items.id AND revision=?)")
        .bind(text.trim()).bind(now_ms()).bind(id).bind(expected_revision).execute(pool).await?;
    if result.rows_affected() == 0 { return Ok(None); }
    get(pool, id).await
}

pub async fn dismiss(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    let now = now_ms();
    let mut tx = pool.begin().await?;
    let result = sqlx::query("UPDATE inbox_items SET processed_at=?, updated_at=? WHERE id=? AND processed_at IS NULL")
        .bind(now).bind(now).bind(id).execute(&mut *tx).await?;
    if result.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    // Atomic disposition differentiates a genuine discard from a conversion.
    sqlx::query("INSERT INTO inbox_dispositions(item_id,state,target_id,changed_at)
        VALUES(?,'dismissed',NULL,?)
        ON CONFLICT(item_id) DO UPDATE SET state=excluded.state,target_id=NULL,changed_at=excluded.changed_at")
        .bind(id).bind(now).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

fn split_capture(text: &str) -> (String, String) {
    let cleaned = text.trim();
    let (first_line, rest) = cleaned.split_once('\n').unwrap_or((cleaned, ""));
    let first_line = first_line.trim_end_matches('\r').trim();
    let title: String = first_line.chars().take(90).collect();
    let body = if first_line.chars().count() > 90 { cleaned.to_string() } else { rest.trim().to_string() };
    (title, body)
}

/// Preserve the historical idea links and copy them to its converted successor.
/// Both links and the conversion commit together; invalid/dormant targets remain dormant.
async fn copy_links_after_conversion(
    tx: &mut sqlx::Transaction<'_,sqlx::Sqlite>, idea_id: &str, kind: &str, new_id: &str,
) -> Result<(),sqlx::Error> {
    let relationships:Vec<(String,String,String,String)>=sqlx::query_as(
        "SELECT left_type,left_id,right_type,right_id FROM item_links
         WHERE (left_type='idea' AND left_id=?) OR (right_type='idea' AND right_id=?)"
    ).bind(idea_id).bind(idea_id).fetch_all(&mut **tx).await?;
    for (left_type,left_id,right_type,right_id) in relationships {
        let other = if left_type=="idea" && left_id==idea_id {
            (right_type,right_id)
        } else { (left_type,left_id) };
        let next=(kind.to_string(),new_id.to_string());
        if next == other { continue; }
        let (left,right)=if next<other {(next,other)}else{(other,next)};
        sqlx::query("INSERT OR IGNORE INTO item_links(
            id,left_type,left_id,right_type,right_id,created_at) VALUES(?,?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(left.0).bind(left.1)
            .bind(right.0).bind(right.1).bind(now_ms()).execute(&mut **tx).await?;
    }
    Ok(())
}

pub async fn to_note(pool: &SqlitePool, id: &str) -> Result<db::Note, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let (text,): (String,) = sqlx::query_as("SELECT text FROM inbox_items
        WHERE id=? AND processed_at IS NULL")
        .bind(id).fetch_optional(&mut *transaction).await?.ok_or(sqlx::Error::RowNotFound)?;
    let (title, body) = split_capture(&text);
    let now = now_ms();
    let result = sqlx::query("UPDATE inbox_items SET processed_at=?,updated_at=? WHERE id=? AND processed_at IS NULL")
        .bind(now).bind(now).bind(id).execute(&mut *transaction).await?;
    if result.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    let note_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO notes (id,title,body,kind,created_at,updated_at) VALUES (?,?,?,'note',?,?)")
        .bind(&note_id).bind(title).bind(body).bind(now).bind(now).execute(&mut *transaction).await?;
    sqlx::query("INSERT INTO inbox_dispositions(item_id,state,target_id,changed_at)
        VALUES(?,'converted_note',?,?)
        ON CONFLICT(item_id) DO UPDATE SET state=excluded.state,target_id=excluded.target_id,changed_at=excluded.changed_at")
        .bind(id).bind(&note_id).bind(now).execute(&mut *transaction).await?;
    copy_links_after_conversion(&mut transaction,id,"note",&note_id).await?;
    transaction.commit().await?;
    db::get(pool, &note_id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn to_task(pool: &SqlitePool, id: &str) -> Result<Task, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let (text,): (String,) = sqlx::query_as("SELECT text FROM inbox_items
        WHERE id=? AND processed_at IS NULL")
        .bind(id).fetch_optional(&mut *transaction).await?.ok_or(sqlx::Error::RowNotFound)?;
    let (title, details) = split_capture(&text);
    let now = now_ms();
    let result = sqlx::query("UPDATE inbox_items SET processed_at=?,updated_at=? WHERE id=? AND processed_at IS NULL")
        .bind(now).bind(now).bind(id).execute(&mut *transaction).await?;
    if result.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    let task_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO tasks(id,title,details,created_at,updated_at) VALUES (?,?,?,?,?)")
        .bind(&task_id).bind(title).bind(details).bind(now).bind(now).execute(&mut *transaction).await?;
    sqlx::query("INSERT INTO inbox_dispositions(item_id,state,target_id,changed_at)
        VALUES(?,'converted_task',?,?)
        ON CONFLICT(item_id) DO UPDATE SET state=excluded.state,target_id=excluded.target_id,changed_at=excluded.changed_at")
        .bind(id).bind(&task_id).bind(now).execute(&mut *transaction).await?;
    copy_links_after_conversion(&mut transaction,id,"task",&task_id).await?;
    transaction.commit().await?;
    crate::tasks::get(pool, &task_id).await?.ok_or(sqlx::Error::RowNotFound)
}

#[cfg(test)]
mod tests {
    use super::split_capture;
    #[tokio::test]
    async fn simultaneous_idea_edits_preserve_first_write() {
        let dir = std::env::temp_dir().join(format!("recall-idea-cas-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let pool = crate::db::connect(&dir.join("memoria.sqlite")).await.unwrap();
        let idea = super::capture(&pool, "Primeira versão").await.unwrap();
        let (a,b) = tokio::join!(
            super::update(&pool, &idea.id, "Texto A", idea.revision),
            super::update(&pool, &idea.id, "Texto B", idea.revision)
        );
        assert_ne!(a.unwrap().is_some(), b.unwrap().is_some());
        assert_eq!(super::get(&pool, &idea.id).await.unwrap().unwrap().revision, 1);
        sqlx::query("UPDATE inbox_items SET text='Edição externa' WHERE id=?")
            .bind(&idea.id).execute(&pool).await.unwrap();
        assert!(super::update(&pool, &idea.id, "Obsoleto", 1).await.unwrap().is_none());
        pool.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn idea_conversions_keep_related_items_visible_on_new_entity() {
        let dir=std::env::temp_dir().join(format!("recall-conversion-links-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let pool=crate::db::connect(&dir.join("memoria.sqlite")).await.unwrap();
        let other=crate::tasks::create(&pool,"Trabalho").await.unwrap();
        let idea=super::capture(&pool,"Ideia de implementação").await.unwrap();
        crate::links::add(&pool,"idea",&idea.id,"task",&other.id).await.unwrap();
        let note=super::to_note(&pool,&idea.id).await.unwrap();
        assert_eq!(crate::links::list(&pool,"note",&note.id).await.unwrap().len(),1);
        let old_links:i64=sqlx::query_scalar("SELECT COUNT(*) FROM item_links
            WHERE (left_type='idea' AND left_id=?) OR (right_type='idea' AND right_id=?)")
            .bind(&idea.id).bind(&idea.id).fetch_one(&pool).await.unwrap();
        assert_eq!(old_links,1,"historical links must remain after conversion");
        let idea2=super::capture(&pool,"Segunda ideia").await.unwrap();
        crate::links::add(&pool,"idea",&idea2.id,"note",&note.id).await.unwrap();
        let new_task=super::to_task(&pool,&idea2.id).await.unwrap();
        assert_eq!(crate::links::list(&pool,"task",&new_task.id).await.unwrap().len(),1);
        pool.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[test] fn converts_first_line_to_title() {
        assert_eq!(split_capture("Revisar slides\nAntes de dormir"), ("Revisar slides".into(), "Antes de dormir".into()));
    }
    #[test] fn preserves_a_long_first_line() {
        let text = "a".repeat(110);
        let (title, body) = split_capture(&text);
        assert_eq!(title.chars().count(), 90);
        assert_eq!(body, text);
    }
}
