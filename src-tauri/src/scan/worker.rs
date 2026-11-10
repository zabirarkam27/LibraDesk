use std::{
    cell::Cell,
    fs,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter};

use crate::{commands::database_path, database::Database};

use super::{
    files::{self, Pdf},
    repository::{Discovery, Work},
};

const SCAN_BATCH: usize = 64;
const IMPORT_BATCH: i64 = 32;
pub const EVENT: &str = "scan-updated";

pub fn start(app: AppHandle) {
    thread::spawn(move || {
        let database = loop {
            let opened = database_path(&app)
                .ok()
                .and_then(|file| Database::open(&file).ok());
            if let Some(database) = opened.filter(|db| db.recover_scans().is_ok()) {
                break database;
            }
            thread::sleep(Duration::from_secs(2));
        };
        let _ = app.emit(EVENT, ());
        let last_event = Cell::new(Instant::now());
        let update_pending = Cell::new(false);
        loop {
            match database.next_work() {
                Ok(Some(work)) => {
                    let notify = || {
                        if last_event.get().elapsed() >= Duration::from_millis(150) {
                            let _ = app.emit(EVENT, work.id);
                            last_event.set(Instant::now());
                            update_pending.set(false);
                        } else {
                            update_pending.set(true);
                        }
                    };
                    if let Err(error) = step(&database, &work, notify) {
                        eprintln!("LibraDesk scan job {} failed: {error}", work.id);
                        let _ = database.fail_work(&work, "database_error");
                        thread::sleep(Duration::from_secs(1));
                    }
                    notify();
                }
                Ok(None) => {
                    if update_pending.replace(false) {
                        let _ = app.emit(EVENT, ());
                        last_event.set(Instant::now());
                    }
                    thread::sleep(Duration::from_millis(350));
                }
                Err(_) => thread::sleep(Duration::from_secs(2)),
            }
        }
    });
}

pub(super) fn step(db: &Database, work: &Work, notify: impl Fn()) -> rusqlite::Result<()> {
    let root = Path::new(&work.root);
    if work.importing {
        let checked = db
            .import_candidates(work.id, IMPORT_BATCH)?
            .into_iter()
            .map(|candidate| {
                let stamp = match files::validate(root, Path::new(&candidate.key)) {
                    Pdf::Valid(stamp)
                        if stamp.bytes == candidate.bytes && stamp.token == candidate.token =>
                    {
                        Some(stamp)
                    }
                    _ => None,
                };
                (candidate, stamp)
            })
            .collect();
        return db.commit_import(work, checked);
    }

    let Some(directory) = db.next_directory(work.id)? else {
        return db.finish_scan(work);
    };
    let entries = match files::read_directory(root, Path::new(&directory)) {
        Ok(entries) => entries,
        Err(_) if directory == work.root => {
            db.record_discovery(work, &directory, Vec::new(), Some(false))?;
            return db.fail_work(work, "root_unavailable");
        }
        Err(_) => {
            db.record_discovery(work, &directory, Vec::new(), Some(false))?;
            return Ok(());
        }
    };

    let mut batch = Vec::with_capacity(SCAN_BATCH);
    let mut accessible = true;
    for (index, entry) in entries.enumerate() {
        match entry {
            Ok(entry) => {
                if let Some(found) = inspect_entry(root, &entry.path()) {
                    batch.push(found);
                }
            }
            Err(_) => accessible = false,
        }
        if (index + 1) % SCAN_BATCH == 0 {
            let entries = std::mem::replace(&mut batch, Vec::with_capacity(SCAN_BATCH));
            if !db.record_discovery(work, &directory, entries, None)? {
                return Ok(());
            }
            notify();
        }
    }
    db.record_discovery(work, &directory, batch, Some(accessible))?;
    Ok(())
}

fn inspect_entry(root: &Path, path: &Path) -> Option<Discovery> {
    let key = files::key(path).ok()?.to_owned();
    match fs::symlink_metadata(path) {
        Ok(metadata) if files::reparse(&metadata) => None,
        Ok(metadata) if metadata.is_dir() => Some(Discovery::Directory(key)),
        _ if files::is_pdf(path) => Some(Discovery::File(key, files::validate(root, path))),
        _ => None,
    }
}
