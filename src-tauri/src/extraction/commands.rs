use super::repository::{ExtractionStatus, Failure, JobKey, PageText, TextTask};
use crate::commands::{unavailable, with_database, CommandError};
use tauri::{ipc::Response, AppHandle, Emitter};

fn changed(app: &AppHandle, file: i64) {
    let _ = app.emit("extraction-updated", file);
}
fn stale() -> CommandError {
    CommandError {
        code: "extraction_stale",
        message: "This extraction job has been replaced.",
        retryable: false,
    }
}
#[tauri::command]
pub async fn claim_text_extraction(app: AppHandle) -> Result<Option<TextTask>, CommandError> {
    let task = with_database(app.clone(), |db| db.claim_text_extraction()).await?;
    if let Some(task) = &task {
        changed(&app, task.file_id);
    }
    Ok(task)
}
#[tauri::command]
pub async fn read_extraction_range(
    app: AppHandle,
    key: JobKey,
    offset: i64,
    length: i64,
) -> Result<Response, CommandError> {
    let source = with_database(app, move |db| db.extraction_source(key))
        .await?
        .ok_or_else(stale)?;
    let bytes = tauri::async_runtime::spawn_blocking(move || source.read_range(offset, length))
        .await
        .map_err(|_| unavailable())?
        .map_err(|_| CommandError {
            code: "reader_source_unavailable",
            message: "The PDF changed or cannot be read. Rescan its drive.",
            retryable: true,
        })?;
    Ok(Response::new(bytes))
}
#[tauri::command]
pub async fn renew_text_extraction(app: AppHandle, key: JobKey) -> Result<bool, CommandError> {
    with_database(app, move |db| db.renew_text_extraction(key)).await
}
#[tauri::command]
pub async fn commit_page_text(
    app: AppHandle,
    key: JobKey,
    total: i64,
    pages: Vec<PageText>,
) -> Result<bool, CommandError> {
    let result = with_database(app.clone(), move |db| {
        db.commit_page_text(key, total, &pages)
    })
    .await?;
    if result {
        changed(&app, key.file_id);
    }
    Ok(result)
}
#[tauri::command]
pub async fn release_text_extraction(app: AppHandle, key: JobKey) -> Result<(), CommandError> {
    with_database(app.clone(), move |db| db.release_text_extraction(key)).await?;
    changed(&app, key.file_id);
    Ok(())
}
#[tauri::command]
pub async fn fail_text_extraction(
    app: AppHandle,
    key: JobKey,
    reason: Failure,
) -> Result<(), CommandError> {
    with_database(app.clone(), move |db| db.fail_text_extraction(key, reason)).await?;
    changed(&app, key.file_id);
    Ok(())
}
#[tauri::command]
pub async fn retry_text_extraction(
    app: AppHandle,
    file: i64,
    revision: i64,
) -> Result<bool, CommandError> {
    let result = with_database(app.clone(), move |db| {
        db.retry_text_extraction(file, revision)
    })
    .await?;
    if result {
        changed(&app, file);
    }
    Ok(result)
}
#[tauri::command]
pub async fn text_extraction_status(
    app: AppHandle,
    file: i64,
) -> Result<Option<ExtractionStatus>, CommandError> {
    with_database(app, move |db| db.text_extraction_status(file)).await
}
