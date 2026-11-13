//! Alpha 8.1: additive, transactional content-history and inbox disposition schema.
//! The database remains the source of truth; this module never restores or purges data.
use sqlx::{SqlitePool, Transaction, Sqlite};

/// Installs v5 in ONE SQLite transaction. Call only after a verified pre-v5 backup.
pub async fn migrate_v5(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    install_schema(&mut tx).await?;
    capture_baselines(&mut tx).await?;
    install_triggers(&mut tx).await?;
    sqlx::query("PRAGMA user_version = 5").execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn install_schema(tx: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    sqlx::query("CREATE TABLE content_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        item_type TEXT NOT NULL CHECK(item_type IN ('note','task','idea','resume')),
        item_id TEXT NOT NULL,
        source_revision INTEGER NOT NULL CHECK(source_revision >= 0),
        snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
        reason TEXT NOT NULL CHECK(reason IN ('baseline','before_edit')),
        captured_at INTEGER NOT NULL,
        UNIQUE(item_type,item_id,source_revision)
    )").execute(&mut **tx).await?;
    sqlx::query("CREATE INDEX idx_content_history_item
        ON content_history(item_type,item_id,captured_at DESC,id DESC)")
        .execute(&mut **tx).await?;
    // processed_at has historically meant BOTH conversion and dismissal. Only new
    // operations get an explicit disposition; never infer one for legacy rows.
    sqlx::query("CREATE TABLE inbox_dispositions (
        item_id TEXT PRIMARY KEY NOT NULL REFERENCES inbox_items(id),
        state TEXT NOT NULL CHECK(state IN ('dismissed','converted_note','converted_task','restored')),
        target_id TEXT,
        changed_at INTEGER NOT NULL,
        CHECK ((state IN ('converted_note','converted_task') AND target_id IS NOT NULL)
            OR (state IN ('dismissed','restored') AND target_id IS NULL))
    )").execute(&mut **tx).await?;
    sqlx::query("CREATE INDEX idx_inbox_dispositions_state
        ON inbox_dispositions(state,changed_at DESC)")
        .execute(&mut **tx).await?;
    Ok(())
}

async fn capture_baselines(tx: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    // One baseline per existing entity, INCLUDING soft-deleted and processed rows.
    // These snapshots describe the state at upgrade; earlier states are unknowable.
    sqlx::query("INSERT INTO content_history
        (item_type,item_id,source_revision,snapshot_json,reason,captured_at)
        SELECT 'note',n.id,COALESCE(v.revision,0),
        json_object('schema_version',1,'title',n.title,'body',n.body,'kind',n.kind),
        'baseline',CAST(strftime('%s','now') AS INTEGER)*1000
        FROM notes n LEFT JOIN note_revisions v ON v.note_id=n.id")
        .execute(&mut **tx).await?;
    sqlx::query("INSERT INTO content_history
        (item_type,item_id,source_revision,snapshot_json,reason,captured_at)
        SELECT 'task',t.id,COALESCE(v.revision,0),
        json_object('schema_version',1,'title',t.title,'details',t.details),
        'baseline',CAST(strftime('%s','now') AS INTEGER)*1000
        FROM tasks t LEFT JOIN task_revisions v ON v.task_id=t.id")
        .execute(&mut **tx).await?;
    sqlx::query("INSERT INTO content_history
        (item_type,item_id,source_revision,snapshot_json,reason,captured_at)
        SELECT 'idea',i.id,COALESCE(v.revision,0),
        json_object('schema_version',1,'text',i.text),
        'baseline',CAST(strftime('%s','now') AS INTEGER)*1000
        FROM inbox_items i LEFT JOIN inbox_revisions v ON v.item_id=i.id")
        .execute(&mut **tx).await?;
    sqlx::query("INSERT INTO content_history
        (item_type,item_id,source_revision,snapshot_json,reason,captured_at)
        SELECT 'resume',r.task_id,r.revision,
        json_object('schema_version',1,'last_context',r.last_context,'next_step',r.next_step),
        'baseline',CAST(strftime('%s','now') AS INTEGER)*1000
        FROM task_resumes r")
        .execute(&mut **tx).await?;
    Ok(())
}

async fn install_triggers(tx: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
    // BEFORE means OLD content and OLD revision are saved regardless of the
    // relative execution order of existing AFTER revision triggers.
    sqlx::query("CREATE TRIGGER history_note_before_edit
        BEFORE UPDATE OF title,body ON notes
        WHEN OLD.title IS NOT NEW.title OR OLD.body IS NOT NEW.body
        BEGIN INSERT OR IGNORE INTO content_history
        (item_type,item_id,source_revision,snapshot_json,reason,captured_at)
        VALUES ('note',OLD.id,COALESCE((SELECT revision FROM note_revisions WHERE note_id=OLD.id),0),
        json_object('schema_version',1,'title',OLD.title,'body',OLD.body,'kind',OLD.kind),
        'before_edit',CAST(strftime('%s','now') AS INTEGER)*1000);
        END").execute(&mut **tx).await?;
    sqlx::query("CREATE TRIGGER history_task_before_edit
        BEFORE UPDATE OF title,details ON tasks
        WHEN OLD.title IS NOT NEW.title OR OLD.details IS NOT NEW.details
        BEGIN INSERT OR IGNORE INTO content_history
        (item_type,item_id,source_revision,snapshot_json,reason,captured_at)
        VALUES ('task',OLD.id,COALESCE((SELECT revision FROM task_revisions WHERE task_id=OLD.id),0),
        json_object('schema_version',1,'title',OLD.title,'details',OLD.details),
        'before_edit',CAST(strftime('%s','now') AS INTEGER)*1000);
        END").execute(&mut **tx).await?;
    sqlx::query("CREATE TRIGGER history_idea_before_edit
        BEFORE UPDATE OF text ON inbox_items
        WHEN OLD.text IS NOT NEW.text
        BEGIN INSERT OR IGNORE INTO content_history
        (item_type,item_id,source_revision,snapshot_json,reason,captured_at)
        VALUES ('idea',OLD.id,COALESCE((SELECT revision FROM inbox_revisions WHERE item_id=OLD.id),0),
        json_object('schema_version',1,'text',OLD.text),
        'before_edit',CAST(strftime('%s','now') AS INTEGER)*1000);
        END").execute(&mut **tx).await?;
    sqlx::query("CREATE TRIGGER history_resume_before_edit
        BEFORE UPDATE OF last_context,next_step ON task_resumes
        WHEN OLD.last_context IS NOT NEW.last_context OR OLD.next_step IS NOT NEW.next_step
        BEGIN INSERT OR IGNORE INTO content_history
        (item_type,item_id,source_revision,snapshot_json,reason,captured_at)
        VALUES ('resume',OLD.task_id,OLD.revision,
        json_object('schema_version',1,'last_context',OLD.last_context,'next_step',OLD.next_step),
        'before_edit',CAST(strftime('%s','now') AS INTEGER)*1000);
        END").execute(&mut **tx).await?;
    // TaskResume CAS updates the revision itself; raw writes from future/older
    // clients that forget to bump it still invalidate stale editor snapshots.
    sqlx::query("CREATE TRIGGER history_resume_external_revision
        AFTER UPDATE OF last_context,next_step ON task_resumes
        WHEN NEW.revision=OLD.revision AND
             (OLD.last_context IS NOT NEW.last_context OR OLD.next_step IS NOT NEW.next_step)
        BEGIN UPDATE task_resumes SET revision=revision+1 WHERE task_id=NEW.task_id;
        END").execute(&mut **tx).await?;
    Ok(())
}

/// Fail closed for a damaged v5 schema; do not silently recreate version history.
pub async fn verify_v5(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let tables: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master
        WHERE type='table' AND name IN ('content_history','inbox_dispositions')")
        .fetch_one(pool).await?;
    let triggers: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master
        WHERE type='trigger' AND name IN ('history_note_before_edit',
        'history_task_before_edit','history_idea_before_edit',
        'history_resume_before_edit','history_resume_external_revision')")
        .fetch_one(pool).await?;
    if tables != 2 || triggers != 5 {
        return Err(sqlx::Error::Protocol("Migração Alpha 8 incompleta: tabelas ou triggers ausentes".into()));
    }
    sqlx::query("SELECT item_type,item_id,source_revision,snapshot_json,reason,captured_at
        FROM content_history LIMIT 0").fetch_all(pool).await?;
    sqlx::query("SELECT item_id,state,target_id,changed_at
        FROM inbox_dispositions LIMIT 0").fetch_all(pool).await?;
    Ok(())
}
