use crate::{database::Database, metadata::Source};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
const NOW: &str = "CAST(unixepoch('subsec')*1000 AS INTEGER)";
pub(crate) fn enqueue(tx: &Transaction<'_>, book: i64) -> rusqlite::Result<()> {
    tx.execute(&format!("INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at) SELECT id,1,'queued',{NOW} FROM book_files WHERE book_id=?1
      ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at"),[book])?;
    Ok(())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub title: String,
    pub subject: Option<String>,
    pub genre: Option<String>,
    pub publisher: Option<String>,
    pub isbn: Option<String>,
    pub year: Option<i64>,
    pub language: Option<String>,
    pub authors: Vec<String>,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub collections: Vec<String>,
    pub favorite: bool,
    pub reading_status: String,
}
pub fn snapshot(db: &Connection, book: i64) -> rusqlite::Result<Snapshot> {
    let mut data=db.query_row("SELECT title,subject,genre,publisher,isbn,publication_year,language,is_favorite,reading_status FROM books WHERE id=?1",[book],|r|Ok(Snapshot{
        title:r.get(0)?,subject:r.get(1)?,genre:r.get(2)?,publisher:r.get(3)?,isbn:r.get(4)?,year:r.get(5)?,language:r.get(6)?,favorite:r.get(7)?,reading_status:r.get(8)?,
        authors:Vec::new(),categories:Vec::new(),tags:Vec::new(),collections:Vec::new(),
    }))?;
    let strings = |sql: &str| -> rusqlite::Result<Vec<String>> {
        let mut query = db.prepare(sql)?;
        let rows = query.query_map([book], |r| r.get(0))?;
        rows.collect()
    };
    data.authors=strings("SELECT a.display_name FROM book_authors ba JOIN authors a ON a.id=ba.author_id WHERE ba.book_id=?1 ORDER BY ba.position")?;
    data.tags=strings("SELECT t.name FROM book_tags bt JOIN tags t ON t.id=bt.tag_id WHERE bt.book_id=?1 ORDER BY t.id")?;
    data.collections=strings("SELECT c.name FROM collection_books cb JOIN collections c ON c.id=cb.collection_id WHERE cb.book_id=?1 ORDER BY c.id")?;
    let categories: HashMap<i64, (String, Option<i64>)> = {
        let mut query = db.prepare("SELECT id,name,parent_id FROM categories")?;
        let rows = query.query_map([], |r| Ok((r.get(0)?, (r.get(1)?, r.get(2)?))))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let mut query = db
        .prepare("SELECT category_id FROM book_categories WHERE book_id=?1 ORDER BY category_id")?;
    let ids = query.query_map([book], |r| r.get::<_, i64>(0))?;
    for id in ids {
        let mut current = Some(id?);
        let mut seen = HashSet::new();
        let mut names = Vec::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                break;
            }
            let Some((name, parent)) = categories.get(&id) else {
                break;
            };
            names.push(name.clone());
            current = *parent;
        }
        names.reverse();
        data.categories.push(names.join(" / "));
    }
    Ok(data)
}
pub(crate) fn source(db: &Connection, file: i64) -> rusqlite::Result<Source> {
    db.query_row("SELECT id,revision,path_key,size_bytes,modified_at,source_token FROM book_files WHERE id=?1",[file],|r|Ok(Source{id:r.get(0)?,revision:r.get(1)?,path:r.get(2)?,bytes:r.get(3)?,modified_at:r.get(4)?,token:r.get(5)?}))
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub file_id: i64,
    pub status: String,
    pub error: Option<String>,
    pub updated_at: Option<i64>,
}
impl Database {
    pub fn pdf_write_status(&self, book: i64) -> rusqlite::Result<Vec<Status>> {
        let mut query=self.0.prepare("SELECT f.id,coalesce(j.status,'not_requested'),j.error_code,j.updated_at FROM book_files f LEFT JOIN pdf_write_jobs j ON j.file_id=f.id WHERE f.book_id=?1 ORDER BY f.id")?;
        let rows = query.query_map([book], |r| {
            let code: Option<String> = r.get(2)?;
            Ok(Status {
                file_id: r.get(0)?,
                status: r.get(1)?,
                error: code.map(|c| super::worker::message(&c).into()),
                updated_at: r.get(3)?,
            })
        })?;
        rows.collect()
    }
}
pub struct Attempt {
    pub file: i64,
    pub generation: i64,
    pub revision: i64,
    pub path: String,
    pub token: String,
    pub original: Vec<u8>,
    pub prepared: Vec<u8>,
    pub stage: String,
}
pub fn attempt(db: &Connection, file: Option<i64>) -> rusqlite::Result<Option<Attempt>> {
    db.query_row("SELECT a.file_id,a.generation,a.source_revision,a.source_path,a.source_token,a.original_sha256,a.prepared_sha256,a.stage_path
    FROM pdf_write_attempts a JOIN pdf_write_jobs j ON j.file_id=a.file_id WHERE (?1 IS NULL AND j.status IN ('queued','running')) OR a.file_id=?1 ORDER BY a.file_id LIMIT 1",
    [file],|r|Ok(Attempt{file:r.get(0)?,generation:r.get(1)?,revision:r.get(2)?,path:r.get(3)?,token:r.get(4)?,original:r.get(5)?,prepared:r.get(6)?,stage:r.get(7)?})).optional()
}
pub fn stage_attempt(
    db: &Database,
    source: &Source,
    generation: i64,
    stage: &std::path::Path,
    original: &[u8],
    prepared: &[u8],
    token: &str,
) -> rusqlite::Result<bool> {
    let tx = db.0.unchecked_transaction()?;
    let current = source_for_compare(&tx, source.id)?;
    let queued: i64 = tx.query_row(
        "SELECT generation FROM pdf_write_jobs WHERE file_id=?1",
        [source.id],
        |r| r.get(0),
    )?;
    if current.0 != source.revision || current.1 != source.path || queued != generation {
        return Ok(false);
    }
    tx.execute(&format!("INSERT INTO pdf_write_attempts(file_id,generation,source_revision,source_path,source_token,original_sha256,prepared_sha256,stage_path,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,{NOW})"),
    params![source.id,generation,source.revision,source.path,token,original,prepared,stage.to_string_lossy()])?;
    tx.commit()?;
    Ok(true)
}
fn source_for_compare(db: &Connection, id: i64) -> rusqlite::Result<(i64, String)> {
    db.query_row(
        "SELECT revision,path_key FROM book_files WHERE id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
}
pub fn finalize(
    db: &Database,
    attempt: &Attempt,
    stamp: &crate::scan::files::Stamp,
) -> rusqlite::Result<()> {
    let tx = db.0.unchecked_transaction()?;
    let current = source_for_compare(&tx, attempt.file)?;
    if current.1 != attempt.path || current.0 < attempt.revision {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let revision = if current.0 == attempt.revision {
        attempt.revision + 1
    } else {
        let hash: Option<Vec<u8>> = tx.query_row(
            "SELECT sha256 FROM book_files WHERE id=?1",
            [attempt.file],
            |r| r.get(0),
        )?;
        if hash.as_deref() != Some(attempt.prepared.as_slice()) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        current.0
    };
    if current.0 == attempt.revision {
        tx.execute("UPDATE book_files SET revision=revision+1,size_bytes=?2,modified_at=?3,source_token=?4,sha256=?5,availability='available' WHERE id=?1",
        params![attempt.file,stamp.bytes,stamp.modified_at,stamp.token,attempt.prepared])?;
    }
    for table in ["notes", "bookmarks", "reading_progress"] {
        tx.execute(
            &format!("UPDATE {table} SET file_revision=?2 WHERE file_id=?1 AND file_revision=?3"),
            params![attempt.file, revision, attempt.revision],
        )?;
    }
    tx.execute(
        "DELETE FROM pdf_write_attempts WHERE file_id=?1",
        [attempt.file],
    )?;
    tx.execute(&format!("UPDATE pdf_write_jobs SET status=CASE WHEN generation=?2 THEN 'completed' ELSE 'queued' END,error_code=NULL,updated_at={NOW} WHERE file_id=?1"),params![attempt.file,attempt.generation])?;
    tx.commit()
}
