use tauri::{AppHandle, Emitter};

use crate::commands::{with_database, CommandError};

use super::repository::{Author, BookDetails, BookFile, BookInput, BookSummary};

pub(super) async fn write<T: Send + 'static>(
    app: AppHandle,
    operation: impl FnOnce(&crate::database::Database) -> Result<T, crate::database::Error>
        + Send
        + 'static,
) -> Result<T, CommandError> {
    let result = with_database(app.clone(), operation).await?;
    let _ = app.emit("catalog-updated", ());
    Ok(result)
}

#[tauri::command]
pub async fn list_books(
    app: AppHandle,
    before: Option<i64>,
) -> Result<Vec<BookSummary>, CommandError> {
    with_database(app, move |db| Ok(db.list_books(before, true)?)).await
}

#[tauri::command]
pub async fn get_book(app: AppHandle, id: i64) -> Result<BookDetails, CommandError> {
    with_database(app, move |db| db.get_book(id)).await
}

#[tauri::command]
pub async fn create_book(app: AppHandle, input: BookInput) -> Result<BookDetails, CommandError> {
    write(app, move |db| db.create_book(&input)).await
}

#[tauri::command]
pub async fn update_book(
    app: AppHandle,
    id: i64,
    revision: i64,
    input: BookInput,
) -> Result<BookDetails, CommandError> {
    write(app, move |db| db.update_book(id, revision, &input)).await
}

#[tauri::command]
pub async fn delete_book(app: AppHandle, id: i64, revision: i64) -> Result<(), CommandError> {
    write(app, move |db| db.delete_book(id, revision)).await
}

#[tauri::command]
pub async fn get_book_file(app: AppHandle, id: i64) -> Result<BookFile, CommandError> {
    with_database(app, move |db| db.get_book_file(id)).await
}

#[tauri::command]
pub async fn create_author(app: AppHandle, display_name: String) -> Result<Author, CommandError> {
    write(app, move |db| db.create_author(&display_name)).await
}

#[tauri::command]
pub async fn list_authors(app: AppHandle, after: Option<i64>) -> Result<Vec<Author>, CommandError> {
    with_database(app, move |db| db.list_authors(after)).await
}
