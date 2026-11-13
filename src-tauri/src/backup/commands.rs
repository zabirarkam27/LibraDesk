use super::archive::{self, Summary};
use crate::commands::{unavailable, CommandError};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
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

static NEXT_RESTORE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
#[tauri::command]
pub async fn inspect_backup(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<Option<super::restore::Preview>, CommandError> {
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Choose a LibraDesk backup folder containing manifest.json")
            .blocking_pick_folder()
    })
    .await
    .map_err(|_| unavailable())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|_| unavailable())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _access = crate::maintenance::ACCESS
            .read()
            .map_err(|_| unavailable())?;
        let root = crate::storage_management::root(&app)?;
        let id = NEXT_RESTORE.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let draft = super::restore::prepare(&root, &path, id).map_err(failure)?;
        let preview = draft.preview();
        *app.state::<super::RestoreState>()
            .0
            .lock()
            .map_err(|_| unavailable())? = Some(draft);
        Ok(Some(preview))
    })
    .await
    .map_err(|_| unavailable())?
}
#[tauri::command]
pub async fn cancel_restore(app: AppHandle, token: u64) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<super::RestoreState>();
        let mut saved = state.0.lock().map_err(|_| unavailable())?;
        if saved.as_ref().is_some_and(|draft| draft.id == token) {
            saved.take();
        }
        Ok(())
    })
    .await
    .map_err(|_| unavailable())?
}
#[tauri::command]
pub async fn restore_backup(
    app: AppHandle,
    token: u64,
) -> Result<super::restore::Restored, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        let draft = {
            let state = app.state::<super::RestoreState>();
            let mut saved = state.0.lock().map_err(|_| unavailable())?;
            if saved.as_ref().is_none_or(|draft| draft.id != token) {
                return Err(unavailable());
            }
            saved.take().ok_or_else(unavailable)?
        };
        let _access = crate::maintenance::ACCESS
            .write()
            .map_err(|_| unavailable())?;
        let root = crate::storage_management::root(&app)?;
        let state = app.state::<crate::commands::State>();
        let mut live = state.0.lock().map_err(|_| unavailable())?;
        if live.is_none() {
            *live = Some(
                crate::database::Database::open(&crate::commands::database_path(&app)?)
                    .map_err(|error| failure(archive::Error::Database(error)))?,
            );
        }
        let result =
            super::restore::apply(live.as_mut().ok_or_else(unavailable)?, &root, draft, false);
        crate::maintenance::advance();
        let result = result.map_err(failure)?;
        let _ = app.emit("library-restored", ());
        Ok(result)
    })
    .await
    .map_err(|_| unavailable())?
}
