use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;
use crate::db::now_ms;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub details: String,
    pub due_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

const FIELDS: &str = "id,title,details,due_at,completed_at,created_at,updated_at,deleted_at";

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("CREATE TABLE IF NOT EXISTS tasks (
        id TEXT PRIMARY KEY NOT NULL,
        title TEXT NOT NULL,
        details TEXT NOT NULL DEFAULT '',
        due_at INTEGER,
        completed_at INTEGER,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        deleted_at INTEGER
    )").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_open ON tasks (completed_at, due_at, updated_at) WHERE deleted_at IS NULL")
        .execute(pool).await?;
    Ok(())
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(&format!("SELECT {FIELDS} FROM tasks WHERE deleted_at IS NULL
        ORDER BY (completed_at IS NOT NULL), (due_at IS NULL), due_at, updated_at DESC"))
        .fetch_all(pool).await
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(&format!("SELECT {FIELDS} FROM tasks WHERE id=? AND deleted_at IS NULL"))
        .bind(id).fetch_optional(pool).await
}

pub async fn create(pool: &SqlitePool, title: &str) -> Result<Task, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query("INSERT INTO tasks (id,title,created_at,updated_at) VALUES (?,?,?,?)")
        .bind(&id).bind(title.trim()).bind(now).bind(now).execute(pool).await?;
    get(pool, &id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn update(pool: &SqlitePool, id: &str, title: &str, details: &str, due_at: Option<i64>) -> Result<Task, sqlx::Error> {
    let updated = sqlx::query("UPDATE tasks SET title=?,details=?,due_at=?,updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(title.trim()).bind(details).bind(due_at).bind(now_ms()).bind(id).execute(pool).await?;
    if updated.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn set_completed(pool: &SqlitePool, id: &str, completed: bool) -> Result<Task, sqlx::Error> {
    let at = if completed { Some(now_ms()) } else { None };
    let updated = sqlx::query("UPDATE tasks SET completed_at=?,updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(at).bind(now_ms()).bind(id).execute(pool).await?;
    if updated.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    let now = now_ms();
    let updated = sqlx::query("UPDATE tasks SET deleted_at=?,updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(now).bind(now).bind(id).execute(pool).await?;
    if updated.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    Ok(())
}
