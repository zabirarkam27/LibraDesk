use std::collections::HashSet;

use rusqlite::{params, Connection, OptionalExtension, Row, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};

use crate::{
    database::{Database, Error},
    scan::files::{self, Stamp},
};

type Result<T> = std::result::Result<T, Error>;
const NOW: &str = "CAST(unixepoch('subsec') * 1000 AS INTEGER)";
const FILE_COLUMNS: &str = "id,book_id,original_path,size_bytes,modified_at,last_verified_at,page_count,availability,revision,is_primary";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookSummary {
    id: i64,
    title: String,
    path: Option<String>,
    bytes: Option<i64>,
    file_id: Option<i64>,
    author: Option<String>,
    pages: Option<i64>,
    cover_revision: Option<i64>,
    duplicate: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookDetails {
    pub id: i64,
    pub title: String,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub publisher: Option<String>,
    pub isbn: Option<String>,
    pub publication_year: Option<i64>,
    pub language: Option<String>,
    pub revision: i64,
    pub authors: Vec<Author>,
    pub files: Vec<BookFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookFile {
    pub id: i64,
    book_id: i64,
    path: String,
    bytes: i64,
    modified_at: i64,
    last_verified_at: Option<i64>,
    pages: Option<i64>,
    availability: String,
    revision: i64,
    is_primary: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    pub id: i64,
    pub display_name: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookInput {
    pub title: String,
    pub subject: Option<String>,
    pub publisher: Option<String>,
    pub isbn: Option<String>,
    pub publication_year: Option<i64>,
    pub language: Option<String>,
    pub author_ids: Option<Vec<i64>>,
}

fn trimmed(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

impl BookInput {
    fn validate(&self) -> Result<&str> {
        let title = trimmed(&self.title).ok_or(Error::InvalidCatalogInput)?;
        let fields = [
            Some(title),
            self.subject.as_deref(),
            self.publisher.as_deref(),
            self.isbn.as_deref(),
            self.language.as_deref(),
        ];
        if fields
            .into_iter()
            .flatten()
            .any(|value| value.len() > 4096 || value.contains('\0'))
            || self
                .publication_year
                .is_some_and(|year| !(1..=9999).contains(&year))
        {
            return Err(Error::InvalidCatalogInput);
        }
        if let Some(ids) = &self.author_ids {
            let mut seen = HashSet::new();
            if ids.len() > 100 || ids.iter().any(|id| *id <= 0 || !seen.insert(*id)) {
                return Err(Error::InvalidCatalogInput);
            }
        }
        Ok(title)
    }
}

fn reserve_book_id(tx: &Transaction<'_>) -> rusqlite::Result<i64> {
    tx.query_row(
        "UPDATE catalog_sequences SET last_id=max(last_id,(SELECT coalesce(max(id),0) FROM books))+1
         WHERE kind='book' RETURNING last_id", [], |row| row.get(0),
    )
}

pub(crate) fn insert_book(tx: &Transaction<'_>, title: &str) -> rusqlite::Result<i64> {
    let id = reserve_book_id(tx)?;
    tx.execute(&format!(
        "INSERT INTO books(id,title,title_sort_key,created_at,updated_at) VALUES (?1,?2,?3,{NOW},{NOW})"
    ), params![id, title, title.to_lowercase()])?;
    Ok(id)
}

pub(crate) fn insert_file(
    tx: &Transaction<'_>,
    book_id: i64,
    path_key: &str,
    stamp: &Stamp,
) -> rusqlite::Result<i64> {
    let id: i64 = tx.query_row(
        "UPDATE catalog_sequences SET last_id=max(last_id,(SELECT coalesce(max(id),0) FROM book_files))+1
         WHERE kind='file' RETURNING last_id", [], |row| row.get(0),
    )?;
    tx.execute(&format!(
        "INSERT INTO book_files(id,book_id,original_path,path_key,size_bytes,modified_at,last_verified_at,source_token)
         VALUES (?1,?2,?3,?4,?5,?6,{NOW},?7)"
    ), params![id, book_id, files::display(path_key), path_key, stamp.bytes, stamp.modified_at, stamp.token])?;
    Ok(id)
}

pub(crate) fn save_override(
    tx: &Transaction<'_>,
    id: i64,
    field: &str,
    value: Option<&str>,
) -> rusqlite::Result<()> {
    tx.execute(&format!(
        "INSERT INTO metadata_overrides(book_id,field,value,updated_at) VALUES (?1,?2,?3,{NOW})
         ON CONFLICT(book_id,field) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at"
    ), params![id, field, value])?;
    Ok(())
}

fn insert_author(connection: &Connection, name: &str) -> rusqlite::Result<Author> {
    connection.query_row(
        "INSERT INTO authors(display_name,name_sort_key) VALUES (?1,lower(?1))
         RETURNING id,display_name",
        [name],
        author_row,
    )
}

fn author_row(row: &Row<'_>) -> rusqlite::Result<Author> {
    Ok(Author {
        id: row.get(0)?,
        display_name: row.get(1)?,
    })
}

fn file_row(row: &Row<'_>) -> rusqlite::Result<BookFile> {
    Ok(BookFile {
        id: row.get(0)?,
        book_id: row.get(1)?,
        path: row.get(2)?,
        bytes: row.get(3)?,
        modified_at: row.get(4)?,
        last_verified_at: row.get(5)?,
        pages: row.get(6)?,
        availability: row.get(7)?,
        revision: row.get(8)?,
        is_primary: row.get(9)?,
    })
}

fn book_files(connection: &Connection, id: i64) -> rusqlite::Result<Vec<BookFile>> {
    let mut query = connection.prepare(&format!(
        "SELECT {FILE_COLUMNS} FROM book_files WHERE book_id=?1 ORDER BY is_primary DESC,id"
    ))?;
    let rows = query.query_map([id], file_row)?;
    rows.collect()
}

fn book_details(connection: &Connection, id: i64) -> Result<BookDetails> {
    let mut book = connection.query_row(
        "SELECT id,title,author,subject,publisher,isbn,publication_year,language,revision FROM books WHERE id=?1",
        [id], |row| Ok(BookDetails {
            id: row.get(0)?,
            title: row.get(1)?,
            author: row.get(2)?,
            subject: row.get(3)?,
            publisher: row.get(4)?,
            isbn: row.get(5)?,
            publication_year: row.get(6)?,
            language: row.get(7)?,
            revision: row.get(8)?,
            authors: Vec::new(),
            files: Vec::new(),
        }),
    ).optional()?.ok_or(Error::CatalogNotFound)?;
    let mut query = connection.prepare(
        "SELECT a.id,a.display_name FROM book_authors ba JOIN authors a ON a.id=ba.author_id
         WHERE ba.book_id=?1 ORDER BY ba.position",
    )?;
    book.authors = query
        .query_map([id], author_row)?
        .collect::<rusqlite::Result<_>>()?;
    book.files = book_files(connection, id)?;
    Ok(book)
}

fn replace_authors(
    tx: &Transaction<'_>,
    book_id: i64,
    ids: &[i64],
    display: Option<&str>,
) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM book_authors WHERE book_id=?1", [book_id])?;
    let mut insert =
        tx.prepare("INSERT INTO book_authors(book_id,author_id,position) VALUES (?1,?2,?3)")?;
    for (position, id) in ids.iter().enumerate() {
        insert.execute(params![book_id, id, position as i64])?;
    }
    tx.execute(
        "UPDATE books SET author=?2 WHERE id=?1",
        params![book_id, display],
    )?;
    Ok(())
}

pub(crate) fn set_author_text(
    tx: &Transaction<'_>,
    book_id: i64,
    value: Option<&str>,
) -> rusqlite::Result<()> {
    let previous: Option<String> =
        tx.query_row("SELECT author FROM books WHERE id=?1", [book_id], |row| {
            row.get(0)
        })?;
    if previous.as_deref() == value {
        return Ok(());
    }
    let author = value.map(|name| insert_author(tx, name)).transpose()?;
    let id = author.map(|author| author.id);
    replace_authors(tx, book_id, id.as_slice(), value)
}

fn check_revision(tx: &Transaction<'_>, id: i64, expected: i64) -> Result<()> {
    let revision = tx
        .query_row("SELECT revision FROM books WHERE id=?1", [id], |row| {
            row.get::<_, i64>(0)
        })
        .optional()?
        .ok_or(Error::CatalogNotFound)?;
    if revision != expected {
        return Err(Error::CatalogConflict);
    }
    Ok(())
}

fn persist_metadata(tx: &Transaction<'_>, id: i64, input: &BookInput, title: &str) -> Result<()> {
    let subject = input.subject.as_deref().and_then(trimmed);
    tx.execute(
        "UPDATE books SET title=?2,title_sort_key=?3,subject=?4,publisher=?5,isbn=?6,publication_year=?7,language=?8
         WHERE id=?1",
        params![id, title, title.to_lowercase(), subject, input.publisher.as_deref().and_then(trimmed),
                input.isbn.as_deref().and_then(trimmed), input.publication_year, input.language.as_deref().and_then(trimmed)],
    )?;
    save_override(tx, id, "title", Some(title))?;
    save_override(tx, id, "subject", subject)?;

    if let Some(ids) = &input.author_ids {
        let mut query = tx.prepare("SELECT display_name FROM authors WHERE id=?1")?;
        let names = ids
            .iter()
            .map(|id| {
                query
                    .query_row([id], |row| row.get::<_, String>(0))
                    .optional()?
                    .ok_or(Error::UnknownAuthor)
            })
            .collect::<Result<Vec<_>>>()?;
        let display = (!names.is_empty()).then(|| names.join(", "));
        replace_authors(tx, id, ids, display.as_deref())?;
        save_override(tx, id, "author", display.as_deref())?;
    }
    Ok(())
}

impl Database {
    pub fn list_books(
        &self,
        before: Option<i64>,
        include_fileless: bool,
    ) -> rusqlite::Result<Vec<BookSummary>> {
        let mut query = self.0.prepare(
            "SELECT b.id,b.title,f.original_path,f.size_bytes,f.id,b.author,f.page_count,t.file_revision,
                    EXISTS(SELECT 1 FROM book_files other WHERE other.sha256=f.sha256 AND other.id<>f.id)
             FROM books b
             LEFT JOIN book_files f ON f.book_id=b.id AND f.is_primary=1
             LEFT JOIN thumbnails t ON t.file_id=f.id AND t.file_revision=f.revision
             WHERE b.id<=coalesce(?1,9223372036854775807) AND b.id IS NOT ?1
               AND (?2 OR f.id IS NOT NULL)
             ORDER BY b.id DESC LIMIT 51",
        )?;
        let rows = query.query_map(params![before, include_fileless], |row| {
            Ok(BookSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                path: row.get(2)?,
                bytes: row.get(3)?,
                file_id: row.get(4)?,
                author: row.get(5)?,
                pages: row.get(6)?,
                cover_revision: row.get(7)?,
                duplicate: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_book(&self, id: i64) -> Result<BookDetails> {
        let tx = self.0.unchecked_transaction()?;
        let book = book_details(&tx, id)?;
        tx.commit()?;
        Ok(book)
    }

    pub fn create_book(&self, input: &BookInput) -> Result<BookDetails> {
        let title = input.validate()?;
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        let id = insert_book(&tx, title)?;
        persist_metadata(&tx, id, input, title)?;
        let book = book_details(&tx, id)?;
        tx.commit()?;
        Ok(book)
    }

    pub fn update_book(&self, id: i64, expected: i64, input: &BookInput) -> Result<BookDetails> {
        let title = input.validate()?;
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        check_revision(&tx, id, expected)?;
        persist_metadata(&tx, id, input, title)?;
        tx.execute(
            &format!("UPDATE books SET revision=revision+1,updated_at={NOW} WHERE id=?1"),
            [id],
        )?;
        let book = book_details(&tx, id)?;
        tx.commit()?;
        Ok(book)
    }

    pub fn delete_book(&self, id: i64, expected: i64) -> Result<()> {
        let tx = Transaction::new_unchecked(&self.0, TransactionBehavior::Immediate)?;
        check_revision(&tx, id, expected)?;
        tx.execute("DELETE FROM books WHERE id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    pub fn get_book_file(&self, id: i64) -> Result<BookFile> {
        self.0
            .query_row(
                &format!("SELECT {FILE_COLUMNS} FROM book_files WHERE id=?1"),
                [id],
                file_row,
            )
            .optional()?
            .ok_or(Error::CatalogNotFound)
    }

    pub fn create_author(&self, display_name: &str) -> Result<Author> {
        let name = trimmed(display_name).ok_or(Error::InvalidCatalogInput)?;
        if name.len() > 4096 || name.contains('\0') {
            return Err(Error::InvalidCatalogInput);
        }
        Ok(insert_author(&self.0, name)?)
    }

    pub fn list_authors(&self, after: Option<i64>) -> Result<Vec<Author>> {
        let mut query = self.0.prepare(
            "SELECT id,display_name FROM authors WHERE id>coalesce(?1,0) ORDER BY id LIMIT 51",
        )?;
        let authors = query
            .query_map([after], author_row)?
            .collect::<rusqlite::Result<_>>()?;
        Ok(authors)
    }
}
