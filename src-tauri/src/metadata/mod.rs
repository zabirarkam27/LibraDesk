pub mod commands;
mod cover;
pub mod manual_cover;
mod pdf;
mod repository;

#[cfg(test)]
mod tests;

pub(crate) use pdf::filename_title;
pub(crate) use pdf::Source;
pub(crate) use pdf::{analyze, hash, Analysis};
pub(crate) use repository::refresh_scanned_files;

pub const EVENT: &str = "metadata-updated";
pub(crate) fn backup_thumbnail(
    root: &std::path::Path,
    id: i64,
    revision: i64,
) -> std::io::Result<Vec<u8>> {
    cover::load_png(root, id, revision)
}
pub(crate) fn valid_cache_generation(generation: &str) -> bool {
    generation.starts_with("covers-")
        && generation.len() > 7
        && generation.len() <= 100
        && generation
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
pub(crate) fn resolve_cache_root(
    root: &std::path::Path,
    generation: Option<&str>,
) -> std::io::Result<std::path::PathBuf> {
    let Some(generation) = generation else {
        return Ok(root.to_path_buf());
    };
    if !valid_cache_generation(generation) {
        return Err(std::io::ErrorKind::InvalidInput.into());
    }
    let _guard = crate::storage_management::directory(root)?;
    let cache = root.join("cache");
    let _cache_guard = crate::storage_management::directory(&cache)?;
    let selected = cache.join(generation);
    crate::storage::prepare_directory(&selected)?;
    crate::storage::prepare_directory(&selected.join("thumbnails"))?;
    Ok(selected)
}

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
