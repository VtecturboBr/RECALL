use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;
use crate::{db::now_ms, reminders};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub details: String,
    pub due_at: Option<i64>,
    pub bucket: String,
    pub planned_for: Option<i64>,
    pub completed_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TaskResume {
    pub task_id: String,
    pub last_context: String,
    pub next_step: String,
    pub updated_at: i64,
    pub revision: i64,
}

const FIELDS: &str = "t.id,t.title,t.details,t.due_at,t.completed_at,t.created_at,t.updated_at,t.deleted_at,COALESCE(p.bucket,'today') AS bucket,p.planned_for,COALESCE(v.revision,0) AS revision";

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
    // Additive planning table: existing tasks keep their original columns and ids.
    sqlx::query("CREATE TABLE IF NOT EXISTS task_plans (
        task_id TEXT PRIMARY KEY NOT NULL REFERENCES tasks(id),
        bucket TEXT NOT NULL CHECK (bucket IN ('today','later','someday')),
        planned_for INTEGER
    )").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_task_plans_bucket ON task_plans(bucket,planned_for)")
        .execute(pool).await?;
    // Trigger detects edits made by older application versions as well.
    sqlx::query("CREATE TABLE IF NOT EXISTS task_revisions (
        task_id TEXT PRIMARY KEY NOT NULL,
        revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0)
    )").execute(pool).await?;
    sqlx::query("INSERT OR IGNORE INTO task_revisions(task_id,revision) SELECT id,0 FROM tasks").execute(pool).await?;
    sqlx::query("CREATE TRIGGER IF NOT EXISTS task_revision_insert AFTER INSERT ON tasks
        BEGIN INSERT OR IGNORE INTO task_revisions(task_id,revision) VALUES(NEW.id,0); END").execute(pool).await?;
    sqlx::query("CREATE TRIGGER IF NOT EXISTS task_revision_edit AFTER UPDATE OF title,details,due_at,completed_at,deleted_at ON tasks
        BEGIN UPDATE task_revisions SET revision=revision+1 WHERE task_id=NEW.id; END").execute(pool).await?;
    // A small, additive checkpoint per task, independent of task text and planning.
    sqlx::query("CREATE TABLE IF NOT EXISTS task_resumes(
        task_id TEXT PRIMARY KEY NOT NULL REFERENCES tasks(id),
        last_context TEXT NOT NULL DEFAULT '',
        next_step TEXT NOT NULL DEFAULT '',
        updated_at INTEGER NOT NULL,
        revision INTEGER NOT NULL DEFAULT 0 CHECK(revision>=0)
    )").execute(pool).await?;
    Ok(())
}

pub fn valid_plan(bucket: &str, planned_for: Option<i64>) -> bool {
    matches!(bucket, "today" | "later" | "someday")
        && (bucket == "later" || planned_for.is_none())
        && planned_for.map(|value| value >= 0).unwrap_or(true)
}

pub async fn create_planned(
    pool: &SqlitePool, title: &str, details: &str,
    bucket: &str, planned_for: Option<i64>, due_at: Option<i64>,
) -> Result<Task, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO tasks(id,title,details,due_at,created_at,updated_at) VALUES(?,?,?,?,?,?)")
        .bind(&id).bind(title.trim()).bind(details).bind(due_at).bind(now).bind(now)
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO task_plans(task_id,bucket,planned_for) VALUES(?,?,?)")
        .bind(&id).bind(bucket).bind(planned_for).execute(&mut *tx).await?;
    tx.commit().await?;
    get(pool, &id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn set_plan(pool: &SqlitePool, id: &str, bucket: &str, planned_for: Option<i64>) -> Result<Task, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let updated = sqlx::query("UPDATE tasks SET updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(now_ms()).bind(id).execute(&mut *tx).await?;
    if updated.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    sqlx::query("INSERT INTO task_plans(task_id,bucket,planned_for) VALUES(?,?,?)
        ON CONFLICT(task_id) DO UPDATE SET bucket=excluded.bucket,planned_for=excluded.planned_for")
        .bind(id).bind(bucket).bind(planned_for).execute(&mut *tx).await?;
    tx.commit().await?;
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(&format!("SELECT {FIELDS} FROM tasks t LEFT JOIN task_plans p ON p.task_id=t.id LEFT JOIN task_revisions v ON v.task_id=t.id WHERE t.deleted_at IS NULL
        ORDER BY (t.completed_at IS NOT NULL), (t.due_at IS NULL), t.due_at, t.updated_at DESC"))
        .fetch_all(pool).await
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Task>, sqlx::Error> {
    sqlx::query_as::<_, Task>(&format!("SELECT {FIELDS} FROM tasks t LEFT JOIN task_plans p ON p.task_id=t.id LEFT JOIN task_revisions v ON v.task_id=t.id WHERE t.id=? AND t.deleted_at IS NULL"))
        .bind(id).fetch_optional(pool).await
}

pub async fn create(pool: &SqlitePool, title: &str) -> Result<Task, sqlx::Error> {
    let id = Uuid::new_v4().to_string();
    let now = now_ms();
    sqlx::query("INSERT INTO tasks (id,title,created_at,updated_at) VALUES (?,?,?,?)")
        .bind(&id).bind(title.trim()).bind(now).bind(now).execute(pool).await?;
    get(pool, &id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn update(pool: &SqlitePool, id: &str, title: &str, details: &str, due_at: Option<i64>, expected_revision: i64) -> Result<Option<Task>, sqlx::Error> {
    let updated = sqlx::query("UPDATE tasks SET title=?,details=?,due_at=?,updated_at=? WHERE id=? AND deleted_at IS NULL
        AND EXISTS(SELECT 1 FROM task_revisions WHERE task_id=tasks.id AND revision=?)")
        .bind(title.trim()).bind(details).bind(due_at).bind(now_ms()).bind(id).bind(expected_revision).execute(pool).await?;
    if updated.rows_affected() == 0 { return Ok(None); }
    get(pool, id).await
}

pub async fn set_completed(pool: &SqlitePool, id: &str, completed: bool) -> Result<Task, sqlx::Error> {
    set_completed_with_summary(pool, id, completed, None).await
}

pub async fn set_completed_with_summary(pool: &SqlitePool, id: &str, completed: bool, summary: Option<&str>) -> Result<Task, sqlx::Error> {
    let at = if completed { Some(now_ms()) } else { None };
    let mut tx = pool.begin().await?;
    let updated = sqlx::query("UPDATE tasks SET completed_at=?,updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(at).bind(now_ms()).bind(id).execute(&mut *tx).await?;
    if updated.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    if completed {
        reminders::cancel_in_tx(&mut tx, "task", id).await?;
        crate::focus::finish_task_with_summary_in_tx(&mut tx, id, summary).await?;
    }
    tx.commit().await?;
    get(pool, id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn get_resume(pool: &SqlitePool, task_id: &str) -> Result<Option<TaskResume>, sqlx::Error> {
    sqlx::query_as::<_, TaskResume>("SELECT r.task_id,r.last_context,r.next_step,r.updated_at,r.revision
        FROM task_resumes r INNER JOIN tasks t ON t.id=r.task_id
        WHERE r.task_id=? AND t.deleted_at IS NULL")
        .bind(task_id).fetch_optional(pool).await
}

/// A first save inserts revision 1; later saves must supply the current revision.
/// No overwrite is possible from two windows starting with the same revision.
pub async fn save_resume(
    pool: &SqlitePool, task_id: &str, last_context: &str, next_step: &str,
    expected_revision: i64,
) -> Result<Option<TaskResume>, sqlx::Error> {
    let result = sqlx::query("INSERT INTO task_resumes(task_id,last_context,next_step,updated_at,revision)
        SELECT id,?,?,?,1 FROM tasks WHERE id=? AND deleted_at IS NULL
        ON CONFLICT(task_id) DO UPDATE SET
          last_context=excluded.last_context, next_step=excluded.next_step,
          updated_at=excluded.updated_at, revision=task_resumes.revision+1
        WHERE task_resumes.revision=?")
        .bind(last_context).bind(next_step).bind(now_ms()).bind(task_id).bind(expected_revision)
        .execute(pool).await?;
    if result.rows_affected()==0 { return Ok(None); }
    get_resume(pool, task_id).await
}

pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    delete_with_summary(pool,id,None).await
}
pub async fn delete_with_summary(pool: &SqlitePool, id: &str, summary: Option<&str>) -> Result<(), sqlx::Error> {
    let now = now_ms();
    let mut tx = pool.begin().await?;
    let updated = sqlx::query("UPDATE tasks SET deleted_at=?,updated_at=? WHERE id=? AND deleted_at IS NULL")
        .bind(now).bind(now).bind(id).execute(&mut *tx).await?;
    if updated.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    reminders::cancel_in_tx(&mut tx, "task", id).await?;
    crate::focus::finish_task_with_summary_in_tx(&mut tx, id, summary).await?;
    tx.commit().await
}


#[cfg(test)]
mod planning_tests {
    use super::valid_plan;
    fn test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("recall-task-cas-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn concurrent_edits_reject_stale_task_revision() {
        let dir = test_dir();
        let pool = crate::db::connect(&dir.join("memoria.sqlite")).await.unwrap();
        let task = super::create(&pool, "Original").await.unwrap();
        let (a,b) = tokio::join!(
            super::update(&pool, &task.id, "Versão A", "A", None, task.revision),
            super::update(&pool, &task.id, "Versão B", "B", None, task.revision)
        );
        assert_ne!(a.unwrap().is_some(), b.unwrap().is_some());
        assert_eq!(super::get(&pool, &task.id).await.unwrap().unwrap().revision, 1);
        // Older binaries bypassing CAS still advance the revision.
        sqlx::query("UPDATE tasks SET details='Externo' WHERE id=?")
            .bind(&task.id).execute(&pool).await.unwrap();
        assert!(super::update(&pool, &task.id, "Stale", "", None, 1).await.unwrap().is_none());
        pool.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn task_delete_undoes_everything_when_reminder_cancel_fails() {
        let dir = test_dir();
        let path = dir.join("memoria.sqlite");
        let pool = crate::db::connect(&path).await.unwrap();
        let task = super::create(&pool, "Importante").await.unwrap();
        crate::reminders::schedule(&pool, "task", &task.id,
            crate::db::now_ms() + 3_600_000, "none").await.unwrap();
        sqlx::query("CREATE TRIGGER reject_task_delete_cancel BEFORE INSERT ON reminder_events
            WHEN NEW.kind='cancelled' BEGIN SELECT RAISE(ABORT,'simulated SQL failure'); END")
            .execute(&pool).await.unwrap();
        assert!(super::delete(&pool, &task.id).await.is_err());
        assert!(super::get(&pool, &task.id).await.unwrap().is_some());
        sqlx::query("DROP TRIGGER reject_task_delete_cancel").execute(&pool).await.unwrap();
        super::delete(&pool, &task.id).await.unwrap();
        assert!(super::get(&pool, &task.id).await.unwrap().is_none());
        let title: String = sqlx::query_scalar("SELECT title FROM tasks WHERE id=?")
            .bind(&task.id).fetch_one(&pool).await.unwrap();
        assert_eq!(title, "Importante");
        pool.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn task_completion_and_reminder_cancel_roll_back_together() {
        let dir = test_dir();
        let pool = crate::db::connect(&dir.join("memoria.sqlite")).await.unwrap();
        let task = super::create(&pool, "Pagar conta").await.unwrap();
        crate::reminders::schedule(&pool, "task", &task.id, crate::db::now_ms()+60000, "none").await.unwrap();
        sqlx::query("CREATE TRIGGER block_cancel BEFORE INSERT ON reminder_events
            WHEN NEW.kind='cancelled' BEGIN SELECT RAISE(FAIL, 'simulated crash'); END")
            .execute(&pool).await.unwrap();
        assert!(super::set_completed(&pool, &task.id, true).await.is_err());
        assert_eq!(super::get(&pool, &task.id).await.unwrap().unwrap().completed_at, None);
        assert_eq!(crate::reminders::find(&pool, "task", &task.id).await.unwrap().unwrap().state, "scheduled");
        sqlx::query("DROP TRIGGER block_cancel").execute(&pool).await.unwrap();
        let changed = super::set_completed(&pool, &task.id, true).await.unwrap();
        assert!(changed.completed_at.is_some());
        let state: String = sqlx::query_scalar("SELECT state FROM reminder_rules WHERE target_type='task' AND target_id=?")
            .bind(&task.id).fetch_one(&pool).await.unwrap();
        assert_eq!(state, "cancelled");
        pool.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn migrating_existing_task_creates_verified_backup() {
        let dir = test_dir(); let path=dir.join("memoria.sqlite");
        let pool=crate::db::connect(&path).await.unwrap();
        let task=super::create(&pool, "Legado").await.unwrap();
        sqlx::query("DROP TRIGGER task_revision_insert").execute(&pool).await.unwrap();
        sqlx::query("DROP TRIGGER task_revision_edit").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE task_revisions").execute(&pool).await.unwrap();
        pool.close().await;
        let reopened=crate::db::connect(&path).await.unwrap();
        assert_eq!(super::get(&reopened, &task.id).await.unwrap().unwrap().title, "Legado");
        assert!(crate::db::list_backups(&path).unwrap().iter()
            .any(|x| x.file_name.contains("pre-content-revisions")));
        reopened.close().await;
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn resume_checkpoint_persists_and_rejects_stale_writes() {
        let dir=test_dir(); let path=dir.join("memoria.sqlite");
        let pool=crate::db::connect(&path).await.unwrap();
        let task=super::create(&pool,"Explicar projeto").await.unwrap();
        assert!(super::get_resume(&pool,&task.id).await.unwrap().is_none());
        let (first,second)=tokio::join!(
            super::save_resume(&pool,&task.id,"Falei sobre a arquitetura","Mostrar demo",0),
            super::save_resume(&pool,&task.id,"Outra versão","Voltar ao começo",0)
        );
        assert_ne!(first.unwrap().is_some(),second.unwrap().is_some());
        let row=super::get_resume(&pool,&task.id).await.unwrap().unwrap();
        assert_eq!(row.revision,1);
        let next=super::save_resume(&pool,&task.id,"Explicação finalizada","Responder perguntas",1)
            .await.unwrap().unwrap();
        assert_eq!(next.revision,2);
        assert!(super::save_resume(&pool,&task.id,"Obsoleto","Não pode",1).await.unwrap().is_none());
        pool.close().await;
        let reopened=crate::db::connect(&path).await.unwrap();
        let persisted=super::get_resume(&reopened,&task.id).await.unwrap().unwrap();
        assert_eq!(persisted.next_step,"Responder perguntas");
        reopened.close().await; let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn resume_migration_preserves_existing_tasks_and_creates_backup() {
        let dir=test_dir(); let path=dir.join("memoria.sqlite");
        let pool=crate::db::connect(&path).await.unwrap();
        let task=super::create(&pool,"Tarefa pré-existente").await.unwrap();
        // Reconstruct a genuine pre-Alpha-7/8 fixture. A v5 database missing
        // its history tables must be rejected, not silently "repaired".
        for trigger in ["history_note_before_edit","history_task_before_edit",
            "history_idea_before_edit","history_resume_before_edit",
            "history_resume_external_revision"] {
            sqlx::query(&format!("DROP TRIGGER IF EXISTS {trigger}"))
                .execute(&pool).await.unwrap();
        }
        sqlx::query("DROP TABLE item_links").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE focus_sessions").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE content_history").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE inbox_dispositions").execute(&pool).await.unwrap();
        sqlx::query("DROP TABLE task_resumes").execute(&pool).await.unwrap();
        sqlx::query("PRAGMA user_version = 3").execute(&pool).await.unwrap();
        pool.close().await;
        let reopened=crate::db::connect(&path).await.unwrap();
        assert_eq!(super::get(&reopened,&task.id).await.unwrap().unwrap().title,"Tarefa pré-existente");
        let backups=crate::db::list_backups(&path).unwrap();
        assert!(backups.iter().any(|x| x.file_name.contains("pre-resume")));
        let rev: i64=sqlx::query_scalar("PRAGMA user_version").fetch_one(&reopened).await.unwrap();
        assert_eq!(rev,7);
        reopened.close().await; let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn closing_task_preserves_focus_summary_transactionally() {
        let dir=test_dir();let path=dir.join("memoria.sqlite");
        let pool=crate::db::connect(&path).await.unwrap();
        let task=super::create(&pool,"Preservar progresso").await.unwrap();
        let session=crate::focus::start(&pool,&task.id).await.unwrap();
        super::set_completed_with_summary(&pool,&task.id,true,Some("Resumo que estava em edição"))
            .await.unwrap();
        let history=crate::focus::history(&pool,&task.id).await.unwrap();
        let saved=history.iter().find(|row|row.id==session.id).unwrap();
        assert_eq!(saved.state,"interrupted");
        assert_eq!(saved.summary,"Resumo que estava em edição");
        assert!(super::get(&pool,&task.id).await.unwrap().unwrap().completed_at.is_some());
        pool.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn deleting_task_preserves_focus_summary_without_erasing_task_data() {
        let dir=test_dir();let path=dir.join("memoria.sqlite");
        let pool=crate::db::connect(&path).await.unwrap();
        let task=super::create(&pool,"Guardar tudo").await.unwrap();
        crate::focus::start(&pool,&task.id).await.unwrap();
        super::delete_with_summary(&pool,&task.id,Some("Progresso da tarefa removida"))
            .await.unwrap();
        let history=crate::focus::history(&pool,&task.id).await.unwrap();
        assert_eq!(history[0].summary,"Progresso da tarefa removida");
        let stored:String=sqlx::query_scalar("SELECT title FROM tasks WHERE id=?")
            .bind(&task.id).fetch_one(&pool).await.unwrap();
        assert_eq!(stored,"Guardar tudo");
        pool.close().await;let _=std::fs::remove_dir_all(dir);
    }

    #[test]
    fn accepts_only_valid_bucket_and_schedule() {
        assert!(valid_plan("today", None));
        assert!(valid_plan("later", Some(1_800_000_000_000)));
        assert!(valid_plan("later", None));
        assert!(valid_plan("someday", None));
        assert!(!valid_plan("someday", Some(1)));
        assert!(!valid_plan("today", Some(1)));
        assert!(!valid_plan("trash", None));
        assert!(!valid_plan("later", Some(-10)));
    }
}
