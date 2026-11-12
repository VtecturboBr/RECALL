use serde::Serialize;
use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteJournalMode}, FromRow, SqlitePool};
use std::{path::Path, time::{SystemTime, UNIX_EPOCH}, time::Duration};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub body: String,
    pub kind: String,
    pub pinned: bool,
    pub remind_at: Option<i64>,
    pub notified_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}

const FIELDS: &str = "id,title,body,kind,pinned,remind_at,notified_at,created_at,updated_at,deleted_at";

pub async fn connect(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS notes (
        id TEXT PRIMARY KEY NOT NULL,
        title TEXT NOT NULL DEFAULT '',
        body TEXT NOT NULL DEFAULT '',
        kind TEXT NOT NULL DEFAULT 'note' CHECK(kind IN ('note', 'presentation')),
        pinned INTEGER NOT NULL DEFAULT 0,
        remind_at INTEGER,
        notified_at INTEGER,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        deleted_at INTEGER
    )").execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_active_notes ON notes (deleted_at, updated_at DESC)").execute(&pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_due_notes ON notes (remind_at) WHERE deleted_at IS NULL AND notified_at IS NULL").execute(&pool).await?;
    Ok(pool)
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Note>, sqlx::Error> {
    sqlx::query_as::<_, Note>(&format!("SELECT {FIELDS} FROM notes WHERE deleted_at IS NULL ORDER BY updated_at DESC"))
        .fetch_all(pool).await
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Note>, sqlx::Error> {
    sqlx::query_as::<_, Note>(&format!("SELECT {FIELDS} FROM notes WHERE id=? AND deleted_at IS NULL"))
        .bind(id).fetch_optional(pool).await
}

pub async fn create(pool: &SqlitePool, kind: &str) -> Result<Note, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query("INSERT INTO notes (id, title, body, kind, created_at, updated_at) VALUES (?, '', '', ?, ?, ?)")
        .bind(&id).bind(kind).bind(now).bind(now).execute(pool).await?;
    get(pool, &id).await?.ok_or(sqlx::Error::RowNotFound)
}

// A quick capture is persisted in one INSERT, preventing a blank note if the app closes.
// The first line becomes the title, while remaining lines become the body.
pub async fn capture(pool: &SqlitePool, text: &str) -> Result<Note, sqlx::Error> {
    let cleaned = text.trim();
    let (first_line, remaining) = cleaned.split_once('\n').unwrap_or((cleaned, ""));
    let first_line = first_line.trim_end_matches('\r').trim();
    let title: String = first_line.chars().take(90).collect();
    // Preserve a long single line in the body rather than silently truncating it.
    let body = if first_line.chars().count() > 90 {
        cleaned
    } else {
        remaining.trim()
    };
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query("INSERT INTO notes (id, title, body, kind, created_at, updated_at) VALUES (?, ?, ?, 'note', ?, ?)")
        .bind(&id).bind(title).bind(body).bind(now).bind(now).execute(pool).await?;
    get(pool, &id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn save(pool: &SqlitePool, id: &str, title: &str, body: &str) -> Result<Note, sqlx::Error> {
    let result = sqlx::query("UPDATE notes SET title=?, body=?, updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(title).bind(body).bind(now_ms()).bind(id).execute(pool).await?;
    if result.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn set_pin(pool: &SqlitePool, id: &str, pinned: bool) -> Result<Note, sqlx::Error> {
    let result = sqlx::query("UPDATE notes SET pinned=?, updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(pinned).bind(now_ms()).bind(id).execute(pool).await?;
    if result.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn set_reminder(pool: &SqlitePool, id: &str, remind_at: Option<i64>) -> Result<Note, sqlx::Error> {
    let result = sqlx::query("UPDATE notes SET remind_at=?, notified_at=NULL, updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(remind_at).bind(now_ms()).bind(id).execute(pool).await?;
    if result.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE notes SET deleted_at=?, updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(now_ms()).bind(now_ms()).bind(id).execute(pool).await?;
    Ok(())
}

pub async fn due(pool: &SqlitePool) -> Result<Vec<Note>, sqlx::Error> {
    sqlx::query_as::<_, Note>(&format!("SELECT {FIELDS} FROM notes WHERE deleted_at IS NULL AND remind_at IS NOT NULL AND remind_at<=? AND notified_at IS NULL ORDER BY remind_at LIMIT 30"))
        .bind(now_ms()).fetch_all(pool).await
}

pub async fn mark_notified(pool: &SqlitePool, id: &str, remind_at: i64) -> Result<(), sqlx::Error> {
    // Compare the timestamp: a changed reminder cannot be accidentally dismissed by an old worker.
    sqlx::query("UPDATE notes SET notified_at=? WHERE id=? AND remind_at=? AND notified_at IS NULL AND deleted_at IS NULL")
        .bind(now_ms()).bind(id).bind(remind_at).execute(pool).await?;
    Ok(())
}
