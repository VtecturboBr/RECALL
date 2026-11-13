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
    let result = sqlx::query("UPDATE inbox_items SET processed_at=?, updated_at=? WHERE id=? AND processed_at IS NULL")
        .bind(now).bind(now).bind(id).execute(pool).await?;
    if result.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
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
    transaction.commit().await?;
    crate::tasks::get(pool, &task_id).await?.ok_or(sqlx::Error::RowNotFound)
}

#[cfg(test)]
mod tests {
    use super::split_capture;
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
