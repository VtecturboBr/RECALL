use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, FromRow)]
pub struct BoardItem {
    pub id: String, pub kind: String, pub x: f64, pub y: f64,
    pub width: f64, pub height: f64, pub content: String,
    pub revision: i64, pub created_at: i64, pub updated_at: i64,
}

pub async fn migrate_v8(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS whiteboard_items (
        id TEXT PRIMARY KEY NOT NULL,
        kind TEXT NOT NULL CHECK(kind IN ('note','link','table','image')),
        x REAL NOT NULL, y REAL NOT NULL, width REAL NOT NULL, height REAL NOT NULL,
        content TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0),
        created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, deleted_at INTEGER
    )").execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_whiteboard_active
        ON whiteboard_items(deleted_at,created_at,id)").execute(&mut *tx).await?;
    // Assets are in SQLite so verified data backups include user images.
    sqlx::query("CREATE TABLE IF NOT EXISTS whiteboard_assets (
        item_id TEXT PRIMARY KEY NOT NULL REFERENCES whiteboard_items(id),
        data_url TEXT NOT NULL, created_at INTEGER NOT NULL
    )").execute(&mut *tx).await?;
    sqlx::query("PRAGMA user_version=8").execute(&mut *tx).await?;
    tx.commit().await
}
pub async fn verify_v8(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table'
        AND name IN ('whiteboard_items','whiteboard_assets')").fetch_one(pool).await?;
    if count != 2 { return Err(sqlx::Error::Protocol("RECALL Quadro: schema v8 incompleto".into())); }
    sqlx::query("SELECT id,kind,x,y,width,height,content,revision,created_at,updated_at,deleted_at
        FROM whiteboard_items LIMIT 0").fetch_all(pool).await?;
    sqlx::query("SELECT item_id,data_url,created_at FROM whiteboard_assets LIMIT 0")
        .fetch_all(pool).await?;
    Ok(())
}
pub async fn list(pool: &SqlitePool) -> Result<Vec<BoardItem>, sqlx::Error> {
    sqlx::query_as("SELECT id,kind,x,y,width,height,content,revision,created_at,updated_at
        FROM whiteboard_items WHERE deleted_at IS NULL ORDER BY created_at,id")
        .fetch_all(pool).await
}
async fn get(pool: &SqlitePool, id: &str) -> Result<Option<BoardItem>, sqlx::Error> {
    sqlx::query_as("SELECT id,kind,x,y,width,height,content,revision,created_at,updated_at
        FROM whiteboard_items WHERE id=? AND deleted_at IS NULL")
        .bind(id).fetch_optional(pool).await
}
fn valid_geometry(x: f64,y: f64,w: f64,h: f64) -> bool {
    x.is_finite() && y.is_finite() && x.abs() <= 1e7 && y.abs() <= 1e7
        && w.is_finite() && h.is_finite() && (160.0..=1600.0).contains(&w)
        && (110.0..=1600.0).contains(&h)
}
fn valid_asset(value: &str) -> bool {
    if value.len() > 7_000_000 { return false; }
    let prefix = ["data:image/png;base64,","data:image/jpeg;base64,",
        "data:image/webp;base64,","data:image/gif;base64,"]
        .into_iter().find(|p| value.starts_with(p));
    let Some(prefix) = prefix else { return false; };
    let bytes = &value[prefix.len()..];
    !bytes.is_empty() && bytes.len()%4==0
        && bytes.bytes().all(|ch| ch.is_ascii_alphanumeric()
            || ch==b'+' || ch==b'/' || ch==b'=')
}
pub async fn create(
    pool: &SqlitePool, kind: &str, x: f64, y: f64, content: &str,
    asset: Option<&str>
) -> Result<BoardItem, String> {
    let (w,h) = match kind {
        "note"=>(280.0,210.0), "link"=>(290.0,160.0),
        "table"=>(420.0,260.0), "image"=>(360.0,280.0),
        _=>return Err("Tipo de cartão inválido".into())
    };
    if !valid_geometry(x,y,w,h) { return Err("Posição inválida".into()); }
    if content.len()>100_000 { return Err("Conteúdo excede o limite de 100 KB".into()); }
    if kind=="image" {
        if !asset.is_some_and(valid_asset) {
            return Err("Imagem inválida ou acima de 5 MB".into());
        }
    } else if asset.is_some() { return Err("Arquivo só é permitido em imagens".into()); }
    let id=Uuid::new_v4().to_string();
    let now=crate::db::now_ms();
    let mut tx=pool.begin().await.map_err(|e|e.to_string())?;
    sqlx::query("INSERT INTO whiteboard_items
        (id,kind,x,y,width,height,content,revision,created_at,updated_at)
        VALUES(?,?,?,?,?,?,?,0,?,?)")
        .bind(&id).bind(kind).bind(x).bind(y).bind(w).bind(h)
        .bind(content).bind(now).bind(now).execute(&mut *tx).await.map_err(|e|e.to_string())?;
    if let Some(data_url)=asset {
        sqlx::query("INSERT INTO whiteboard_assets(item_id,data_url,created_at) VALUES(?,?,?)")
            .bind(&id).bind(data_url).bind(now).execute(&mut *tx).await.map_err(|e|e.to_string())?;
    }
    tx.commit().await.map_err(|e|e.to_string())?;
    get(pool,&id).await.map_err(|e|e.to_string())?.ok_or("Cartão não encontrado".into())
}
pub async fn update(
    pool: &SqlitePool,id: &str,x: f64,y: f64,w: f64,h: f64,
    content: &str,expected_revision: i64
) -> Result<Option<BoardItem>,String> {
    if !valid_geometry(x,y,w,h) || expected_revision<0 { return Err("Geometria ou revisão inválida".into()); }
    if content.len()>100_000 { return Err("Conteúdo excede 100 KB".into()); }
    let result=sqlx::query("UPDATE whiteboard_items SET
        x=?,y=?,width=?,height=?,content=?,revision=revision+1,updated_at=?
        WHERE id=? AND revision=? AND deleted_at IS NULL")
        .bind(x).bind(y).bind(w).bind(h).bind(content).bind(crate::db::now_ms())
        .bind(id).bind(expected_revision).execute(pool).await.map_err(|e|e.to_string())?;
    if result.rows_affected()==0 { return Ok(None); }
    get(pool,id).await.map_err(|e|e.to_string())
}
pub async fn delete(pool: &SqlitePool,id: &str,expected_revision: i64) -> Result<bool,String> {
    if expected_revision<0 { return Err("Revisão inválida".into()); }
    let now=crate::db::now_ms();
    let result=sqlx::query("UPDATE whiteboard_items SET deleted_at=?,updated_at=?,revision=revision+1
        WHERE id=? AND revision=? AND deleted_at IS NULL")
        .bind(now).bind(now).bind(id).bind(expected_revision).execute(pool).await
        .map_err(|e|e.to_string())?;
    Ok(result.rows_affected()>0)
}
pub async fn asset(pool: &SqlitePool,id: &str) -> Result<Option<String>,sqlx::Error> {
    sqlx::query_scalar("SELECT a.data_url FROM whiteboard_assets a
        JOIN whiteboard_items i ON i.id=a.item_id
        WHERE a.item_id=? AND i.kind='image' AND i.deleted_at IS NULL")
        .bind(id).fetch_optional(pool).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use std::fs;

    #[tokio::test]
    async fn whiteboard_persists_rejects_stale_edits_and_keeps_images_in_backup() {
        let folder=std::env::temp_dir().join(format!("recall-whiteboard-{}",Uuid::new_v4()));
        fs::create_dir_all(&folder).unwrap();
        let path=folder.join("memoria.sqlite");
        let pool=db::connect(&path).await.unwrap();
        let note=create(&pool,"note",10.0,-5.0,"Minha ideia",None).await.unwrap();
        let saved=update(&pool,&note.id,20.0,30.0,300.0,200.0,"Editada",0)
            .await.unwrap().unwrap();
        assert_eq!(saved.revision,1);
        assert!(update(&pool,&note.id,0.0,0.0,300.0,200.0,"Stale",0)
            .await.unwrap().is_none());
        assert!(update(&pool,&note.id,f64::NAN,0.0,300.0,200.0,"Bad",1).await.is_err());
        assert!(create(&pool,"image",0.0,0.0,"svg",Some("data:image/svg+xml;base64,PHN2Zz4="))
            .await.is_err());
        let img=create(&pool,"image",0.0,0.0,"foto",Some("data:image/png;base64,AAAA"))
            .await.unwrap();
        let backup=db::create_backup(&pool,&path,"test-board").await.unwrap();
        assert!(std::path::Path::new(&backup.file_path).is_file());
        pool.close().await;
        let reopened=db::connect(&path).await.unwrap();
        assert_eq!(list(&reopened).await.unwrap().len(),2);
        assert_eq!(asset(&reopened,&img.id).await.unwrap().as_deref(),Some("data:image/png;base64,AAAA"));
        assert!(!delete(&reopened,&note.id,0).await.unwrap());
        assert!(delete(&reopened,&note.id,1).await.unwrap());
        assert_eq!(list(&reopened).await.unwrap().len(),1);
        reopened.close().await;
        let _=fs::remove_dir_all(folder);
    }
}
