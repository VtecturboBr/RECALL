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

/// Select only live destinations *before* LIMIT and resolve them in one query.
/// This keeps 50 deleted newer associations from obscuring older valid links.
pub async fn list(pool: &SqlitePool, kind: &str, id: &str) -> Result<Vec<ItemLink>, sqlx::Error> {
    validate_endpoint(kind,id)?;
    if !exists_active(pool,kind,id).await? { return Err(sqlx::Error::RowNotFound); }
    sqlx::query_as::<_,ItemLink>(
        "WITH endpoints AS (
          SELECT l.id, l.created_at,
            CASE WHEN l.left_type=? AND l.left_id=? THEN l.right_type ELSE l.left_type END AS item_type,
            CASE WHEN l.left_type=? AND l.left_id=? THEN l.right_id ELSE l.left_id END AS item_id
          FROM item_links l
          WHERE (l.left_type=? AND l.left_id=?) OR (l.right_type=? AND l.right_id=?)
        )
        SELECT e.id,e.item_type,e.item_id,e.created_at,
          CASE e.item_type
            WHEN 'note' THEN COALESCE(NULLIF(TRIM(n.title),''),'Sem título')
            WHEN 'task' THEN t.title ELSE SUBSTR(i.text,1,90) END AS title,
          CASE e.item_type
            WHEN 'note' THEN SUBSTR(n.body,1,180)
            WHEN 'task' THEN SUBSTR(t.details,1,180)
            ELSE SUBSTR(i.text,1,180) END AS preview
        FROM endpoints e
        LEFT JOIN notes n ON e.item_type='note' AND n.id=e.item_id AND n.deleted_at IS NULL
        LEFT JOIN tasks t ON e.item_type='task' AND t.id=e.item_id AND t.deleted_at IS NULL
        LEFT JOIN inbox_items i ON e.item_type='idea' AND i.id=e.item_id AND i.processed_at IS NULL
        WHERE (e.item_type='note' AND n.id IS NOT NULL)
           OR (e.item_type='task' AND t.id IS NOT NULL)
           OR (e.item_type='idea' AND i.id IS NOT NULL)
        ORDER BY e.created_at DESC,e.id DESC LIMIT 50"
    ).bind(kind).bind(id).bind(kind).bind(id).bind(kind).bind(id).bind(kind).bind(id)
        .fetch_all(pool).await
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
        assert_eq!(version,8);
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
    async fn ignores_deleted_links_before_limiting_fifty_live_destinations() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let root=tasks::create(&pool,"Origem").await.unwrap();
        for n in 0..10 {
            let item=db::create_copy(&pool,&format!("Ativa {n}"),"Corpo","note").await.unwrap();
            add(&pool,"task",&root.id,"note",&item.id).await.unwrap();
        }
        for n in 0..60 {
            let item=db::create_copy(&pool,&format!("Oculta {n}"),"Corpo","note").await.unwrap();
            add(&pool,"task",&root.id,"note",&item.id).await.unwrap();
            db::delete(&pool,&item.id).await.unwrap();
        }
        let links=list(&pool,"task",&root.id).await.unwrap();
        assert_eq!(links.len(),10,"deleted newest links cannot hide older live links");
        assert!(links.iter().all(|row|row.title.starts_with("Ativa ")));
        pool.close().await;let _=std::fs::remove_dir_all(dir);
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
