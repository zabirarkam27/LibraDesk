mod file_security;
mod pdf;
mod repository;
#[cfg(test)]
mod tests;
mod worker;
mod xmp;
use crate::commands::{with_database, CommandError};
pub(crate) use repository::enqueue;
use tauri::AppHandle;
pub use worker::start;

#[tauri::command]
pub async fn pdf_metadata_status(
    app: AppHandle,
    book: i64,
) -> std::result::Result<Vec<repository::Status>, CommandError> {
    with_database(app, move |db| Ok(db.pdf_write_status(book)?)).await
}
#[tauri::command]
pub async fn retry_pdf_metadata(
    app: AppHandle,
    file: i64,
) -> std::result::Result<(), CommandError> {
    with_database(app,move|db|{
        let changed=db.0.execute("INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at) SELECT id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files WHERE id=?1
        ON CONFLICT(file_id) DO UPDATE SET status='queued',error_code=NULL,updated_at=excluded.updated_at",[file])?;
        if changed==0{return Err(crate::database::Error::CatalogNotFound);}
        Ok(())
    }).await
}
#[tauri::command]
pub async fn cancel_pdf_metadata(
    app: AppHandle,
    file: i64,
) -> std::result::Result<(), CommandError> {
    let result = tauri::async_runtime::spawn_blocking(move || {
        use tauri::Manager;
        let _access = crate::maintenance::ACCESS
            .write()
            .map_err(|_| crate::commands::unavailable())?;
        let state = app.state::<crate::commands::State>();
        let mut locked = state.0.lock().map_err(|_| crate::commands::unavailable())?;
        if locked.is_none() {
            *locked = Some(
                crate::database::Database::open(&crate::commands::database_path(&app)?)
                    .map_err(|_| crate::commands::unavailable())?,
            );
        }
        worker::cancel(
            locked.as_ref().ok_or_else(crate::commands::unavailable)?,
            file,
        )
        .map_err(|error| CommandError {
            code: error.code,
            message: worker::message(error.code),
            retryable: true,
        })
    })
    .await
    .map_err(|_| crate::commands::unavailable())?;
    result
}
#[derive(Debug)]
pub(crate) struct Failure {
    pub code: &'static str,
}
impl From<std::io::Error> for Failure {
    fn from(_: std::io::Error) -> Self {
        Self {
            code: "file_unavailable",
        }
    }
}
impl From<rusqlite::Error> for Failure {
    fn from(_: rusqlite::Error) -> Self {
        Self {
            code: "database_unavailable",
        }
    }
}
impl From<serde_json::Error> for Failure {
    fn from(_: serde_json::Error) -> Self {
        Self {
            code: "metadata_invalid",
        }
    }
}
pub(crate) type Result<T> = std::result::Result<T, Failure>;
pub(crate) fn fail<T>(code: &'static str) -> Result<T> {
    Err(Failure { code })
}
