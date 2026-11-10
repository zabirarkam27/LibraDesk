use std::path::Path;

use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::database::Database;

use super::files::{self, Pdf, Stamp};

type Result<T> = rusqlite::Result<T>;
const NOW: &str = "CAST(unixepoch('subsec') * 1000 AS INTEGER)";
const ACTIVE: &str = "'queued','running','paused','awaiting_confirmation','importing'";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub(super) id: i64,
    path: String,
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub(super) id: i64,
    root_id: i64,
    path: String,
    pub(super) status: String,
    started_at: Option<i64>,
    finished_at: Option<i64>,
    pub(super) error: Option<String>,
    pub(super) found: i64,
    bytes: i64,
    new_files: i64,
    pub(super) existing: i64,
    pub(super) duplicates: i64,
    pub(super) invalid: i64,
    inaccessible: i64,
    directory_errors: i64,
    pub(super) imported: i64,
    pub(super) failed: i64,
    skipped: i64,
}

#[derive(Serialize)]
pub struct Snapshot {
    pub(super) roots: Vec<Root>,
    pub(super) jobs: Vec<Job>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    id: i64,
    title: String,
    path: String,
    bytes: i64,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Pause,
    Resume,
    Cancel,
    Confirm,
}

pub struct Work {
    pub id: i64,
    pub root: String,
    pub revision: i64,
    pub importing: bool,
}

pub enum Discovery {
    Directory(String),
    File(String, Pdf),
}

pub struct Candidate {
    pub key: String,
    pub bytes: i64,
    pub token: String,
}

fn owns_work(db: &Transaction<'_>, work: &Work, status: &str) -> Result<bool> {
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM scan_jobs WHERE id=?1 AND revision=?2 AND status=?3)",
        params![work.id, work.revision, status],
        |row| row.get(0),
    )
}

impl Database {
    fn scan_transaction(&self) -> Result<Transaction<'_>> {
        Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)
    }

    pub fn scan_snapshot(&self, before: Option<i64>) -> Result<Snapshot> {
        let mut roots = self.0.prepare(
            "SELECT id, display_path, is_enabled FROM scan_roots WHERE retired_at IS NULL ORDER BY display_path",
        )?;
        let roots = roots
            .query_map([], |row| {
                Ok(Root {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    enabled: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        let mut jobs = self.0.prepare(&format!(
            "WITH selected AS (
                SELECT * FROM scan_jobs WHERE status IN ({ACTIVE})
                OR id IN (SELECT id FROM scan_jobs WHERE status NOT IN ({ACTIVE})
                          AND (?1 IS NULL OR id<?1) ORDER BY id DESC LIMIT 31)
             ), counts AS (
                SELECT c.job_id, count(*) AS found, total(c.size_bytes) AS bytes,
                       sum(classification='new') AS new_files, sum(classification='existing') AS existing,
                       sum(classification='duplicate_candidate') AS duplicates, sum(classification='invalid') AS invalid,
                       sum(classification='inaccessible') AS inaccessible, sum(import_state='imported') AS imported,
                       sum(import_state='failed') AS failed, sum(import_state='skipped') AS skipped
                FROM scan_candidates c JOIN selected j ON j.id=c.job_id GROUP BY c.job_id
             )
             SELECT j.id,j.root_id,r.display_path,j.status,j.started_at,j.finished_at,j.error_code,
                    coalesce(c.found,0),coalesce(CAST(c.bytes AS INTEGER),0),coalesce(c.new_files,0),
                    coalesce(c.existing,0),coalesce(c.duplicates,0),coalesce(c.invalid,0),coalesce(c.inaccessible,0),
                    (SELECT count(*) FROM scan_frontier f WHERE f.job_id=j.id AND f.status='inaccessible'),
                    coalesce(c.imported,0),coalesce(c.failed,0),coalesce(c.skipped,0)
             FROM selected j JOIN scan_roots r ON r.id=j.root_id LEFT JOIN counts c ON c.job_id=j.id
             ORDER BY j.id DESC"
        ))?;
        let jobs = jobs
            .query_map([before], |row| {
                Ok(Job {
                    id: row.get(0)?,
                    root_id: row.get(1)?,
                    path: row.get(2)?,
                    status: row.get(3)?,
                    started_at: row.get(4)?,
                    finished_at: row.get(5)?,
                    error: row.get(6)?,
                    found: row.get(7)?,
                    bytes: row.get(8)?,
                    new_files: row.get(9)?,
                    existing: row.get(10)?,
                    duplicates: row.get(11)?,
                    invalid: row.get(12)?,
                    inaccessible: row.get(13)?,
                    directory_errors: row.get(14)?,
                    imported: row.get(15)?,
                    failed: row.get(16)?,
                    skipped: row.get(17)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;
        Ok(Snapshot { roots, jobs })
    }

    pub fn register_roots(&self, paths: &[String]) -> Result<()> {
        let tx = self.scan_transaction()?;
        for path in paths {
            tx.execute(&format!(
                "INSERT INTO scan_roots(display_path,path_key,created_at) VALUES (?1,?2,{NOW})
                 ON CONFLICT(path_key) DO UPDATE SET retired_at=NULL,is_enabled=1 WHERE scan_roots.retired_at IS NOT NULL"
            ), params![files::display(path), path])?;
        }
        tx.commit()
    }

    pub fn update_root(&self, id: i64, enabled: Option<bool>) -> Result<bool> {
        let tx = self.scan_transaction()?;
        let updated = tx.execute(&format!(
            "UPDATE scan_roots SET is_enabled=coalesce(?2,0), retired_at=CASE WHEN ?2 IS NULL THEN {NOW} END
             WHERE id=?1 AND retired_at IS NULL
             AND NOT EXISTS (SELECT 1 FROM scan_jobs WHERE root_id=?1 AND status='importing')"
        ), params![id, enabled])? != 0;
        if updated && enabled != Some(true) {
            tx.execute(&format!(
                "UPDATE scan_jobs SET status='cancelled',revision=revision+1,finished_at={NOW}
                 WHERE root_id=?1 AND status IN ('queued','running','paused','awaiting_confirmation')"
            ), [id])?;
        }
        tx.commit()?;
        Ok(updated)
    }

    pub fn queue_scan(&self, root: i64) -> Result<bool> {
        let tx = self.scan_transaction()?;
        if tx.execute(&format!(
            "INSERT INTO scan_jobs(root_id,status,created_at)
             SELECT id,'queued',{NOW} FROM scan_roots WHERE id=?1 AND is_enabled=1 AND retired_at IS NULL
             AND NOT EXISTS(SELECT 1 FROM scan_jobs WHERE root_id=?1 AND status IN ({ACTIVE}))"
        ), [root])? == 0 {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO scan_frontier(job_id,directory_key) SELECT ?1,path_key FROM scan_roots WHERE id=?2",
            params![tx.last_insert_rowid(), root],
        )?;
        tx.commit()?;
        Ok(true)
    }

    pub fn control_job(&self, id: i64, action: Action) -> Result<bool> {
        let (next, allowed) = match action {
            Action::Pause => ("paused", "'queued','running'"),
            Action::Resume => ("queued", "'paused'"),
            Action::Cancel => (
                "cancelled",
                "'queued','running','paused','awaiting_confirmation'",
            ),
            Action::Confirm => ("importing", "'awaiting_confirmation'"),
        };
        Ok(self.0.execute(
            &format!(
                "UPDATE scan_jobs SET status=?2,revision=revision+1,
             finished_at=CASE WHEN ?2='cancelled' THEN {NOW} END
             WHERE id=?1 AND status IN ({allowed})"
            ),
            params![id, next],
        )? != 0)
    }

    pub fn recover_scans(&self) -> Result<()> {
        self.0.execute(
            "UPDATE scan_jobs SET status='paused',revision=revision+1 WHERE status='running'",
            [],
        )?;
        Ok(())
    }

    pub fn next_work(&self) -> Result<Option<Work>> {
        let tx = self.scan_transaction()?;
        let next = tx.query_row(
            "SELECT j.id,r.path_key,j.revision,j.status FROM scan_jobs j
             JOIN scan_roots r ON r.id=j.root_id WHERE j.status IN ('queued','running','importing')
             ORDER BY CASE j.status WHEN 'importing' THEN 0 WHEN 'running' THEN 1 ELSE 2 END,j.id LIMIT 1",
            [], |row| Ok((Work {
                id: row.get(0)?, root: row.get(1)?, revision: row.get(2)?, importing: false,
            }, row.get::<_, String>(3)?)),
        ).optional()?;
        let Some((mut work, status)) = next else {
            return Ok(None);
        };
        if status == "queued" {
            tx.execute(&format!(
                "UPDATE scan_jobs SET status='running',revision=revision+1,started_at=coalesce(started_at,{NOW}) WHERE id=?1"
            ), [work.id])?;
            work.revision += 1;
        }
        work.importing = status == "importing";
        tx.commit()?;
        Ok(Some(work))
    }

    pub fn next_directory(&self, job: i64) -> Result<Option<String>> {
        self.0.query_row(
            "SELECT directory_key FROM scan_frontier WHERE job_id=?1 AND status='pending' ORDER BY directory_key LIMIT 1",
            [job], |row| row.get(0),
        ).optional()
    }

    pub fn record_discovery(
        &self,
        work: &Work,
        directory: &str,
        entries: Vec<Discovery>,
        done: Option<bool>,
    ) -> Result<bool> {
        let tx = self.scan_transaction()?;
        if !owns_work(&tx, work, "running")? {
            return Ok(false);
        }
        for entry in entries {
            match entry {
                Discovery::Directory(path) => {
                    tx.execute(
                        "INSERT OR IGNORE INTO scan_frontier(job_id,directory_key) VALUES (?1,?2)",
                        params![work.id, path],
                    )?;
                }
                Discovery::File(path, pdf) => {
                    let (class, bytes, modified, token, error) = match pdf {
                        Pdf::Valid(stamp) => (
                            "new",
                            Some(stamp.bytes),
                            Some(stamp.modified_at),
                            Some(stamp.token),
                            None,
                        ),
                        Pdf::Invalid(bytes) => {
                            ("invalid", Some(bytes), None, None, Some("invalid_pdf"))
                        }
                        Pdf::Inaccessible => {
                            ("inaccessible", None, None, None, Some("unreadable_file"))
                        }
                    };
                    tx.execute(
                        "INSERT INTO scan_candidates(job_id,path_key,display_path,classification,size_bytes,modified_at,modified_token,error_code)
                         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
                         ON CONFLICT(job_id,path_key) DO UPDATE SET classification=excluded.classification,
                         size_bytes=excluded.size_bytes,modified_at=excluded.modified_at,modified_token=excluded.modified_token,
                         error_code=excluded.error_code",
                        params![work.id, path, files::display(&path), class, bytes, modified, token, error],
                    )?;
                }
            }
        }
        if let Some(accessible) = done {
            tx.execute(
                "UPDATE scan_frontier SET status=?3 WHERE job_id=?1 AND directory_key=?2",
                params![
                    work.id,
                    directory,
                    if accessible { "done" } else { "inaccessible" }
                ],
            )?;
        }
        tx.commit()?;
        Ok(true)
    }

    pub fn finish_scan(&self, work: &Work) -> Result<()> {
        let tx = self.scan_transaction()?;
        if !owns_work(&tx, work, "running")? {
            return Ok(());
        }
        tx.execute(
            "UPDATE scan_candidates AS c SET classification='existing'
             WHERE job_id=?1 AND classification='new'
             AND EXISTS(SELECT 1 FROM book_files b WHERE b.path_key=c.path_key)",
            [work.id],
        )?;
        tx.execute(
            "UPDATE scan_candidates AS c SET classification='duplicate_candidate'
             WHERE job_id=?1 AND classification='new' AND
             (EXISTS(SELECT 1 FROM book_files b WHERE b.size_bytes=c.size_bytes)
              OR EXISTS(SELECT 1 FROM scan_candidates other
                  WHERE other.job_id=c.job_id AND other.size_bytes=c.size_bytes
                  AND other.classification IN ('new','duplicate_candidate') AND other.path_key<>c.path_key))",
            [work.id],
        )?;
        tx.execute(
            "UPDATE scan_jobs SET status='awaiting_confirmation',revision=revision+1 WHERE id=?1",
            [work.id],
        )?;
        tx.commit()
    }

    pub fn import_candidates(&self, job: i64, limit: i64) -> Result<Vec<Candidate>> {
        let mut statement = self.0.prepare(
            "SELECT path_key,size_bytes,modified_token FROM scan_candidates
             WHERE job_id=?1 AND import_state='pending' AND classification IN ('new','duplicate_candidate')
             ORDER BY classification,path_key LIMIT ?2",
        )?;
        let rows = statement
            .query_map(params![job, limit], |row| {
                Ok(Candidate {
                    key: row.get(0)?,
                    bytes: row.get(1)?,
                    token: row.get(2)?,
                })
            })?
            .collect();
        rows
    }

    pub fn commit_import(
        &self,
        work: &Work,
        checked: Vec<(Candidate, Option<Stamp>)>,
    ) -> Result<()> {
        let tx = self.scan_transaction()?;
        if !owns_work(&tx, work, "importing")? {
            return Ok(());
        }
        if checked.is_empty() {
            tx.execute(&format!("UPDATE scan_jobs SET status='completed',revision=revision+1,finished_at={NOW} WHERE id=?1"), [work.id])?;
        }
        for (candidate, stamp) in checked {
            let existing = tx
                .query_row(
                    "SELECT id FROM book_files WHERE path_key=?1",
                    [&candidate.key],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?;
            let (state, file_id, error) = match (existing, stamp) {
                (Some(id), _) => ("skipped", Some(id), None),
                (None, Some(stamp)) => {
                    let title = Path::new(&candidate.key)
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .map(str::trim)
                        .filter(|title| !title.is_empty())
                        .unwrap_or("Untitled PDF");
                    tx.execute(&format!(
                        "INSERT INTO books(title,title_sort_key,created_at,updated_at) VALUES (?1,?2,{NOW},{NOW})"
                    ), params![title, title.to_lowercase()])?;
                    tx.execute(&format!(
                        "INSERT INTO book_files(book_id,original_path,path_key,size_bytes,modified_at,last_verified_at)
                         VALUES (?1,?2,?3,?4,?5,{NOW})"
                    ), params![tx.last_insert_rowid(), files::display(&candidate.key), candidate.key, stamp.bytes, stamp.modified_at])?;
                    ("imported", Some(tx.last_insert_rowid()), None)
                }
                (None, None) => ("failed", None, Some("source_changed_or_unreadable")),
            };
            tx.execute(
                "UPDATE scan_candidates SET import_state=?3,file_id=?4,error_code=?5 WHERE job_id=?1 AND path_key=?2",
                params![work.id, candidate.key, state, file_id, error],
            )?;
        }
        tx.commit()
    }

    pub fn fail_work(&self, work: &Work, reason: &str) -> Result<()> {
        self.0.execute(&format!(
            "UPDATE scan_jobs SET status='failed',revision=revision+1,finished_at={NOW},error_code=?3
             WHERE id=?1 AND revision=?2 AND status IN ('running','importing')"
        ), params![work.id, work.revision, reason])?;
        Ok(())
    }

    pub fn list_imported(&self, before: Option<i64>) -> Result<Vec<Book>> {
        let mut statement = self.0.prepare(
            "SELECT b.id,b.title,f.original_path,f.size_bytes FROM books b
             JOIN book_files f ON f.book_id=b.id AND f.is_primary=1
             WHERE (?1 IS NULL OR b.id<?1) ORDER BY b.id DESC LIMIT 51",
        )?;
        let rows = statement
            .query_map([before], |row| {
                Ok(Book {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    path: row.get(2)?,
                    bytes: row.get(3)?,
                })
            })?
            .collect();
        rows
    }
}
