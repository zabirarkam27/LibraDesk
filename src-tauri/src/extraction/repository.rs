use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::{
    database::{Database, Error},
    metadata::Source,
};

type Result<T> = std::result::Result<T, Error>;
const NOW: &str = "CAST(unixepoch('subsec')*1000 AS INTEGER)";
const LEASE_MS: i64 = 120_000;
pub const BATCH_PAGES: usize = 4;
pub const MAX_PAGE_BYTES: usize = 256 * 1024;
const MAX_BATCH_BYTES: usize = 1024 * 1024;
const MAX_PAGES: i64 = 1_000_000;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobKey {
    pub file_id: i64,
    pub revision: i64,
    pub lease: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextTask {
    pub file_id: i64,
    pub revision: i64,
    pub lease: i64,
    pub length: i64,
    pub next_page: i64,
    pub total_pages: Option<i64>,
}
#[derive(Deserialize)]
pub struct PageText {
    pub number: i64,
    pub text: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionStatus {
    pub revision: i64,
    pub status: String,
    pub completed_pages: i64,
    pub total_pages: Option<i64>,
    pub pages_with_text: i64,
    pub error: Option<String>,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Failure {
    PasswordRequired,
    InvalidPdf,
    SourceUnavailable,
    PageTextLimit,
    ExtractionTimeout,
    ExtractionFailed,
}
impl Failure {
    fn key(self) -> &'static str {
        match self {
            Self::PasswordRequired => "password_required",
            Self::InvalidPdf => "invalid_pdf",
            Self::SourceUnavailable => "source_unavailable",
            Self::PageTextLimit => "page_text_limit",
            Self::ExtractionTimeout => "extraction_timeout",
            Self::ExtractionFailed => "extraction_failed",
        }
    }
}

fn source_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Source> {
    Ok(Source {
        id: row.get(0)?,
        revision: row.get(1)?,
        path: row.get(2)?,
        bytes: row.get(3)?,
        modified_at: row.get(4)?,
        token: row.get(5)?,
    })
}
fn current_source(connection: &rusqlite::Connection, key: JobKey) -> Result<Option<Source>> {
    Ok(connection
        .query_row(
            &format!(
                "SELECT f.id,f.revision,f.path_key,f.size_bytes,f.modified_at,f.source_token
   FROM text_extraction_jobs j JOIN book_files f ON f.id=j.file_id AND f.revision=j.file_revision
   WHERE j.file_id=?1 AND j.file_revision=?2 AND j.lease=?3 AND j.status='running'
     AND j.lease_expires_at>{NOW} AND f.availability='available'"
            ),
            params![key.file_id, key.revision, key.lease],
            source_row,
        )
        .optional()?)
}
impl Database {
    pub fn claim_text_extraction(&self) -> Result<Option<TextTask>> {
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        let job=tx.query_row(&format!(
   "SELECT j.file_id,j.file_revision,j.lease+1,f.size_bytes,j.next_page,j.total_pages
    FROM text_extraction_jobs j JOIN book_files f ON f.id=j.file_id AND f.revision=j.file_revision
    WHERE f.availability='available' AND (j.status='queued' OR (j.status='running' AND j.lease_expires_at<={NOW}))
    ORDER BY j.updated_at,j.file_id LIMIT 1"
  ),[],|row|Ok(TextTask{file_id:row.get(0)?,revision:row.get(1)?,lease:row.get(2)?,length:row.get(3)?,next_page:row.get(4)?,total_pages:row.get(5)?})).optional()?;
        if let Some(job) = &job {
            tx.execute(&format!(
    "UPDATE text_extraction_jobs SET status='running',lease=?2,lease_expires_at={NOW}+?3,error_code=NULL,updated_at={NOW} WHERE file_id=?1"
   ),params![job.file_id,job.lease,LEASE_MS])?;
        }
        tx.commit()?;
        Ok(job)
    }
    pub fn extraction_source(&self, key: JobKey) -> Result<Option<Source>> {
        current_source(&self.0, key)
    }
    pub fn renew_text_extraction(&self, key: JobKey) -> Result<bool> {
        Ok(self.0.execute(&format!(
   "UPDATE text_extraction_jobs SET lease_expires_at={NOW}+?4
    WHERE file_id=?1 AND file_revision=?2 AND lease=?3 AND status='running' AND lease_expires_at>{NOW}"
  ),params![key.file_id,key.revision,key.lease,LEASE_MS])?==1)
    }
    pub fn commit_page_text(&self, key: JobKey, total: i64, pages: &[PageText]) -> Result<bool> {
        let bytes = pages.iter().map(|page| page.text.len()).sum::<usize>();
        if !(1..=MAX_PAGES).contains(&total)
            || pages.is_empty()
            || pages.len() > BATCH_PAGES
            || !(1..=total).contains(&pages[0].number)
            || bytes > MAX_BATCH_BYTES
            || pages.iter().enumerate().any(|(index, page)| {
                page.number != pages[0].number + index as i64
                    || !(1..=total).contains(&page.number)
                    || page.text.len() > MAX_PAGE_BYTES
                    || page.text.contains('\0')
            })
        {
            return Err(Error::InvalidCatalogInput);
        }
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        let job:Option<(i64,Option<i64>,String)>=tx.query_row(
   "SELECT next_page,total_pages,status FROM text_extraction_jobs WHERE file_id=?1 AND file_revision=?2 AND lease=?3",
   params![key.file_id,key.revision,key.lease],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
  ).optional()?;
        let Some((next, known_total, status)) = job else {
            return Ok(false);
        };
        if known_total.is_some_and(|known| known != total) {
            return Err(Error::InvalidCatalogInput);
        }
        let end = pages[0].number + pages.len() as i64;
        if end <= next && matches!(status.as_str(), "running" | "completed") {
            for page in pages {
                let saved:Option<String>=tx.query_row(
     "SELECT text FROM page_text WHERE file_id=?1 AND file_revision=?2 AND page_number=?3",
     params![key.file_id,key.revision,page.number],|row|row.get(0),
    ).optional()?;
                if saved.as_deref() != Some(&page.text) {
                    return Err(Error::InvalidCatalogInput);
                }
            }
            return Ok(true);
        }
        if pages[0].number != next || status != "running" {
            return Ok(false);
        }
        let Some(source) = current_source(&tx, key)? else {
            return Ok(false);
        };
        source.open().map_err(|_| Error::ReaderSourceUnavailable)?;
        for page in pages {
            tx.execute(
                &format!(
    "INSERT INTO page_text(file_id,file_revision,page_number,text,normalized_text,extracted_at)
     VALUES (?1,?2,?3,?4,?5,{NOW})"
   ),
                params![
                    key.file_id,
                    key.revision,
                    page.number,
                    page.text,
                    crate::catalog::search::normalize(&page.text)
                ],
            )?;
        }
        tx.execute(
            &format!(
                "UPDATE text_extraction_jobs SET next_page=?4,total_pages=?5,
    status=CASE WHEN ?4>?5 THEN 'completed' ELSE 'running' END,
    lease_expires_at=CASE WHEN ?4>?5 THEN NULL ELSE {NOW}+?6 END,error_code=NULL,updated_at={NOW}
    WHERE file_id=?1 AND file_revision=?2 AND lease=?3"
            ),
            params![key.file_id, key.revision, key.lease, end, total, LEASE_MS],
        )?;
        tx.commit()?;
        Ok(true)
    }
    pub fn release_text_extraction(&self, key: JobKey) -> Result<()> {
        self.0.execute(
            &format!(
   "UPDATE text_extraction_jobs SET status='queued',lease_expires_at=NULL,updated_at={NOW}
    WHERE file_id=?1 AND file_revision=?2 AND lease=?3 AND status='running'"
  ),
            params![key.file_id, key.revision, key.lease],
        )?;
        Ok(())
    }
    pub fn fail_text_extraction(&self, key: JobKey, reason: Failure) -> Result<()> {
        self.0.execute(&format!(
   "UPDATE text_extraction_jobs SET status='failed',error_code=?4,lease_expires_at=NULL,updated_at={NOW}
    WHERE file_id=?1 AND file_revision=?2 AND lease=?3 AND status='running' AND lease_expires_at>{NOW}"
  ),params![key.file_id,key.revision,key.lease,reason.key()])?;
        Ok(())
    }
    pub fn retry_text_extraction(&self, file: i64, revision: i64) -> Result<bool> {
        Ok(self.0.execute(&format!(
   "UPDATE text_extraction_jobs SET status='queued',lease_expires_at=NULL,error_code=NULL,updated_at={NOW}
    WHERE file_id=?1 AND file_revision=?2 AND status='failed'
    AND EXISTS(SELECT 1 FROM book_files f WHERE f.id=?1 AND f.revision=?2 AND f.availability='available')"
  ),params![file,revision])?==1)
    }
    pub fn text_extraction_status(&self, file: i64) -> Result<Option<ExtractionStatus>> {
        Ok(self.0.query_row(
   "SELECT j.file_revision,j.status,j.next_page-1,j.total_pages,
    (SELECT count(*) FROM page_text p WHERE p.file_id=j.file_id AND p.file_revision=j.file_revision AND p.normalized_text<>''),
    j.error_code FROM text_extraction_jobs j JOIN book_files f ON f.id=j.file_id AND f.revision=j.file_revision
    WHERE j.file_id=?1",
   [file],|row|Ok(ExtractionStatus{revision:row.get(0)?,status:row.get(1)?,completed_pages:row.get(2)?,
      total_pages:row.get(3)?,pages_with_text:row.get(4)?,error:row.get(5)?}),
  ).optional()?)
    }
}
