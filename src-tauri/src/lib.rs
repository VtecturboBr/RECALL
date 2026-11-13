mod db;
mod tasks;
mod inbox;
mod reminders;
mod history;
mod focus;

use sqlx::SqlitePool;
use tauri::{Emitter, Manager};
#[cfg(desktop)]
use tauri::menu::{Menu, MenuItem};
#[cfg(desktop)]
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use tauri_plugin_notification::NotificationExt;

struct AppState { pool: SqlitePool, db_path: std::path::PathBuf }

fn reveal_capture(app: &tauri::AppHandle) -> Result<(), String> {
    let window = app.get_webview_window("capture")
        .ok_or_else(|| "Janela de captura não encontrada".to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    let _ = app.emit_to("capture", "quick-capture:opened", ());
    Ok(())
}

#[tauri::command]
fn show_capture(app: tauri::AppHandle) -> Result<(), String> {
    reveal_capture(&app)
}

#[tauri::command]
async fn capture_note(state: tauri::State<'_, AppState>, app: tauri::AppHandle, text: String) -> Result<db::Note, String> {
    if text.trim().is_empty() { return Err("Escreva alguma coisa antes de salvar".into()); }
    if text.chars().count() > 100_000 { return Err("A anotação excedeu o limite de 100 mil caracteres".into()); }
    let note = db::capture(&state.pool, &text).await.map_err(|e| e.to_string())?;
    let _ = app.emit("notes:changed", &note.id);
    Ok(note)
}


#[tauri::command]
async fn list_notes(state: tauri::State<'_, AppState>) -> Result<Vec<db::Note>, String> {
    db::list(&state.pool).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn get_note(state: tauri::State<'_, AppState>, id: String) -> Result<Option<db::Note>, String> {
    db::get(&state.pool, &id).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn create_note(state: tauri::State<'_, AppState>, app: tauri::AppHandle, kind: String) -> Result<db::Note, String> {
    if kind != "note" && kind != "presentation" { return Err("Tipo de nota inválido".into()); }
    let note = db::create(&state.pool, &kind).await.map_err(|e| e.to_string())?;
    let _ = app.emit("notes:changed", &note.id);
    Ok(note)
}
#[tauri::command]
async fn save_note(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, title: String, body: String, expected_revision: i64) -> Result<db::Note, String> {
    if title.chars().count() > 250 || body.chars().count() > 100_000 { return Err("Nota muito longa".into()); }
    if expected_revision < 0 { return Err("Revisão inválida".into()); }
    let note = db::save(&state.pool, &id, &title, &body, expected_revision).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "CONFLITO_REVISAO: A nota foi alterada em outra janela ou excluída. Sua edição não foi sobrescrita.".to_string())?;
    let _ = app.emit("notes:changed", &note.id);
    Ok(note)
}

#[tauri::command]
async fn create_note_copy(state: tauri::State<'_, AppState>, app: tauri::AppHandle, title: String, body: String, kind: String) -> Result<db::Note, String> {
    if title.chars().count() > 250 || body.chars().count() > 100_000 || (kind != "note" && kind != "presentation") {
        return Err("Conteúdo de cópia inválido".into());
    }
    let note = db::create_copy(&state.pool, &title, &body, &kind).await.map_err(|e| e.to_string())?;
    let _ = app.emit("notes:changed", &note.id);
    Ok(note)
}

#[tauri::command]
async fn create_data_backup(state: tauri::State<'_, AppState>) -> Result<db::BackupInfo, String> {
    db::create_backup(&state.pool, &state.db_path, "manual").await
}

#[tauri::command]
async fn list_data_backups(state: tauri::State<'_, AppState>) -> Result<Vec<db::BackupInfo>, String> {
    db::list_backups(&state.db_path)
}
#[tauri::command]
async fn set_note_pin(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, pinned: bool) -> Result<db::Note, String> {
    let note = db::set_pin(&state.pool, &id, pinned).await.map_err(|e| e.to_string())?;
    let _ = app.emit("notes:changed", &note.id);
    Ok(note)
}
#[tauri::command]
async fn set_note_reminder(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, remind_at: Option<i64>) -> Result<db::Note, String> {
    if let Some(at) = remind_at {
        if !reminders::valid_date(at) { return Err("Data inválida".into()); }
        reminders::schedule(&state.pool, "note", &id, at, "none")
            .await.map_err(|e| e.to_string())?;
    } else {
        reminders::cancel(&state.pool, "note", &id).await.map_err(|e| e.to_string())?;
    }
    let note = db::get(&state.pool, &id).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Nota não encontrada".to_string())?;
    let _ = app.emit("notes:changed", &id);
    let _ = app.emit("reminders:changed", &id);
    Ok(note)
}
#[tauri::command]
async fn delete_note(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String) -> Result<(), String> {
    db::delete(&state.pool, &id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("reminders:changed", &id);
    let _ = app.emit("notes:changed", &id);
    Ok(())
}

#[tauri::command]
async fn list_tasks(state: tauri::State<'_, AppState>) -> Result<Vec<tasks::Task>, String> {
    tasks::list(&state.pool).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn create_task(state: tauri::State<'_, AppState>, app: tauri::AppHandle, title: String) -> Result<tasks::Task, String> {
    if title.trim().is_empty() || title.chars().count() > 250 { return Err("Título inválido".into()); }
    let task = tasks::create(&state.pool, &title).await.map_err(|e| e.to_string())?;
    let _ = app.emit("tasks:changed", &task.id);
    Ok(task)
}
#[tauri::command]
async fn update_task(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, title: String, details: String, due_at: Option<i64>, expected_revision: i64) -> Result<tasks::Task, String> {
    if title.trim().is_empty() || title.chars().count() > 250 || details.chars().count() > 100_000 { return Err("Tarefa inválida".into()); }
    if expected_revision < 0 { return Err("Revisão inválida".into()); }
    let task = tasks::update(&state.pool, &id, &title, &details, due_at, expected_revision).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "CONFLITO_REVISAO: A tarefa mudou em outra janela. Salve uma cópia para preservar ambas as versões.".to_string())?;
    let _ = app.emit("tasks:changed", &id);
    Ok(task)
}
#[tauri::command]
async fn get_active_focus(state: tauri::State<'_, AppState>) -> Result<Option<focus::FocusSession>, String> {
    focus::active(&state.pool).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn list_focus_sessions(
    state: tauri::State<'_, AppState>, task_id: String,
) -> Result<Vec<focus::FocusSession>, String> {
    focus::history(&state.pool, &task_id).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn start_focus_session(
    state: tauri::State<'_, AppState>, app: tauri::AppHandle, task_id: String,
) -> Result<focus::FocusSession, String> {
    let session = focus::start(&state.pool, &task_id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("focus:changed", &session.task_id);
    Ok(session)
}
#[tauri::command]
async fn heartbeat_focus_session(
    state: tauri::State<'_, AppState>, id: String,
) -> Result<focus::FocusSession, String> {
    focus::heartbeat(&state.pool, &id).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn stop_focus_session(
    state: tauri::State<'_, AppState>, app: tauri::AppHandle,
    id: String, reason: String, summary: String,
) -> Result<focus::FocusSession, String> {
    let session=focus::stop(&state.pool, &id, &reason, &summary).await.map_err(|e| e.to_string())?;
    let _ = app.emit("focus:changed", &session.task_id);
    Ok(session)
}
#[tauri::command]
async fn get_task_resume(
    state: tauri::State<'_, AppState>, task_id: String,
) -> Result<Option<tasks::TaskResume>, String> {
    tasks::get_resume(&state.pool, &task_id).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn save_task_resume(
    state: tauri::State<'_, AppState>, app: tauri::AppHandle,
    task_id: String, last_context: String, next_step: String, expected_revision: i64,
) -> Result<tasks::TaskResume, String> {
    if expected_revision < 0 || last_context.chars().count() > 15_000 || next_step.chars().count() > 2_000 {
        return Err("Ponto de retomada inválido".into());
    }
    let saved = tasks::save_resume(
        &state.pool, &task_id, &last_context, &next_step, expected_revision
    ).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "CONFLITO_REVISAO: O ponto de retomada mudou em outra janela ou a tarefa foi excluída.".to_string())?;
    let _ = app.emit("tasks:resume-changed", &task_id);
    Ok(saved)
}
#[tauri::command]
async fn set_task_completed(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, completed: bool) -> Result<tasks::Task, String> {
    let task = tasks::set_completed(&state.pool, &id, completed).await.map_err(|e| e.to_string())?;
    if completed { let _ = app.emit("reminders:changed", &id); }
    let _ = app.emit("tasks:changed", &id);
    if completed { let _ = app.emit("focus:changed", &id); }
    Ok(task)
}
#[tauri::command]
async fn delete_task(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String) -> Result<(), String> {
    tasks::delete(&state.pool, &id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("reminders:changed", &id);
    let _ = app.emit("tasks:changed", &id);
    let _ = app.emit("focus:changed", &id);
    Ok(())
}
#[tauri::command]
async fn create_planned_task(
    state: tauri::State<'_, AppState>, app: tauri::AppHandle,
    title: String, details: String, bucket: String,
    planned_for: Option<i64>, due_at: Option<i64>,
) -> Result<tasks::Task, String> {
    if title.trim().is_empty() || title.chars().count() > 250 || details.chars().count() > 100_000
        || !tasks::valid_plan(&bucket, planned_for) || due_at.map(|at| at < 0).unwrap_or(false) {
        return Err("Dados de planejamento inválidos".into());
    }
    let task = tasks::create_planned(&state.pool, &title, &details, &bucket, planned_for, due_at)
        .await.map_err(|e| e.to_string())?;
    let _ = app.emit("tasks:changed", &task.id);
    Ok(task)
}
#[tauri::command]
async fn set_task_plan(
    state: tauri::State<'_, AppState>, app: tauri::AppHandle,
    id: String, bucket: String, planned_for: Option<i64>,
) -> Result<tasks::Task, String> {
    if !tasks::valid_plan(&bucket, planned_for) {
        return Err("Planejamento inválido".into());
    }
    let task = tasks::set_plan(&state.pool, &id, &bucket, planned_for)
        .await.map_err(|e| e.to_string())?;
    let _ = app.emit("tasks:changed", &id);
    Ok(task)
}

#[tauri::command]
async fn list_reminders(state: tauri::State<'_, AppState>) -> Result<Vec<reminders::Reminder>, String> {
    reminders::list(&state.pool).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn list_reminder_events(state: tauri::State<'_, AppState>) -> Result<Vec<reminders::ReminderEvent>, String> {
    reminders::list_events(&state.pool).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn schedule_reminder(
    state: tauri::State<'_, AppState>, app: tauri::AppHandle,
    target_type: String, target_id: String, due_at: i64, repeat_rule: String,
) -> Result<reminders::Reminder, String> {
    if !reminders::valid_target(&target_type) || !reminders::valid_rule(&repeat_rule)
        || !reminders::valid_date(due_at) {
        return Err("Dados de lembrete inválidos".into());
    }
    let result = reminders::schedule(&state.pool, &target_type, &target_id, due_at, &repeat_rule)
        .await.map_err(|e| e.to_string())?;
    let _ = app.emit("reminders:changed", &result.id);
    if target_type == "note" { let _ = app.emit("notes:changed", &target_id); }
    Ok(result)
}
#[tauri::command]
async fn cancel_reminder(
    state: tauri::State<'_, AppState>, app: tauri::AppHandle,
    target_type: String, target_id: String,
) -> Result<(), String> {
    if !reminders::valid_target(&target_type) { return Err("Tipo inválido".into()); }
    reminders::cancel(&state.pool, &target_type, &target_id)
        .await.map_err(|e| e.to_string())?;
    let _ = app.emit("reminders:changed", &target_id);
    if target_type == "note" { let _ = app.emit("notes:changed", &target_id); }
    Ok(())
}
#[tauri::command]
async fn snooze_reminder(
    state: tauri::State<'_, AppState>, app: tauri::AppHandle,
    id: String, minutes: i64,
) -> Result<reminders::Reminder, String> {
    if !matches!(minutes, 5 | 15 | 30 | 60 | 180 | 1440) {
        return Err("Escolha um tempo de adiamento válido".into());
    }
    let result = reminders::snooze(&state.pool, &id, minutes, db::now_ms())
        .await.map_err(|e| e.to_string())?;
    let _ = app.emit("reminders:changed", &result.id);
    if result.target_type == "note" { let _ = app.emit("notes:changed", &result.target_id); }
    Ok(result)
}
#[tauri::command]
async fn acknowledge_reminder(
    state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String,
) -> Result<reminders::Reminder, String> {
    let result = reminders::acknowledge(&state.pool, &id, db::now_ms())
        .await.map_err(|e| e.to_string())?;
    let _ = app.emit("reminders:changed", &result.id);
    if result.target_type == "note" { let _ = app.emit("notes:changed", &result.target_id); }
    Ok(result)
}

#[tauri::command]
async fn list_inbox(state: tauri::State<'_, AppState>) -> Result<Vec<inbox::InboxItem>, String> {
    inbox::list(&state.pool).await.map_err(|e| e.to_string())
}
#[tauri::command]
async fn capture_inbox(state: tauri::State<'_, AppState>, app: tauri::AppHandle, text: String) -> Result<inbox::InboxItem, String> {
    if text.trim().is_empty() || text.chars().count() > 100_000 { return Err("Escreva uma ideia válida".into()); }
    let item = inbox::capture(&state.pool, &text).await.map_err(|e| e.to_string())?;
    let _ = app.emit("inbox:changed", &item.id);
    Ok(item)
}
#[tauri::command]
async fn update_inbox(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, text: String, expected_revision: i64) -> Result<inbox::InboxItem, String> {
    if text.trim().is_empty() || text.chars().count() > 100_000 { return Err("Escreva uma ideia válida".into()); }
    if expected_revision < 0 { return Err("Revisão inválida".into()); }
    let item = inbox::update(&state.pool, &id, &text, expected_revision).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "CONFLITO_REVISAO: A ideia mudou em outra janela. Salve uma cópia para preservar ambas as versões.".to_string())?;
    let _ = app.emit("inbox:changed", &id);
    Ok(item)
}
#[tauri::command]
async fn dismiss_inbox(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String) -> Result<(), String> {
    inbox::dismiss(&state.pool, &id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("inbox:changed", &id);
    Ok(())
}
#[tauri::command]
async fn inbox_to_note(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String) -> Result<db::Note, String> {
    let note = inbox::to_note(&state.pool, &id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("inbox:changed", &id);
    let _ = app.emit("notes:changed", &note.id);
    Ok(note)
}
#[tauri::command]
async fn inbox_to_task(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String) -> Result<tasks::Task, String> {
    let task = inbox::to_task(&state.pool, &id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("inbox:changed", &id);
    let _ = app.emit("tasks:changed", &task.id);
    Ok(task)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![list_notes, get_note, create_note, save_note, create_note_copy, create_data_backup, list_data_backups, set_note_pin, set_note_reminder, delete_note, show_capture, capture_note, list_tasks, create_task, create_planned_task, set_task_plan, update_task, get_task_resume, save_task_resume,
            get_active_focus, list_focus_sessions, start_focus_session, heartbeat_focus_session, stop_focus_session,
            set_task_completed, delete_task, list_inbox, capture_inbox, update_inbox, dismiss_inbox, inbox_to_note, inbox_to_task,
            list_reminders, list_reminder_events, schedule_reminder, cancel_reminder,
            snooze_reminder, acknowledge_reminder])
        .setup(|app| {
            let path = app.path().app_data_dir()?;
            std::fs::create_dir_all(&path)?;
            let db_path = path.join("memoria.sqlite");
            let pool = tauri::async_runtime::block_on(db::connect(&db_path))?;
            app.manage(AppState { pool: pool.clone(), db_path: db_path.clone() });

            #[cfg(desktop)] {
                use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
                let capture_shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space);
                app.handle().plugin(
                    tauri_plugin_global_shortcut::Builder::new()
                        .with_handler(move |app, shortcut, event| {
                            if shortcut == &capture_shortcut && event.state() == ShortcutState::Pressed {
                                if let Err(e) = reveal_capture(app) { eprintln!("Falha ao abrir captura: {e}"); }
                            }
                        }).build(),
                )?;
                // Conflicting shortcuts must not prevent the application from starting.
                if let Err(e) = app.global_shortcut().register(capture_shortcut) {
                    eprintln!("Atalho Ctrl+Shift+Espaço indisponível: {e}");
                }

                let capture_item = MenuItem::with_id(app, "capture", "Capturar ideia  (Ctrl+Shift+Espaço)", true, None::<&str>)?;
                let show_item = MenuItem::with_id(app, "open", "Abrir RECALL", true, None::<&str>)?;
                let exit_item = MenuItem::with_id(app, "quit", "Sair do aplicativo", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&capture_item, &show_item, &exit_item])?;
                let mut builder = TrayIconBuilder::new()
                    .tooltip("RECALL — lembretes ativos")
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "capture" => { if let Err(e) = reveal_capture(app) { eprintln!("Falha ao abrir captura: {e}"); } }
                        "open" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {button: MouseButton::Left, button_state: MouseButtonState::Up, ..} = event {
                            let app = tray.app_handle();
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    });
                if let Some(icon) = app.default_window_icon() {
                    builder = builder.icon(icon.clone());
                }
                builder.build(app)?;
            }

            // Native scheduler survives hidden windows. It runs on startup,
            // recovering overdue reminders into the persistent attention inbox.
            // While the EXE or computer is off, Windows toasts cannot be sent.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut last_bulk_toast_at: i64 = 0;
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
                loop {
                    interval.tick().await;
                    let now = db::now_ms();
                    let pending_count = reminders::count_due(&pool, now).await.unwrap_or(0);
                    let due = match reminders::claim_due(&pool, now).await {
                        Ok(items) => items,
                        Err(e) => { eprintln!("Falha ao verificar lembretes: {e}"); continue; }
                    };
                    if due.is_empty() { continue; }
                    let _ = handle.emit("reminders:changed", ());
                    // After a long offline period, avoid a flood of individual toasts:
                    // one short summary for a batch, while each item stays in the inbox.
                    if due.len() > 3 || due.iter().any(|r| r.due_at < now - 6 * 60 * 60 * 1000) {
                        let ids: Vec<String> = due.iter().map(|r| r.id.clone()).collect();
                        if now - last_bulk_toast_at < 60 * 60 * 1000 {
                            // Already covered by the earlier summary: preserve the
                            // durable attention state and record the suppressed hint.
                            if let Err(e) = reminders::record_suppressed(&pool, &ids, now).await {
                                eprintln!("Falha ao registrar resumo agrupado: {e}");
                            }
                            continue;
                        }
                        last_bulk_toast_at = now;
                        // The first summary covers the whole backlog, not only LIMIT 150.
                        let body = format!("{} lembrete(s) precisam da sua atenção. Abra o RECALL.",
                            pending_count.max(due.len() as i64));
                        let result = handle.notification().builder()
                            .title("RECALL — lembretes pendentes").body(&body).show();
                        let error = result.as_ref().err().map(|e| e.to_string());
                        if let Err(e) = reminders::record_dispatch(&pool, &ids, now, error.as_deref()).await {
                            eprintln!("Erro ao registrar despacho de lembretes: {e}");
                        }
                    } else {
                        for item in due {
                            // Windows notifications may appear on the locked screen:
                            // never expose personal note/task titles or bodies by default.
                            let result = handle.notification().builder()
                                .title("RECALL — lembrete")
                                .body("Você tem algo para lembrar. Abra o RECALL.").show();
                            let error = result.as_ref().err().map(|e| e.to_string());
                            if let Err(e) = reminders::record_dispatch(
                                &pool, &[item.id.clone()], now, error.as_deref(),
                            ).await {
                                eprintln!("Falha ao salvar estado da notificação: {e}");
                            }
                        }
                    }
                    let _ = handle.emit("reminders:changed", ());
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // The main window is hidden, not destroyed, so the tray and reminders keep running.
            // Preserve both main and capture windows; hiding them keeps shortcuts active.
            if window.label() == "main" || window.label() == "capture" || window.label().starts_with("note_") {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Não foi possível iniciar o RECALL");
}
