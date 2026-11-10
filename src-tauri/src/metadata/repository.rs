use std::path::Path;

use rusqlite::{params, OptionalExtension, Row, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::database::Database;

use super::pdf::{filename_title, Analysis, RawMetadata, Source};

type Result<T> = rusqlite::Result<T>;
const NOW: &str = "CAST(unixepoch('subsec') * 1000 AS INTEGER)";
const SOURCE_COLUMNS: &str = "f.id,f.revision,f.path_key,f.size_bytes,f.modified_at,f.source_token";

fn source_row(row: &Row<'_>) -> Result<Source> {
    Ok(Source {
        id: row.get(0)?,
        revision: row.get(1)?,
        path: row.get(2)?,
        bytes: row.get(3)?,
        modified_at: row.get(4)?,
        token: row.get(5)?,
    })
}

pub fn refresh_scanned_files(tx: &Transaction<'_>, job_id: i64) -> Result<()> {
    tx.execute(
        "UPDATE book_files AS f SET size_bytes=c.size_bytes,modified_at=c.modified_at,
         source_token=c.modified_token,revision=f.revision+1,sha256=NULL,page_count=NULL,availability='available'
         FROM scan_candidates c
         WHERE c.job_id=?1 AND c.classification='new' AND f.path_key=c.path_key
         AND (f.availability<>'available' OR f.size_bytes<>c.size_bytes OR f.modified_at<>c.modified_at
              OR (f.source_token IS NOT NULL AND f.source_token<>c.modified_token))",
        [job_id],
    )?;
    tx.execute(
        "UPDATE book_files AS f SET availability=c.classification,revision=f.revision+1,sha256=NULL,page_count=NULL
         FROM scan_candidates c WHERE c.job_id=?1 AND f.path_key=c.path_key
         AND c.classification IN ('invalid','inaccessible') AND f.availability<>c.classification",
        [job_id],
    )?;
    tx.execute(
        "UPDATE processing_jobs AS p SET status='queued',error_code=NULL
         FROM book_files f JOIN scan_candidates c ON c.path_key=f.path_key
         WHERE c.job_id=?1 AND c.classification='new' AND p.file_id=f.id AND p.file_revision=f.revision
         AND p.kind='analysis' AND p.status='failed'",
        [job_id],
    )?;
    Ok(())
}

fn effective(raw: Option<&String>) -> Option<&str> {
    raw.map(|value| value.trim())
        .filter(|value| !value.is_empty())
}

fn update_catalog(tx: &Transaction<'_>, source: &Source, raw: Option<&RawMetadata>) -> Result<()> {
    let fallback = filename_title(Path::new(&source.path));
    let title = effective(raw.and_then(|raw| raw.title.as_ref())).unwrap_or(&fallback);
    tx.execute(&format!(
        "UPDATE books AS b SET
         title=CASE WHEN EXISTS(SELECT 1 FROM metadata_overrides WHERE book_id=b.id AND field='title') THEN title ELSE ?2 END,
         title_sort_key=CASE WHEN EXISTS(SELECT 1 FROM metadata_overrides WHERE book_id=b.id AND field='title') THEN title_sort_key ELSE ?3 END,
         author=CASE WHEN EXISTS(SELECT 1 FROM metadata_overrides WHERE book_id=b.id AND field='author') THEN author ELSE ?4 END,
         subject=CASE WHEN EXISTS(SELECT 1 FROM metadata_overrides WHERE book_id=b.id AND field='subject') THEN subject ELSE ?5 END,
         revision=revision+1,updated_at={NOW}
         WHERE b.id=(SELECT book_id FROM book_files WHERE id=?1 AND is_primary=1)"
    ), params![source.id, title, title.to_lowercase(),
               effective(raw.and_then(|raw| raw.author.as_ref())), effective(raw.and_then(|raw| raw.subject.as_ref()))])?;
    Ok(())
}

fn store_raw(tx: &Transaction<'_>, source: &Source, raw: &RawMetadata) -> Result<()> {
    tx.execute(&format!(
        "INSERT INTO pdf_metadata VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,{NOW})
         ON CONFLICT(file_id) DO UPDATE SET file_revision=excluded.file_revision,title=excluded.title,
         author=excluded.author,subject=excluded.subject,creator=excluded.creator,producer=excluded.producer,
         creation_date_raw=excluded.creation_date_raw,page_count=excluded.page_count,extracted_at=excluded.extracted_at"
    ), params![source.id,source.revision,raw.title,raw.author,raw.subject,raw.creator,
               raw.producer,raw.creation_date_raw,raw.page_count])?;
    tx.execute(
        "UPDATE book_files SET page_count=?2 WHERE id=?1",
        params![source.id, raw.page_count],
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverTask {
    pub file_id: i64,
    pub revision: i64,
    pub length: i64,
    pub needs_metadata: bool,
}

pub struct CachedCover {
    pub relative_path: String,
    pub width: i64,
    pub height: i64,
    pub bytes: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRow {
    pub id: i64,
    pub file_id: i64,
    pub title: String,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub path: String,
    pub raw: RawMetadata,
    pub needs_review: bool,
    pub pending: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    Title,
    Author,
    Subject,
}

impl Field {
    fn key(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Author => "author",
            Self::Subject => "subject",
        }
    }
}

#[derive(Deserialize)]
pub struct Edit {
    pub field: Field,
    pub value: String,
}

#[derive(Serialize)]
pub struct DuplicateMember {
    pub title: String,
    pub path: String,
}

#[derive(Serialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub count: i64,
    pub reviewed: bool,
    pub members: Vec<DuplicateMember>,
}

impl Database {
    fn metadata_transaction(&self) -> Result<Transaction<'_>> {
        Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)
    }

    pub fn next_analysis(&self) -> Result<Option<Source>> {
        self.0
            .query_row(
                &format!(
                    "SELECT {SOURCE_COLUMNS} FROM processing_jobs p
             JOIN book_files f ON f.id=p.file_id AND f.revision=p.file_revision
             WHERE p.kind='analysis' AND p.status='queued' AND f.availability='available' ORDER BY p.file_id LIMIT 1"
                ),
                [],
                source_row,
            )
            .optional()
    }

    pub fn complete_analysis(&self, source: &Source, result: Analysis) -> Result<()> {
        let tx = self.metadata_transaction()?;
        if tx.execute(
            "UPDATE processing_jobs SET status='completed',error_code=NULL
             WHERE file_id=?1 AND file_revision=?2 AND kind='analysis' AND status='queued'",
            params![source.id, source.revision],
        )? == 0
        {
            return Ok(());
        }
        tx.execute(
            "UPDATE book_files SET sha256=?2,source_token=?3 WHERE id=?1 AND revision=?4",
            params![
                source.id,
                result.digest.as_slice(),
                result.stamp.token,
                source.revision
            ],
        )?;
        if let Some(raw) = result.metadata.as_ref() {
            store_raw(&tx, source, raw)?;
        }
        update_catalog(&tx, source, result.metadata.as_ref())?;
        tx.execute(
            "INSERT OR IGNORE INTO processing_jobs(file_id,file_revision,kind) VALUES (?1,?2,'cover')",
            params![source.id,source.revision],
        )?;
        tx.commit()
    }

    pub fn fail_processing(&self, id: i64, revision: i64, kind: &str, code: &str) -> Result<()> {
        self.0.execute(
            "UPDATE processing_jobs SET status='failed',error_code=?4
             WHERE file_id=?1 AND file_revision=?2 AND kind=?3 AND status='queued'",
            params![id, revision, kind, code],
        )?;
        Ok(())
    }

    pub fn next_cover(&self) -> Result<Option<CoverTask>> {
        self.0
            .query_row(
                "SELECT f.id,f.revision,f.size_bytes,m.file_id IS NULL FROM processing_jobs p
             JOIN book_files f ON f.id=p.file_id AND f.revision=p.file_revision
             LEFT JOIN pdf_metadata m ON m.file_id=f.id AND m.file_revision=f.revision
             WHERE p.kind='cover' AND p.status='queued' ORDER BY p.file_id LIMIT 1",
                [],
                |row| {
                    Ok(CoverTask {
                        file_id: row.get(0)?,
                        revision: row.get(1)?,
                        length: row.get(2)?,
                        needs_metadata: row.get(3)?,
                    })
                },
            )
            .optional()
    }

    pub fn cover_source(&self, id: i64, revision: i64) -> Result<Option<Source>> {
        self.0.query_row(&format!(
            "SELECT {SOURCE_COLUMNS} FROM processing_jobs p JOIN book_files f ON f.id=p.file_id AND f.revision=p.file_revision
             WHERE f.id=?1 AND f.revision=?2 AND p.kind='cover' AND p.status='queued'"
        ), params![id,revision], source_row).optional()
    }

    pub fn complete_cover(
        &self,
        source: &Source,
        image: Option<CachedCover>,
        raw: Option<RawMetadata>,
    ) -> Result<bool> {
        let tx = self.metadata_transaction()?;
        let state = if image.is_some() {
            "completed"
        } else {
            "failed"
        };
        let error = image.is_none().then_some("cover_unavailable");
        if tx.execute(
            "UPDATE processing_jobs SET status=?3,error_code=?4
             WHERE file_id=?1 AND file_revision=?2 AND kind='cover' AND status='queued'",
            params![source.id, source.revision, state, error],
        )? == 0
        {
            return Ok(false);
        }
        if let Some(raw) = raw {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM pdf_metadata WHERE file_id=?1 AND file_revision=?2)",
                params![source.id, source.revision],
                |row| row.get(0),
            )?;
            if !exists {
                store_raw(&tx, source, &raw)?;
                update_catalog(&tx, source, Some(&raw))?;
            }
        }
        if let Some(image) = image {
            tx.execute(
                &format!(
                    "INSERT INTO thumbnails VALUES (?1,?2,?3,?4,?5,?6,{NOW})
                 ON CONFLICT(file_id) DO UPDATE SET file_revision=excluded.file_revision,
                 relative_path=excluded.relative_path,width=excluded.width,height=excluded.height,
                 size_bytes=excluded.size_bytes,created_at=excluded.created_at"
                ),
                params![
                    source.id,
                    source.revision,
                    image.relative_path,
                    image.width,
                    image.height,
                    image.bytes
                ],
            )?;
        }
        tx.commit()?;
        Ok(true)
    }

    pub fn cover_revision(&self, id: i64) -> Result<Option<i64>> {
        self.0.query_row(
            "SELECT t.file_revision FROM thumbnails t JOIN book_files f ON f.id=t.file_id
             JOIN processing_jobs p ON p.file_id=f.id AND p.file_revision=f.revision AND p.kind='cover'
             WHERE t.file_id=?1 AND t.file_revision=f.revision AND p.status='completed'",
            [id], |row| row.get(0),
        ).optional()
    }

    pub fn requeue_cover(&self, id: i64, revision: i64) -> Result<()> {
        let tx = self.metadata_transaction()?;
        tx.execute(
            "DELETE FROM thumbnails WHERE file_id=?1 AND file_revision=?2",
            params![id, revision],
        )?;
        tx.execute(
            "UPDATE processing_jobs SET status='queued',error_code=NULL
             WHERE file_id=?1 AND file_revision=?2 AND kind='cover' AND status='completed'",
            params![id, revision],
        )?;
        tx.commit()
    }

    pub fn review_page(&self, before: Option<i64>, missing_only: bool) -> Result<Vec<ReviewRow>> {
        let mut statement = self.0.prepare(
            "WITH review AS (
                SELECT b.id,f.id AS file_id,b.title,b.author,b.subject,f.original_path,
                       m.title AS raw_title,m.author AS raw_author,m.subject AS raw_subject,
                       m.creator,m.producer,m.creation_date_raw,m.page_count,
                       f.availability='available' AND (a.status='queued' OR (m.file_id IS NULL AND coalesce(c.status='queued',0))) AS pending,
                       coalesce(CASE WHEN f.availability<>'available' THEN f.availability END,
                                a.error_code, CASE WHEN m.file_id IS NULL THEN c.error_code END) AS error,
                       ((m.title IS NULL OR trim(m.title)='') AND NOT EXISTS(
                           SELECT 1 FROM metadata_overrides WHERE book_id=b.id AND field='title'
                       )) OR ((m.author IS NULL OR trim(m.author)='') AND NOT EXISTS(
                           SELECT 1 FROM metadata_overrides WHERE book_id=b.id AND field='author'
                       )) AS needs_review
                FROM books b JOIN book_files f ON f.book_id=b.id AND f.is_primary=1
                JOIN processing_jobs a ON a.file_id=f.id AND a.file_revision=f.revision AND a.kind='analysis'
                LEFT JOIN processing_jobs c ON c.file_id=f.id AND c.file_revision=f.revision AND c.kind='cover'
                LEFT JOIN pdf_metadata m ON m.file_id=f.id AND m.file_revision=f.revision
                WHERE (?1 IS NULL OR b.id<?1)
             ) SELECT * FROM review WHERE NOT ?2 OR (needs_review AND NOT pending)
             ORDER BY id DESC LIMIT 51",
        )?;
        let rows = statement
            .query_map(params![before, missing_only], |row| {
                Ok(ReviewRow {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    title: row.get(2)?,
                    author: row.get(3)?,
                    subject: row.get(4)?,
                    path: row.get(5)?,
                    raw: RawMetadata {
                        title: row.get(6)?,
                        author: row.get(7)?,
                        subject: row.get(8)?,
                        creator: row.get(9)?,
                        producer: row.get(10)?,
                        creation_date_raw: row.get(11)?,
                        page_count: row.get(12)?,
                    },
                    pending: row.get(13)?,
                    error: row.get(14)?,
                    needs_review: row.get(15)?,
                })
            })?
            .collect();
        rows
    }

    pub fn save_metadata(&self, ids: &[i64], edits: &[Edit]) -> Result<bool> {
        let tx = self.metadata_transaction()?;
        for id in ids {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM books WHERE id=?1)",
                [id],
                |row| row.get(0),
            )?;
            if !exists {
                return Ok(false);
            }
            for edit in edits {
                let field = edit.field.key();
                let value = edit.value.trim();
                let value = (!value.is_empty()).then_some(value);
                tx.execute(&format!(
                    "INSERT INTO metadata_overrides(book_id,field,value,updated_at) VALUES (?1,?2,?3,{NOW})
                     ON CONFLICT(book_id,field) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at"
                ), params![id,field,value])?;
                if edit.field == Field::Title {
                    tx.execute(
                        "UPDATE books SET title=?2,title_sort_key=?3 WHERE id=?1",
                        params![id, value, value.map(str::to_lowercase)],
                    )?;
                } else {
                    tx.execute(
                        &format!("UPDATE books SET {field}=?2 WHERE id=?1"),
                        params![id, value],
                    )?;
                }
            }
            tx.execute(
                &format!("UPDATE books SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
                [id],
            )?;
        }
        tx.commit()?;
        Ok(true)
    }

    pub fn duplicate_groups(&self, after: Option<Vec<u8>>) -> Result<Vec<DuplicateGroup>> {
        let mut statement = self.0.prepare(
            "SELECT f.sha256,count(*),count(*)=count(r.file_id) FROM book_files f
             LEFT JOIN duplicate_reviews r ON r.file_id=f.id AND r.sha256=f.sha256
             WHERE f.sha256 IS NOT NULL AND (?1 IS NULL OR f.sha256>?1)
             GROUP BY f.sha256 HAVING count(*)>1 ORDER BY f.sha256 LIMIT 21",
        )?;
        let groups = statement
            .query_map([after], |row| {
                Ok((
                    row.get::<_, Vec<u8>>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>>>()?;
        let mut members = self.0.prepare(
            "SELECT b.title,f.original_path FROM book_files f JOIN books b ON b.id=f.book_id
             WHERE f.sha256=?1 ORDER BY f.id LIMIT 10",
        )?;
        groups
            .into_iter()
            .map(|(hash, count, reviewed)| {
                let rows = members
                    .query_map([&hash], |row| {
                        Ok(DuplicateMember {
                            title: row.get(0)?,
                            path: row.get(1)?,
                        })
                    })?
                    .collect::<Result<Vec<_>>>()?;
                Ok(DuplicateGroup {
                    hash: hash.iter().map(|byte| format!("{byte:02x}")).collect(),
                    count,
                    reviewed,
                    members: rows,
                })
            })
            .collect()
    }

    pub fn review_duplicate(&self, hash: &[u8], reviewed: bool) -> Result<bool> {
        let tx = self.metadata_transaction()?;
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM book_files WHERE sha256=?1",
            [hash],
            |row| row.get(0),
        )?;
        if count < 2 {
            return Ok(false);
        }
        if reviewed {
            tx.execute(
                &format!(
                    "INSERT OR IGNORE INTO duplicate_reviews
                 SELECT sha256,id,'keep',{NOW} FROM book_files WHERE sha256=?1"
                ),
                [hash],
            )?;
        } else {
            tx.execute("DELETE FROM duplicate_reviews WHERE sha256=?1", [hash])?;
        }
        tx.commit()?;
        Ok(true)
    }
}
