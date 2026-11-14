use crate::{
    commands::{with_database, CommandError},
    database::{Database, Error},
};
use rusqlite::{params, OptionalExtension};
use tauri::{AppHandle, Emitter};
impl Database {
    pub fn custom_cover(&self, file: i64, revision: i64) -> Result<Option<Vec<u8>>, Error> {
        let Some(revision) = revision.checked_neg().filter(|r| *r > 0) else {
            return Ok(None);
        };
        Ok(self.0.query_row("SELECT c.png FROM custom_covers c JOIN book_files f ON f.book_id=c.book_id WHERE f.id=?1 AND c.revision=?2",params![file,revision],|r|r.get(0)).optional()?)
    }
    pub fn save_custom_cover(
        &self,
        book: i64,
        revision: i64,
        png: Option<&[u8]>,
    ) -> Result<(), Error> {
        if png.is_some_and(|bytes| super::cover::check_png(bytes).is_err()) {
            return Err(Error::InvalidCatalogInput);
        }
        let tx = self.0.unchecked_transaction()?;
        let current: i64 = tx
            .query_row("SELECT revision FROM books WHERE id=?1", [book], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or(Error::CatalogNotFound)?;
        if current != revision {
            return Err(Error::CatalogConflict);
        }
        if let Some(png) = png {
            tx.execute("INSERT INTO custom_covers(book_id,revision,png,updated_at) VALUES(?1,CAST(unixepoch('subsec')*1000 AS INTEGER),?2,CAST(unixepoch('subsec')*1000 AS INTEGER))
            ON CONFLICT(book_id) DO UPDATE SET revision=max(custom_covers.revision+1,excluded.revision),png=excluded.png,updated_at=excluded.updated_at",params![book,png])?;
        } else {
            tx.execute("DELETE FROM custom_covers WHERE book_id=?1", [book])?;
        }
        tx.execute("UPDATE books SET revision=revision+1,updated_at=CAST(unixepoch('subsec')*1000 AS INTEGER) WHERE id=?1",[book])?;
        tx.commit()?;
        Ok(())
    }
}
#[tauri::command]
pub async fn save_custom_cover(
    app: AppHandle,
    book: i64,
    revision: i64,
    png: Option<Vec<u8>>,
) -> Result<(), CommandError> {
    with_database(app.clone(), move |db| {
        db.save_custom_cover(book, revision, png.as_deref())
    })
    .await?;
    let _ = app.emit(super::EVENT, ());
    let _ = app.emit("catalog-updated", ());
    Ok(())
}
