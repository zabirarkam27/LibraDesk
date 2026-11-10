use std::{path::PathBuf, sync::Mutex};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::{
    database::{self, Database, Preferences, Scope, Theme},
    storage,
};

#[derive(Default)]
pub struct State(Mutex<Option<Database>>);

#[derive(Serialize)]
pub struct CommandError {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
    pub(crate) retryable: bool,
}

fn database_error(error: database::Error) -> CommandError {
    eprintln!("LibraDesk database error: {error}");
    match error {
        database::Error::IncompatibleSchema => CommandError {
            code: "incompatible_schema",
            message: "This library database was created by a different version of LibraDesk.",
            retryable: false,
        },
        _ => CommandError {
            code: "database_unavailable",
            message: "LibraDesk could not open or update its library database.",
            retryable: true,
        },
    }
}

pub(crate) async fn with_database<T: Send + 'static>(
    app: AppHandle,
    operation: impl FnOnce(&Database) -> Result<T, database::Error> + Send + 'static,
) -> Result<T, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
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
    let path = app
        .path()
        .local_data_dir()
        .map_err(|_| storage_unavailable())?;
    storage::initialize(&path.join("LibraDesk")).map_err(|_| storage_unavailable())
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
