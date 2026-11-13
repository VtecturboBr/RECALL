//! Alpha 9.2: an entirely read-only, on-demand daily review.
//! No background notification, auto-completion, task rescheduling or extra schema.
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use std::collections::HashSet;
use crate::{db::now_ms, reminders};

const HOUR_MS: i64 = 3_600_000;
const DAY_MS: i64 = 24 * HOUR_MS;
const REVIEW_LIMIT: usize = 12;

#[derive(Debug, Clone, Serialize)]
pub struct ReviewItem {
    pub kind: String,
    pub target_type: String,
    pub target_id: String,
    pub title: String,
    pub description: String,
    pub occurred_at: i64,
}

#[derive(Debug, FromRow)]
struct ReviewTask {
    id: String,
    title: String,
    details: String,
    due_at: Option<i64>,
    created_at: i64,
    updated_at: i64,
    bucket: String,
    planned_for: Option<i64>,
    next_step: String,
    focus_state: Option<String>,
    focus_at: Option<i64>,
}

/// Fetch once when a person opens "Revisão diária". Unlike reminders,
/// this never mutates the database or triggers the Windows notification API.
pub async fn list(pool: &SqlitePool) -> Result<Vec<ReviewItem>, sqlx::Error> {
    list_at(pool, now_ms()).await
}

async fn list_at(pool: &SqlitePool, now: i64) -> Result<Vec<ReviewItem>, sqlx::Error> {
    let mut ranked: Vec<(u8, ReviewItem)> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();

    // Existing reminder logic is the only source of reminder truth. This
    // includes overdue scheduled reminders that the scheduler has not claimed.
    for reminder in reminders::list(pool).await? {
        let needs_attention = reminder.state == "attention"
            || (matches!(reminder.state.as_str(), "scheduled" | "snoozed")
                && reminder.due_at <= now);
        if !needs_attention { continue; }
        let key = (reminder.target_type.clone(), reminder.target_id.clone());
        if !seen.insert(key) { continue; }
        ranked.push((0, ReviewItem {
            kind: "reminder".into(),
            target_type: reminder.target_type,
            target_id: reminder.target_id,
            title: reminder.title,
            description: "Lembrete pendente. Abra o item para decidir o que fazer.".into(),
            occurred_at: reminder.due_at,
        }));
    }

    // One query for all open tasks, and a subquery for each task's *latest*
    // focus session. An earlier pause cannot override a newer finished session.
    let rows: Vec<ReviewTask> = sqlx::query_as(
        "SELECT t.id,t.title,t.details,t.due_at,t.created_at,t.updated_at,
                COALESCE(p.bucket,'today') AS bucket,p.planned_for,
                COALESCE(s.next_step,'') AS next_step,
                f.state AS focus_state,
                COALESCE(f.ended_at,f.last_tick_at) AS focus_at
         FROM tasks t
         LEFT JOIN task_plans p ON p.task_id=t.id
         LEFT JOIN task_resumes s ON s.task_id=t.id
         LEFT JOIN focus_sessions f ON f.id=(
            SELECT fs.id FROM focus_sessions fs WHERE fs.task_id=t.id
            ORDER BY fs.started_at DESC,fs.rowid DESC LIMIT 1
         )
         WHERE t.deleted_at IS NULL AND t.completed_at IS NULL"
    ).fetch_all(pool).await?;

    for task in rows {
        if seen.contains(&("task".into(), task.id.clone())) { continue; }
        // An explicit "Algum dia" or future scheduled task should not be
        // treated as neglected, even if its last update was weeks ago.
        let postponed = task.bucket == "someday" ||
            (task.bucket == "later" && task.planned_for.map(|at| at > now).unwrap_or(false));
        let overdue = task.due_at.filter(|at| *at < now);
        let activity_at = task.updated_at.max(task.focus_at.unwrap_or(0));
        let age_ms = now.saturating_sub(activity_at);
        let missing_step = task.details.trim().is_empty() && task.next_step.trim().is_empty();
        let choice: Option<(u8, &str, &str, i64)> = if let Some(due_at) = overdue {
            Some((1, "overdue_task", "O prazo registrado passou. Confira a tarefa.", due_at))
        } else if !postponed && matches!(task.focus_state.as_deref(),Some("paused" | "interrupted")) {
            Some((2, "interrupted_focus", "A última sessão foi interrompida ou pausada. Retome o contexto quando quiser.", task.focus_at.unwrap_or(task.updated_at)))
        } else if !postponed && missing_step && now.saturating_sub(task.created_at) >= 3*DAY_MS && age_ms >= 3*DAY_MS {
            Some((3, "missing_step", "Esta tarefa está sem detalhes ou próximo passo há alguns dias.", task.updated_at))
        } else if !postponed && age_ms >= 14*DAY_MS {
            Some((4, "stale_task", "Esta tarefa está há duas semanas sem atividade registrada.", activity_at))
        } else { None };

        if let Some((priority, kind, description, occurred_at)) = choice {
            seen.insert(("task".into(), task.id.clone()));
            ranked.push((priority, ReviewItem {
                kind: kind.into(), target_type: "task".into(),
                target_id: task.id, title: task.title,
                description: description.into(), occurred_at,
            }));
        }
    }

    // Least cognitive overhead: one reason per item and at most 12 entries.
    ranked.sort_by(|(p_a,a),(p_b,b)| p_a.cmp(p_b)
        .then(a.occurred_at.cmp(&b.occurred_at))
        .then(a.target_type.cmp(&b.target_type))
        .then(a.target_id.cmp(&b.target_id)));
    Ok(ranked.into_iter().take(REVIEW_LIMIT).map(|(_,entry)|entry).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db,focus,tasks,reminders};
    use uuid::Uuid;

    fn fixture() -> (std::path::PathBuf,std::path::PathBuf) {
        let folder=std::env::temp_dir().join(format!("recall-review-{}",Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        (folder.clone(),folder.join("memoria.sqlite"))
    }

    #[tokio::test]
    async fn empty_review_is_read_only_and_repeatable() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let now=db::now_ms();
        assert!(list_at(&pool,now).await.unwrap().is_empty());
        assert!(list_at(&pool,now).await.unwrap().is_empty());
        let history_count: i64=sqlx::query_scalar("SELECT COUNT(*) FROM reminder_events")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(history_count,0);
        pool.close().await;
        let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn reminder_and_overdue_task_are_prioritized_and_deduplicated() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let now=db::now_ms();
        let task=tasks::create(&pool,"Atrasada").await.unwrap();
        sqlx::query("UPDATE tasks SET due_at=? WHERE id=?")
            .bind(now-DAY_MS).bind(&task.id).execute(&pool).await.unwrap();
        let note=db::create_copy(&pool,"Recado","Conteúdo","note").await.unwrap();
        reminders::schedule(&pool,"note",&note.id,now+HOUR_MS,"none").await.unwrap();
        sqlx::query("UPDATE reminder_rules SET state='attention',due_at=? WHERE target_type='note' AND target_id=?")
            .bind(now-2*DAY_MS).bind(&note.id).execute(&pool).await.unwrap();
        reminders::schedule(&pool,"task",&task.id,now+HOUR_MS,"none").await.unwrap();
        sqlx::query("UPDATE reminder_rules SET due_at=? WHERE target_type='task' AND target_id=?")
            .bind(now-DAY_MS).bind(&task.id).execute(&pool).await.unwrap();
        let rows=list_at(&pool,now).await.unwrap();
        assert_eq!(rows.len(),2);
        assert_eq!(rows[0].kind,"reminder");
        assert_eq!(rows[0].target_id,note.id);
        assert_eq!(rows[1].kind,"reminder");
        assert_eq!(rows[1].target_id,task.id);
        assert!(list_at(&pool,now).await.unwrap().iter().all(|r|r.kind=="reminder"));
        db::delete(&pool,&note.id).await.unwrap();
        tasks::set_completed(&pool,&task.id,true).await.unwrap();
        assert!(list_at(&pool,now).await.unwrap().is_empty(),
            "deleted notes and completed tasks must not appear");
        pool.close().await;
        let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn paused_sessions_return_only_if_the_latest_session_is_paused() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let now=db::now_ms();
        let task=tasks::create(&pool,"Continuar trabalho").await.unwrap();
        let first=focus::start(&pool,&task.id).await.unwrap();
        focus::stop(&pool,&first.id,"paused","Fiz a primeira parte").await.unwrap();
        assert_eq!(list_at(&pool,now).await.unwrap()[0].kind,"interrupted_focus");
        let next=focus::start(&pool,&task.id).await.unwrap();
        focus::stop(&pool,&next.id,"finished","Concluí a sessão").await.unwrap();
        assert!(list_at(&pool,now).await.unwrap().is_empty(),
            "a completed newer session supersedes an old paused one");
        pool.close().await;
        let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn postponed_someday_items_never_look_stale_but_explicit_due_dates_do() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let now=db::now_ms();
        let t=tasks::create(&pool,"Algum dia").await.unwrap();
        tasks::set_plan(&pool,&t.id,"someday",None).await.unwrap();
        sqlx::query("UPDATE tasks SET created_at=?,updated_at=? WHERE id=?")
            .bind(now-30*DAY_MS).bind(now-30*DAY_MS).bind(&t.id)
            .execute(&pool).await.unwrap();
        assert!(list_at(&pool,now).await.unwrap().is_empty());
        sqlx::query("UPDATE tasks SET due_at=? WHERE id=?")
            .bind(now-DAY_MS).bind(&t.id).execute(&pool).await.unwrap();
        let rows=list_at(&pool,now).await.unwrap();
        assert_eq!(rows.len(),1);
        assert_eq!(rows[0].kind,"overdue_task");
        pool.close().await;
        let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn three_day_missing_step_and_two_week_stale_are_conservative() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let now=db::now_ms();
        let missing=tasks::create(&pool,"Sem detalhes").await.unwrap();
        sqlx::query("UPDATE tasks SET created_at=?,updated_at=? WHERE id=?")
            .bind(now-4*DAY_MS).bind(now-4*DAY_MS).bind(&missing.id)
            .execute(&pool).await.unwrap();
        let stale=tasks::create_planned(&pool,"Tarefa antiga","Com detalhes","today",None,None)
            .await.unwrap();
        sqlx::query("UPDATE tasks SET created_at=?,updated_at=? WHERE id=?")
            .bind(now-20*DAY_MS).bind(now-20*DAY_MS).bind(&stale.id)
            .execute(&pool).await.unwrap();
        let rows=list_at(&pool,now).await.unwrap();
        assert_eq!(rows.len(),2);
        assert_eq!(rows[0].kind,"missing_step");
        assert_eq!(rows[1].kind,"stale_task");
        assert_eq!(list_at(&pool,now).await.unwrap().len(),2);
        pool.close().await;
        let _=std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn review_limits_payload_without_changing_any_items() {
        let (dir,path)=fixture();
        let pool=db::connect(&path).await.unwrap();
        let now=db::now_ms();
        for i in 0..20 {
            let task=tasks::create(&pool,&format!("Tarefa {i}")).await.unwrap();
            sqlx::query("UPDATE tasks SET created_at=?,updated_at=? WHERE id=?")
                .bind(now-10*DAY_MS).bind(now-10*DAY_MS).bind(&task.id)
                .execute(&pool).await.unwrap();
        }
        let rows=list_at(&pool,now).await.unwrap();
        assert_eq!(rows.len(),REVIEW_LIMIT);
        let all:i64=sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE deleted_at IS NULL")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(all,20);
        assert!(rows.iter().all(|item|item.kind=="missing_step"));
        pool.close().await;
        let _=std::fs::remove_dir_all(dir);
    }
}
