use super::probe::Observation;
use crate::{
    database::{Database, Error},
    metadata::{Analysis, Source},
    scan::files,
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::Serialize;
type Result<T> = std::result::Result<T, Error>;
type FileIdentity = (String, i64, i64, Option<String>, Option<Vec<u8>>);
type DraftRow = (
    i64,
    i64,
    String,
    Option<Vec<u8>>,
    String,
    i64,
    i64,
    String,
    Vec<u8>,
);
pub const NOW: &str = "CAST(unixepoch('subsec')*1000 AS INTEGER)";

#[derive(Clone)]
pub struct RegisteredFile {
    pub source: Source,
    pub hash: Option<Vec<u8>>,
    pub availability: String,
}
fn registered(row: &rusqlite::Row<'_>) -> rusqlite::Result<RegisteredFile> {
    Ok(RegisteredFile {
        source: Source {
            id: row.get(0)?,
            revision: row.get(1)?,
            path: row.get(2)?,
            bytes: row.get(3)?,
            modified_at: row.get(4)?,
            token: row.get(5)?,
        },
        hash: row.get(6)?,
        availability: row.get(7)?,
    })
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRow {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub path: String,
    pub revision: i64,
    pub availability: String,
    pub bytes: i64,
    pub pages: Option<i64>,
    pub checked_at: Option<i64>,
    pub result: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub status: String,
    pub checked: i64,
    pub total: i64,
    pub missing: i64,
    pub inaccessible: i64,
    pub invalid: i64,
    pub changed: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelinkPreview {
    pub draft: i64,
    pub file_id: i64,
    pub title: String,
    pub original_path: String,
    pub replacement_path: String,
    pub old_hash: Option<String>,
    pub new_hash: String,
    pub identical: bool,
    pub old_pages: Option<i64>,
    pub new_pages: Option<i64>,
    pub old_title: Option<String>,
    pub new_title: Option<String>,
    pub old_author: Option<String>,
    pub new_author: Option<String>,
    pub bytes: i64,
}
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub struct Draft {
    pub id: i64,
    pub file: RegisteredFile,
    pub candidate: Source,
    pub digest: Vec<u8>,
}
impl Database {
    pub fn recovery_file(&self, id: i64) -> Result<RegisteredFile> {
        self.0.query_row("SELECT id,revision,path_key,size_bytes,modified_at,source_token,sha256,availability FROM book_files WHERE id=?1",
   [id],registered).optional()?.ok_or(Error::CatalogNotFound)
    }
    pub fn start_verification(&self) -> Result<()> {
        self.0.execute(
            &format!("DELETE FROM relink_drafts WHERE created_at<{NOW}-900000"),
            [],
        )?;
        self.0.execute(
            &format!(
                "INSERT INTO verification_state(id,status,upper_id,started_at)
   VALUES(1,'running',(SELECT coalesce(max(id),0) FROM book_files),{NOW})
   ON CONFLICT(id) DO UPDATE SET status='running',cursor=0,upper_id=excluded.upper_id,checked=0,
   started_at=excluded.started_at,finished_at=NULL WHERE verification_state.status='completed'"
            ),
            [],
        )?;
        Ok(())
    }
    pub fn verification_next(&self) -> Result<Option<RegisteredFile>> {
        Ok(self.0.query_row("SELECT f.id,f.revision,f.path_key,f.size_bytes,f.modified_at,f.source_token,f.sha256,f.availability
   FROM book_files f,verification_state v WHERE v.id=1 AND v.status='running' AND f.id>v.cursor AND f.id<=v.upper_id
   ORDER BY f.id LIMIT 1",[],registered).optional()?)
    }
    pub fn finish_verification(&self) -> Result<bool> {
        let changed=self.0.execute(&format!("UPDATE verification_state SET status='completed',finished_at={NOW}
   WHERE id=1 AND status='running' AND NOT EXISTS(SELECT 1 FROM book_files WHERE id>cursor AND id<=upper_id)"),[])?;
        Ok(changed == 1)
    }
    pub fn verification_due(&self) -> Result<bool> {
        Ok(self.0.query_row(
            &format!(
                "SELECT NOT EXISTS(SELECT 1 FROM verification_state WHERE id=1 AND
   (status='running' OR finished_at>{NOW}-3600000))"
            ),
            [],
            |r| r.get(0),
        )?)
    }
    pub fn apply_verification(
        &self,
        file: &RegisteredFile,
        observation: Observation,
    ) -> Result<bool> {
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        let current:Option<FileIdentity>=tx.query_row(
   "SELECT path_key,size_bytes,modified_at,source_token,sha256 FROM book_files WHERE id=?1 AND revision=?2",
   params![file.source.id,file.source.revision],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional()?;
        let owned = current.as_ref().is_some_and(|c| {
            c.0 == file.source.path
                && c.1 == file.source.bytes
                && c.2 == file.source.modified_at
                && c.3 == file.source.token
                && c.4 == file.hash
        });
        if !owned {
            return Ok(false);
        }
        let identical = observation
            .analysis
            .as_ref()
            .is_some_and(|a| file.hash.as_deref() == Some(a.digest.as_slice()));
        let content_changed = observation.analysis.is_some() && !identical;
        let result = if observation.status != "available" {
            observation.status
        } else if content_changed {
            "changed"
        } else if identical {
            "unchanged_content"
        } else if file.availability != "available" {
            "restored"
        } else {
            "unchanged"
        };
        if let Some(stamp) = observation
            .stamp
            .as_ref()
            .filter(|_| observation.status == "available")
        {
            tx.execute(&format!("UPDATE book_files SET availability='available',size_bytes=?3,modified_at=?4,source_token=?5,
    revision=revision+?6,sha256=CASE WHEN ?6 THEN NULL ELSE sha256 END,
    page_count=CASE WHEN ?6 THEN NULL ELSE page_count END,last_verified_at={NOW} WHERE id=?1 AND revision=?2"),
    params![file.source.id,file.source.revision,stamp.bytes,stamp.modified_at,stamp.token,content_changed])?;
        } else {
            tx.execute(&format!("UPDATE book_files SET availability=?3,last_verified_at={NOW} WHERE id=?1 AND revision=?2"),
    params![file.source.id,file.source.revision,observation.status])?;
        }
        tx.execute(&format!("INSERT INTO file_health(file_id,result,checked_at,changed_at) VALUES(?1,?2,{NOW},CASE WHEN ?3 THEN {NOW} END)
   ON CONFLICT(file_id) DO UPDATE SET result=excluded.result,checked_at=excluded.checked_at,
   changed_at=coalesce(excluded.changed_at,file_health.changed_at)"),
   params![file.source.id,result,content_changed])?;
        tx.execute("UPDATE verification_state SET cursor=?1,checked=checked+1 WHERE id=1 AND status='running' AND cursor<?1",[file.source.id])?;
        tx.commit()?;
        if content_changed {
            if let Some(analysis) = observation.analysis {
                let source = self.recovery_file(file.source.id)?.source;
                // The existing repository preserves metadata overrides and queues a fresh cover.
                if source.revision == file.source.revision + 1 {
                    self.complete_analysis(&source, analysis)?;
                }
            }
        }
        Ok(true)
    }
    pub fn recovery_snapshot(&self) -> Result<Verification> {
        let (status, checked, upper): (String, i64, i64) = self
            .0
            .query_row(
                "SELECT status,checked,upper_id FROM verification_state WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?
            .unwrap_or(("completed".into(), 0, 0));
        let total = self.0.query_row(
            "SELECT count(*) FROM book_files WHERE id<=?1",
            [upper],
            |r| r.get(0),
        )?;
        let (missing,inaccessible,invalid)=self.0.query_row("SELECT coalesce(sum(availability='missing'),0),
   coalesce(sum(availability='inaccessible'),0),coalesce(sum(availability='invalid'),0) FROM book_files",[],
   |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        let changed = self.0.query_row(
            "SELECT count(*) FROM file_health WHERE changed_at IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        Ok(Verification {
            status,
            checked,
            total,
            missing,
            inaccessible,
            invalid,
            changed,
        })
    }
    pub fn recovery_files(&self, after: i64, unavailable: bool) -> Result<Vec<FileRow>> {
        if after < 0 {
            return Err(Error::InvalidCatalogInput);
        }
        let mut statement=self.0.prepare("SELECT f.id,f.book_id,b.title,f.original_path,f.revision,f.availability,
   f.size_bytes,f.page_count,h.checked_at,h.result FROM book_files f JOIN books b ON b.id=f.book_id
   LEFT JOIN file_health h ON h.file_id=f.id WHERE f.id>?1 AND (NOT ?2 OR f.availability<>'available') ORDER BY f.id LIMIT 51")?;
        let rows = statement
            .query_map(params![after, unavailable], |r| {
                Ok(FileRow {
                    id: r.get(0)?,
                    book_id: r.get(1)?,
                    title: r.get(2)?,
                    path: r.get(3)?,
                    revision: r.get(4)?,
                    availability: r.get(5)?,
                    bytes: r.get(6)?,
                    pages: r.get(7)?,
                    checked_at: r.get(8)?,
                    result: r.get(9)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }
    pub fn create_relink_draft(
        &self,
        file: &RegisteredFile,
        candidate: &Source,
        analysis: &Analysis,
    ) -> Result<RelinkPreview> {
        let current = self.recovery_file(file.source.id)?;
        if current.source.revision != file.source.revision
            || current.source.path != file.source.path
            || current.hash != file.hash
        {
            return Err(Error::CatalogConflict);
        }
        let collision: bool = self.0.query_row(
            "SELECT EXISTS(SELECT 1 FROM book_files WHERE path_key=?1)",
            [&candidate.path],
            |r| r.get(0),
        )?;
        if collision {
            return Err(Error::CatalogConflict);
        }
        let tx = self.0.unchecked_transaction()?;
        tx.execute(
            &format!("DELETE FROM relink_drafts WHERE created_at<{NOW}-900000 OR file_id=?1"),
            [file.source.id],
        )?;
        tx.execute(&format!("INSERT INTO relink_drafts(file_id,file_revision,old_path,old_hash,candidate_path,candidate_bytes,candidate_modified,
   candidate_token,candidate_hash,metadata_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,{NOW})"),
   params![file.source.id,file.source.revision,file.source.path,file.hash,candidate.path,candidate.bytes,candidate.modified_at,
    candidate.token,analysis.digest.as_slice(),serde_json::to_string(&analysis.metadata)?])?;
        let draft = tx.last_insert_rowid();
        let (title,path,old_pages,old_title,old_author):(String,String,Option<i64>,Option<String>,Option<String>)=tx.query_row(
   "SELECT b.title,f.original_path,f.page_count,m.title,m.author FROM book_files f JOIN books b ON b.id=f.book_id
    LEFT JOIN pdf_metadata m ON m.file_id=f.id AND m.file_revision=f.revision WHERE f.id=?1",[file.source.id],
    |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        tx.commit()?;
        Ok(RelinkPreview {
            draft,
            file_id: file.source.id,
            title,
            original_path: path,
            replacement_path: files::display(&candidate.path),
            old_hash: file.hash.as_deref().map(hex),
            new_hash: hex(&analysis.digest),
            identical: file.hash.as_deref() == Some(analysis.digest.as_slice()),
            old_pages,
            new_pages: analysis.metadata.as_ref().and_then(|r| r.page_count),
            old_title,
            old_author,
            new_title: analysis.metadata.as_ref().and_then(|r| r.title.clone()),
            new_author: analysis.metadata.as_ref().and_then(|r| r.author.clone()),
            bytes: candidate.bytes,
        })
    }
    pub fn relink_draft(&self, id: i64) -> Result<Draft> {
        let (file,revision,old_path,old_hash,path,bytes,modified,token,digest):DraftRow=
   self.0.query_row(&format!("SELECT file_id,file_revision,old_path,old_hash,candidate_path,candidate_bytes,candidate_modified,
    candidate_token,candidate_hash FROM relink_drafts WHERE id=?1 AND created_at>{NOW}-900000"),[id],
    |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?)))
    .optional()?.ok_or(Error::CatalogConflict)?;
        let current = self.recovery_file(file)?;
        if current.source.revision != revision
            || current.source.path != old_path
            || current.hash != old_hash
        {
            return Err(Error::CatalogConflict);
        }
        Ok(Draft {
            id,
            file: current,
            candidate: Source {
                id: file,
                revision,
                path,
                bytes,
                modified_at: modified,
                token: Some(token),
            },
            digest,
        })
    }
    pub fn confirm_relink(&self, draft: &Draft) -> Result<()> {
        // Commands hash the replacement outside the database lock before entering this transaction.
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        let current = self.relink_draft(draft.id)?;
        if current.candidate.path != draft.candidate.path || current.digest != draft.digest {
            return Err(Error::CatalogConflict);
        }
        if tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM book_files WHERE path_key=?1)",
            [&draft.candidate.path],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(Error::CatalogConflict);
        }
        let identical = draft.file.hash.as_deref() == Some(draft.digest.as_slice());
        let book: i64 = tx.query_row(
            "SELECT book_id FROM book_files WHERE id=?1",
            [draft.file.source.id],
            |r| r.get(0),
        )?;
        // Preserve the catalog as currently edited, including when a different PDF is selected.
        for field in ["title", "author", "subject"] {
            tx.execute(&format!("INSERT OR IGNORE INTO metadata_overrides(book_id,field,value,updated_at) SELECT id,?2,{field},{NOW} FROM books WHERE id=?1"),
    params![book,field])?;
        }
        tx.execute(&format!("UPDATE book_files SET path_key=?2,original_path=?3,size_bytes=?4,modified_at=?5,source_token=?6,
   availability='available',revision=revision+?7,sha256=CASE WHEN ?7 THEN NULL ELSE sha256 END,
   page_count=CASE WHEN ?7 THEN NULL ELSE page_count END,last_verified_at={NOW} WHERE id=?1"),
   params![draft.file.source.id,draft.candidate.path,files::display(&draft.candidate.path),draft.candidate.bytes,
    draft.candidate.modified_at,draft.candidate.token,!identical])?;
        tx.execute(
            &format!("UPDATE books SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
            [book],
        )?;
        tx.execute(&format!("INSERT INTO file_health(file_id,result,checked_at,changed_at) VALUES(?1,?2,{NOW},CASE WHEN ?3 THEN {NOW} END)
   ON CONFLICT(file_id) DO UPDATE SET result=excluded.result,checked_at=excluded.checked_at,changed_at=coalesce(excluded.changed_at,file_health.changed_at)"),
   params![draft.file.source.id,if identical {"relinked_same"} else {"relinked_changed"},!identical])?;
        tx.execute("DELETE FROM relink_drafts WHERE id=?1", [draft.id])?;
        tx.commit()?;
        Ok(())
    }
    pub fn cancel_relink(&self, draft: i64) -> Result<()> {
        self.0
            .execute("DELETE FROM relink_drafts WHERE id=?1", [draft])?;
        Ok(())
    }
}
