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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, inbox, tasks};
    use std::path::PathBuf;

    fn fixture() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("recall-alpha8-migration-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("memoria.sqlite");
        (dir, db_path)
    }

    /// Construct a database matching the Alpha 7 schema without carrying v5 artifacts.
    async fn downgrade_fixture_to_v4(pool: &SqlitePool) {
        for name in [
            "history_note_before_edit", "history_task_before_edit",
            "history_idea_before_edit", "history_resume_before_edit",
            "history_resume_external_revision"
        ] {
            sqlx::query(&format!("DROP TRIGGER IF EXISTS {name}"))
                .execute(pool).await.unwrap();
        }
        sqlx::query("DROP TABLE focus_sessions").execute(pool).await.unwrap();
        sqlx::query("DROP TABLE content_history").execute(pool).await.unwrap();
        sqlx::query("DROP TABLE inbox_dispositions").execute(pool).await.unwrap();
        sqlx::query("PRAGMA user_version = 4").execute(pool).await.unwrap();
    }

    #[tokio::test]
    async fn migration_is_idempotent_and_preserves_every_content_type() {
        let (dir, path) = fixture();
        let pool = db::connect(&path).await.unwrap();
        let note = db::create(&pool, "presentation").await.unwrap();
        let note = db::save(&pool, &note.id, "Apresentação", "Olá 🎯\n---\nSegundo cartão", note.revision)
            .await.unwrap().unwrap();
        let task = tasks::create(&pool, "Trabalho de faculdade").await.unwrap();
        let idea = inbox::capture(&pool, "Ideia com acento: ação").await.unwrap();
        let resume = tasks::save_resume(&pool, &task.id, "Comecei o capítulo", "Revisar exemplos", 0)
            .await.unwrap().unwrap();
        assert_eq!(resume.revision, 1);
        downgrade_fixture_to_v4(&pool).await;
        pool.close().await;

        let upgraded = db::connect(&path).await.unwrap();
        let version: i64 = sqlx::query_scalar("PRAGMA user_version").fetch_one(&upgraded).await.unwrap();
        assert_eq!(version, 6);
        assert_eq!(db::get(&upgraded, &note.id).await.unwrap().unwrap().body, note.body);
        assert_eq!(tasks::get(&upgraded, &task.id).await.unwrap().unwrap().title, "Trabalho de faculdade");
        assert_eq!(inbox::get(&upgraded, &idea.id).await.unwrap().unwrap().text, "Ideia com acento: ação");
        assert_eq!(tasks::get_resume(&upgraded, &task.id).await.unwrap().unwrap().next_step, "Revisar exemplos");
        let history_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM content_history")
            .fetch_one(&upgraded).await.unwrap();
        assert_eq!(history_count, 4);
        let backup_count = db::list_backups(&path).unwrap();
        assert_eq!(backup_count.len(), 2);
        assert!(backup_count.iter().any(|b| b.file_name.contains("pre-alpha8")));
        assert!(backup_count.iter().any(|b| b.file_name.contains("pre-alpha9")));
        let check: String = sqlx::query_scalar("PRAGMA quick_check").fetch_one(&upgraded).await.unwrap();
        assert_eq!(check, "ok");
        upgraded.close().await;

        let reopened = db::connect(&path).await.unwrap();
        let new_history_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM content_history")
            .fetch_one(&reopened).await.unwrap();
        assert_eq!(new_history_count, history_count, "baseline must not duplicate");
        assert_eq!(db::list_backups(&path).unwrap().len(), backup_count.len(), "backup must not repeat");
        reopened.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn history_captures_old_content_from_raw_sql_and_skips_noop() {
        let (dir, path) = fixture();
        let pool = db::connect(&path).await.unwrap();
        let note = db::create(&pool, "note").await.unwrap();
        let saved = db::save(&pool, &note.id, "Título", "Olá 🧠", note.revision)
            .await.unwrap().unwrap();
        let initial: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM content_history WHERE item_type='note' AND item_id=?")
            .bind(&note.id).fetch_one(&pool).await.unwrap();
        assert_eq!(initial, 1, "new item first edit retains its blank original");
        sqlx::query("UPDATE notes SET title='Título',body='Olá 🧠' WHERE id=?")
            .bind(&note.id).execute(&pool).await.unwrap();
        let after_noop: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM content_history WHERE item_type='note' AND item_id=?")
            .bind(&note.id).fetch_one(&pool).await.unwrap();
        assert_eq!(after_noop, initial, "same text cannot create redundant history");
        sqlx::query("UPDATE notes SET body='Outro texto' WHERE id=?")
            .bind(&note.id).execute(&pool).await.unwrap();
        let old_body: String = sqlx::query_scalar("SELECT json_extract(snapshot_json,'$.body')
            FROM content_history WHERE item_type='note' AND item_id=? ORDER BY id DESC LIMIT 1")
            .bind(&note.id).fetch_one(&pool).await.unwrap();
        assert_eq!(old_body, "Olá 🧠");
        let stale = db::save(&pool, &note.id, "Não sobrescrever", "Perdido", saved.revision).await.unwrap();
        assert!(stale.is_none(), "legacy SQL update must invalidate CAS");
        pool.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn resume_raw_edits_invalidate_cas_and_keep_previous_text() {
        let (dir, path) = fixture();
        let pool = db::connect(&path).await.unwrap();
        let task = tasks::create(&pool, "Retomar").await.unwrap();
        let resume = tasks::save_resume(&pool, &task.id, "Anterior", "Próximo", 0)
            .await.unwrap().unwrap();
        sqlx::query("UPDATE task_resumes SET next_step='Mudei externamente' WHERE task_id=?")
            .bind(&task.id).execute(&pool).await.unwrap();
        let actual = tasks::get_resume(&pool, &task.id).await.unwrap().unwrap();
        assert_eq!(actual.revision, resume.revision + 1);
        assert!(tasks::save_resume(&pool, &task.id, "Obsoleto", "Não gravar", resume.revision)
            .await.unwrap().is_none());
        let previous: String = sqlx::query_scalar("SELECT json_extract(snapshot_json,'$.next_step')
            FROM content_history WHERE item_type='resume' AND item_id=? ORDER BY id DESC LIMIT 1")
            .bind(&task.id).fetch_one(&pool).await.unwrap();
        assert_eq!(previous, "Próximo");
        pool.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn disposition_records_discard_and_both_conversions_without_guessing_legacy() {
        let (dir, path) = fixture();
        let pool = db::connect(&path).await.unwrap();
        let legacy = inbox::capture(&pool, "Ideia processada anteriormente").await.unwrap();
        inbox::dismiss(&pool, &legacy.id).await.unwrap();
        downgrade_fixture_to_v4(&pool).await;
        pool.close().await;
        let pool = db::connect(&path).await.unwrap();
        let legacy_state: Option<String> = sqlx::query_scalar("SELECT state FROM inbox_dispositions WHERE item_id=?")
            .bind(&legacy.id).fetch_optional(&pool).await.unwrap();
        assert!(legacy_state.is_none(), "processed_at from older versions is ambiguous");
        let dismissed = inbox::capture(&pool, "Descartar").await.unwrap();
        inbox::dismiss(&pool, &dismissed.id).await.unwrap();
        let note_source = inbox::capture(&pool, "Transformar em nota").await.unwrap();
        let note = inbox::to_note(&pool, &note_source.id).await.unwrap();
        let task_source = inbox::capture(&pool, "Transformar em tarefa").await.unwrap();
        let task = inbox::to_task(&pool, &task_source.id).await.unwrap();
        for (id,expected_state,expected_target) in [
            (&dismissed.id,"dismissed",None),
            (&note_source.id,"converted_note",Some(note.id.as_str())),
            (&task_source.id,"converted_task",Some(task.id.as_str()))
        ] {
            let (state,target): (String,Option<String>) =
                sqlx::query_as("SELECT state,target_id FROM inbox_dispositions WHERE item_id=?")
                    .bind(id).fetch_one(&pool).await.unwrap();
            assert_eq!(state,expected_state);
            assert_eq!(target.as_deref(),expected_target);
        }
        pool.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn failed_disposition_insert_rolls_back_dismiss_and_conversion() {
        let (dir, path) = fixture();
        let pool = db::connect(&path).await.unwrap();
        let idea = inbox::capture(&pool,"Não perder conteúdo").await.unwrap();
        let before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes").fetch_one(&pool).await.unwrap();
        sqlx::query("CREATE TRIGGER reject_disposition BEFORE INSERT ON inbox_dispositions
            BEGIN SELECT RAISE(ABORT,'simulated SQLite failure'); END")
            .execute(&pool).await.unwrap();
        assert!(inbox::dismiss(&pool,&idea.id).await.is_err());
        assert!(inbox::get(&pool,&idea.id).await.unwrap().is_some(), "dismiss must roll back");
        assert!(inbox::to_note(&pool,&idea.id).await.is_err());
        assert!(inbox::get(&pool,&idea.id).await.unwrap().is_some(), "conversion must roll back");
        let after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM notes").fetch_one(&pool).await.unwrap();
        assert_eq!(before,after, "failed conversion cannot leave a ghost note");
        pool.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn failure_mid_migration_rolls_back_tables_and_version() {
        let (dir,path) = fixture();
        let pool=db::connect(&path).await.unwrap();
        let note=db::capture(&pool,"Não apagar a nota").await.unwrap();
        downgrade_fixture_to_v4(&pool).await;
        sqlx::query("CREATE TABLE inbox_dispositions(broken INTEGER)")
            .execute(&pool).await.unwrap();
        pool.close().await;
        assert!(db::connect(&path).await.is_err(), "invalid schema must abort migration");
        let raw=sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(&path)
        ).await.unwrap();
        let version:i64=sqlx::query_scalar("PRAGMA user_version").fetch_one(&raw).await.unwrap();
        assert_eq!(version,4);
        let history_table:i64=sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master
            WHERE type='table' AND name='content_history'").fetch_one(&raw).await.unwrap();
        assert_eq!(history_table,0, "DDL must roll back completely");
        let title:String=sqlx::query_scalar("SELECT title FROM notes WHERE id=?")
            .bind(&note.id).fetch_one(&raw).await.unwrap();
        assert_eq!(title,"Não apagar a nota");
        assert!(db::list_backups(&path).unwrap().iter().any(|b| b.file_name.contains("pre-alpha8")));
        raw.close().await;
        let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn backup_failure_blocks_any_v5_schema_change() {
        let (dir,path) = fixture();
        let pool=db::connect(&path).await.unwrap();
        let note=db::capture(&pool,"Minha informação").await.unwrap();
        downgrade_fixture_to_v4(&pool).await;
        pool.close().await;
        std::fs::write(dir.join("backups"), b"not a folder").unwrap();
        assert!(db::connect(&path).await.is_err(), "no migration without verified backup");
        let raw=sqlx::SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new().filename(&path)
        ).await.unwrap();
        let version:i64=sqlx::query_scalar("PRAGMA user_version").fetch_one(&raw).await.unwrap();
        assert_eq!(version,4);
        let title:String=sqlx::query_scalar("SELECT title FROM notes WHERE id=?")
            .bind(&note.id).fetch_one(&raw).await.unwrap();
        assert_eq!(title,"Minha informação");
        let history_tables:i64=sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE
            type='table' AND name='content_history'").fetch_one(&raw).await.unwrap();
        assert_eq!(history_tables,0);
        raw.close().await;
        let _=std::fs::remove_dir_all(dir);
    }
}
