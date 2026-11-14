use serde::Serialize;
use sqlx::{FromRow, Row, SqlitePool};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, FromRow)]
pub struct BoardItem {
    pub id: String, pub kind: String, pub x: f64, pub y: f64,
    pub width: f64, pub height: f64, pub content: String,
    pub revision: i64, pub created_at: i64, pub updated_at: i64,
    pub board_id: String,
}


#[derive(Clone, Debug, Serialize, FromRow)]
pub struct Whiteboard {
    pub id: String, pub name: String, pub created_at: i64, pub updated_at: i64,
}
#[derive(Clone, Debug, Serialize, FromRow)]
pub struct BoardConnection {
    pub id: String, pub board_id: String, pub from_id: String, pub to_id: String,
    pub created_at: i64,
}
pub async fn migrate_v9(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx=pool.begin().await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS whiteboards (
        id TEXT PRIMARY KEY NOT NULL, name TEXT NOT NULL,
        created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
    )").execute(&mut *tx).await?;
    let now=crate::db::now_ms();
    sqlx::query("INSERT OR IGNORE INTO whiteboards(id,name,created_at,updated_at)
        VALUES('default','Meu quadro',?,?)")
        .bind(now).bind(now).execute(&mut *tx).await?;
    // Idempotent for legacy migration regression fixtures where user_version is
    // rolled back but the additive board schema already exists. Never rebuild
    // tables or overwrite board membership and IDs.
    let columns = sqlx::query("PRAGMA table_info(whiteboard_items)")
        .fetch_all(&mut *tx).await?;
    let has_board_id = columns.iter().any(|column|
        column.try_get::<String, _>("name").as_deref() == Ok("board_id")
    );
    if !has_board_id {
        sqlx::query("ALTER TABLE whiteboard_items
            ADD COLUMN board_id TEXT NOT NULL DEFAULT 'default'")
            .execute(&mut *tx).await?;
    }
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_whiteboard_by_board
        ON whiteboard_items(board_id,deleted_at,created_at,id)")
        .execute(&mut *tx).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS whiteboard_connections (
        id TEXT PRIMARY KEY NOT NULL,
        board_id TEXT NOT NULL REFERENCES whiteboards(id),
        from_id TEXT NOT NULL REFERENCES whiteboard_items(id),
        to_id TEXT NOT NULL REFERENCES whiteboard_items(id),
        created_at INTEGER NOT NULL,
        CHECK(from_id <> to_id), UNIQUE(board_id,from_id,to_id)
    )").execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_whiteboard_connections_by_board
        ON whiteboard_connections(board_id,created_at,id)").execute(&mut *tx).await?;
    sqlx::query("PRAGMA user_version=9").execute(&mut *tx).await?;
    tx.commit().await
}
pub async fn verify_v9(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master
        WHERE type='table' AND name IN
        ('whiteboard_items','whiteboard_assets','whiteboards','whiteboard_connections')")
        .fetch_one(pool).await?;
    if count!=4 { return Err(sqlx::Error::Protocol("RECALL Quadro: schema v9 incompleto".into())); }
    let default_exists:i64=sqlx::query_scalar("SELECT COUNT(*) FROM whiteboards WHERE id='default'")
        .fetch_one(pool).await?;
    if default_exists!=1 { return Err(sqlx::Error::Protocol("RECALL Quadro: quadro original ausente".into())); }
    sqlx::query("SELECT board_id FROM whiteboard_items LIMIT 0").fetch_all(pool).await?;
    sqlx::query("SELECT id,board_id,from_id,to_id,created_at FROM whiteboard_connections LIMIT 0")
        .fetch_all(pool).await?;
    Ok(())
}
pub async fn list_boards(pool: &SqlitePool) -> Result<Vec<Whiteboard>,sqlx::Error> {
    sqlx::query_as("SELECT id,name,created_at,updated_at FROM whiteboards ORDER BY created_at,id")
        .fetch_all(pool).await
}
pub async fn create_board(pool: &SqlitePool,name: &str) -> Result<Whiteboard,String> {
    let name=name.trim();
    if name.is_empty()||name.chars().count()>80 {
        return Err("Nome do quadro deve ter entre 1 e 80 caracteres".into());
    }
    let id=Uuid::new_v4().to_string();
    let now=crate::db::now_ms();
    sqlx::query("INSERT INTO whiteboards(id,name,created_at,updated_at) VALUES(?,?,?,?)")
        .bind(&id).bind(name).bind(now).bind(now).execute(pool).await.map_err(|e|e.to_string())?;
    Ok(Whiteboard{id,name:name.to_owned(),created_at:now,updated_at:now})
}
pub async fn rename_board(pool: &SqlitePool,id: &str,name: &str)
    -> Result<Option<Whiteboard>,String>
{
    let name=name.trim();
    if name.is_empty()||name.chars().count()>80 {
        return Err("Nome do quadro deve ter entre 1 e 80 caracteres".into());
    }
    let changed=sqlx::query("UPDATE whiteboards SET name=?,updated_at=? WHERE id=?")
        .bind(name).bind(crate::db::now_ms()).bind(id).execute(pool).await.map_err(|e|e.to_string())?;
    if changed.rows_affected()==0 { return Ok(None); }
    sqlx::query_as("SELECT id,name,created_at,updated_at FROM whiteboards WHERE id=?")
        .bind(id).fetch_optional(pool).await.map_err(|e|e.to_string())
}
pub async fn list_connections(pool: &SqlitePool,board_id: &str)
    -> Result<Vec<BoardConnection>,sqlx::Error>
{
    sqlx::query_as("SELECT c.id,c.board_id,c.from_id,c.to_id,c.created_at
        FROM whiteboard_connections c
        JOIN whiteboard_items a ON a.id=c.from_id
        JOIN whiteboard_items b ON b.id=c.to_id
        WHERE c.board_id=? AND a.board_id=c.board_id AND b.board_id=c.board_id
          AND a.deleted_at IS NULL AND b.deleted_at IS NULL ORDER BY c.created_at,c.id")
        .bind(board_id).fetch_all(pool).await
}
pub async fn connect_items(pool: &SqlitePool,board_id: &str,from_id: &str,to_id: &str)
    -> Result<BoardConnection,String>
{
    if from_id==to_id {return Err("Não é possível conectar um cartão a ele mesmo".into());}
    let mut tx=pool.begin().await.map_err(|e|e.to_string())?;
    let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM whiteboard_items
        WHERE board_id=? AND deleted_at IS NULL AND id IN (?,?)")
        .bind(board_id).bind(from_id).bind(to_id)
        .fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
    if count!=2 {return Err("Conecte apenas cartões ativos do mesmo quadro".into());}
    let id=Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO whiteboard_connections
        (id,board_id,from_id,to_id,created_at) VALUES(?,?,?,?,?)
        ON CONFLICT(board_id,from_id,to_id) DO NOTHING")
        .bind(&id).bind(board_id).bind(from_id).bind(to_id).bind(crate::db::now_ms())
        .execute(&mut *tx).await.map_err(|e|e.to_string())?;
    let link=sqlx::query_as("SELECT id,board_id,from_id,to_id,created_at
        FROM whiteboard_connections WHERE board_id=? AND from_id=? AND to_id=?")
        .bind(board_id).bind(from_id).bind(to_id)
        .fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
    tx.commit().await.map_err(|e|e.to_string())?;
    Ok(link)
}
pub async fn disconnect_items(pool: &SqlitePool,id: &str,board_id: &str)
    -> Result<bool,sqlx::Error>
{
    let result=sqlx::query("DELETE FROM whiteboard_connections WHERE id=? AND board_id=?")
        .bind(id).bind(board_id).execute(pool).await?;
    Ok(result.rows_affected()>0)
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
pub async fn list(pool: &SqlitePool,board_id: &str) -> Result<Vec<BoardItem>,sqlx::Error> {
    sqlx::query_as("SELECT id,kind,x,y,width,height,content,revision,created_at,updated_at,board_id
        FROM whiteboard_items WHERE board_id=? AND deleted_at IS NULL ORDER BY created_at,id")
        .bind(board_id).fetch_all(pool).await
}
pub(crate) async fn get(pool: &SqlitePool, id: &str) -> Result<Option<BoardItem>, sqlx::Error> {
    sqlx::query_as("SELECT id,kind,x,y,width,height,content,revision,created_at,updated_at,board_id
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
    pool: &SqlitePool,board_id: &str,kind: &str,x: f64,y: f64,content: &str,
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
    let board_count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM whiteboards WHERE id=?")
        .bind(board_id).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
    if board_count!=1 {return Err("Quadro não encontrado".into());}
    sqlx::query("INSERT INTO whiteboard_items
        (id,kind,x,y,width,height,content,revision,created_at,updated_at,board_id)
        VALUES(?,?,?,?,?,?,?,0,?,?,?)")
        .bind(&id).bind(kind).bind(x).bind(y).bind(w).bind(h)
        .bind(content).bind(now).bind(now).bind(board_id)
        .execute(&mut *tx).await.map_err(|e|e.to_string())?;
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
    async fn legacy_recheck_keeps_existing_boards_and_connections() {
        let folder=std::env::temp_dir().join(format!("recall-v9-repeat-{}",Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        let path=folder.join("memoria.sqlite");
        let pool=crate::db::connect(&path).await.unwrap();
        let custom=create_board(&pool,"Estudos").await.unwrap();
        let a=create(&pool,&custom.id,"note",0.0,0.0,"A",None).await.unwrap();
        let b=create(&pool,&custom.id,"note",400.0,0.0,"B",None).await.unwrap();
        let edge=connect_items(&pool,&custom.id,&a.id,&b.id).await.unwrap();
        // Mirrors older test fixtures with downgraded version and current tables.
        sqlx::query("PRAGMA user_version=8").execute(&pool).await.unwrap();
        pool.close().await;
        let upgraded=crate::db::connect(&path).await.unwrap();
        assert_eq!(list(&upgraded,&custom.id).await.unwrap().len(),2);
        assert_eq!(list_connections(&upgraded,&custom.id).await.unwrap()[0].id,edge.id);
        assert_eq!(list_boards(&upgraded).await.unwrap().len(),2);
        upgraded.close().await;
        let _=std::fs::remove_dir_all(folder);
    }

    #[tokio::test]
    async fn upgrading_from_v8_keeps_cards_images_and_creates_one_backup() {
        let folder=std::env::temp_dir().join(format!("recall-board-upgrade-{}",Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        let path=folder.join("memoria.sqlite");
        let pool=crate::db::connect(&path).await.unwrap();
        let original=create(&pool,"default","image",10.0,20.0,"original",
            Some("data:image/png;base64,AAAA")).await.unwrap();
        // Reconstruct the exact v8 whiteboard schema while retaining the rest
        // of the already-initialized production database.
        sqlx::query("DROP TABLE whiteboard_connections").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE whiteboards").execute(&pool).await.unwrap();
        sqlx::query("DROP INDEX idx_whiteboard_by_board").execute(&pool).await.unwrap();
        sqlx::query("ALTER TABLE whiteboard_items DROP COLUMN board_id").execute(&pool).await.unwrap();
        sqlx::query("PRAGMA user_version=8").execute(&pool).await.unwrap();
        pool.close().await;
        let migrated=crate::db::connect(&path).await.unwrap();
        let current=get(&migrated,&original.id).await.unwrap().unwrap();
        assert_eq!(current.id,original.id);
        assert_eq!(current.revision,original.revision);
        assert_eq!(current.board_id,"default");
        assert_eq!(asset(&migrated,&current.id).await.unwrap(),
            Some("data:image/png;base64,AAAA".into()));
        let backups=crate::db::list_backups(&path).unwrap();
        assert!(backups.iter().any(|b|b.file_name.contains("pre-alpha10-1-boards")));
        let backup_count=backups.len();
        migrated.close().await;
        let reopened=crate::db::connect(&path).await.unwrap();
        assert_eq!(crate::db::list_backups(&path).unwrap().len(),backup_count);
        reopened.close().await;
        let _=std::fs::remove_dir_all(folder);
    }
    #[tokio::test]
    async fn boards_connections_are_durable_isolated_and_reject_invalid_edges() {
        let folder=std::env::temp_dir().join(format!("recall-boards-v9-{}",Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        let path=folder.join("memoria.sqlite");
        let pool=crate::db::connect(&path).await.unwrap();
        let a=create(&pool,"default","note",0.0,0.0,"A",None).await.unwrap();
        let b=create(&pool,"default","note",350.0,0.0,"B",None).await.unwrap();
        let created=create_board(&pool,"Projeto X").await.unwrap();
        let foreign=create(&pool,&created.id,"note",0.0,0.0,"Fora",None).await.unwrap();
        assert!(connect_items(&pool,"default",&a.id,&a.id).await.is_err());
        assert!(connect_items(&pool,"default",&a.id,&foreign.id).await.is_err());
        let first=connect_items(&pool,"default",&a.id,&b.id).await.unwrap();
        let duplicate=connect_items(&pool,"default",&a.id,&b.id).await.unwrap();
        assert_eq!(first.id,duplicate.id);
        assert_eq!(list_connections(&pool,"default").await.unwrap().len(),1);
        assert_eq!(list_connections(&pool,&created.id).await.unwrap().len(),0);
        assert_eq!(list(&pool,"default").await.unwrap().len(),2);
        assert_eq!(list(&pool,&created.id).await.unwrap().len(),1);
        assert_eq!(rename_board(&pool,&created.id,"Renomeado").await.unwrap().unwrap().name,"Renomeado");
        pool.close().await;
        let again=crate::db::connect(&path).await.unwrap();
        assert_eq!(list_boards(&again).await.unwrap().len(),2);
        assert_eq!(list_connections(&again,"default").await.unwrap().len(),1);
        assert!(delete(&again,&b.id,0).await.unwrap());
        assert!(list_connections(&again,"default").await.unwrap().is_empty());
        assert!(!disconnect_items(&again,&first.id,&created.id).await.unwrap());
        assert!(disconnect_items(&again,&first.id,"default").await.unwrap());
        again.close().await;
        let _=std::fs::remove_dir_all(folder);
    }

    #[tokio::test]
    async fn whiteboard_persists_rejects_stale_edits_and_keeps_images_in_backup() {
        let folder=std::env::temp_dir().join(format!("recall-whiteboard-{}",Uuid::new_v4()));
        fs::create_dir_all(&folder).unwrap();
        let path=folder.join("memoria.sqlite");
        let pool=db::connect(&path).await.unwrap();
        let note=create(&pool,"default","note",10.0,-5.0,"Minha ideia",None).await.unwrap();
        let saved=update(&pool,&note.id,20.0,30.0,300.0,200.0,"Editada",0)
            .await.unwrap().unwrap();
        assert_eq!(saved.revision,1);
        assert!(update(&pool,&note.id,0.0,0.0,300.0,200.0,"Stale",0)
            .await.unwrap().is_none());
        assert!(update(&pool,&note.id,f64::NAN,0.0,300.0,200.0,"Bad",1).await.is_err());
        assert!(create(&pool,"default","image",0.0,0.0,"svg",Some("data:image/svg+xml;base64,PHN2Zz4="))
            .await.is_err());
        let img=create(&pool,"default","image",0.0,0.0,"foto",Some("data:image/png;base64,AAAA"))
            .await.unwrap();
        let backup=db::create_backup(&pool,&path,"test-board").await.unwrap();
        assert!(std::path::Path::new(&backup.file_path).is_file());
        pool.close().await;
        let reopened=db::connect(&path).await.unwrap();
        assert_eq!(list(&reopened,"default").await.unwrap().len(),2);
        assert_eq!(asset(&reopened,&img.id).await.unwrap().as_deref(),Some("data:image/png;base64,AAAA"));
        assert!(!delete(&reopened,&note.id,0).await.unwrap());
        assert!(delete(&reopened,&note.id,1).await.unwrap());
        assert_eq!(list(&reopened,"default").await.unwrap().len(),1);
        reopened.close().await;
        let _=fs::remove_dir_all(folder);
    }
}
