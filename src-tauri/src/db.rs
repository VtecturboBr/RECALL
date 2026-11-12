use serde::Serialize;
use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteJournalMode}, FromRow, SqlitePool};
use std::{path::Path, time::{SystemTime, UNIX_EPOCH}, time::Duration};
use std::fs;
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
    pub revision: i64,
    pub deleted_at: Option<i64>,
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i64
}

const FIELDS: &str = "n.id,n.title,n.body,n.kind,n.pinned,n.remind_at,n.notified_at,n.created_at,n.updated_at,n.deleted_at,COALESCE(r.revision,0) AS revision";
const JOIN: &str = "FROM notes n LEFT JOIN note_revisions r ON r.note_id=n.id";

#[derive(Debug, Clone, Serialize)]
pub struct BackupInfo {
    pub file_name: String,
    pub created_at: i64,
    pub size_bytes: u64,
}

// SQLite itself creates a consistent WAL-aware snapshot. A .partial file is never
// exposed as a valid backup; a quick_check is required before the final rename.
pub async fn create_backup(pool: &SqlitePool, db_path: &Path, purpose: &str) -> Result<BackupInfo, String> {
    let folder = db_path.parent().ok_or("Caminho do banco inválido")?.join("backups");
    fs::create_dir_all(&folder).map_err(|e| format!("Não foi possível criar a pasta de backup: {e}"))?;
    let created_at = now_ms();
    let file_name = format!("recall-{purpose}-{created_at}-{}.sqlite", Uuid::new_v4());
    let target = folder.join(&file_name);
    let partial = folder.join(format!("{file_name}.partial"));
    let result: Result<(), String> = async {
        sqlx::query("VACUUM INTO ?")
            .bind(partial.to_string_lossy().to_string())
            .execute(pool).await.map_err(|e| format!("Falha no snapshot SQLite: {e}"))?;
        let options = SqliteConnectOptions::new().filename(&partial).read_only(true);
        let check_pool = SqlitePoolOptions::new().max_connections(1).connect_with(options)
            .await.map_err(|e| format!("Não foi possível abrir o backup para validar: {e}"))?;
        let result = sqlx::query_scalar::<_, String>("PRAGMA quick_check")
            .fetch_one(&check_pool).await;
        check_pool.close().await;
        match result {
            Ok(value) if value == "ok" => (),
            Ok(value) => return Err(format!("Backup reprovado no quick_check: {value}")),
            Err(e) => return Err(format!("Backup reprovado na validação: {e}")),
        }
        fs::rename(&partial, &target).map_err(|e| format!("Não foi possível finalizar o backup: {e}"))?;
        Ok(())
    }.await;
    if result.is_err() { let _ = fs::remove_file(&partial); }
    result?;
    let size_bytes = fs::metadata(&target).map_err(|e| e.to_string())?.len();
    Ok(BackupInfo { file_name, created_at, size_bytes })
}

pub fn list_backups(db_path: &Path) -> Result<Vec<BackupInfo>, String> {
    let dir = db_path.parent().ok_or("Caminho do banco inválido")?.join("backups");
    if !dir.exists() { return Ok(Vec::new()); }
    let mut items = Vec::new();
    for e in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let name = e.file_name().to_string_lossy().to_string();
        if !name.starts_with("recall-") || !name.ends_with(".sqlite") { continue; }
        let metadata = e.metadata().map_err(|e| e.to_string())?;
        if !metadata.is_file() { continue; }
        let modified = metadata.modified().ok().and_then(|m| m.duration_since(UNIX_EPOCH).ok())
            .map(|m| m.as_millis() as i64).unwrap_or(0);
        items.push(BackupInfo { file_name: name, created_at: modified, size_bytes: metadata.len() });
    }
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}


pub async fn connect(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
    // On upgrades, the first integrity-schema change is preceded by a verified
    // snapshot. New databases do not need a pre-migration backup.
    let old_notes: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='notes'"
    ).fetch_one(&pool).await?;
    let old_revision: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='note_revisions'"
    ).fetch_one(&pool).await?;
    if old_notes > 0 && old_revision == 0 {
        create_backup(&pool, path, "pre-integrity").await
            .map_err(|e| sqlx::Error::Protocol(format!("Backup antes da migração falhou: {e}")))?;
    }
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
    // The dedicated revision table keeps the existing notes schema intact.
    // Triggers also observe edits made by older app builds writing to notes.
    sqlx::query("CREATE TABLE IF NOT EXISTS note_revisions (
        note_id TEXT PRIMARY KEY NOT NULL,
        revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0)
    )").execute(&pool).await?;
    sqlx::query("INSERT OR IGNORE INTO note_revisions(note_id,revision) SELECT id,0 FROM notes")
        .execute(&pool).await?;
    sqlx::query("CREATE TRIGGER IF NOT EXISTS notes_revision_insert AFTER INSERT ON notes
        BEGIN INSERT OR IGNORE INTO note_revisions(note_id,revision) VALUES(NEW.id,0); END")
        .execute(&pool).await?;
    sqlx::query("CREATE TRIGGER IF NOT EXISTS notes_revision_edit AFTER UPDATE OF title,body ON notes
        BEGIN UPDATE note_revisions SET revision=revision+1 WHERE note_id=NEW.id; END")
        .execute(&pool).await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(&pool).await?;
    if version < 1 { sqlx::query("PRAGMA user_version = 1").execute(&pool).await?; }
    // Safe additive migrations: existing notes from v0.4 are preserved.
    crate::tasks::initialize(&pool).await?;
    crate::inbox::initialize(&pool).await?;
    Ok(pool)
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Note>, sqlx::Error> {
    sqlx::query_as::<_, Note>(&format!("SELECT {FIELDS} {JOIN} WHERE n.deleted_at IS NULL ORDER BY n.updated_at DESC"))
        .fetch_all(pool).await
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Note>, sqlx::Error> {
    sqlx::query_as::<_, Note>(&format!("SELECT {FIELDS} {JOIN} WHERE n.id=? AND n.deleted_at IS NULL"))
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

pub async fn save(pool: &SqlitePool, id: &str, title: &str, body: &str, expected_revision: i64) -> Result<Option<Note>, sqlx::Error> {
    // Atomic compare-and-swap. The trigger increments revision in the same
    // statement. Different WebViews cannot overwrite stale contents.
    let result = sqlx::query("UPDATE notes SET title=?, body=?, updated_at=?
        WHERE id=? AND deleted_at IS NULL
        AND EXISTS (SELECT 1 FROM note_revisions WHERE note_id=notes.id AND revision=?)")
        .bind(title).bind(body).bind(now_ms()).bind(id).bind(expected_revision)
        .execute(pool).await?;
    if result.rows_affected() == 0 { return Ok(None); }
    get(pool, id).await
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
    sqlx::query_as::<_, Note>(&format!("SELECT {FIELDS} {JOIN} WHERE n.deleted_at IS NULL AND n.remind_at IS NOT NULL AND n.remind_at<=? AND n.notified_at IS NULL ORDER BY n.remind_at LIMIT 30"))
        .bind(now_ms()).fetch_all(pool).await
}

pub async fn mark_notified(pool: &SqlitePool, id: &str, remind_at: i64) -> Result<(), sqlx::Error> {
    // Compare the timestamp: a changed reminder cannot be accidentally dismissed by an old worker.
    sqlx::query("UPDATE notes SET notified_at=? WHERE id=? AND remind_at=? AND notified_at IS NULL AND deleted_at IS NULL")
        .bind(now_ms()).bind(id).bind(remind_at).execute(pool).await?;
    Ok(())
}
