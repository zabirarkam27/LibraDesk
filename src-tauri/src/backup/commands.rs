use super::archive::{self, Summary};
use crate::commands::{unavailable, CommandError};
use tauri::{AppHandle, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
fn failure(error: archive::Error) -> CommandError {
    CommandError {
        code: "backup_failed",
        message: error.message(),
        retryable: true,
    }
}
#[tauri::command]
pub async fn create_backup(
    app: AppHandle,
    window: WebviewWindow,
    include_thumbnails: bool,
    choose_destination: bool,
) -> Result<Option<Summary>, CommandError> {
    let root = crate::storage_management::root(&app)?;
    let parent = if choose_destination {
        let picker = app.clone();
        let selected = tauri::async_runtime::spawn_blocking(move || {
            picker
                .dialog()
                .file()
                .set_parent(&window)
                .set_title("Choose backup destination")
                .blocking_pick_folder()
        })
        .await
        .map_err(|_| unavailable())?;
        let Some(selected) = selected else {
            return Ok(None);
        };
        selected.into_path().map_err(|_| unavailable())?
    } else {
        root.join("backups")
    };
    tauri::async_runtime::spawn_blocking(move || {
        let _access = crate::maintenance::ACCESS
            .read()
            .map_err(|_| unavailable())?;
        let path = crate::commands::database_path(&app)?;
        let db = crate::database::Database::open(&path)
            .map_err(|error| failure(archive::Error::Database(error)))?;
        archive::create(&db, &root, &parent, include_thumbnails)
            .map(Some)
            .map_err(failure)
    })
    .await
    .map_err(|_| unavailable())?
}
