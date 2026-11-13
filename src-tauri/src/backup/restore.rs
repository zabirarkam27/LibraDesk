use super::archive::{self, Error, Manifest, Result, Summary};
use crate::database::Database;
use rusqlite::Connection;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
pub struct Draft {
    pub id: u64,
    pub stage: tempfile::TempDir,
    pub cache: tempfile::TempDir,
    pub archive: PathBuf,
    pub manifest_hash: String,
    pub prepared_hash: String,
    pub created_at: u64,
    pub manifest: Manifest,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub token: u64,
    pub backup: Summary,
    pub restored_schema: u32,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Restored {
    pub safety_backup: String,
    pub books: i64,
}
fn app_version(value: &str) -> Option<[u32; 3]> {
    let parts = value
        .split('-')
        .next()?
        .split('.')
        .map(str::parse::<u32>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .ok()?;
    parts.try_into().ok()
}
pub fn prepare(root: &Path, folder: &Path, id: u64) -> Result<Draft> {
    let _root_guard = crate::storage_management::directory(root)?;
    let _temp_guard = crate::storage_management::directory(&root.join("temp"))?;
    let value = archive::manifest(folder)?;
    let current = app_version(env!("CARGO_PKG_VERSION"))
        .ok_or(Error::Invalid("Invalid application version."))?;
    let source = app_version(&value.app_version)
        .ok_or(Error::Invalid("The backup application version is invalid."))?;
    if source > current {
        return Err(Error::Invalid(
            "This backup was created by a newer LibraDesk version.",
        ));
    }
    let (_, manifest_hash) = archive::file_digest(&folder.join("manifest.json"))?;
    let stage = tempfile::Builder::new()
        .prefix("restore-stage-")
        .tempdir_in(root.join("temp"))?;
    archive::copy_verified(folder, stage.path(), &value)?;
    let file = stage.path().join("library.sqlite3");
    if crate::database::validate_snapshot(&file)? != value.database {
        return Err(Error::Invalid(
            "The manifest does not match the backup database.",
        ));
    }
    let _cache_guard = crate::storage_management::directory(&root.join("cache"))?;
    let cache = tempfile::Builder::new()
        .prefix("covers-")
        .tempdir_in(root.join("cache"))?;
    fs::create_dir(cache.path().join("thumbnails"))?;
    for entry in value
        .files
        .iter()
        .filter(|entry| entry.path.starts_with("thumbnails/"))
    {
        let name = entry
            .path
            .strip_prefix("thumbnails/")
            .and_then(|s| s.strip_suffix(".png"))
            .ok_or(Error::Invalid("Invalid thumbnail entry."))?;
        let (id, revision) = name
            .split_once('-')
            .ok_or(Error::Invalid("Invalid thumbnail entry."))?;
        let id = id
            .parse()
            .map_err(|_| Error::Invalid("Invalid thumbnail ID."))?;
        let revision = revision
            .parse()
            .map_err(|_| Error::Invalid("Invalid thumbnail revision."))?;
        let bytes = crate::metadata::backup_thumbnail(stage.path(), id, revision)?;
        fs::write(cache.path().join(&entry.path), bytes)?;
    }
    let db = Database::open(&file)?;
    db.preferences(None)?;
    db.recover_scans()?;
    db.0.execute_batch("UPDATE reading_sessions SET ended_at=last_checkpoint_at WHERE ended_at IS NULL;
  UPDATE text_extraction_jobs SET lease=lease+1,lease_expires_at=NULL,status=CASE WHEN status='running' THEN 'queued' ELSE status END;
  DELETE FROM relink_drafts;DELETE FROM verification_state;
  INSERT INTO book_search_fts(book_search_fts) VALUES('rebuild');
  INSERT INTO page_text_fts(page_text_fts) VALUES('rebuild');
  INSERT INTO book_search_fts(book_search_fts,rank) VALUES('integrity-check',1);
  INSERT INTO page_text_fts(page_text_fts,rank) VALUES('integrity-check',1);")?;
    let generation = cache
        .path()
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or(Error::Invalid("Invalid cache generation."))?;
    db.0.execute("INSERT INTO settings(scope,key,value_json,updated_at) VALUES('app','cover_cache_generation',?1,?2)
  ON CONFLICT(scope,key) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at",
  rusqlite::params![serde_json::to_string(generation)?,archive::timestamp() as i64])?;
    db.0.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);PRAGMA journal_mode=DELETE;")?;
    drop(db);
    crate::database::validate_snapshot(&file)?;
    let (_, prepared_hash) = archive::file_digest(&file)?;
    Ok(Draft {
        id,
        stage,
        cache,
        archive: fs::canonicalize(folder)?,
        manifest_hash,
        prepared_hash,
        created_at: archive::timestamp(),
        manifest: value,
    })
}
impl Draft {
    pub fn preview(&self) -> Preview {
        Preview {
            token: self.id,
            backup: archive::summarize(&self.archive, &self.manifest),
            restored_schema: crate::database::schema_version() as u32,
        }
    }
}
fn replace(db: &mut Database, path: &Path) -> Result<()> {
    let source = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    {
        let backup = rusqlite::backup::Backup::new(&source, &mut db.0)?;
        backup.run_to_completion(128, Duration::from_millis(5), None)?;
    }
    db.0.flush_prepared_statement_cache();
    db.0.pragma_update(None, "foreign_keys", true)?;
    db.0.pragma_update(None, "journal_mode", "WAL")?;
    let check: String = db.0.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if check != "ok" {
        return Err(Error::Invalid(
            "The restored database failed its integrity check.",
        ));
    }
    Ok(())
}
pub fn apply(
    db: &mut Database,
    root: &Path,
    draft: Draft,
    fail_after_copy: bool,
) -> Result<Restored> {
    if archive::timestamp().saturating_sub(draft.created_at) > 900_000 {
        return Err(Error::Invalid(
            "Restore preview expired. Inspect the backup again.",
        ));
    }
    if archive::file_digest(&draft.archive.join("manifest.json"))?.1 != draft.manifest_hash {
        return Err(Error::Invalid(
            "The selected backup changed after inspection.",
        ));
    }
    archive::verify_files(&draft.archive, &draft.manifest)?;
    let staged = draft.stage.path().join("library.sqlite3");
    if archive::file_digest(&staged)?.1 != draft.prepared_hash {
        return Err(Error::Invalid(
            "The staged database changed after inspection.",
        ));
    }
    // Keep both cache generations. The selected generation changes in the same SQLite copy as library data.
    let safety = archive::create(db, root, &root.join("backups"), false)?;
    let result = replace(db, &staged).and_then(|_| {
        if fail_after_copy {
            Err(Error::Invalid("Injected restore failure."))
        } else {
            Ok(())
        }
    });
    if let Err(error) = result {
        let rollback = replace(db, &Path::new(&safety.path).join("library.sqlite3"));
        if rollback.is_err() {
            let _ = draft.cache.keep();
            return Err(Error::Invalid("Restore and automatic recovery failed. Preserve the safety backup in the managed backups folder and restart LibraDesk."));
        }
        return Err(error);
    }
    let _ = draft.cache.keep();
    Ok(Restored {
        safety_backup: safety.path,
        books: draft.manifest.database.books,
    })
}
