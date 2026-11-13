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
    pub file_path: String,
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
    Ok(BackupInfo { file_name, created_at, size_bytes, file_path: target.to_string_lossy().to_string() })
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
        items.push(BackupInfo { file_name: name, created_at: modified, size_bytes: metadata.len(), file_path: e.path().to_string_lossy().to_string() });
    }
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}


pub async fn connect(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    // An interrupted offline restore can strand the old DB as
    // .recall-original-*.sqlite. Never silently create a blank database over it.
    if !path.exists() {
        if let Some(folder) = path.parent() {
            for entry in fs::read_dir(folder)
                .map_err(|e| sqlx::Error::Protocol(format!("Falha ao verificar arquivos de restauração: {e}")))? {
                let entry = entry.map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(".recall-original-") && name.ends_with(".sqlite") {
                    return Err(sqlx::Error::Protocol(
                        "Restauração interrompida encontrada. O banco antigo foi preservado; execute o recuperador offline antes de iniciar o RECALL.".into()
                    ));
                }
            }
        }
    }
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(options).await?;
    let starting_version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(&pool).await?;
    if starting_version > 7 {
        return Err(sqlx::Error::Protocol(
            "Banco de dados criado por versão mais recente do RECALL; atualização necessária".into()
        ));
    }
    // The verified pre-v5 backup must happen BEFORE any legacy migrations or
    // v5 schema changes. A new empty database needs no pre-migration snapshot.
    let existing_user_tables: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table'
            AND name IN ('notes','tasks','inbox_items')"
    ).fetch_one(&pool).await?;
    if starting_version < 5 && existing_user_tables > 0 {
        create_backup(&pool, path, "pre-alpha8").await.map_err(|e|
            sqlx::Error::Protocol(format!("Backup obrigatório pré-Alpha 8 falhou: {e}"))
        )?;
    }
    if starting_version < 6 && existing_user_tables > 0 {
        create_backup(&pool, path, "pre-alpha9").await.map_err(|e|
            sqlx::Error::Protocol(format!("Backup obrigatório pré-Alpha 9 falhou: {e}"))
        )?;
    }
    if starting_version < 7 && existing_user_tables > 0 {
        create_backup(&pool, path, "pre-alpha9-links").await.map_err(|e|
            sqlx::Error::Protocol(format!("Backup obrigatório pré-Alpha 9.1 falhou: {e}"))
        )?;
    }
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
    } else if old_notes > 0 {
        let has_reminders: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='reminder_rules'"
        ).fetch_one(&pool).await?;
        if has_reminders == 0 {
            create_backup(&pool, path, "pre-reminders").await
                .map_err(|e| sqlx::Error::Protocol(format!("Backup pré-lembretes falhou: {e}")))?;
        }
    }
    // Back up existing user content before adding task/idea revision tables.
    let needs_content_revisions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('tasks','inbox_items')"
    ).fetch_one(&pool).await?;
    let present_content_revisions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('task_revisions','inbox_revisions')"
    ).fetch_one(&pool).await?;
    if needs_content_revisions > 0 && present_content_revisions < 2 {
        create_backup(&pool, path, "pre-content-revisions").await
            .map_err(|e| sqlx::Error::Protocol(format!("Backup pré-revisões de tarefas e ideias falhou: {e}")))?;
    }
    // A migration from an existing tasks database must have a verified recovery point.
    let old_task_table: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tasks'"
    ).fetch_one(&pool).await?;
    let old_resume_table: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='task_resumes'"
    ).fetch_one(&pool).await?;
    if old_task_table > 0 && old_resume_table == 0 {
        create_backup(&pool, path, "pre-resume").await
            .map_err(|e| sqlx::Error::Protocol(format!("Backup pré-retomada falhou: {e}")))?;
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
    crate::reminders::initialize(&pool).await?;
    let version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(&pool).await?;
    if version < 4 { sqlx::query("PRAGMA user_version = 4").execute(&pool).await?; }
    if starting_version < 5 {
        crate::history::migrate_v5(&pool).await?;
    }
    crate::history::verify_v5(&pool).await?;
    if starting_version < 6 {
        crate::focus::migrate_v6(&pool).await?;
    }
    crate::focus::verify_v6(&pool).await?;
    if starting_version < 7 {
        crate::links::migrate_v7(&pool).await?;
    }
    crate::links::verify_v7(&pool).await?;
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

pub async fn create_copy(pool: &SqlitePool, title: &str, body: &str, kind: &str) -> Result<Note, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query("INSERT INTO notes(id,title,body,kind,created_at,updated_at) VALUES(?,?,?,?,?,?)")
        .bind(&id).bind(title).bind(body).bind(kind).bind(now).bind(now)
        .execute(pool).await?;
    get(pool, &id).await?.ok_or(sqlx::Error::RowNotFound)
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
    // Soft-delete and reminder cancellation must commit together. A failure in
    // the reminder event log must not leave a note silently deleted.
    let now = now_ms();
    let mut tx = pool.begin().await?;
    let result = sqlx::query("UPDATE notes SET deleted_at=?, updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(now).bind(now).bind(id).execute(&mut *tx).await?;
    if result.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    crate::reminders::cancel_in_tx(&mut tx, "note", id).await?;
    tx.commit().await
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


#[cfg(test)]
mod integrity_tests {
    use super::*;
    use std::path::PathBuf;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    fn temporary_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("recall-integrity-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn stranded_restore_original_blocks_blank_database_creation() {
        let dir=temporary_dir();
        let path=dir.join("memoria.sqlite");
        let original=dir.join(".recall-original-crash.sqlite");
        fs::write(&original,b"SQLite safety sentinel").unwrap();
        let result=connect(&path).await;
        assert!(result.is_err());
        assert!(!path.exists(),"must not silently create empty DB after interrupted restore");
        assert!(original.exists(),"must never discard stranded original");
        let _=fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn legacy_upgrade_snapshots_before_creating_revision_table() {
        let dir = temporary_dir();
        let path = dir.join("memoria.sqlite");
        let opts = SqliteConnectOptions::new().filename(&path).create_if_missing(true);
        let legacy = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
        sqlx::query("CREATE TABLE notes (
            id TEXT PRIMARY KEY, title TEXT NOT NULL DEFAULT '', body TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'note', pinned INTEGER NOT NULL DEFAULT 0,
            remind_at INTEGER, notified_at INTEGER, created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL, deleted_at INTEGER
        )").execute(&legacy).await.unwrap();
        sqlx::query("INSERT INTO notes(id,title,body,created_at,updated_at) VALUES('old','Meu texto','Preservar',1,1)")
            .execute(&legacy).await.unwrap();
        legacy.close().await;

        let pool = connect(&path).await.unwrap();
        let first = get(&pool, "old").await.unwrap().unwrap();
        assert_eq!(first.revision, 0);
        assert_eq!(first.title, "Meu texto");
        assert_eq!(first.body, "Preservar");
        let backups = list_backups(&path).unwrap();
        assert_eq!(backups.len(), 4, "each legacy upgrade needs its own verified snapshot");
        assert!(backups.iter().any(|b| b.file_name.contains("pre-alpha8") && b.size_bytes > 0));
        assert!(backups.iter().any(|b| b.file_name.contains("pre-alpha9") && b.size_bytes > 0));
        assert!(backups.iter().any(|b| b.file_name.contains("pre-alpha9-links") && b.size_bytes > 0));
        assert!(backups.iter().any(|b| b.file_name.contains("pre-integrity") && b.size_bytes > 0));

        let changed = save(&pool, "old", "Revisado", "Conteúdo", 0).await.unwrap().unwrap();
        assert_eq!(changed.revision, 1);
        assert!(save(&pool, "old", "Obsoleto", "Antigo", 0).await.unwrap().is_none());
        assert_eq!(get(&pool, "old").await.unwrap().unwrap().title, "Revisado");

        let second_pool = connect(&path).await.unwrap();
        assert_eq!(list_backups(&path).unwrap().len(), backups.len(), "migration backups cannot repeat");
        second_pool.close().await;
        pool.close().await;
        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn simultaneous_saves_reject_one_stale_revision() {
        let dir = temporary_dir();
        let path = dir.join("memoria.sqlite");
        let pool = connect(&path).await.unwrap();
        let note = create(&pool, "note").await.unwrap();
        let (a, b) = tokio::join!(
            save(&pool, &note.id, "Primeira", "A", note.revision),
            save(&pool, &note.id, "Segunda", "B", note.revision)
        );
        let a = a.unwrap().is_some();
        let b = b.unwrap().is_some();
        assert_ne!(a, b, "exactly one of the same-revision saves may succeed");
        assert_eq!(get(&pool, &note.id).await.unwrap().unwrap().revision, 1);

        // A save from an older binary still increments the note revision via trigger.
        sqlx::query("UPDATE notes SET body='Edição externa' WHERE id=?")
            .bind(&note.id).execute(&pool).await.unwrap();
        assert_eq!(get(&pool, &note.id).await.unwrap().unwrap().revision, 2);
        assert!(save(&pool, &note.id, "Stale", "X", 1).await.unwrap().is_none());

        pool.close().await;
        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn delete_note_is_atomic_even_when_cancelling_reminder_fails() {
        let dir = temporary_dir();
        let path = dir.join("memoria.sqlite");
        let pool = connect(&path).await.unwrap();
        let note = create_copy(&pool, "Important", "Não perder", "note").await.unwrap();
        crate::reminders::schedule(&pool, "note", &note.id, now_ms() + 3_600_000, "none")
            .await.unwrap();
        sqlx::query("CREATE TRIGGER stop_note_cancel BEFORE INSERT ON reminder_events
            WHEN NEW.kind='cancelled' BEGIN SELECT RAISE(ABORT, 'simulated cancel failure'); END")
            .execute(&pool).await.unwrap();
        assert!(delete(&pool, &note.id).await.is_err(), "delete must roll back");
        let kept = get(&pool, &note.id).await.unwrap().unwrap();
        assert_eq!(kept.body, "Não perder");
        let state: String = sqlx::query_scalar("SELECT state FROM reminder_rules WHERE target_type='note' AND target_id=?")
            .bind(&note.id).fetch_one(&pool).await.unwrap();
        assert_eq!(state, "scheduled");

        sqlx::query("DROP TRIGGER stop_note_cancel").execute(&pool).await.unwrap();
        delete(&pool, &note.id).await.unwrap();
        assert!(get(&pool, &note.id).await.unwrap().is_none());
        let archived_body: String = sqlx::query_scalar("SELECT body FROM notes WHERE id=?")
            .bind(&note.id).fetch_one(&pool).await.unwrap();
        assert_eq!(archived_body, "Não perder", "deletion must be reversible later");
        let cancelled: String = sqlx::query_scalar("SELECT state FROM reminder_rules WHERE target_type='note' AND target_id=?")
            .bind(&note.id).fetch_one(&pool).await.unwrap();
        assert_eq!(cancelled, "cancelled");
        pool.close().await;
        let _ = fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn snapshot_has_sqlite_integrity_and_all_committed_data() {
        let dir = temporary_dir();
        let path = dir.join("memoria.sqlite");
        let pool = connect(&path).await.unwrap();
        let note = create_copy(&pool, "Importante", "Salvar no WAL", "note").await.unwrap();
        let item = create_backup(&pool, &path, "manual").await.unwrap();
        assert!(item.file_name.starts_with("recall-manual-"));
        assert!(item.file_path.ends_with(".sqlite"));
        let options = SqliteConnectOptions::new().filename(&item.file_path).read_only(true);
        let other = SqlitePoolOptions::new().max_connections(1).connect_with(options).await.unwrap();
        let check: String = sqlx::query_scalar("PRAGMA quick_check").fetch_one(&other).await.unwrap();
        assert_eq!(check, "ok");
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE id=?")
            .bind(&note.id).fetch_one(&other).await.unwrap();
        assert_eq!(count, 1);
        other.close().await;
        pool.close().await;
        let _ = fs::remove_dir_all(dir);
    }
}
