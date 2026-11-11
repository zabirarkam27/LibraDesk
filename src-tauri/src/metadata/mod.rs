pub mod commands;
mod cover;
mod pdf;
mod repository;

#[cfg(test)]
mod tests;

pub(crate) use pdf::filename_title;
pub(crate) use pdf::Source;
pub(crate) use repository::refresh_scanned_files;

pub const EVENT: &str = "metadata-updated";

pub fn process_next(db: &crate::database::Database) -> rusqlite::Result<bool> {
    let Some(source) = db.next_analysis()? else {
        return Ok(false);
    };
    match pdf::analyze(&source) {
        Ok(result) => db.complete_analysis(&source, result)?,
        Err(_) => {
            db.fail_processing(source.id, source.revision, "analysis", "source_unavailable")?
        }
    }
    Ok(true)
}
