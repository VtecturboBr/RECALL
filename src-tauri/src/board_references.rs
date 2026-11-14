use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::whiteboard::BoardItem;

#[derive(Clone, Debug, Serialize, FromRow)]
pub struct ReferenceTarget {
    pub target_type: String,
    pub target_id: String,
    pub title: String,
    pub preview: String,
}

#[derive(Clone, Debug, Serialize, FromRow)]
pub struct BoardReference {
    pub item_id: String,
    pub target_type: String,
    pub target_id: String,
    pub title: String,
    pub preview: String,
    pub available: bool,
    pub completed: bool,
}

pub async fn migrate_v10(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx=pool.begin().await?;
    // Keep v8 card CHECK unchanged; linked cards use kind='note' with an
    // immutable reference in a dedicated table instead of duplicating content.
    sqlx::query("CREATE TABLE IF NOT EXISTS whiteboard_references (
        item_id TEXT PRIMARY KEY NOT NULL REFERENCES whiteboard_items(id),
        target_type TEXT NOT NULL CHECK(target_type IN ('note','task')),
        target_id TEXT NOT NULL,
        created_at INTEGER NOT NULL
    )").execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_whiteboard_references_target
        ON whiteboard_references(target_type,target_id)")
        .execute(&mut *tx).await?;
    sqlx::query("PRAGMA user_version=10").execute(&mut *tx).await?;
    tx.commit().await
}

pub async fn verify_v10(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master
        WHERE type='table' AND name='whiteboard_references'")
        .fetch_one(pool).await?;
    if count!=1 {
        return Err(sqlx::Error::Protocol("RECALL Quadro: tabela v10 ausente".into()));
    }
    sqlx::query("SELECT item_id,target_type,target_id,created_at
        FROM whiteboard_references LIMIT 0").fetch_all(pool).await?;
    Ok(())
}

pub async fn search_targets(pool: &SqlitePool, query: &str) -> Result<Vec<ReferenceTarget>,String> {
    let query=query.trim();
    if query.chars().count()>100 {
        return Err("Busca limitada a 100 caracteres".into());
    }
    // No text is copied to a board. This bounded query searches live entities.
    sqlx::query_as("SELECT target_type,target_id,title,preview FROM (
        SELECT 'note' AS target_type,id AS target_id,
            COALESCE(NULLIF(TRIM(title),''),'Nota sem título') AS title,
            SUBSTR(body,1,180) AS preview, updated_at AS modified
        FROM notes WHERE deleted_at IS NULL
        UNION ALL
        SELECT 'task' AS target_type,id AS target_id,
            COALESCE(NULLIF(TRIM(title),''),'Nova tarefa') AS title,
            SUBSTR(details,1,180) AS preview, updated_at AS modified
        FROM tasks WHERE deleted_at IS NULL
    ) WHERE title LIKE '%'||?||'%' OR preview LIKE '%'||?||'%'
      ORDER BY modified DESC LIMIT 40")
        .bind(query).bind(query).fetch_all(pool).await.map_err(|e|e.to_string())
}

pub async fn list_references(pool: &SqlitePool,board_id: &str)
    -> Result<Vec<BoardReference>,sqlx::Error>
{
    sqlx::query_as("SELECT r.item_id,r.target_type,r.target_id,
        CASE
          WHEN r.target_type='note' AND n.id IS NOT NULL AND n.deleted_at IS NULL
            THEN COALESCE(NULLIF(TRIM(n.title),''),'Nota sem título')
          WHEN r.target_type='task' AND t.id IS NOT NULL AND t.deleted_at IS NULL
            THEN COALESCE(NULLIF(TRIM(t.title),''),'Nova tarefa')
          WHEN r.target_type='note' THEN 'Nota indisponível'
          ELSE 'Tarefa indisponível' END AS title,
        CASE
          WHEN r.target_type='note' AND n.deleted_at IS NULL
            THEN SUBSTR(COALESCE(n.body,''),1,240)
          WHEN r.target_type='task' AND t.deleted_at IS NULL
            THEN SUBSTR(COALESCE(t.details,''),1,240)
          ELSE '' END AS preview,
        CASE
          WHEN r.target_type='note' AND n.id IS NOT NULL AND n.deleted_at IS NULL THEN 1
          WHEN r.target_type='task' AND t.id IS NOT NULL AND t.deleted_at IS NULL THEN 1
          ELSE 0 END AS available,
        CASE WHEN r.target_type='task' AND t.completed_at IS NOT NULL
          AND t.deleted_at IS NULL THEN 1 ELSE 0 END AS completed
        FROM whiteboard_references r
        JOIN whiteboard_items i ON i.id=r.item_id
        LEFT JOIN notes n ON r.target_type='note' AND n.id=r.target_id
        LEFT JOIN tasks t ON r.target_type='task' AND t.id=r.target_id
        WHERE i.board_id=? AND i.deleted_at IS NULL
        ORDER BY i.created_at,i.id")
        .bind(board_id).fetch_all(pool).await
}

pub async fn create_reference(
    pool: &SqlitePool,board_id: &str,target_type: &str,target_id: &str,
    x: f64,y: f64
) -> Result<BoardItem,String> {
    if !x.is_finite() || !y.is_finite() || x.abs()>1e7 || y.abs()>1e7 {
        return Err("Posição do cartão inválida".into());
    }
    let table=match target_type {
        "note"=>"notes", "task"=>"tasks",
        _=>return Err("Apenas notas e tarefas podem ser vinculadas".into()),
    };
    if target_id.is_empty() || target_id.len()>256 {
        return Err("Identificador de destino inválido".into());
    }
    let mut tx=pool.begin().await.map_err(|e|e.to_string())?;
    let board_count:i64=sqlx::query_scalar("SELECT COUNT(*) FROM whiteboards WHERE id=?")
        .bind(board_id).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
    if board_count!=1 {return Err("Quadro não encontrado".into());}
    // Identifier cannot be SQL-injected: table name is selected only from two
    // compile-time string literals.
    let count:i64=sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM {table} WHERE id=? AND deleted_at IS NULL"))
        .bind(target_id).fetch_one(&mut *tx).await.map_err(|e|e.to_string())?;
    if count!=1 {return Err("Nota ou tarefa indisponível".into());}
    let id=Uuid::new_v4().to_string();
    let now=crate::db::now_ms();
    sqlx::query("INSERT INTO whiteboard_items
        (id,kind,x,y,width,height,content,revision,created_at,updated_at,board_id)
        VALUES(?,'note',?,?,330,210,'',0,?,?,?)")
        .bind(&id).bind(x).bind(y).bind(now).bind(now).bind(board_id)
        .execute(&mut *tx).await.map_err(|e|e.to_string())?;
    sqlx::query("INSERT INTO whiteboard_references
        (item_id,target_type,target_id,created_at) VALUES(?,?,?,?)")
        .bind(&id).bind(target_type).bind(target_id).bind(now)
        .execute(&mut *tx).await.map_err(|e|e.to_string())?;
    tx.commit().await.map_err(|e|e.to_string())?;
    crate::whiteboard::get(pool,&id).await.map_err(|e|e.to_string())?
        .ok_or_else(||"Cartão vinculado não encontrado".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db,tasks};
    use std::fs;

    #[tokio::test]
    async fn references_follow_source_edits_and_soft_delete_without_copying_text() {
        let folder=std::env::temp_dir().join(format!("recall-dynrefs-{}",Uuid::new_v4()));
        fs::create_dir_all(&folder).unwrap();
        let path=folder.join("memoria.sqlite");
        let pool=db::connect(&path).await.unwrap();
        let note=db::create(&pool,"note").await.unwrap();
        let task=tasks::create(&pool,"Primeira tarefa").await.unwrap();
        let board=crate::whiteboard::create_board(&pool,"Visão Geral").await.unwrap();
        let card=create_reference(&pool,&board.id,"note",&note.id,0.0,0.0).await.unwrap();
        let task_card=create_reference(&pool,&board.id,"task",&task.id,400.0,0.0).await.unwrap();
        assert_eq!(card.content,"");
        let before=list_references(&pool,&board.id).await.unwrap();
        assert_eq!(before.len(),2);
        assert!(before.iter().find(|r|r.item_id==card.id).unwrap().available);
        sqlx::query("UPDATE notes SET title='Título novo',body='Editado no Arquivo'
            WHERE id=?").bind(&note.id).execute(&pool).await.unwrap();
        sqlx::query("UPDATE tasks SET title='Tarefa alterada',details='Próximo passo'
            WHERE id=?").bind(&task.id).execute(&pool).await.unwrap();
        let fresh=list_references(&pool,&board.id).await.unwrap();
        assert_eq!(fresh.iter().find(|r|r.item_id==card.id).unwrap().title,"Título novo");
        assert_eq!(fresh.iter().find(|r|r.item_id==card.id).unwrap().preview,"Editado no Arquivo");
        assert_eq!(fresh.iter().find(|r|r.item_id==task_card.id).unwrap().title,"Tarefa alterada");
        let target=search_targets(&pool,"Título novo").await.unwrap();
        assert!(target.iter().any(|r|r.target_id==note.id));
        sqlx::query("UPDATE notes SET deleted_at=12345 WHERE id=?")
            .bind(&note.id).execute(&pool).await.unwrap();
        let missing=list_references(&pool,&board.id).await.unwrap();
        let state=missing.iter().find(|r|r.item_id==card.id).unwrap();
        assert!(!state.available);
        assert!(state.preview.is_empty());
        assert!(create_reference(&pool,&board.id,"note",&note.id,10.0,20.0).await.is_err());
        assert!(create_reference(&pool,&board.id,"idea",&task.id,10.0,20.0).await.is_err());
        assert!(create_reference(&pool,"missing","task",&task.id,10.0,20.0).await.is_err());
        pool.close().await;
        let _=fs::remove_dir_all(folder);
    }

    #[tokio::test]
    async fn references_survive_reopen_and_do_not_create_duplicated_note_content() {
        let folder=std::env::temp_dir().join(format!("recall-ref-reopen-{}",Uuid::new_v4()));
        fs::create_dir_all(&folder).unwrap();
        let path=folder.join("memoria.sqlite");
        let pool=db::connect(&path).await.unwrap();
        let note=db::create(&pool,"note").await.unwrap();
        let card=create_reference(&pool,"default","note",&note.id,0.0,0.0).await.unwrap();
        assert!(sqlx::query_scalar::<_,String>("SELECT content FROM whiteboard_items WHERE id=?")
            .bind(&card.id).fetch_one(&pool).await.unwrap().is_empty());
        pool.close().await;
        let reopened=db::connect(&path).await.unwrap();
        assert_eq!(list_references(&reopened,"default").await.unwrap().len(),1);
        let version:i64=sqlx::query_scalar("PRAGMA user_version").fetch_one(&reopened).await.unwrap();
        assert_eq!(version,10);
        reopened.close().await;
        let _=fs::remove_dir_all(folder);
    }
}
