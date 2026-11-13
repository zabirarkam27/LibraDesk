use crate::commands::{unavailable, CommandError};
use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Emitter, Manager};
const LIMIT: usize = 100_000;
#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub database: u64,
    pub thumbnails: u64,
    pub cache: u64,
    pub temporary: u64,
    pub backups: u64,
    pub logs: u64,
    pub files: u64,
    pub skipped: u64,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheKind {
    Cache,
    Thumbnails,
    Temporary,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cleanup {
    pub removed: u64,
    pub bytes: u64,
    pub retained: u64,
    pub usage: Usage,
}
pub(crate) fn root(app: &AppHandle) -> Result<PathBuf, CommandError> {
    // Test-only profile isolation is compiled out of release builds.
    #[cfg(debug_assertions)]
    if let Some(value) = std::env::var_os("LIBRADESK_QA_DATA_DIR") {
        let path = PathBuf::from(value);
        if !path.is_absolute() {
            return Err(unavailable());
        }
        crate::storage::prepare_directory(&path).map_err(|_| unavailable())?;
        return fs::canonicalize(path).map_err(|_| unavailable());
    }
    app.path()
        .local_data_dir()
        .map(|p| p.join("LibraDesk"))
        .map_err(|_| unavailable())
}
pub(crate) fn directory(path: &Path) -> io::Result<fs::File> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() || crate::scan::files::reparse(&meta) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
            FILE_SHARE_WRITE,
        };
        options
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
    }
    let file = options.open(path)?;
    if crate::scan::files::reparse(&file.metadata()?) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    Ok(file)
}
fn measure(path: &Path, depth: usize, usage: &mut Usage) -> io::Result<u64> {
    if depth > 16 {
        return Err(io::ErrorKind::InvalidData.into());
    }
    let _guard = directory(path)?;
    let mut bytes = 0u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = fs::symlink_metadata(entry.path())?;
        usage.files += 1;
        if usage.files as usize > LIMIT {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if crate::scan::files::reparse(&meta) {
            usage.skipped += 1;
            continue;
        }
        if meta.is_dir() {
            bytes = bytes.saturating_add(measure(&entry.path(), depth + 1, usage)?);
        } else if meta.is_file() {
            bytes = bytes.saturating_add(meta.len());
        }
    }
    Ok(bytes)
}
pub fn usage(root: &Path) -> io::Result<Usage> {
    let _guard = directory(root)?;
    let mut out = Usage::default();
    out.database = measure(&root.join("database"), 0, &mut out)?;
    out.thumbnails = measure(&root.join("thumbnails"), 0, &mut out)?;
    out.cache = measure(&root.join("cache"), 0, &mut out)?;
    for folder in cover_folders(root)? {
        let _generation_guard = directory(folder.parent().ok_or(io::ErrorKind::InvalidInput)?)?;
        let bytes = measure(&folder, 0, &mut Usage::default())?;
        out.cache = out.cache.saturating_sub(bytes);
        out.thumbnails = out.thumbnails.saturating_add(bytes);
    }
    out.temporary = measure(&root.join("temp"), 0, &mut out)?;
    out.backups = measure(&root.join("backups"), 0, &mut out)?;
    out.logs = measure(&root.join("logs"), 0, &mut out)?;
    Ok(out)
}
fn generated_png(name: &str) -> bool {
    name.strip_suffix(".png")
        .and_then(|s| s.split_once('-'))
        .is_some_and(|(id, revision)| {
            !id.is_empty()
                && !revision.is_empty()
                && id.bytes().all(|b| b.is_ascii_digit())
                && revision.bytes().all(|b| b.is_ascii_digit())
        })
}
fn protected_source(path: &Path) -> bool {
    use std::io::Read;
    if crate::scan::files::is_pdf(path) {
        return true;
    }
    let Ok(mut file) = crate::scan::files::open_regular_readonly(path) else {
        return true;
    };
    let mut header = [0; 5];
    file.read_exact(&mut header).is_ok() && &header == b"%PDF-"
}
fn cover_folders(root: &Path) -> io::Result<Vec<PathBuf>> {
    let cache = root.join("cache");
    let _guard = directory(&cache)?;
    let mut folders = Vec::new();
    for entry in fs::read_dir(cache)?.take(LIMIT + 1) {
        let entry = entry?;
        if !crate::metadata::valid_cache_generation(&entry.file_name().to_string_lossy()) {
            continue;
        }
        let Ok(_generation_guard) = directory(&entry.path()) else {
            continue;
        };
        let folder = entry.path().join("thumbnails");
        if directory(&folder).is_ok() {
            folders.push(folder);
        }
    }
    Ok(folders)
}
pub fn clean(root: &Path, kind: CacheKind) -> io::Result<Cleanup> {
    let _root_guard = directory(root)?;
    let root = fs::canonicalize(root)?;
    let folder = root.join(match kind {
        CacheKind::Cache => "cache",
        CacheKind::Thumbnails => "thumbnails",
        CacheKind::Temporary => "temp",
    });
    let mut result = Cleanup {
        removed: 0,
        bytes: 0,
        retained: 0,
        usage: Usage::default(),
    };
    clean_folder(&folder, kind, &mut result)?;
    if matches!(kind, CacheKind::Thumbnails) {
        for folder in cover_folders(&root)? {
            let parent = folder.parent().ok_or(io::ErrorKind::InvalidInput)?;
            let _generation_guard = directory(parent)?;
            clean_folder(&folder, kind, &mut result)?;
        }
    }
    result.usage = usage(&root)?;
    Ok(result)
}
fn clean_folder(folder: &Path, kind: CacheKind, result: &mut Cleanup) -> io::Result<()> {
    let _guard = directory(folder)?;
    if fs::canonicalize(folder)? != folder {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    for entry in fs::read_dir(folder)?.take(LIMIT + 1) {
        if result.removed + result.retained >= LIMIT as u64 {
            return Err(io::ErrorKind::InvalidData.into());
        }
        let path = entry?.path();
        let meta = fs::symlink_metadata(&path)?;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let eligible = meta.is_file()
            && !crate::scan::files::reparse(&meta)
            && !crate::scan::files::is_pdf(&path)
            && match kind {
                CacheKind::Thumbnails => generated_png(name),
                CacheKind::Temporary => meta
                    .modified()
                    .ok()
                    .and_then(|time| SystemTime::now().duration_since(time).ok())
                    .is_some_and(|age| age >= Duration::from_secs(86400)),
                CacheKind::Cache => true,
            };
        if !eligible {
            result.retained += 1;
            continue;
        }
        if protected_source(&path) {
            result.retained += 1;
            continue;
        }
        // Direct files only: subdirectories and reparse points are never traversed during deletion.
        match fs::remove_file(&path) {
            Ok(()) => {
                result.removed += 1;
                result.bytes = result.bytes.saturating_add(meta.len());
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::PermissionDenied | io::ErrorKind::NotFound
                ) =>
            {
                result.retained += 1
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
fn failure() -> CommandError {
    CommandError{code:"storage_operation_failed",message:"Storage could not be inspected or cleaned. Close active work and retry. Protected or linked entries are retained.",retryable:true}
}
#[tauri::command]
pub async fn storage_usage(app: AppHandle) -> Result<Usage, CommandError> {
    let root = root(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _access = crate::maintenance::ACCESS
            .read()
            .map_err(|_| unavailable())?;
        usage(&root).map_err(|_| failure())
    })
    .await
    .map_err(|_| unavailable())?
}
#[tauri::command]
pub async fn clear_cache(app: AppHandle, kind: CacheKind) -> Result<Cleanup, CommandError> {
    let root = root(&app)?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _access = crate::maintenance::ACCESS
            .write()
            .map_err(|_| unavailable())?;
        clean(&root, kind).map_err(|_| failure())
    })
    .await
    .map_err(|_| unavailable())??;
    let _ = app.emit("metadata-updated", ());
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restored_cover_generations_are_accounted_and_cleaned_without_touching_sources() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("LibraDesk");
        crate::storage::initialize(&root).unwrap();
        let covers = crate::metadata::resolve_cache_root(&root, Some("covers-test")).unwrap();
        fs::write(covers.join("thumbnails/1-1.png"), b"png").unwrap();
        fs::write(covers.join("thumbnails/2-1.png"), b"%PDF-protected").unwrap();
        fs::write(root.join("cache/other"), b"cache").unwrap();
        let before = usage(&root).unwrap();
        assert_eq!(before.thumbnails, 17);
        assert_eq!(before.cache, 5);
        let cleaned = clean(&root, CacheKind::Thumbnails).unwrap();
        assert_eq!(cleaned.removed, 1);
        assert_eq!(cleaned.usage.thumbnails, 14);
        assert!(covers.join("thumbnails/2-1.png").exists());
        assert!(root.join("cache/other").exists());
    }
    #[test]
    fn storage_totals_and_cleanup_preserve_database_backups_pdfs_and_nested_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("LibraDesk");
        crate::storage::initialize(&root).unwrap();
        fs::write(root.join("database/library.sqlite3"), b"database").unwrap();
        fs::write(root.join("backups/keep"), b"backup").unwrap();
        fs::write(root.join("cache/drop"), b"cache").unwrap();
        fs::write(root.join("cache/ORIGINAL.PDF"), b"original").unwrap();
        fs::create_dir(root.join("cache/nested")).unwrap();
        fs::write(root.join("cache/nested/keep"), b"nested").unwrap();
        fs::write(root.join("thumbnails/1-1.png"), b"png").unwrap();
        fs::write(root.join("thumbnails/private.png"), b"keep").unwrap();
        fs::write(root.join("thumbnails/2-1.png"), b"%PDF-hidden").unwrap();
        fs::write(root.join("temp/recent"), b"recent").unwrap();
        fs::write(root.join("temp/old"), b"old").unwrap();
        let file = fs::OpenOptions::new()
            .write(true)
            .open(root.join("temp/old"))
            .unwrap();
        file.set_times(
            fs::FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(90000)),
        )
        .unwrap();
        drop(file);
        let before = usage(&root).unwrap();
        assert_eq!(before.database, 8);
        assert_eq!(before.cache, 19);
        let cleared = clean(&root, CacheKind::Cache).unwrap();
        assert_eq!(cleared.removed, 1);
        assert!(root.join("cache/ORIGINAL.PDF").exists());
        assert!(root.join("cache/nested/keep").exists());
        assert_eq!(clean(&root, CacheKind::Thumbnails).unwrap().removed, 1);
        assert!(root.join("thumbnails/private.png").exists());
        assert!(root.join("thumbnails/2-1.png").exists());
        assert_eq!(clean(&root, CacheKind::Temporary).unwrap().removed, 1);
        assert!(root.join("temp/recent").exists());
        assert_eq!(
            fs::read(root.join("database/library.sqlite3")).unwrap(),
            b"database"
        );
        assert_eq!(fs::read(root.join("backups/keep")).unwrap(), b"backup");
    }
}
