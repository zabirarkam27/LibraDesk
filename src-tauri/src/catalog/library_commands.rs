use crate::commands::{with_database, CommandError};
use tauri::AppHandle;

use super::{
    commands::write,
    organization::{GroupInput, GroupKind, MemberAction, Organization},
    repository::ReadingStatus,
    search::{CatalogPage, CatalogQuery, Dashboard},
};

#[tauri::command]
pub async fn edit_collection_member(
    app: AppHandle,
    id: i64,
    revision: i64,
    book: i64,
    action: MemberAction,
) -> Result<(), CommandError> {
    write(app, move |db| {
        db.edit_collection_member(id, revision, book, action)
    })
    .await
}
#[tauri::command]
pub async fn query_catalog(
    app: AppHandle,
    query: CatalogQuery,
) -> Result<CatalogPage, CommandError> {
    with_database(app, move |db| db.query_catalog(&query)).await
}
#[tauri::command]
pub async fn get_organization(app: AppHandle) -> Result<Organization, CommandError> {
    with_database(app, move |db| db.organization()).await
}
#[tauri::command]
pub async fn save_group(
    app: AppHandle,
    kind: GroupKind,
    input: GroupInput,
) -> Result<i64, CommandError> {
    write(app, move |db| db.save_group(kind, &input)).await
}
#[tauri::command]
pub async fn delete_group(
    app: AppHandle,
    kind: GroupKind,
    id: i64,
    revision: i64,
) -> Result<(), CommandError> {
    write(app, move |db| db.delete_group(kind, id, revision)).await
}
#[tauri::command]
pub async fn set_collection_books(
    app: AppHandle,
    id: i64,
    revision: i64,
    books: Vec<i64>,
) -> Result<(), CommandError> {
    write(app, move |db| db.set_collection_books(id, revision, &books)).await
}
#[tauri::command]
pub async fn set_book_flags(
    app: AppHandle,
    id: i64,
    revision: i64,
    favorite: Option<bool>,
    status: Option<ReadingStatus>,
) -> Result<(), CommandError> {
    write(app, move |db| {
        db.set_book_flags(id, revision, favorite, status)
    })
    .await
}
#[tauri::command]
pub async fn get_dashboard(app: AppHandle) -> Result<Dashboard, CommandError> {
    with_database(app, move |db| db.dashboard()).await
}
