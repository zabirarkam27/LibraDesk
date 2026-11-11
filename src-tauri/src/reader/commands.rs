use super::repository::{
    AnnotationInput, AnnotationKind, Annotations, ProgressInput, ReaderDocument, ReaderPreferences,
};
use crate::commands::{unavailable, with_database, CommandError};
use tauri::{ipc::Response, AppHandle, Emitter};

#[tauri::command]
pub async fn open_reader(
    app: AppHandle,
    book: i64,
    file: Option<i64>,
) -> Result<ReaderDocument, CommandError> {
    with_database(app, move |db| db.open_reader(book, file)).await
}
#[tauri::command]
pub async fn read_reader_range(
    app: AppHandle,
    session: i64,
    offset: i64,
    length: i64,
) -> Result<Response, CommandError> {
    let source = with_database(app, move |db| db.reader_source(session)).await?;
    let bytes = tauri::async_runtime::spawn_blocking(move || source.read_range(offset, length))
        .await
        .map_err(|_| unavailable())?
        .map_err(|_| CommandError {
            code: "reader_source_unavailable",
            message:
                "The PDF changed or cannot be read. Check its drive and rescan before reopening.",
            retryable: true,
        })?;
    Ok(Response::new(bytes))
}
#[tauri::command]
pub async fn save_reader_progress(
    app: AppHandle,
    input: ProgressInput,
) -> Result<(), CommandError> {
    with_database(app.clone(), move |db| db.save_reader_progress(&input)).await?;
    let _ = app.emit("catalog-updated", ());
    Ok(())
}
#[tauri::command]
pub async fn close_reader(app: AppHandle, session: i64) -> Result<(), CommandError> {
    with_database(app, move |db| db.close_reader(session)).await
}
#[tauri::command]
pub async fn save_reader_preferences(
    app: AppHandle,
    input: ReaderPreferences,
) -> Result<(), CommandError> {
    with_database(app, move |db| db.save_reader_preferences(&input)).await
}
#[tauri::command]
pub async fn reader_annotations(app: AppHandle, book: i64) -> Result<Annotations, CommandError> {
    with_database(app, move |db| db.reader_annotations(book)).await
}
#[tauri::command]
pub async fn save_reader_annotation(
    app: AppHandle,
    kind: AnnotationKind,
    input: AnnotationInput,
) -> Result<i64, CommandError> {
    with_database(app, move |db| db.save_reader_annotation(kind, &input)).await
}
#[tauri::command]
pub async fn delete_reader_annotation(
    app: AppHandle,
    kind: AnnotationKind,
    session: i64,
    id: i64,
    revision: i64,
) -> Result<(), CommandError> {
    with_database(app, move |db| {
        db.delete_reader_annotation(kind, session, id, revision)
    })
    .await
}
