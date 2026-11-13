use super::repository::{RegisteredFile, NOW};
use crate::database::{Database, Error};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
type Result<T> = std::result::Result<T, Error>;
#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateAction {
    Prefer,
    Remove,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    pub id: i64,
    pub revision: i64,
    pub book_id: i64,
    pub title: String,
    pub author: Option<String>,
    pub path: String,
    pub bytes: i64,
    pub pages: Option<i64>,
    pub availability: String,
    pub checked_at: Option<i64>,
    pub preferred: bool,
    pub notes: i64,
    pub bookmarks: i64,
}
fn digest(hash: &str) -> Result<Vec<u8>> {
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::InvalidCatalogInput);
    }
    (0..64)
        .step_by(2)
        .map(|i| u8::from_str_radix(&hash[i..i + 2], 16).map_err(|_| Error::InvalidCatalogInput))
        .collect()
}
impl Database {
    pub fn duplicate_files(&self, hash: &str, after: i64) -> Result<Vec<DuplicateFile>> {
        if after < 0 {
            return Err(Error::InvalidCatalogInput);
        }
        let digest = digest(hash)?;
        let mut s=self.0.prepare("SELECT f.id,f.revision,b.id,b.title,b.author,f.original_path,f.size_bytes,f.page_count,f.availability,
   f.last_verified_at,EXISTS(SELECT 1 FROM duplicate_preferences d WHERE d.sha256=f.sha256 AND d.file_id=f.id),
   (SELECT count(*) FROM notes WHERE book_id=b.id),(SELECT count(*) FROM bookmarks WHERE book_id=b.id)
   FROM book_files f JOIN books b ON b.id=f.book_id WHERE f.sha256=?1 AND f.id>?2 ORDER BY f.id LIMIT 51")?;
        let rows = s
            .query_map(params![digest, after], |r| {
                Ok(DuplicateFile {
                    id: r.get(0)?,
                    revision: r.get(1)?,
                    book_id: r.get(2)?,
                    title: r.get(3)?,
                    author: r.get(4)?,
                    path: r.get(5)?,
                    bytes: r.get(6)?,
                    pages: r.get(7)?,
                    availability: r.get(8)?,
                    checked_at: r.get(9)?,
                    preferred: r.get(10)?,
                    notes: r.get(11)?,
                    bookmarks: r.get(12)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(rows)
    }
    pub fn prefer_duplicate(&self, hash: &str, file: i64, revision: i64) -> Result<()> {
        let digest = digest(hash)?;
        if !self.0.query_row(
            "SELECT EXISTS(SELECT 1 FROM book_files WHERE id=?1 AND revision=?2 AND sha256=?3)",
            params![file, revision, digest],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(Error::CatalogConflict);
        }
        self.0.execute("INSERT INTO duplicate_preferences(sha256,file_id) VALUES(?1,?2) ON CONFLICT(sha256) DO UPDATE SET file_id=excluded.file_id",
   params![digest,file])?;
        Ok(())
    }
    pub fn duplicate_keep_source(&self, hash: &str, keep: i64) -> Result<RegisteredFile> {
        let digest = digest(hash)?;
        let source = self.recovery_file(keep)?;
        if source.hash.as_deref() != Some(&digest) || source.availability != "available" {
            return Err(Error::CatalogConflict);
        }
        Ok(source)
    }
    pub fn remove_duplicate(&self, file: i64, revision: i64, keep: &RegisteredFile) -> Result<()> {
        if file == keep.source.id {
            return Err(Error::InvalidCatalogInput);
        }
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        let protected = self.recovery_file(keep.source.id)?;
        if protected.source.revision != keep.source.revision
            || protected.source.path != keep.source.path
            || protected.hash != keep.hash
            || protected.availability != "available"
        {
            return Err(Error::CatalogConflict);
        }
        let book:Option<(i64,bool)>=tx.query_row("SELECT book_id,is_primary FROM book_files WHERE id=?1 AND revision=?2 AND sha256=?3",
   params![file,revision,keep.hash],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
        let Some((book, primary)) = book else {
            return Err(Error::CatalogConflict);
        };
        let total: i64 = tx.query_row(
            "SELECT count(*) FROM book_files WHERE book_id=?1",
            [book],
            |r| r.get(0),
        )?;
        if total == 1 {
            tx.execute(
                &format!(
                    "UPDATE collections SET revision=revision+1,updated_at={NOW} WHERE id IN
    (SELECT collection_id FROM collection_books WHERE book_id=?1)"
                ),
                [book],
            )?;
            tx.execute("DELETE FROM books WHERE id=?1", [book])?;
        } else {
            tx.execute("DELETE FROM book_files WHERE id=?1", [file])?;
            if primary {
                tx.execute("UPDATE book_files SET is_primary=1 WHERE id=(SELECT min(id) FROM book_files WHERE book_id=?1)",[book])?;
            }
            tx.execute(
                &format!("UPDATE books SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
                [book],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}
