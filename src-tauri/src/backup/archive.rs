use crate::{
    database::{Database, SnapshotInfo},
    storage_management::directory,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Sqlite(rusqlite::Error),
    Database(crate::database::Error),
    Json(serde_json::Error),
    Invalid(&'static str),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<rusqlite::Error> for Error {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sqlite(e)
    }
}
impl From<crate::database::Error> for Error {
    fn from(e: crate::database::Error) -> Self {
        Self::Database(e)
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl Error {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Invalid(message) => message,
            Self::Io(error) => {
                let _ = error.kind();
                "Backup files could not be read or written. Check the selected folder and free space."
            }
            Self::Sqlite(error) => {
                let _ = error.sqlite_error_code();
                "The database snapshot could not be completed or verified."
            }
            Self::Database(error) => {
                let _ = error.to_string();
                "The backup database is corrupt, incompatible, or has an unexpected schema."
            }
            Self::Json(error) => {
                let _ = error.line();
                "The backup manifest is invalid."
            }
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Entry {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    pub app_version: String,
    pub created_at: u64,
    pub database: SnapshotInfo,
    pub thumbnails_requested: bool,
    pub originals_included: bool,
    pub files: Vec<Entry>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub path: String,
    pub created_at: u64,
    pub schema_version: u32,
    pub books: i64,
    pub notes: i64,
    pub bookmarks: i64,
    pub reading_sessions: i64,
    pub thumbnails: usize,
    pub bytes: u64,
}
pub fn timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub fn file_digest(path: &Path) -> Result<(u64, String)> {
    let mut handle = crate::scan::files::open_regular_readonly(path)?;
    let before = crate::scan::files::stamp(&handle)?;
    let hash = crate::metadata::hash(&mut handle)?;
    if before != crate::scan::files::stamp(&handle)? {
        return Err(Error::Invalid("A backup file changed during verification."));
    }
    Ok((before.bytes as u64, hex(&hash)))
}
fn payload(path: &str) -> bool {
    if path == "library.sqlite3" {
        return true;
    }
    let Some(name) = path
        .strip_prefix("thumbnails/")
        .and_then(|s| s.strip_suffix(".png"))
    else {
        return false;
    };
    let Some((id, revision)) = name.split_once('-') else {
        return false;
    };
    match (id.parse::<i64>(), revision.parse::<i64>()) {
        (Ok(id), Ok(revision)) => {
            id > 0 && revision > 0 && format!("thumbnails/{id}-{revision}.png") == path
        }
        _ => false,
    }
}
pub fn manifest(folder: &Path) -> Result<Manifest> {
    let _guard = directory(folder)?;
    let handle = crate::scan::files::open_regular_readonly(&folder.join("manifest.json"))?;
    if handle.metadata()?.len() > 16 * 1024 * 1024 {
        return Err(Error::Invalid(
            "The backup manifest exceeds the supported size.",
        ));
    }
    let value: Manifest = serde_json::from_reader(handle)?;
    if value.format != 1
        || value.originals_included
        || value.files.is_empty()
        || value.files.len() > 100_001
    {
        return Err(Error::Invalid("This backup format is not supported."));
    }
    if value.database.schema_version == 0
        || value.database.schema_version as usize > crate::database::schema_version()
    {
        return Err(Error::Invalid(
            "This backup requires a newer LibraDesk version.",
        ));
    }
    let mut names = BTreeSet::new();
    for entry in &value.files {
        if !payload(&entry.path)
            || !names.insert(entry.path.clone())
            || entry.sha256.len() != 64
            || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            || entry.path.starts_with("thumbnails/") && entry.bytes > 512 * 1024
        {
            return Err(Error::Invalid(
                "The backup contains unsafe, duplicate, or unsupported entries.",
            ));
        }
    }
    if !names.contains("library.sqlite3") {
        return Err(Error::Invalid("The backup database is missing."));
    }
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_str().unwrap_or_default();
        if name != "manifest.json"
            && name != "library.sqlite3"
            && !(name == "thumbnails" && value.thumbnails_requested)
        {
            return Err(Error::Invalid("The selected folder contains unlisted files. Choose an intact LibraDesk backup folder."));
        }
    }
    if value.thumbnails_requested && folder.join("thumbnails").exists() {
        let _guard = directory(&folder.join("thumbnails"))?;
        for entry in fs::read_dir(folder.join("thumbnails"))? {
            let entry = entry?;
            let name = format!("thumbnails/{}", entry.file_name().to_string_lossy());
            if !names.contains(&name) {
                return Err(Error::Invalid("The backup contains an unlisted thumbnail."));
            }
        }
    }
    Ok(value)
}
pub fn verify_files(folder: &Path, manifest: &Manifest) -> Result<()> {
    let _guard = directory(folder)?;
    let _thumbnails = if manifest
        .files
        .iter()
        .any(|e| e.path.starts_with("thumbnails/"))
    {
        Some(directory(&folder.join("thumbnails"))?)
    } else {
        None
    };
    for entry in &manifest.files {
        let (bytes, hash) = file_digest(&folder.join(&entry.path))?;
        if bytes != entry.bytes || !hash.eq_ignore_ascii_case(&entry.sha256) {
            return Err(Error::Invalid(
                "A backup file failed its size or SHA-256 check.",
            ));
        }
    }
    Ok(())
}
pub fn summarize(folder: &Path, manifest: &Manifest) -> Summary {
    Summary {
        path: folder.to_string_lossy().into_owned(),
        created_at: manifest.created_at,
        schema_version: manifest.database.schema_version,
        books: manifest.database.books,
        notes: manifest.database.notes,
        bookmarks: manifest.database.bookmarks,
        reading_sessions: manifest.database.reading_sessions,
        thumbnails: manifest
            .files
            .iter()
            .filter(|e| e.path.starts_with("thumbnails/"))
            .count(),
        bytes: manifest.files.iter().fold(
            fs::metadata(folder.join("manifest.json"))
                .map(|m| m.len())
                .unwrap_or(0),
            |sum, e| sum.saturating_add(e.bytes),
        ),
    }
}
pub fn snapshot(db: &Database, path: &Path) -> Result<()> {
    let source = db.0.unchecked_transaction()?;
    source.query_row("SELECT count(*) FROM sqlite_schema", [], |row| {
        row.get::<_, i64>(0)
    })?;
    let mut destination = Connection::open(path)?;
    {
        let backup = rusqlite::backup::Backup::new(&db.0, &mut destination)?;
        backup.run_to_completion(128, Duration::from_millis(5), None)?;
    }
    source.commit()?;
    destination.pragma_update(None, "journal_mode", "DELETE")?;
    destination.close().map_err(|(_, error)| error)?;
    Ok(())
}
fn add_entry(folder: &Path, path: String, entries: &mut Vec<Entry>) -> Result<()> {
    let (bytes, sha256) = file_digest(&folder.join(&path))?;
    entries.push(Entry {
        path,
        bytes,
        sha256,
    });
    Ok(())
}
pub fn create(
    db: &Database,
    root: &Path,
    parent: &Path,
    include_thumbnails: bool,
) -> Result<Summary> {
    let _root_guard = directory(root)?;
    let _parent_guard = directory(parent)?;
    let parent = fs::canonicalize(parent)?;
    let temporary = tempfile::Builder::new()
        .prefix(".libradesk-pending-")
        .tempdir_in(&parent)?;
    let stage = temporary.path();
    snapshot(db, &stage.join("library.sqlite3"))?;
    let info = crate::database::validate_snapshot(&stage.join("library.sqlite3"))?;
    let mut entries = Vec::new();
    add_entry(stage, "library.sqlite3".into(), &mut entries)?;
    if include_thumbnails {
        fs::create_dir(stage.join("thumbnails"))?;
        let snapshot = Connection::open_with_flags(
            stage.join("library.sqlite3"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;
        let mut query = snapshot.prepare(
            "SELECT file_id,file_revision FROM thumbnails ORDER BY file_id LIMIT 100001",
        )?;
        let rows = query.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?;
        for (index, row) in rows.enumerate() {
            if index >= 100000 {
                return Err(Error::Invalid("Too many thumbnails for one backup."));
            }
            let (id, revision) = row?;
            // Missing/invalid disposable covers are omitted and regenerate after restore.
            if let Ok(bytes) = crate::metadata::backup_thumbnail(root, id, revision) {
                let name = format!("thumbnails/{id}-{revision}.png");
                let mut output = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(stage.join(&name))?;
                output.write_all(&bytes)?;
                output.sync_all()?;
                drop(output);
                add_entry(stage, name, &mut entries)?;
            }
        }
    }
    let manifest = Manifest {
        format: 1,
        app_version: env!("CARGO_PKG_VERSION").into(),
        created_at: timestamp(),
        database: info,
        thumbnails_requested: include_thumbnails,
        originals_included: false,
        files: entries,
    };
    let mut output = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(stage.join("manifest.json"))?;
    serde_json::to_writer_pretty(&mut output, &manifest)?;
    output.write_all(b"\n")?;
    output.sync_all()?;
    drop(output);
    let checked = self::manifest(stage)?;
    verify_files(stage, &checked)?;
    let suffix = stage
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(Error::Invalid("Invalid backup staging folder."))?;
    let final_path = parent.join(format!(
        "LibraDesk-backup-{}-{}",
        manifest.created_at,
        suffix.trim_start_matches(".libradesk-pending-")
    ));
    fs::rename(stage, &final_path)?;
    Ok(summarize(&final_path, &checked))
}
// Copy only whitelisted manifest payloads into an isolated folder. Adjacent WAL/SHM files are never trusted.
#[cfg(test)]
pub fn copy_verified(folder: &Path, destination: &Path, value: &Manifest) -> Result<()> {
    let _guard = directory(folder)?;
    let _thumbs = if value
        .files
        .iter()
        .any(|e| e.path.starts_with("thumbnails/"))
    {
        Some(directory(&folder.join("thumbnails"))?)
    } else {
        None
    };
    for entry in &value.files {
        if !payload(&entry.path) {
            return Err(Error::Invalid("Unsafe backup entry."));
        }
        let mut input = crate::scan::files::open_regular_readonly(&folder.join(&entry.path))?;
        if entry.path.starts_with("thumbnails/") {
            fs::create_dir_all(destination.join("thumbnails"))?;
        }
        let target = destination.join(&entry.path);
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        io::copy(&mut input, &mut output)?;
        output.sync_all()?;
        drop(output);
        let (bytes, hash) = file_digest(&target)?;
        if bytes != entry.bytes || !hash.eq_ignore_ascii_case(&entry.sha256) {
            return Err(Error::Invalid(
                "A backup file changed or failed its SHA-256 check.",
            ));
        }
    }
    Ok(())
}
