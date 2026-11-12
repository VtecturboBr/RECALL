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
    Ok(())
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<InboxItem>, sqlx::Error> {
    sqlx::query_as::<_, InboxItem>("SELECT id,text,created_at,updated_at,processed_at FROM inbox_items
        WHERE processed_at IS NULL ORDER BY created_at DESC")
        .fetch_all(pool).await
}

pub async fn capture(pool: &SqlitePool, text: &str) -> Result<InboxItem, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query("INSERT INTO inbox_items(id,text,created_at,updated_at) VALUES (?,?,?,?)")
        .bind(&id).bind(text.trim()).bind(now).bind(now).execute(pool).await?;
    sqlx::query_as::<_, InboxItem>("SELECT id,text,created_at,updated_at,processed_at FROM inbox_items WHERE id=?")
        .bind(id).fetch_one(pool).await
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
    let item = sqlx::query_as::<_, InboxItem>("SELECT id,text,created_at,updated_at,processed_at FROM inbox_items
        WHERE id=? AND processed_at IS NULL")
        .bind(id).fetch_optional(&mut *transaction).await?.ok_or(sqlx::Error::RowNotFound)?;
    let (title, body) = split_capture(&item.text);
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
    let item = sqlx::query_as::<_, InboxItem>("SELECT id,text,created_at,updated_at,processed_at FROM inbox_items
        WHERE id=? AND processed_at IS NULL")
        .bind(id).fetch_optional(&mut *transaction).await?.ok_or(sqlx::Error::RowNotFound)?;
    let (title, details) = split_capture(&item.text);
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
