//! Persistent reminder queue: one active rule per note/task, with a durable
//! attention inbox. OS notifications are best-effort hints, never the only record.
use crate::db::now_ms;
use chrono::{Datelike, Duration, Local, Months, NaiveDateTime, TimeZone, Timelike};
use serde::Serialize;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Reminder {
    pub id: String,
    pub target_type: String,
    pub target_id: String,
    pub title: String,
    pub details: String,
    pub due_at: i64,
    pub anchor_at: i64,
    pub repeat_rule: String,
    pub state: String,
    pub alerted_at: Option<i64>,
    pub dispatched_at: Option<i64>,
    pub last_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ReminderEvent {
    pub id: i64,
    pub reminder_id: String,
    pub target_type: String,
    pub target_id: String,
    pub kind: String,
    pub occurred_at: i64,
    pub scheduled_for: i64,
}

const SELECT: &str = "SELECT r.id,r.target_type,r.target_id,
    CASE WHEN r.target_type='note' THEN COALESCE(NULLIF(n.title,''),'Nota sem título')
         ELSE COALESCE(t.title,'Tarefa') END AS title,
    CASE WHEN r.target_type='note' THEN COALESCE(n.body,'')
         ELSE COALESCE(t.details,'') END AS details,
    r.due_at,r.anchor_at,r.repeat_rule,r.state,r.alerted_at,r.dispatched_at,
    r.last_error,r.created_at,r.updated_at
    FROM reminder_rules r
    LEFT JOIN notes n ON r.target_type='note' AND n.id=r.target_id
    LEFT JOIN tasks t ON r.target_type='task' AND t.id=r.target_id";

const ALIVE: &str = "((r.target_type='note' AND n.id IS NOT NULL AND n.deleted_at IS NULL)
    OR (r.target_type='task' AND t.id IS NOT NULL AND t.deleted_at IS NULL))";

pub fn valid_rule(rule: &str) -> bool {
    matches!(rule, "none" | "daily" | "weekly" | "monthly")
}
pub fn valid_target(kind: &str) -> bool { matches!(kind, "note" | "task") }
pub fn valid_date(at: i64) -> bool {
    (1..=253_402_300_799_000).contains(&at)
}

pub async fn initialize(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("CREATE TABLE IF NOT EXISTS reminder_rules(
        id TEXT PRIMARY KEY NOT NULL,
        target_type TEXT NOT NULL CHECK(target_type IN ('note','task')),
        target_id TEXT NOT NULL,
        due_at INTEGER NOT NULL,
        anchor_at INTEGER NOT NULL,
        repeat_rule TEXT NOT NULL DEFAULT 'none' CHECK(repeat_rule IN ('none','daily','weekly','monthly')),
        state TEXT NOT NULL DEFAULT 'scheduled' CHECK(state IN ('scheduled','snoozed','attention','done','cancelled')),
        alerted_at INTEGER,
        dispatched_at INTEGER,
        last_error TEXT,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        UNIQUE(target_type,target_id)
    )").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_reminder_due ON reminder_rules(state,due_at)")
        .execute(pool).await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS reminder_events(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        reminder_id TEXT NOT NULL,
        target_type TEXT NOT NULL,
        target_id TEXT NOT NULL,
        kind TEXT NOT NULL,
        occurred_at INTEGER NOT NULL,
        scheduled_for INTEGER NOT NULL
    )").execute(pool).await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_reminder_events_at ON reminder_events(occurred_at DESC)")
        .execute(pool).await?;
    // Legacy note reminders are migrated without changing or removing the
    // original remind_at/notified_at columns. A previously dispatched notification
    // is shown in the attention inbox, but not re-toasted on startup.
    sqlx::query("INSERT OR IGNORE INTO reminder_rules(
        id,target_type,target_id,due_at,anchor_at,repeat_rule,state,alerted_at,
        dispatched_at,created_at,updated_at)
        SELECT 'legacy-note:' || n.id,'note',n.id,n.remind_at,n.remind_at,'none',
        CASE WHEN n.notified_at IS NULL THEN 'scheduled' ELSE 'attention' END,
        n.notified_at,n.notified_at,n.created_at,n.updated_at
        FROM notes n WHERE n.deleted_at IS NULL AND n.remind_at IS NOT NULL")
        .execute(pool).await?;
    Ok(())
}

pub async fn list(pool: &SqlitePool) -> Result<Vec<Reminder>, sqlx::Error> {
    sqlx::query_as::<_, Reminder>(&format!(
        "{SELECT} WHERE {ALIVE} AND r.state!='cancelled'
         ORDER BY CASE r.state WHEN 'attention' THEN 0
          WHEN 'snoozed' THEN 1 WHEN 'scheduled' THEN 2 ELSE 3 END,
          r.due_at ASC"
    )).fetch_all(pool).await
}

pub async fn list_events(pool: &SqlitePool) -> Result<Vec<ReminderEvent>, sqlx::Error> {
    sqlx::query_as::<_, ReminderEvent>(
        "SELECT id,reminder_id,target_type,target_id,kind,occurred_at,scheduled_for
         FROM reminder_events ORDER BY id DESC LIMIT 60"
    ).fetch_all(pool).await
}

pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Reminder>, sqlx::Error> {
    sqlx::query_as::<_, Reminder>(&format!("{SELECT} WHERE r.id=? AND {ALIVE}"))
        .bind(id).fetch_optional(pool).await
}
pub async fn find(pool: &SqlitePool, kind: &str, target_id: &str) -> Result<Option<Reminder>, sqlx::Error> {
    sqlx::query_as::<_, Reminder>(&format!("{SELECT} WHERE r.target_type=? AND r.target_id=? AND {ALIVE}"))
        .bind(kind).bind(target_id).fetch_optional(pool).await
}

pub async fn schedule(
    pool: &SqlitePool, kind: &str, target_id: &str, due_at: i64, rule: &str,
) -> Result<Reminder, sqlx::Error> {
    let now = now_ms();
    let mut tx = pool.begin().await?;
    let found: i64 = if kind == "note" {
        sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE id=? AND deleted_at IS NULL")
            .bind(target_id).fetch_one(&mut *tx).await?
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM tasks WHERE id=? AND deleted_at IS NULL AND completed_at IS NULL")
            .bind(target_id).fetch_one(&mut *tx).await?
    };
    if found == 0 { return Err(sqlx::Error::RowNotFound); }
    sqlx::query("INSERT INTO reminder_rules(
        id,target_type,target_id,due_at,anchor_at,repeat_rule,state,
        alerted_at,dispatched_at,last_error,created_at,updated_at
    ) VALUES(?,?,?,?,?,'none','scheduled',NULL,NULL,NULL,?,?)
    ON CONFLICT(target_type,target_id) DO UPDATE SET
       due_at=excluded.due_at,anchor_at=excluded.anchor_at,
       state='scheduled',alerted_at=NULL,dispatched_at=NULL,last_error=NULL,
       updated_at=excluded.updated_at")
        .bind(Uuid::new_v4().to_string()).bind(kind).bind(target_id)
        .bind(due_at).bind(due_at).bind(now).bind(now)
        .execute(&mut *tx).await?;
    sqlx::query("UPDATE reminder_rules SET repeat_rule=? WHERE target_type=? AND target_id=?")
        .bind(rule).bind(kind).bind(target_id).execute(&mut *tx).await?;
    if kind == "note" {
        // Keep legacy fields synchronized, including when an older version starts.
        sqlx::query("UPDATE notes SET remind_at=?,notified_at=NULL WHERE id=?")
            .bind(due_at).bind(target_id).execute(&mut *tx).await?;
    }
    let row = sqlx::query_as::<_, (String, i64)>(
        "SELECT id,anchor_at FROM reminder_rules WHERE target_type=? AND target_id=?"
    ).bind(kind).bind(target_id).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO reminder_events(reminder_id,target_type,target_id,kind,occurred_at,scheduled_for)
        VALUES(?,?,?,'scheduled',?,?)")
        .bind(&row.0).bind(kind).bind(target_id).bind(now).bind(row.1)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    find(pool, kind, target_id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub async fn cancel(
    pool: &SqlitePool, kind: &str, target_id: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    let existing = sqlx::query_as::<_, (String, i64)>(
        "SELECT id,anchor_at FROM reminder_rules WHERE target_type=? AND target_id=?"
    ).bind(kind).bind(target_id).fetch_optional(&mut *tx).await?;
    if let Some((id, anchor)) = existing {
        sqlx::query("UPDATE reminder_rules SET state='cancelled',updated_at=?,last_error=NULL
            WHERE id=?")
            .bind(now_ms()).bind(&id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO reminder_events(reminder_id,target_type,target_id,kind,occurred_at,scheduled_for)
             VALUES(?,?,?,'cancelled',?,?)")
            .bind(id).bind(kind).bind(target_id).bind(now_ms()).bind(anchor)
            .execute(&mut *tx).await?;
    }
    if kind == "note" {
        sqlx::query("UPDATE notes SET remind_at=NULL,notified_at=NULL WHERE id=? AND deleted_at IS NULL")
            .bind(target_id).execute(&mut *tx).await?;
    }
    tx.commit().await
}

pub async fn complete_task(pool: &SqlitePool, id: &str) -> Result<(), sqlx::Error> {
    // Task completion hides any pending reminder, without deleting its history.
    cancel(pool, "task", id).await
}

/// Atomically claim a bounded set of overdue occurrences. Once claimed, each
/// remains visible in the attention inbox even if toast dispatch fails or the
/// process exits. No automatic re-toast on app restart.
pub async fn claim_due(pool: &SqlitePool, at: i64) -> Result<Vec<Reminder>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let rows = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT r.id,r.target_type,r.target_id,r.anchor_at
         FROM reminder_rules r
         LEFT JOIN notes n ON r.target_type='note' AND n.id=r.target_id
         LEFT JOIN tasks t ON r.target_type='task' AND t.id=r.target_id
         WHERE r.state IN ('scheduled','snoozed') AND r.due_at<=?
           AND ((r.target_type='note' AND n.deleted_at IS NULL AND n.id IS NOT NULL)
           OR (r.target_type='task' AND t.deleted_at IS NULL AND t.completed_at IS NULL AND t.id IS NOT NULL))
         ORDER BY r.due_at LIMIT 150"
    ).bind(at).fetch_all(&mut *tx).await?;
    for (id,kind,target,anchor) in &rows {
        sqlx::query("UPDATE reminder_rules SET state='attention',alerted_at=?,updated_at=?
            WHERE id=? AND state IN ('scheduled','snoozed')")
            .bind(at).bind(at).bind(id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO reminder_events(reminder_id,target_type,target_id,kind,occurred_at,scheduled_for)
            VALUES(?,?,?,'attention',?,?)")
            .bind(id).bind(kind).bind(target).bind(at).bind(anchor)
            .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    let mut out = Vec::new();
    for (id,_,_,_) in rows {
        if let Some(row) = get(pool, &id).await? { out.push(row); }
    }
    Ok(out)
}

/// This records OS API acceptance, NOT a guarantee of display or user reading.
pub async fn record_dispatch(
    pool: &SqlitePool, ids: &[String], at: i64, error: Option<&str>,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;
    for id in ids {
        // Read through this transaction: the pool has one connection, so
        // calling get(pool, ...) inside it would deadlock.
        let row = sqlx::query_as::<_, (String,String,i64,String,Option<i64>)>(
            "SELECT target_type,target_id,anchor_at,state,dispatched_at
             FROM reminder_rules WHERE id=?"
        ).bind(id).fetch_optional(&mut *tx).await?;
        if let Some((kind, target, anchor, state, dispatched)) = row {
            if state != "attention" || dispatched.is_some() { continue; }
            sqlx::query("UPDATE reminder_rules SET dispatched_at=?,last_error=?,updated_at=? WHERE id=? AND state='attention'")
                .bind(if error.is_none(){Some(at)}else{None})
                .bind(error).bind(at).bind(id).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO reminder_events(
                reminder_id,target_type,target_id,kind,occurred_at,scheduled_for
            ) VALUES(?,?,?,?,?,?)")
                .bind(id).bind(&kind).bind(&target)
                .bind(if error.is_some() {"dispatch_failed"} else {"dispatched"})
                .bind(at).bind(anchor).execute(&mut *tx).await?;
            if error.is_none() && kind == "note" {
                sqlx::query("UPDATE notes SET notified_at=? WHERE id=? AND deleted_at IS NULL")
                    .bind(at).bind(&target).execute(&mut *tx).await?;
            }
        }
    }
    tx.commit().await
}

/// Snooze the same occurrence, without shifting its original recurrence anchor.
pub async fn snooze(
    pool: &SqlitePool, id: &str, minutes: i64, at: i64,
) -> Result<Reminder, sqlx::Error> {
    let next = at.checked_add(minutes.checked_mul(60_000).ok_or(sqlx::Error::RowNotFound)?)
        .ok_or(sqlx::Error::RowNotFound)?;
    let mut tx = pool.begin().await?;
    let changed = sqlx::query(
        "UPDATE reminder_rules SET state='snoozed',due_at=?,alerted_at=NULL,
         dispatched_at=NULL,last_error=NULL,updated_at=? WHERE id=? AND state='attention'"
    ).bind(next).bind(at).bind(id).execute(&mut *tx).await?;
    if changed.rows_affected() == 0 { return Err(sqlx::Error::RowNotFound); }
    // Log using a snapshot before commit.
    let info = sqlx::query_as::<_, (String,String,i64)>(
        "SELECT target_type,target_id,anchor_at FROM reminder_rules WHERE id=?"
    ).bind(id).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO reminder_events(reminder_id,target_type,target_id,kind,occurred_at,scheduled_for)
        VALUES(?,?,?,'snoozed',?,?)")
        .bind(id).bind(&info.0).bind(&info.1).bind(at).bind(info.2).execute(&mut *tx).await?;
    if info.0 == "note" {
        sqlx::query("UPDATE notes SET remind_at=?,notified_at=NULL WHERE id=?")
            .bind(next).bind(&info.1).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    get(pool,id).await?.ok_or(sqlx::Error::RowNotFound)
}

pub fn next_occurrence(anchor_at: i64, rule: &str, now: i64) -> Option<i64> {
    if !valid_rule(rule) || rule == "none" { return None; }
    let original = Local.timestamp_millis_opt(anchor_at).single()?;
    let current = Local.timestamp_millis_opt(now).single()?;
    let base = original.naive_local();
    let days_between = (current.date_naive() - base.date()).num_days().max(0);
    let mut step = match rule {
        "daily" => days_between as u32,
        "weekly" => (days_between / 7) as u32,
        "monthly" => ((current.year() - original.year()) * 12
            + current.month() as i32 - original.month() as i32).max(0) as u32,
        _ => return None,
    }.max(1);
    for _ in 0..500 {
        let date = match rule {
            "daily" => base.date().checked_add_signed(Duration::days(step as i64))?,
            "weekly" => base.date().checked_add_signed(Duration::days(step as i64 * 7))?,
            "monthly" => base.date().checked_add_months(Months::new(step))?,
            _ => return None,
        };
        let candidate: NaiveDateTime = date.and_time(base.time());
        // In DST gaps, shift forward to the next valid local time.
        for shift in 0..=3 {
            let local = candidate + Duration::hours(shift);
            if let Some(dt) = Local.from_local_datetime(&local).earliest() {
                if dt.timestamp_millis() > now { return Some(dt.timestamp_millis()); }
                break;
            }
        }
        step = step.checked_add(1)?;
    }
    None
}

/// Mark the current occurrence acknowledged. Recurrence advances only after
/// explicit acknowledgment, skipping periods that elapsed while offline.
pub async fn acknowledge(
    pool: &SqlitePool, id: &str, at: i64,
) -> Result<Reminder, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let old = sqlx::query_as::<_, (String,String,i64,String,String)>(
        "SELECT target_type,target_id,anchor_at,repeat_rule,state FROM reminder_rules WHERE id=?"
    ).bind(id).fetch_optional(&mut *tx).await?.ok_or(sqlx::Error::RowNotFound)?;
    if old.4 != "attention" { return Err(sqlx::Error::RowNotFound); }
    let next = next_occurrence(old.2,&old.3,at);
    let state = if next.is_some() {"scheduled"} else {"done"};
    let next_due = next.unwrap_or(old.2);
    sqlx::query("UPDATE reminder_rules SET state=?,due_at=?,anchor_at=?,
        alerted_at=NULL,dispatched_at=NULL,last_error=NULL,updated_at=?
        WHERE id=? AND state='attention'")
        .bind(state).bind(next_due).bind(next_due).bind(at).bind(id)
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO reminder_events(reminder_id,target_type,target_id,kind,occurred_at,scheduled_for)
        VALUES(?,?,?,'acknowledged',?,?)")
        .bind(id).bind(&old.0).bind(&old.1).bind(at).bind(old.2)
        .execute(&mut *tx).await?;
    if old.0 == "note" {
        sqlx::query("UPDATE notes SET remind_at=?,notified_at=? WHERE id=? AND deleted_at IS NULL")
            .bind(next).bind(if next.is_some() {None} else {Some(at)})
            .bind(&old.1).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    get(pool,id).await?.ok_or(sqlx::Error::RowNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, tasks};
    use chrono::{LocalResult, TimeZone};
    use std::{fs, path::PathBuf};

    fn folder() -> PathBuf {
        let d=std::env::temp_dir().join(format!("recall-reminders-{}",Uuid::new_v4()));
        fs::create_dir_all(&d).unwrap(); d
    }

    #[test] fn recurrence_skips_overdue_days() {
        let base=Local.with_ymd_and_hms(2026,11,12,15,30,0).single().unwrap().timestamp_millis();
        let now=Local.with_ymd_and_hms(2026,11,21,18,0,0).single().unwrap().timestamp_millis();
        let next=next_occurrence(base,"daily",now).unwrap();
        let next_local=Local.timestamp_millis_opt(next).single().unwrap();
        assert_eq!((next_local.day(), next_local.month()),(22,11));
        assert_eq!((next_local.hour(),next_local.minute()),(15,30));
        assert!(next_occurrence(base,"none",now).is_none());
    }

    #[test] fn recurrence_handles_month_end() {
        let base=Local.with_ymd_and_hms(2026,1,31,10,0,0).single().unwrap().timestamp_millis();
        let now=Local.with_ymd_and_hms(2026,2,2,11,0,0).single().unwrap().timestamp_millis();
        let next=Local.timestamp_millis_opt(next_occurrence(base,"monthly",now).unwrap()).single().unwrap();
        assert_eq!((next.month(),next.day(),next.hour()),(2,28,10));
        let next_after=Local.with_ymd_and_hms(2026,2,28,12,0,0).single().unwrap().timestamp_millis();
        let following=Local.timestamp_millis_opt(next_occurrence(base,"monthly",next_after).unwrap()).single().unwrap();
        assert_eq!((following.month(),following.day()),(3,31));
        assert_eq!(Local.from_local_datetime(&following.naive_local()),LocalResult::Single(following));
    }

    #[tokio::test]
    async fn legacy_note_recovered_without_duplicate_toast() {
        let dir=folder();
        let path=dir.join("memoria.sqlite");
        let pool=db::connect(&path).await.unwrap();
        let note=db::create(&pool,"note").await.unwrap();
        db::set_reminder(&pool,&note.id,Some(1000)).await.unwrap();
        db::mark_notified(&pool,&note.id,1000).await.unwrap();
        initialize(&pool).await.unwrap();
        let reminder=find(&pool,"note",&note.id).await.unwrap().unwrap();
        assert_eq!(reminder.state,"attention");
        assert_eq!(reminder.dispatched_at,db::get(&pool,&note.id).await.unwrap().unwrap().notified_at);
        assert!(claim_due(&pool,now_ms()).await.unwrap().is_empty());
        pool.close().await; let _=fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn claim_snooze_ack_and_recurrence_persist() {
        let dir=folder();
        let path=dir.join("memoria.sqlite");
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Conferir entrega").await.unwrap();
        let now=now_ms();
        let row=schedule(&pool,"task",&task.id,now-1000,"daily").await.unwrap();
        let first=claim_due(&pool,now).await.unwrap();
        assert_eq!(first.len(),1);
        assert_eq!(first[0].state,"attention");
        assert!(claim_due(&pool,now).await.unwrap().is_empty(),"no duplicate alert");
        let snoozed=snooze(&pool,&row.id,15,now).await.unwrap();
        assert_eq!(snoozed.state,"snoozed");
        assert_eq!(snoozed.due_at,now+900_000);
        assert!(claim_due(&pool,now+899_999).await.unwrap().is_empty());
        assert_eq!(claim_due(&pool,now+900_000).await.unwrap().len(),1);
        let next=acknowledge(&pool,&row.id,now+900_000).await.unwrap();
        assert_eq!(next.state,"scheduled");
        assert!(next.due_at>now+900_000);
        assert_eq!(list_events(&pool).await.unwrap().len(),5);
        pool.close().await;
        let pool2=db::connect(&path).await.unwrap();
        assert_eq!(find(&pool2,"task",&task.id).await.unwrap().unwrap().state,"scheduled");
        pool2.close().await; let _=fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn completed_task_never_triggers() {
        let dir=folder();let path=dir.join("memoria.sqlite");
        let pool=db::connect(&path).await.unwrap();
        let task=tasks::create(&pool,"Concluída").await.unwrap();
        schedule(&pool,"task",&task.id,now_ms()-1000,"none").await.unwrap();
        tasks::set_completed(&pool,&task.id,true).await.unwrap();
        assert!(claim_due(&pool,now_ms()).await.unwrap().is_empty());
        pool.close().await;let _=fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn migration_preserves_notes_tasks_and_reminder_legacy_columns() {
        let dir=folder();let path=dir.join("memoria.sqlite");
        let pool=db::connect(&path).await.unwrap();
        let note=db::create(&pool,"note").await.unwrap();
        db::set_reminder(&pool,&note.id,Some(now_ms()+500000)).await.unwrap();
        let task=tasks::create(&pool,"Antiga tarefa").await.unwrap();
        initialize(&pool).await.unwrap();
        assert_eq!(find(&pool,"note",&note.id).await.unwrap().unwrap().state,"scheduled");
        assert_eq!(db::get(&pool,&note.id).await.unwrap().unwrap().remind_at.is_some(),true);
        assert!(tasks::get(&pool,&task.id).await.unwrap().is_some());
        pool.close().await;let _=fs::remove_dir_all(dir);
    }
}
