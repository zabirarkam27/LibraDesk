use std::{path::PathBuf, sync::Mutex};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::{
    database::{self, Database, Preferences, Scope, Theme},
    storage,
};

#[derive(Default)]
pub struct State(pub(crate) Mutex<Option<Database>>);

#[derive(Serialize)]
pub struct CommandError {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
    pub(crate) retryable: bool,
}

fn database_error(error: database::Error) -> CommandError {
    match error {
        database::Error::Sqlite(rusqlite::Error::SqliteFailure(_,Some(message))) if message=="PDF metadata save in progress; retry removal" => CommandError{
            code:"pdf_write_in_progress",message:"A PDF metadata save is still pending. Finish or stop its sync in Book details, then retry removal.",retryable:true,
        },
        database::Error::ReaderSourceUnavailable => CommandError {
            code: "reader_source_unavailable",
            message: "The PDF is missing, inaccessible, or changed. Check its drive and rescan the folder before reopening.",
            retryable: true,
        },
        database::Error::OrganizationConflict => CommandError {
            code: "organization_conflict",
            message:
                "This name already exists, or the selected category parent would create a cycle.",
            retryable: false,
        },
        database::Error::CategoryHasChildren => CommandError {
            code: "category_has_children",
            message: "Move or remove child categories before deleting this category.",
            retryable: false,
        },
        database::Error::InvalidCatalogInput => CommandError {
            code: "invalid_catalog_input",
            message: "Check the book metadata or author details and try again.",
            retryable: false,
        },
        database::Error::CatalogNotFound => CommandError {
            code: "catalog_not_found",
            message: "This catalog record no longer exists.",
            retryable: false,
        },
        database::Error::CatalogConflict => CommandError {
            code: "catalog_conflict",
            message: "This record has changed. Reload it before making more changes.",
            retryable: false,
        },
        database::Error::UnknownAuthor => CommandError {
            code: "unknown_author",
            message: "An assigned author no longer exists.",
            retryable: false,
        },
        database::Error::IncompatibleSchema => CommandError {
            code: "incompatible_schema",
            message: "This library database was created by a different version of LibraDesk.",
            retryable: false,
        },
        _ => {
            eprintln!("LibraDesk database error: {error}");
            CommandError {
                code: "database_unavailable",
                message: "LibraDesk could not open or update its library database.",
                retryable: true,
            }
        }
    }
}

pub(crate) async fn with_database<T: Send + 'static>(
    app: AppHandle,
    operation: impl FnOnce(&Database) -> Result<T, database::Error> + Send + 'static,
) -> Result<T, CommandError> {
    let generation = crate::maintenance::generation();
    tauri::async_runtime::spawn_blocking(move || {
        let _access = crate::maintenance::ACCESS
            .read()
            .map_err(|_| unavailable())?;
        if generation != crate::maintenance::generation() {
            return Err(unavailable());
        }
        let state = app.state::<State>();
        let mut database = state.0.lock().map_err(|_| unavailable())?;
        if database.is_none() {
            let file = database_path(&app)?;
            *database = Some(Database::open(&file).map_err(database_error)?);
        }
        operation(database.as_ref().ok_or_else(unavailable)?).map_err(database_error)
    })
    .await
    .map_err(|_| unavailable())?
}

pub(crate) fn database_path(app: &AppHandle) -> Result<PathBuf, CommandError> {
    let path = crate::storage_management::root(app)?;
    storage::initialize(&path).map_err(|_| storage_unavailable())
}

pub(crate) fn unavailable() -> CommandError {
    CommandError {
        code: "unavailable",
        message: "LibraDesk storage is temporarily unavailable. Try restarting the app.",
        retryable: false,
    }
}

fn storage_unavailable() -> CommandError {
    CommandError {
        code: "storage_unavailable",
        message: "LibraDesk could not access its folders in Local AppData. Check the folder permissions and try again.",
        retryable: true,
    }
}

#[tauri::command]
pub async fn get_preferences(
    app: AppHandle,
    legacy_theme: Option<Theme>,
) -> Result<Preferences, CommandError> {
    with_database(app, move |db| db.preferences(legacy_theme)).await
}

#[tauri::command]
pub async fn set_theme(app: AppHandle, scope: Scope, theme: Theme) -> Result<(), CommandError> {
    with_database(app, move |db| db.set_theme(scope, theme)).await
}
