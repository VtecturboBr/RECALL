//! Alpha 9.0: local focus sessions. The database is authoritative for elapsed
//! time. A checkpoint caps elapsed wall-clock time to avoid counting hours after
//! crashes, sleep or WebView suspension. No internet or background timer service.
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;
use crate::db::now_ms;

const MAX_TICK_MS: i64 = 20_000;
const ROW: &str = "id,task_id,started_at,last_tick_at,active_ms,ended_at,state,summary";

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct FocusSession {
    pub id: String,
    pub task_id: String,
    pub started_at: i64,
    pub last_tick_at: i64,
    pub active_ms: i64,
    pub ended_at: Option<i64>,
    pub state: String,
    pub summary: String,
}

/// Migration v5→v6. Call only after verified backup and successful history v5.
pub async fn migrate_v6(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("CREATE TABLE focus_sessions (
        id TEXT PRIMARY KEY NOT NULL,
        task_id TEXT NOT NULL REFERENCES tasks(id),
        started_at INTEGER NOT NULL,
        last_tick_at INTEGER NOT NULL,
        active_ms INTEGER NOT NULL DEFAULT 0 CHECK(active_ms>=0),
        ended_at INTEGER,
        state TEXT NOT NULL DEFAULT 'active'
            CHECK(state IN ('active','paused','finished','interrupted')),
        summary TEXT NOT NULL DEFAULT '',
        CHECK((state='active' AND ended_at IS NULL)
           OR (state!='active' AND ended_at IS NOT NULL))
    )").execute(&mut *tx).await?;
    sqlx::query("CREATE UNIQUE INDEX focus_at_most_one_active
        ON focus_sessions(state) WHERE state='active'").execute(&mut *tx).await?;
    sqlx::query("CREATE INDEX focus_by_task ON focus_sessions(task_id,started_at DESC)")
        .execute(&mut *tx).await?;
    sqlx::query("PRAGMA user_version=6").execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn verify_v6(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let schema: (i64,i64) = sqlx::query_as("SELECT
       (SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='focus_sessions'),
       (SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name IN
           ('focus_at_most_one_active','focus_by_task'))").fetch_one(pool).await?;
    if schema != (1,2) {
        return Err(sqlx::Error::Protocol("Alpha 9: esquema de foco ausente ou incompleto".into()));
    }
    sqlx::query(&format!("SELECT {ROW} FROM focus_sessions LIMIT 0")).fetch_all(pool).await?;
    Ok(())
}

pub async fn active(pool: &SqlitePool) -> Result<Option<FocusSession>, sqlx::Error> {
    sqlx::query_as::<_,FocusSession>(&format!("SELECT {ROW} FROM focus_sessions WHERE state='active' LIMIT 1"))
        .fetch_optional(pool).await
}

pub async fn history(pool: &SqlitePool, task_id: &str) -> Result<Vec<FocusSession>, sqlx::Error> {
    sqlx::query_as::<_,FocusSession>(&format!("SELECT {ROW} FROM focus_sessions WHERE task_id=?
       ORDER BY started_at DESC,id DESC LIMIT 8"))
        .bind(task_id).fetch_all(pool).await
}

/// Only an existing, uncompleted, nondeleted task may receive a focus session.
/// Unique partial index prevents two windows from starting separate active sessions.
pub async fn start(pool: &SqlitePool, task_id: &str) -> Result<FocusSession, sqlx::Error> {
    if let Some(session) = active(pool).await? {
        if session.task_id == task_id { return Ok(session); }
        return Err(sqlx::Error::Protocol(
            "FOCUS_BUSY: pause a sessão de outra tarefa antes de começar esta".into()));
    }
    let id=Uuid::new_v4().to_string();
    let now=now_ms();
    let result = sqlx::query("INSERT INTO focus_sessions(id,task_id,started_at,last_tick_at,active_ms,state)
        SELECT ?,id,?,?,0,'active' FROM tasks
        WHERE id=? AND deleted_at IS NULL AND completed_at IS NULL")
        .bind(&id).bind(now).bind(now).bind(task_id).execute(pool).await?;
    if result.rows_affected()==0 { return Err(sqlx::Error::RowNotFound); }
    sqlx::query_as::<_,FocusSession>(&format!("SELECT {ROW} FROM focus_sessions WHERE id=?"))
        .bind(&id).fetch_one(pool).await
}

/// Heartbeats record at most 20s; a PC sleeping for hours cannot inflate time.
pub async fn heartbeat(pool: &SqlitePool, id: &str) -> Result<FocusSession, sqlx::Error> {
    let now=now_ms();
    let result=sqlx::query("UPDATE focus_sessions SET
        active_ms=active_ms+MIN(MAX(0,?-last_tick_at),?),
        last_tick_at=?
        WHERE id=? AND state='active'
          AND EXISTS (SELECT 1 FROM tasks WHERE tasks.id=focus_sessions.task_id
            AND deleted_at IS NULL AND completed_at IS NULL)")
        .bind(now).bind(MAX_TICK_MS).bind(now).bind(id).execute(pool).await?;
    if result.rows_affected()==0 { return Err(sqlx::Error::RowNotFound); }
    sqlx::query_as::<_,FocusSession>(&format!("SELECT {ROW} FROM focus_sessions WHERE id=?"))
        .bind(id).fetch_one(pool).await
}

pub async fn stop(pool: &SqlitePool, id: &str, state: &str, summary: &str) -> Result<FocusSession, sqlx::Error> {
    if !matches!(state,"paused"|"finished") || summary.chars().count()>2_000 {
        return Err(sqlx::Error::Protocol("Parada de foco inválida".into()));
    }
    let now=now_ms();
    let result=sqlx::query("UPDATE focus_sessions SET
        active_ms=active_ms+MIN(MAX(0,?-last_tick_at),?),
        last_tick_at=?, ended_at=?, state=?, summary=?
        WHERE id=? AND state='active'")
        .bind(now).bind(MAX_TICK_MS).bind(now).bind(now).bind(state).bind(summary)
        .bind(id).execute(pool).await?;
    if result.rows_affected()==0 {
        return Err(sqlx::Error::Protocol("Sessão já encerrada ou modificada".into()));
    }
    sqlx::query_as::<_,FocusSession>(&format!("SELECT {ROW} FROM focus_sessions WHERE id=?"))
        .bind(id).fetch_one(pool).await
}

/// Called inside task update transactions so a deleted or completed task can
/// never keep a zombie active focus session. No user summary is destroyed.
pub async fn finish_task_in_tx(
    tx: &mut sqlx::Transaction<'_,sqlx::Sqlite>, task_id: &str,
) -> Result<(),sqlx::Error> {
    let now=now_ms();
    sqlx::query("UPDATE focus_sessions SET
       active_ms=active_ms+MIN(MAX(0,?-last_tick_at),?),
       last_tick_at=?,ended_at=?,state='interrupted'
       WHERE task_id=? AND state='active'")
        .bind(now).bind(MAX_TICK_MS).bind(now).bind(now).bind(task_id)
        .execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db,tasks};
    fn fixture()->(std::path::PathBuf,std::path::PathBuf){
        let dir=std::env::temp_dir().join(format!("recall-focus9-{}",Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        (dir.clone(),dir.join("memoria.sqlite"))
    }

    #[tokio::test]
    async fn focus_lifecycle_and_restart_persist_summary() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Revisar apresentação").await.unwrap();
        let start=super::start(&pool,&task.id).await.unwrap();
        assert_eq!(super::start(&pool,&task.id).await.unwrap().id,start.id,"idempotent start");
        let tick=heartbeat(&pool,&start.id).await.unwrap();
        assert!(tick.active_ms>=0);
        let done=stop(&pool,&start.id,"paused","Já revi os dois primeiros cartões").await.unwrap();
        assert_eq!(done.state,"paused");
        assert_eq!(done.summary,"Já revi os dois primeiros cartões");
        assert!(active(&pool).await.unwrap().is_none());
        let resumed=super::start(&pool,&task.id).await.unwrap();
        assert_ne!(resumed.id,start.id);
        let completed=stop(&pool,&resumed.id,"finished","Apresentação pronta").await.unwrap();
        assert_eq!(completed.state,"finished");
        pool.close().await;
        let reopened=db::connect(&path).await.unwrap();
        let rows=history(&reopened,&task.id).await.unwrap();
        assert_eq!(rows.len(),2);
        assert_eq!(rows[0].summary,"Apresentação pronta");
        assert_eq!(rows[1].summary,"Já revi os dois primeiros cartões");
        reopened.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn only_one_active_session_and_no_deleted_completed_tasks() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let a=tasks::create(&pool,"A").await.unwrap();
        let b=tasks::create(&pool,"B").await.unwrap();
        let first=super::start(&pool,&a.id).await.unwrap();
        assert!(super::start(&pool,&b.id).await.is_err());
        assert!(stop(&pool,&first.id,"finished","").await.is_ok());
        tasks::set_completed(&pool,&a.id,true).await.unwrap();
        assert!(super::start(&pool,&a.id).await.is_err());
        let next=super::start(&pool,&b.id).await.unwrap();
        tasks::delete(&pool,&b.id).await.unwrap();
        assert!(active(&pool).await.unwrap().is_none());
        assert_eq!(history(&pool,&b.id).await.unwrap()[0].state,"interrupted");
        assert!(heartbeat(&pool,&next.id).await.is_err());
        pool.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn heartbeat_caps_sleep_time_and_stop_validates() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Trabalhar").await.unwrap();
        let start=super::start(&pool,&task.id).await.unwrap();
        sqlx::query("UPDATE focus_sessions SET last_tick_at=? WHERE id=?")
            .bind(now_ms()-3_600_000).bind(&start.id).execute(&pool).await.unwrap();
        let tick=heartbeat(&pool,&start.id).await.unwrap();
        assert!(tick.active_ms<=MAX_TICK_MS);
        assert!(tick.active_ms>0);
        assert!(stop(&pool,&start.id,"paused",&"x".repeat(2001)).await.is_err());
        assert!(stop(&pool,&start.id,"invalid","").await.is_err());
        stop(&pool,&start.id,"paused","").await.unwrap();
        assert!(stop(&pool,&start.id,"paused","").await.is_err());
        pool.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn migration_from_v5_keeps_original_tasks_and_verified_backup() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Dados antigos").await.unwrap();
        pool.close().await;
        let raw=sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(&path)).await.unwrap();
        sqlx::query("DROP TABLE focus_sessions").execute(&raw).await.unwrap();
        sqlx::query("PRAGMA user_version=5").execute(&raw).await.unwrap();
        raw.close().await;
        let upgraded=db::connect(&path).await.unwrap();
        assert_eq!(tasks::get(&upgraded,&task.id).await.unwrap().unwrap().title,"Dados antigos");
        let version:i64=sqlx::query_scalar("PRAGMA user_version").fetch_one(&upgraded).await.unwrap();
        assert_eq!(version,6);
        assert!(db::list_backups(&path).unwrap().iter().any(|x|x.file_name.contains("pre-alpha9")));
        upgraded.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn broken_migration_rolls_back_and_original_remains_intact() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Não perder").await.unwrap();
        pool.close().await;
        let raw=sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(&path)).await.unwrap();
        sqlx::query("DROP TABLE focus_sessions").execute(&raw).await.unwrap();
        sqlx::query("PRAGMA user_version=5").execute(&raw).await.unwrap();
        sqlx::query("CREATE TABLE focus_sessions(broken INTEGER)").execute(&raw).await.unwrap();
        raw.close().await;
        assert!(db::connect(&path).await.is_err());
        let raw=sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(&path)).await.unwrap();
        let v:i64=sqlx::query_scalar("PRAGMA user_version").fetch_one(&raw).await.unwrap();
        assert_eq!(v,5);
        let title:String=sqlx::query_scalar("SELECT title FROM tasks WHERE id=?")
            .bind(&task.id).fetch_one(&raw).await.unwrap();
        assert_eq!(title,"Não perder");
        raw.close().await;let _=std::fs::remove_dir_all(dir);
    }
}
