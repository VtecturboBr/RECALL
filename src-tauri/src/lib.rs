mod db;
mod tasks;
mod inbox;

use sqlx::SqlitePool;
use tauri::{Emitter, Manager};
#[cfg(desktop)]
use tauri::menu::{Menu, MenuItem};
#[cfg(desktop)]
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};
use tauri_plugin_notification::NotificationExt;

struct AppState { pool: SqlitePool }

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
async fn save_note(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, title: String, body: String) -> Result<db::Note, String> {
    if title.chars().count() > 250 || body.chars().count() > 100_000 { return Err("Nota muito longa".into()); }
    let note = db::save(&state.pool, &id, &title, &body).await.map_err(|e| e.to_string())?;
    let _ = app.emit("notes:changed", &note.id);
    Ok(note)
}
#[tauri::command]
async fn set_note_pin(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, pinned: bool) -> Result<db::Note, String> {
    let note = db::set_pin(&state.pool, &id, pinned).await.map_err(|e| e.to_string())?;
    let _ = app.emit("notes:changed", &note.id);
    Ok(note)
}
#[tauri::command]
async fn set_note_reminder(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, remind_at: Option<i64>) -> Result<db::Note, String> {
    let note = db::set_reminder(&state.pool, &id, remind_at).await.map_err(|e| e.to_string())?;
    let _ = app.emit("notes:changed", &note.id);
    Ok(note)
}
#[tauri::command]
async fn delete_note(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String) -> Result<(), String> {
    db::delete(&state.pool, &id).await.map_err(|e| e.to_string())?;
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
async fn update_task(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, title: String, details: String, due_at: Option<i64>) -> Result<tasks::Task, String> {
    if title.trim().is_empty() || title.chars().count() > 250 || details.chars().count() > 100_000 { return Err("Tarefa inválida".into()); }
    let task = tasks::update(&state.pool, &id, &title, &details, due_at).await.map_err(|e| e.to_string())?;
    let _ = app.emit("tasks:changed", &id);
    Ok(task)
}
#[tauri::command]
async fn set_task_completed(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, completed: bool) -> Result<tasks::Task, String> {
    let task = tasks::set_completed(&state.pool, &id, completed).await.map_err(|e| e.to_string())?;
    let _ = app.emit("tasks:changed", &id);
    Ok(task)
}
#[tauri::command]
async fn delete_task(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String) -> Result<(), String> {
    tasks::delete(&state.pool, &id).await.map_err(|e| e.to_string())?;
    let _ = app.emit("tasks:changed", &id);
    Ok(())
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
async fn update_inbox(state: tauri::State<'_, AppState>, app: tauri::AppHandle, id: String, text: String) -> Result<inbox::InboxItem, String> {
    if text.trim().is_empty() || text.chars().count() > 100_000 { return Err("Escreva uma ideia válida".into()); }
    let item = inbox::update(&state.pool, &id, &text).await.map_err(|e| e.to_string())?;
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
        .invoke_handler(tauri::generate_handler![list_notes, get_note, create_note, save_note, set_note_pin, set_note_reminder, delete_note, show_capture, capture_note, list_tasks, create_task, update_task, set_task_completed, delete_task, list_inbox, capture_inbox, update_inbox, dismiss_inbox, inbox_to_note, inbox_to_task])
        .setup(|app| {
            let path = app.path().app_data_dir()?;
            std::fs::create_dir_all(&path)?;
            let db_path = path.join("memoria.sqlite");
            let pool = tauri::async_runtime::block_on(db::connect(&db_path))?;
            app.manage(AppState { pool: pool.clone() });

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

            // Poll reminders in native code, independent of the main webview.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
                loop {
                    interval.tick().await;
                    let due = match db::due(&pool).await {
                        Ok(notes) => notes,
                        Err(e) => { eprintln!("Falha ao consultar lembretes: {e}"); continue; }
                    };
                    for note in due {
                        let title = if note.title.trim().is_empty() { "Lembrete" } else { &note.title };
                        let content = if note.body.trim().is_empty() { "Você tem algo para lembrar." } else { &note.body };
                        let body: String = content.chars().take(160).collect();
                        match handle.notification().builder().title(title).body(&body).show() {
                            Ok(_) => {
                                if let Some(at) = note.remind_at { let _ = db::mark_notified(&pool, &note.id, at).await; }
                                let _ = handle.emit("notes:changed", &note.id);
                            }
                            Err(e) => eprintln!("Falha na notificação: {e}")
                        }
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // The main window is hidden, not destroyed, so the tray and reminders keep running.
            // Preserve both main and capture windows; hiding them keeps shortcuts active.
            if window.label() == "main" || window.label() == "capture" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Não foi possível iniciar o RECALL");
}
