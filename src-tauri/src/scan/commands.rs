use tauri::{AppHandle, Emitter, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::catalog::repository::BookSummary;
use crate::commands::{unavailable, with_database, CommandError};
use crate::database::Database;

use super::{
    files,
    repository::{Action, Snapshot},
    worker::EVENT,
};

fn rejected(code: &'static str, message: &'static str) -> CommandError {
    CommandError {
        code,
        message,
        retryable: true,
    }
}

#[tauri::command]
pub async fn scan_snapshot(app: AppHandle, before: Option<i64>) -> Result<Snapshot, CommandError> {
    with_database(app, move |db| Ok(db.scan_snapshot(before)?)).await
}

#[tauri::command]
pub async fn pick_scan_roots(app: AppHandle, window: WebviewWindow) -> Result<bool, CommandError> {
    let dialog_app = app.clone();
    let paths = tauri::async_runtime::spawn_blocking(move || {
        let selected = dialog_app.dialog().file().set_parent(&window)
            .set_title("Choose folders to scan").set_can_create_directories(false)
            .blocking_pick_folders();
        let Some(selected) = selected else { return Ok(Vec::new()) };
        selected.into_iter().map(|path| {
            path.into_path().ok()
                .and_then(|path| files::root_key(&path).ok())
                .and_then(|path| path.into_os_string().into_string().ok())
                .ok_or_else(|| rejected("invalid_root", "One of the selected folders could not be opened as a scan root. Choose an accessible folder."))
        }).collect::<Result<Vec<_>, _>>()
    }).await.map_err(|_| unavailable())??;
    if paths.is_empty() {
        return Ok(false);
    }
    with_database(app.clone(), move |db| Ok(db.register_roots(&paths)?)).await?;
    let _ = app.emit(EVENT, ());
    Ok(true)
}

async fn update(
    app: AppHandle,
    operation: impl FnOnce(&Database) -> rusqlite::Result<bool> + Send + 'static,
    message: &'static str,
) -> Result<(), CommandError> {
    let changed = with_database(app.clone(), move |db| Ok(operation(db)?)).await?;
    if !changed {
        return Err(rejected("scan_state_changed", message));
    }
    let _ = app.emit(EVENT, ());
    Ok(())
}

#[tauri::command]
pub async fn update_scan_root(
    app: AppHandle,
    id: i64,
    enabled: Option<bool>,
) -> Result<(), CommandError> {
    update(
        app,
        move |db| db.update_root(id, enabled),
        "The folder was removed or an import is still running.",
    )
    .await
}

#[tauri::command]
pub async fn start_scan(app: AppHandle, id: i64) -> Result<(), CommandError> {
    update(
        app,
        move |db| db.queue_scan(id),
        "Enable the folder and finish its current scan before starting another.",
    )
    .await
}

#[tauri::command]
pub async fn control_scan(app: AppHandle, id: i64, action: Action) -> Result<(), CommandError> {
    update(
        app,
        move |db| db.control_job(id, action),
        "The scan status changed. Refresh its current status and try again.",
    )
    .await
}

#[tauri::command]
pub async fn list_imported_books(
    app: AppHandle,
    before: Option<i64>,
) -> Result<Vec<BookSummary>, CommandError> {
    with_database(app, move |db| Ok(db.list_books(before, false)?)).await
}
