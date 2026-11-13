//! Alpha 9.1: simple, symmetric relationships between existing items.
//! Soft-deleting/processing an item hides its links without destroying them.
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;
use crate::db::now_ms;

const LINK_FIELDS: &str = "id,left_type,left_id,right_type,right_id,created_at";

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ItemLink {
    pub id: String,
    pub item_type: String,
    pub item_id: String,
    pub title: String,
    pub preview: String,
    pub created_at: i64,
}

pub async fn migrate_v7(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("CREATE TABLE item_links (
        id TEXT PRIMARY KEY NOT NULL,
        left_type TEXT NOT NULL CHECK(left_type IN ('note','task','idea')),
        left_id TEXT NOT NULL,
        right_type TEXT NOT NULL CHECK(right_type IN ('note','task','idea')),
        right_id TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        CHECK(left_type < right_type OR (left_type=right_type AND left_id<right_id)),
        UNIQUE(left_type,left_id,right_type,right_id)
    )").execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX item_links_by_left ON item_links(left_type,left_id,created_at DESC)")
        .execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX item_links_by_right ON item_links(right_type,right_id,created_at DESC)")
        .execute(&mut *tx).await?;
    sqlx::query("PRAGMA user_version=7").execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn verify_v7(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let schema: (i64, i64) = sqlx::query_as("SELECT
      (SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='item_links'),
      (SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name IN
         ('item_links_by_left','item_links_by_right'))").fetch_one(pool).await?;
    if schema != (1,2) {
        return Err(sqlx::Error::Protocol("Alpha 9.1: esquema de relacionamentos incompleto".into()));
    }
    sqlx::query(&format!("SELECT {LINK_FIELDS} FROM item_links LIMIT 0"))
        .fetch_all(pool).await?;
    Ok(())
}

fn validate_endpoint(kind: &str, id: &str) -> Result<(), sqlx::Error> {
    if !matches!(kind, "note" | "task" | "idea") || id.is_empty() || id.len() > 128 {
        return Err(sqlx::Error::Protocol("Tipo ou identificador de item inválido".into()));
    }
    Ok(())
}

fn normalize<'a>(a_type: &'a str, a_id: &'a str, b_type: &'a str, b_id: &'a str)
    -> Result<((&'a str, &'a str), (&'a str, &'a str)), sqlx::Error>
{
    validate_endpoint(a_type, a_id)?;
    validate_endpoint(b_type, b_id)?;
    let a=(a_type,a_id);
    let b=(b_type,b_id);
    if a==b { return Err(sqlx::Error::Protocol("Um item não pode ser ligado a si mesmo".into())); }
    Ok(if a<b {(a,b)} else {(b,a)})
}

async fn exists_active<'e, E>(executor: E, kind: &str, id: &str) -> Result<bool, sqlx::Error>
where E: sqlx::Executor<'e, Database=sqlx::Sqlite>
{
    let sql=match kind {
        "note" => "SELECT EXISTS(SELECT 1 FROM notes WHERE id=? AND deleted_at IS NULL)",
        "task" => "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=? AND deleted_at IS NULL)",
        "idea" => "SELECT EXISTS(SELECT 1 FROM inbox_items WHERE id=? AND processed_at IS NULL)",
        _ => return Err(sqlx::Error::Protocol("Tipo de item inválido".into())),
    };
    let answer:i64=sqlx::query_scalar(sql).bind(id).fetch_one(executor).await?;
    Ok(answer!=0)
}

pub async fn add(pool: &SqlitePool, a_type: &str, a_id: &str, b_type: &str, b_id: &str)
    -> Result<String, sqlx::Error>
{
    let ((left_type,left_id),(right_type,right_id))=normalize(a_type,a_id,b_type,b_id)?;
    let mut tx=pool.begin().await?;
    if !exists_active(&mut *tx,left_type,left_id).await?
        || !exists_active(&mut *tx,right_type,right_id).await? {
        return Err(sqlx::Error::RowNotFound);
    }
    let id=Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO item_links(id,left_type,left_id,right_type,right_id,created_at)
        VALUES (?,?,?,?,?,?) ON CONFLICT(left_type,left_id,right_type,right_id) DO NOTHING")
        .bind(&id).bind(left_type).bind(left_id).bind(right_type).bind(right_id).bind(now_ms())
        .execute(&mut *tx).await?;
    let stored:String=sqlx::query_scalar("SELECT id FROM item_links WHERE
        left_type=? AND left_id=? AND right_type=? AND right_id=?")
        .bind(left_type).bind(left_id).bind(right_type).bind(right_id)
        .fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(stored)
}

pub async fn remove(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    if id.is_empty() || id.len()>128 {return Err(sqlx::Error::RowNotFound);}
    let affected=sqlx::query("DELETE FROM item_links WHERE id=?").bind(id).execute(pool).await?;
    if affected.rows_affected()==0 {return Err(sqlx::Error::RowNotFound);}
    Ok(())
}

/// A 50-item bound prevents huge IPC payloads or N+1 loading in the inspector.
/// Links to processed/excluded items stay in SQLite for later restoration.
/// Filter inactive targets before the 50-item limit and retrieve all
/// previews in ONE SQL query (no N+1 DB roundtrips).
pub async fn list(pool: &SqlitePool, kind: &str, id: &str) -> Result<Vec<ItemLink>,sqlx::Error> {
    validate_endpoint(kind,id)?;
    if !exists_active(pool,kind,id).await? { return Err(sqlx::Error::RowNotFound); }
    sqlx::query_as::<_,ItemLink>(
        "SELECT q.id,q.other_type AS item_type,q.other_id AS item_id,
            COALESCE(NULLIF(TRIM(n.title),''),NULLIF(TRIM(t.title),''),
              NULLIF(TRIM(SUBSTR(i.text,1,90)),''),'Sem título') AS title,
            SUBSTR(COALESCE(n.body,t.details,i.text,''),1,180) AS preview,
            q.created_at
         FROM (
            SELECT id,right_type AS other_type,right_id AS other_id,created_at
              FROM item_links WHERE left_type=? AND left_id=?
            UNION ALL
            SELECT id,left_type AS other_type,left_id AS other_id,created_at
              FROM item_links WHERE right_type=? AND right_id=?
         ) q
         LEFT JOIN notes n ON q.other_type='note' AND n.id=q.other_id AND n.deleted_at IS NULL
         LEFT JOIN tasks t ON q.other_type='task' AND t.id=q.other_id AND t.deleted_at IS NULL
         LEFT JOIN inbox_items i ON q.other_type='idea' AND i.id=q.other_id AND i.processed_at IS NULL
         WHERE n.id IS NOT NULL OR t.id IS NOT NULL OR i.id IS NOT NULL
         ORDER BY q.created_at DESC,q.id DESC LIMIT 50"
    ).bind(kind).bind(id).bind(kind).bind(id).fetch_all(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db,tasks,inbox};
    fn fixture()->(std::path::PathBuf,std::path::PathBuf){
        let dir=std::env::temp_dir().join(format!("recall-links9-{}",Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        (dir.clone(),dir.join("memoria.sqlite"))
    }

    #[tokio::test]
    async fn hidden_targets_cannot_exhaust_first_fifty_visible_links() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let source=tasks::create(&pool,"Central").await.unwrap();
        for number in 0..65 {
            let note=db::create_copy(&pool,&format!("N{number}"),"texto","note").await.unwrap();
            let link_id=add(&pool,"task",&source.id,"note",&note.id).await.unwrap();
            sqlx::query("UPDATE item_links SET created_at=? WHERE id=?")
                .bind(1_000_000_i64+number).bind(&link_id).execute(&pool).await.unwrap();
            if number >= 10 {db::delete(&pool,&note.id).await.unwrap();}
        }
        let shown=list(&pool,"task",&source.id).await.unwrap();
        assert_eq!(shown.len(),10,"active links must be filtered before LIMIT");
        assert!(shown.iter().all(|x| x.title.starts_with('N')));
        pool.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn symmetric_duplicate_safe_and_self_link_rejected() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Testar a interface").await.unwrap();
        let note=db::create_copy(&pool,"Notas de design","Contraste e foco","note").await.unwrap();
        let id=add(&pool,"task",&task.id,"note",&note.id).await.unwrap();
        let reverse=add(&pool,"note",&note.id,"task",&task.id).await.unwrap();
        assert_eq!(id,reverse);
        let task_links=list(&pool,"task",&task.id).await.unwrap();
        assert_eq!(task_links.len(),1);
        assert_eq!(task_links[0].title,"Notas de design");
        let note_links=list(&pool,"note",&note.id).await.unwrap();
        assert_eq!(note_links.len(),1);
        assert_eq!(note_links[0].item_id,task.id);
        assert!(add(&pool,"task",&task.id,"task",&task.id).await.is_err());
        remove(&pool,&id).await.unwrap();
        assert!(list(&pool,"task",&task.id).await.unwrap().is_empty());
        assert!(remove(&pool,&id).await.is_err());
        pool.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn deleted_or_processed_items_hide_links_without_erasing() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Planejar").await.unwrap();
        let note=db::create_copy(&pool,"Rascunho","Corpo","note").await.unwrap();
        let idea=inbox::capture(&pool,"Ideia inicial").await.unwrap();
        add(&pool,"task",&task.id,"note",&note.id).await.unwrap();
        add(&pool,"task",&task.id,"idea",&idea.id).await.unwrap();
        assert_eq!(list(&pool,"task",&task.id).await.unwrap().len(),2);
        db::delete(&pool,&note.id).await.unwrap();
        inbox::dismiss(&pool,&idea.id).await.unwrap();
        assert!(list(&pool,"task",&task.id).await.unwrap().is_empty());
        let remaining:i64=sqlx::query_scalar("SELECT COUNT(*) FROM item_links")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(remaining,2,"soft deletion must preserve association records");
        assert!(add(&pool,"task",&task.id,"idea",&idea.id).await.is_err());
        assert!(list(&pool,"idea",&idea.id).await.is_err());
        pool.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn migrating_v6_preserves_contents_and_makes_verified_snapshot() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Nunca apagar").await.unwrap();
        pool.close().await;
        let raw=sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(&path)).await.unwrap();
        sqlx::query("DROP TABLE item_links").execute(&raw).await.unwrap();
        sqlx::query("PRAGMA user_version=6").execute(&raw).await.unwrap();
        raw.close().await;
        let upgraded=db::connect(&path).await.unwrap();
        assert_eq!(tasks::get(&upgraded,&task.id).await.unwrap().unwrap().title,"Nunca apagar");
        let version:i64=sqlx::query_scalar("PRAGMA user_version").fetch_one(&upgraded).await.unwrap();
        assert_eq!(version,7);
        assert!(db::list_backups(&path).unwrap().iter().any(|b|b.file_name.contains("pre-alpha9-links")));
        upgraded.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn broken_migration_rolls_back_version_and_never_loses_old_task() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Preservado").await.unwrap();
        pool.close().await;
        let raw=sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(&path)).await.unwrap();
        sqlx::query("DROP TABLE item_links").execute(&raw).await.unwrap();
        sqlx::query("PRAGMA user_version=6").execute(&raw).await.unwrap();
        sqlx::query("CREATE TABLE item_links(broken INTEGER)").execute(&raw).await.unwrap();
        raw.close().await;
        assert!(db::connect(&path).await.is_err());
        let inspect=sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(&path)).await.unwrap();
        let v:i64=sqlx::query_scalar("PRAGMA user_version").fetch_one(&inspect).await.unwrap();
        assert_eq!(v,6);
        let title:String=sqlx::query_scalar("SELECT title FROM tasks WHERE id=?")
            .bind(&task.id).fetch_one(&inspect).await.unwrap();
        assert_eq!(title,"Preservado");
        inspect.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn invalid_endpoint_is_rejected_without_insert() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let a=tasks::create(&pool,"Primeiro").await.unwrap();
        let b=tasks::create(&pool,"Segundo").await.unwrap();
        assert!(add(&pool,"other",&a.id,"task",&b.id).await.is_err());
        assert!(add(&pool,"task",&a.id,"task","não-existe").await.is_err());
        assert!(list(&pool,"anything",&a.id).await.is_err());
        let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM item_links").fetch_one(&pool).await.unwrap();
        assert_eq!(count,0);
        pool.close().await;let _=std::fs::remove_dir_all(dir);
    }
}
