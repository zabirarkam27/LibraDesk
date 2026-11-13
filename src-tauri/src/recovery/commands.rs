use super::{
    duplicates::{DuplicateAction, DuplicateFile},
    repository::{FileRow, RelinkPreview, Verification},
};
use crate::commands::{unavailable, with_database, CommandError};
use tauri::{AppHandle, Emitter, WebviewWindow};
use tauri_plugin_dialog::DialogExt;
fn invalid() -> CommandError {
    CommandError{code:"replacement_unavailable",message:"Choose an accessible PDF. The replacement changed, is already registered, or the preview expired; preview again.",retryable:true}
}
fn changed(app: &AppHandle) {
    let _ = app.emit("recovery-updated", ());
    let _ = app.emit("catalog-updated", ());
    let _ = app.emit("metadata-updated", ());
}
#[tauri::command]
pub async fn recovery_snapshot(app: AppHandle) -> Result<Verification, CommandError> {
    with_database(app, |db| db.recovery_snapshot()).await
}
#[tauri::command]
pub async fn recovery_files(
    app: AppHandle,
    after: i64,
    unavailable_only: bool,
) -> Result<Vec<FileRow>, CommandError> {
    with_database(app, move |db| db.recovery_files(after, unavailable_only)).await
}
#[tauri::command]
pub async fn start_file_verification(app: AppHandle) -> Result<(), CommandError> {
    with_database(app.clone(), |db| db.start_verification()).await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn preview_relink(
    app: AppHandle,
    window: WebviewWindow,
    file: i64,
) -> Result<Option<RelinkPreview>, CommandError> {
    let registered = with_database(app.clone(), move |db| db.recovery_file(file)).await?;
    let picker = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        picker
            .dialog()
            .file()
            .set_parent(&window)
            .set_title("Select replacement PDF")
            .add_filter("PDF", &["pdf"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| unavailable())?;
    let Some(selected) = selected else {
        return Ok(None);
    };
    let path = selected.into_path().map_err(|_| invalid())?;
    let (source, analysis) =
        tauri::async_runtime::spawn_blocking(move || super::probe::replacement(&path))
            .await
            .map_err(|_| unavailable())?
            .map_err(|_| invalid())?;
    with_database(app, move |db| {
        db.create_relink_draft(&registered, &source, &analysis)
    })
    .await
    .map(Some)
}
#[tauri::command]
pub async fn confirm_relink(app: AppHandle, draft: i64) -> Result<(), CommandError> {
    let saved = with_database(app.clone(), move |db| db.relink_draft(draft)).await?;
    let (saved, handle) = tauri::async_runtime::spawn_blocking(move || {
        let file = super::probe::verified_handle(&saved.candidate, &saved.digest)
            .map_err(|_| invalid())?;
        Ok((saved, file))
    })
    .await
    .map_err(|_| unavailable())??;
    with_database(app.clone(), move |db| {
        let _read_guard = handle;
        db.confirm_relink(&saved)
    })
    .await?;
    changed(&app);
    Ok(())
}
#[tauri::command]
pub async fn cancel_relink(app: AppHandle, draft: i64) -> Result<(), CommandError> {
    with_database(app, move |db| db.cancel_relink(draft)).await
}
#[tauri::command]
pub async fn duplicate_files(
    app: AppHandle,
    hash: String,
    after: i64,
) -> Result<Vec<DuplicateFile>, CommandError> {
    with_database(app, move |db| db.duplicate_files(&hash, after)).await
}
#[tauri::command]
pub async fn duplicate_action(
    app: AppHandle,
    hash: String,
    file: i64,
    revision: i64,
    keep: i64,
    action: DuplicateAction,
) -> Result<(), CommandError> {
    if matches!(action, DuplicateAction::Remove) {
        let source =
            with_database(app.clone(), move |db| db.duplicate_keep_source(&hash, keep)).await?;
        let checked = source.clone();
        let handle = tauri::async_runtime::spawn_blocking(move || {
            super::probe::verified_handle(
                &checked.source,
                checked.hash.as_deref().ok_or_else(invalid)?,
            )
            .map_err(|_| invalid())
        })
        .await
        .map_err(|_| unavailable())??;
        with_database(app.clone(), move |db| {
            let _read_guard = handle;
            db.remove_duplicate(file, revision, &source)
        })
        .await?;
    } else {
        with_database(app.clone(), move |db| {
            db.prefer_duplicate(&hash, file, revision)
        })
        .await?;
    }
    changed(&app);
    Ok(())
}
