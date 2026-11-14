use super::{
    fail, pdf,
    repository::{self, Attempt},
    Failure, Result,
};
use crate::{database::Database, scan::files};
use rusqlite::{params, OptionalExtension};
use std::{
    fs,
    io::{Seek, SeekFrom},
    path::Path,
    time::Duration,
};
use tauri::{AppHandle, Emitter};
const NOW: &str = "CAST(unixepoch('subsec')*1000 AS INTEGER)";
#[derive(serde::Serialize, Clone)]
struct FailedWrite {
    book: i64,
    title: String,
    message: &'static str,
}
fn emit_failure(app: &AppHandle, db: &Database, file: i64) {
    if let Ok(Some((book,title,code)))=db.0.query_row("SELECT b.id,b.title,j.error_code FROM pdf_write_jobs j JOIN book_files f ON f.id=j.file_id JOIN books b ON b.id=f.book_id WHERE j.file_id=?1 AND j.status='failed'",[file],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).optional(){
        let _=app.emit("pdf-write-failed",FailedWrite{book,title,message:message(&code)});
    }
}
pub fn message(code: &str) -> &'static str {
    match code{
        "source_changed"=>"The source changed or was relinked. Verify/rescan before retrying; no further replacement will be attempted.",
        "pdf_protected"=>"Encrypted or digitally signed PDFs are not modified. Your library edit is saved locally.",
        "pdf_too_large"=>"PDF metadata writes support sources up to 256 MiB. The library edit remains saved locally.",
        "xmp_unsupported"=>"Existing PDF XMP could not be safely preserved. The original PDF was not changed.",
        "pdf_unsupported"=>"The PDF could not be safely parsed or validated. The original PDF was not changed.",
        "metadata_invalid"=>"Metadata is too large or contains unsupported XML characters. Correct the edit and retry.",
        "staging_changed"=>"The staged metadata update is missing or changed. Stop this sync, then retry.",
        "database_unavailable"=>"PDF metadata reconciliation needs database access. Free storage or close locking tools, then retry.",
        "interrupted"=>"The save was interrupted. Retry to reconcile its atomic replacement before further edits.",
        _=>"The source or its folder is locked, read-only or unavailable. Close other readers and retry.",
    }
}
pub fn start(app: AppHandle) {
    let launched = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64;
    std::thread::spawn(move || {
        let db = loop {
            let access = crate::maintenance::ACCESS
                .read()
                .unwrap_or_else(|e| e.into_inner());
            let opened = crate::commands::database_path(&app)
                .ok()
                .and_then(|p| Database::open(&p).ok());
            drop(access);
            if let Some(db) = opened {
                break db;
            }
            std::thread::sleep(Duration::from_secs(2));
        };
        {
            let _access = crate::maintenance::ACCESS
                .read()
                .unwrap_or_else(|e| e.into_inner());
            let _ = db.0.execute(
                "UPDATE pdf_write_jobs SET status='queued' WHERE status='running'",
                [],
            );
            let _=db.0.execute("UPDATE reading_sessions SET ended_at=last_checkpoint_at WHERE ended_at IS NULL AND started_at<?1",[launched]);
        }
        loop {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                process_next(&db, |file| {
                    let _ = app.emit("pdf-write-updated", ());
                    emit_failure(&app, &db, file);
                })
            }))
            .unwrap_or(Err(Failure {
                code: "interrupted",
            }));
            if let Err(error) = &result {
                let _access = crate::maintenance::ACCESS
                    .read()
                    .unwrap_or_else(|e| e.into_inner());
                let _=db.0.execute("UPDATE pdf_write_jobs SET status='failed',error_code=?1 WHERE status='running'",[error.code]);
                let _ = app.emit("pdf-write-updated", ());
            }
            if matches!(result, Ok(true)) {
                let _ = app.emit("metadata-updated", ());
                let _ = app.emit("catalog-updated", ());
            }
            std::thread::sleep(Duration::from_millis(if matches!(result, Ok(true)) {
                150
            } else {
                1000
            }));
        }
    });
}
fn reading(db: &Database, file: i64) -> rusqlite::Result<bool> {
    db.0.query_row(
        "SELECT EXISTS(SELECT 1 FROM reading_sessions WHERE file_id=?1 AND ended_at IS NULL)",
        [file],
        |r| r.get(0),
    )
}
fn failed(db: &Database, file: i64, code: &str, generation: Option<i64>) -> Result<()> {
    db.0.execute(&format!("UPDATE pdf_write_jobs SET status='failed',error_code=?2,updated_at={NOW} WHERE file_id=?1 AND (?3 IS NULL OR generation=?3)"),params![file,code,generation])?;
    Ok(())
}
pub fn process_next(db: &Database, notify: impl Fn(i64)) -> Result<bool> {
    {
        let _access = crate::maintenance::ACCESS.write().map_err(|_| Failure {
            code: "database_unavailable",
        })?;
        if let Some(attempt) = repository::attempt(&db.0, None)? {
            if reading(db, attempt.file)? {
                return Ok(false);
            }
            let result = apply(db, &attempt, false);
            if let Err(error) = result {
                if error.code == "source_changed" {
                    discard(db, &attempt)?;
                }
                failed(db, attempt.file, error.code, None)?;
            }
            notify(attempt.file);
            return Ok(true);
        }
    }
    let access = crate::maintenance::ACCESS.read().map_err(|_| Failure {
        code: "database_unavailable",
    })?;
    let epoch = crate::maintenance::generation();
    let selected=db.0.query_row("SELECT j.file_id FROM pdf_write_jobs j WHERE j.status='queued'
      AND NOT EXISTS(SELECT 1 FROM reading_sessions s WHERE s.file_id=j.file_id AND s.ended_at IS NULL)
      ORDER BY j.file_id LIMIT 1",[],|r|r.get::<_,i64>(0)).optional()?;
    let Some(file) = selected else {
        return Ok(false);
    };
    let tx = db.0.unchecked_transaction()?;
    let source = repository::source(&tx, file)?;
    let generation: i64 = tx.query_row(
        "SELECT generation FROM pdf_write_jobs WHERE file_id=?1",
        [file],
        |r| r.get(0),
    )?;
    let book: i64 = tx.query_row("SELECT book_id FROM book_files WHERE id=?1", [file], |r| {
        r.get(0)
    })?;
    let data = repository::snapshot(&tx, book)?;
    tx.execute(&format!("UPDATE pdf_write_jobs SET status='running',error_code=NULL,updated_at={NOW} WHERE file_id=?1"),[file])?;
    tx.commit()?;
    notify(file);
    let prepared = match pdf::prepare(&source, &data) {
        Ok(value) => value,
        Err(error) => {
            failed(db, file, error.code, Some(generation))?;
            notify(file);
            return Ok(true);
        }
    };
    if !repository::stage_attempt(
        db,
        &source,
        generation,
        prepared.temporary.path(),
        &prepared.original,
        &prepared.digest,
        &prepared.token,
    )? {
        notify(file);
        return Ok(true);
    }
    let (handle, stage) = prepared.temporary.keep().map_err(|_| Failure {
        code: "file_unavailable",
    })?;
    drop(handle);
    let attempt = repository::attempt(&db.0, Some(file))?.ok_or(Failure {
        code: "database_unavailable",
    })?;
    drop(access);
    let _access = crate::maintenance::ACCESS.write().map_err(|_| Failure {
        code: "database_unavailable",
    })?;
    if epoch != crate::maintenance::generation() {
        let _ = fs::remove_file(stage);
        return Ok(false);
    }
    if reading(db, file)? {
        notify(file);
        return Ok(true);
    }
    if let Err(error) = apply(db, &attempt, false) {
        if error.code == "source_changed" {
            discard(db, &attempt)?;
        }
        failed(db, file, error.code, None)?;
    }
    notify(file);
    Ok(true)
}
pub fn apply(db: &Database, attempt: &Attempt, fail_after_replace: bool) -> Result<()> {
    let current = repository::source(&db.0, attempt.file)?;
    if current.path != attempt.path {
        return fail("source_changed");
    }
    let path = Path::new(&attempt.path);
    let parent = path.parent().ok_or(Failure {
        code: "source_changed",
    })?;
    let _parent_guard = crate::storage_management::directory(parent)?;
    let mut file = files::open_checked(parent, path).map_err(|_| Failure {
        code: "file_unavailable",
    })?;
    let stamp = files::stamp(&file)?;
    let digest = crate::metadata::hash(&mut file)?;
    if digest.as_slice() == attempt.prepared {
        repository::finalize(db, attempt, &stamp)?;
        pdf::remove_stage(attempt);
        return Ok(());
    }
    if current.revision != attempt.revision
        || digest.as_slice() != attempt.original
        || stamp.token != attempt.token
    {
        return fail("source_changed");
    }
    let stage = pdf::safe_stage(attempt)?;
    let mut staged = files::open_checked(parent, &stage).map_err(|_| Failure {
        code: "staging_changed",
    })?;
    if crate::metadata::hash(&mut staged)?.as_slice() != attempt.prepared {
        return fail("staging_changed");
    }
    if files::stamp(&file)? != stamp {
        return fail("source_changed");
    }
    drop(file);
    drop(staged);
    pdf::replace(path, &stage)?;
    if fail_after_replace {
        return fail("database_unavailable");
    }
    let mut replaced = files::open_checked(parent, path)?;
    let stamp = files::stamp(&replaced)?;
    replaced.seek(SeekFrom::Start(0))?;
    if crate::metadata::hash(&mut replaced)?.as_slice() != attempt.prepared {
        return fail("source_changed");
    }
    repository::finalize(db, attempt, &stamp)?;
    Ok(())
}
fn discard(db: &Database, attempt: &Attempt) -> Result<()> {
    db.0.execute(
        "DELETE FROM pdf_write_attempts WHERE file_id=?1",
        [attempt.file],
    )?;
    pdf::remove_stage(attempt);
    Ok(())
}
pub fn cancel(db: &Database, file: i64) -> Result<()> {
    if let Some(attempt) = repository::attempt(&db.0, Some(file))? {
        let path = Path::new(&attempt.path);
        let parent = path.parent().ok_or(Failure {
            code: "source_changed",
        })?;
        if let Ok(mut handle) = files::open_checked(parent, path) {
            if crate::metadata::hash(&mut handle)?.as_slice() == attempt.prepared {
                let stamp = files::stamp(&handle)?;
                repository::finalize(db, &attempt, &stamp)?;
            }
        }
        discard(db, &attempt)?;
    }
    db.0.execute("DELETE FROM pdf_write_jobs WHERE file_id=?1", [file])?;
    Ok(())
}
