use std::io::Read;

use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::{
    database::{Database, Error},
    metadata::Source,
};

type Result<T> = std::result::Result<T, Error>;
const NOW: &str = "CAST(unixepoch('subsec')*1000 AS INTEGER)";
pub const MAX_PAGES: i64 = 1_000_000;

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Zoom {
    #[default]
    FitWidth,
    FitPage,
    Custom,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    Single,
    #[default]
    Continuous,
}
#[derive(Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ReaderPreferences {
    pub zoom: Zoom,
    pub scale: i64,
    pub layout: Layout,
    pub fullscreen: bool,
    pub shortcuts: bool,
}
impl Default for ReaderPreferences {
    fn default() -> Self {
        Self {
            zoom: Zoom::FitWidth,
            scale: 100,
            layout: Layout::Continuous,
            fullscreen: false,
            shortcuts: true,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderDocument {
    pub session_id: i64,
    pub book_id: i64,
    pub file_id: i64,
    pub revision: i64,
    pub length: i64,
    pub title: String,
    pub resume_page: i64,
    pub stale_progress: bool,
    pub preferences: ReaderPreferences,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressInput {
    pub session_id: i64,
    pub sequence: i64,
    pub page: i64,
    pub total: i64,
    pub active_ms: i64,
    pub close: bool,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationKind {
    Bookmark,
    Note,
}
impl AnnotationKind {
    fn table(self) -> &'static str {
        match self {
            Self::Bookmark => "bookmarks",
            Self::Note => "notes",
        }
    }
    fn column(self) -> &'static str {
        match self {
            Self::Bookmark => "label",
            Self::Note => "body",
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    id: i64,
    pub file_id: Option<i64>,
    pub file_revision: i64,
    pub page: i64,
    pub text: String,
    pub revision: i64,
}
#[derive(Serialize)]
pub struct Annotations {
    pub bookmarks: Vec<Annotation>,
    pub notes: Vec<Annotation>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationInput {
    pub session_id: i64,
    pub id: Option<i64>,
    pub revision: Option<i64>,
    pub page: i64,
    pub text: String,
    pub client_key: String,
}

fn source_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Source> {
    Ok(Source {
        id: row.get(0)?,
        revision: row.get(1)?,
        path: row.get(2)?,
        bytes: row.get(3)?,
        modified_at: row.get(4)?,
        token: row.get(5)?,
    })
}
fn source_for_session(connection: &rusqlite::Connection, session: i64) -> Result<Source> {
    connection.query_row(
        "SELECT f.id,f.revision,f.path_key,f.size_bytes,f.modified_at,f.source_token
         FROM reading_sessions s JOIN book_files f ON f.id=s.file_id AND f.book_id=s.book_id AND f.revision=s.file_revision
         WHERE s.id=?1 AND s.ended_at IS NULL AND f.availability='available'",
        [session],source_from_row,
    ).optional()?.ok_or(Error::ReaderSourceUnavailable)
}

impl Database {
    pub fn reader_preferences(&self) -> Result<ReaderPreferences> {
        let stored: Option<String> = self
            .0
            .query_row(
                "SELECT value_json FROM settings WHERE scope='reader' AND key='viewer'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let preferences: ReaderPreferences = stored
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_default();
        if !(25..=400).contains(&preferences.scale) {
            return Ok(ReaderPreferences::default());
        }
        Ok(preferences)
    }
    pub fn save_reader_preferences(&self, input: &ReaderPreferences) -> Result<()> {
        if !(25..=400).contains(&input.scale) {
            return Err(Error::InvalidCatalogInput);
        }
        self.0.execute(&format!(
            "INSERT INTO settings(scope,key,value_json,updated_at) VALUES ('reader','viewer',?1,{NOW})
             ON CONFLICT(scope,key) DO UPDATE SET value_json=excluded.value_json,updated_at=excluded.updated_at"
        ),[serde_json::to_string(input)?])?;
        Ok(())
    }
    pub fn open_reader(&self, book: i64, file: Option<i64>) -> Result<ReaderDocument> {
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        let source = tx
            .query_row(
                "SELECT id,revision,path_key,size_bytes,modified_at,source_token FROM book_files
             WHERE book_id=?1 AND (?2 IS NULL OR id=?2) AND availability='available'
             ORDER BY is_primary DESC,id LIMIT 1",
                params![book, file],
                source_from_row,
            )
            .optional()?
            .ok_or(Error::ReaderSourceUnavailable)?;
        if source.bytes > 9_007_199_254_740_991 {
            return Err(Error::InvalidCatalogInput);
        }
        let (mut handle, _) = source.open().map_err(|_| Error::ReaderSourceUnavailable)?;
        let mut header = [0; 5];
        handle
            .read_exact(&mut header)
            .map_err(|_| Error::ReaderSourceUnavailable)?;
        if &header != b"%PDF-" {
            return Err(Error::ReaderSourceUnavailable);
        }
        let title = tx.query_row("SELECT title FROM books WHERE id=?1", [book], |row| {
            row.get(0)
        })?;
        let saved: Option<(i64, Option<i64>, i64)> = tx
            .query_row(
                "SELECT page_number,file_id,file_revision FROM reading_progress WHERE book_id=?1",
                [book],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        let stale_progress = saved.as_ref().is_some_and(|(_, id, revision)| {
            *id != Some(source.id) || *revision != source.revision
        });
        tx.execute(
            "UPDATE reading_sessions SET ended_at=last_checkpoint_at WHERE ended_at IS NULL",
            [],
        )?;
        let session_id=tx.query_row(&format!(
            "INSERT INTO reading_sessions(book_id,file_id,file_revision,started_at,last_checkpoint_at)
             VALUES (?1,?2,?3,{NOW},{NOW}) RETURNING id"
        ),params![book,source.id,source.revision],|row|row.get(0))?;
        let preferences = self.reader_preferences()?;
        tx.commit()?;
        Ok(ReaderDocument {
            session_id,
            book_id: book,
            file_id: source.id,
            revision: source.revision,
            length: source.bytes,
            title,
            resume_page: saved.map(|(page, _, _)| page).unwrap_or(1),
            stale_progress,
            preferences,
        })
    }
    pub fn reader_source(&self, session: i64) -> Result<Source> {
        source_for_session(&self.0, session)
    }

    pub fn save_reader_progress(&self, input: &ProgressInput) -> Result<()> {
        if input.sequence < 1
            || !(1..=MAX_PAGES).contains(&input.total)
            || !(1..=input.total).contains(&input.page)
            || input.active_ms < 0
        {
            return Err(Error::InvalidCatalogInput);
        }
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        let session:Option<(i64,i64,i64,i64,i64,Option<i64>)>=tx.query_row(&format!(
            "SELECT book_id,sequence,active_ms,started_at,{NOW},ended_at FROM reading_sessions WHERE id=?1"
        ),[input.session_id],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?))).optional()?;
        let (book, sequence, active, started, now, ended) =
            session.ok_or(Error::CatalogNotFound)?;
        if ended.is_some() || input.sequence <= sequence {
            return Ok(());
        }
        let source = source_for_session(&tx, input.session_id)?;
        source.open().map_err(|_| Error::ReaderSourceUnavailable)?;
        if input.active_ms < active || input.active_ms > (now - started).max(0) + 2000 {
            return Err(Error::InvalidCatalogInput);
        }
        tx.execute(
            "INSERT OR IGNORE INTO reader_session_pages(session_id,page_number) VALUES (?1,?2)",
            params![input.session_id, input.page],
        )?;
        tx.execute(
            "UPDATE reading_sessions SET sequence=?2,active_ms=?3,total_pages=?4,last_checkpoint_at=?5,
             pages_read=(SELECT count(*) FROM reader_session_pages WHERE session_id=?1),
             ended_at=CASE WHEN ?6 THEN ?5 ELSE NULL END WHERE id=?1",
            params![input.session_id,input.sequence,input.active_ms,input.total,now,input.close],
        )?;
        tx.execute(
            "INSERT INTO reading_progress(book_id,file_id,file_revision,page_number,total_pages,last_read_at,session_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(book_id) DO UPDATE SET file_id=excluded.file_id,file_revision=excluded.file_revision,
             page_number=excluded.page_number,total_pages=excluded.total_pages,last_read_at=excluded.last_read_at,session_id=excluded.session_id
             WHERE coalesce(reading_progress.session_id,0)<=excluded.session_id",
            params![book,source.id,source.revision,input.page,input.total,now,input.session_id],
        )?;
        tx.execute(
            "UPDATE book_files SET page_count=?2 WHERE id=?1 AND revision=?3",
            params![source.id, input.total, source.revision],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn close_reader(&self, session: i64) -> Result<()> {
        self.0.execute(&format!(
            "UPDATE reading_sessions SET ended_at={NOW},last_checkpoint_at={NOW} WHERE id=?1 AND ended_at IS NULL"
        ),[session])?;
        Ok(())
    }

    pub fn reader_annotations(&self, book: i64) -> Result<Annotations> {
        let mut lists = Vec::new();
        for kind in [AnnotationKind::Bookmark, AnnotationKind::Note] {
            let mut query=self.0.prepare(&format!(
                "SELECT id,file_id,file_revision,page_number,{},revision FROM {} WHERE book_id=?1 ORDER BY page_number,id",
                kind.column(),kind.table(),
            ))?;
            lists.push(
                query
                    .query_map([book], |row| {
                        Ok(Annotation {
                            id: row.get(0)?,
                            file_id: row.get(1)?,
                            file_revision: row.get(2)?,
                            page: row.get(3)?,
                            text: row.get(4)?,
                            revision: row.get(5)?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<_>>>()?,
            );
        }
        let mut lists = lists.into_iter();
        Ok(Annotations {
            bookmarks: lists.next().unwrap_or_default(),
            notes: lists.next().unwrap_or_default(),
        })
    }

    pub fn save_reader_annotation(
        &self,
        kind: AnnotationKind,
        input: &AnnotationInput,
    ) -> Result<i64> {
        let text = input.text.trim();
        let limit = match kind {
            AnnotationKind::Bookmark => 512,
            AnnotationKind::Note => 32 * 1024,
        };
        if !(1..=MAX_PAGES).contains(&input.page)
            || text.len() > limit
            || text.contains('\0')
            || matches!(kind, AnnotationKind::Note) && text.is_empty()
            || input.client_key.len() > 128
            || input.client_key.is_empty()
        {
            return Err(Error::InvalidCatalogInput);
        }
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        let source = source_for_session(&tx, input.session_id)?;
        let (book, total): (i64, Option<i64>) = tx.query_row(
            "SELECT book_id,total_pages FROM reading_sessions WHERE id=?1",
            [input.session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if input.id.is_none() && total.is_none_or(|total| input.page > total) {
            return Err(Error::InvalidCatalogInput);
        }
        let table = kind.table();
        let column = kind.column();
        let id = if let Some(id) = input.id {
            let changed = tx.execute(
                &format!(
                    "UPDATE {table} SET {column}=?3,revision=revision+1,updated_at={NOW}
                 WHERE id=?1 AND book_id=?2 AND revision=?4"
                ),
                params![
                    id,
                    book,
                    text,
                    input.revision.ok_or(Error::InvalidCatalogInput)?
                ],
            )?;
            if changed != 1 {
                return Err(Error::CatalogConflict);
            }
            id
        } else {
            tx.execute(&format!(
                "INSERT INTO {table}(book_id,file_id,file_revision,page_number,{column},client_key,created_at,updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,{NOW},{NOW}) ON CONFLICT(client_key) DO NOTHING"
            ),params![book,source.id,source.revision,input.page,text,input.client_key])?;
            tx.query_row(
                &format!("SELECT id FROM {table} WHERE client_key=?1 AND book_id=?2"),
                params![input.client_key, book],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(Error::CatalogConflict)?
        };
        tx.commit()?;
        Ok(id)
    }
    pub fn delete_reader_annotation(
        &self,
        kind: AnnotationKind,
        session: i64,
        id: i64,
        revision: i64,
    ) -> Result<()> {
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        source_for_session(&tx, session)?;
        let changed=tx.execute(&format!(
            "DELETE FROM {} WHERE id=?1 AND revision=?2 AND book_id=(SELECT book_id FROM reading_sessions WHERE id=?3)",kind.table()
        ),params![id,revision,session])?;
        if changed != 1 {
            return Err(Error::CatalogConflict);
        }
        tx.commit()?;
        Ok(())
    }
}
