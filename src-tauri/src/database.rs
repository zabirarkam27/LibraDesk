use std::{path::Path, time::Duration};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MIGRATIONS: &[(&str, &str)] = &[
    ("settings", include_str!("../migrations/0001_settings.sql")),
    (
        "scan_import",
        include_str!("../migrations/0002_scan_import.sql"),
    ),
    ("metadata", include_str!("../migrations/0003_metadata.sql")),
    ("catalog", include_str!("../migrations/0004_catalog.sql")),
    ("library", include_str!("../migrations/0005_library.sql")),
    ("reader", include_str!("../migrations/0006_reader.sql")),
    (
        "text_extraction",
        include_str!("../migrations/0007_text_extraction.sql"),
    ),
    (
        "page_search",
        include_str!("../migrations/0008_page_search.sql"),
    ),
    (
        "file_recovery",
        include_str!("../migrations/0009_file_recovery.sql"),
    ),
];
const UPSERT_THEME: &str = "
    INSERT INTO settings(scope, key, value_json, updated_at)
    VALUES (?1, 'theme', ?2, CAST(unixepoch('subsec') * 1000 AS INTEGER))
    ON CONFLICT(scope, key) DO UPDATE
    SET value_json = excluded.value_json, updated_at = excluded.updated_at";

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    App,
    Reader,
}

impl Scope {
    fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Reader => "reader",
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub app: Theme,
    pub reader: Theme,
    pub schema_version: usize,
}

#[derive(Debug)]
pub enum Error {
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
    IncompatibleSchema,
    UnsupportedFeatures,
    InvalidCatalogInput,
    CatalogNotFound,
    CatalogConflict,
    UnknownAuthor,
    OrganizationConflict,
    CategoryHasChildren,
    ReaderSourceUnavailable,
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "{error}"),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::IncompatibleSchema => {
                formatter.write_str("Database migration history does not match this app")
            }
            Self::UnsupportedFeatures => {
                formatter.write_str("SQLite requires foreign keys, WAL, and FTS5")
            }
            Self::InvalidCatalogInput => formatter.write_str("Invalid catalog input"),
            Self::CatalogNotFound => formatter.write_str("Catalog record not found"),
            Self::CatalogConflict => formatter.write_str("Catalog record has changed"),
            Self::UnknownAuthor => formatter.write_str("Author record not found"),
            Self::ReaderSourceUnavailable => {
                formatter.write_str("Reader source is missing, inaccessible, or changed")
            }
            Self::OrganizationConflict => {
                formatter.write_str("Name already exists or category parent is invalid")
            }
            Self::CategoryHasChildren => {
                formatter.write_str("Move or remove child categories before deleting this category")
            }
        }
    }
}

fn migrate(connection: &mut Connection, migrations: &[(&str, &str)]) -> Result<(), Error> {
    crate::catalog::search::register_normalization(connection)?;
    // Hash normalized line endings so the same migration matches across Git checkouts.
    let checksums: Vec<_> = migrations
        .iter()
        .map(|(_, sql)| format!("{:x}", Sha256::digest(sql.replace("\r\n", "\n"))))
        .collect();

    loop {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 =
            transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let version = usize::try_from(version).map_err(|_| Error::IncompatibleSchema)?;
        if version > migrations.len() {
            return Err(Error::IncompatibleSchema);
        }
        if version == 0 {
            transaction.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS schema_migrations (
                    version INTEGER PRIMARY KEY CHECK (version > 0),
                    name TEXT NOT NULL,
                    checksum TEXT NOT NULL,
                    applied_at INTEGER NOT NULL
                ) STRICT;",
            )?;
        }
        {
            let mut statement = transaction.prepare(
                "SELECT version, name, checksum FROM schema_migrations ORDER BY version",
            )?;
            let mut rows = statement.query([])?;
            for (index, ((name, _), checksum)) in
                migrations.iter().zip(&checksums).take(version).enumerate()
            {
                let row = rows.next()?.ok_or(Error::IncompatibleSchema)?;
                if row.get::<_, i64>(0)? != (index + 1) as i64
                    || row.get::<_, String>(1)? != *name
                    || row.get::<_, String>(2)? != *checksum
                {
                    return Err(Error::IncompatibleSchema);
                }
            }
            if rows.next()?.is_some() {
                return Err(Error::IncompatibleSchema);
            }
        }
        let Some((name, sql)) = migrations.get(version) else {
            transaction.commit()?;
            return Ok(());
        };
        transaction.execute_batch(sql)?;
        let next_version = (version + 1) as i64;
        transaction.execute(
            "INSERT INTO schema_migrations VALUES (?1, ?2, ?3, CAST(unixepoch('subsec') * 1000 AS INTEGER))",
            rusqlite::params![next_version, name, &checksums[version]],
        )?;
        transaction.pragma_update(None, "user_version", next_version)?;
        transaction.commit()?;
    }
}

pub struct Database(pub(crate) Connection);

impl Database {
    pub fn open(path: &Path) -> Result<Self, Error> {
        let mut connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(3))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        let foreign_keys: bool =
            connection.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
        let fts: bool = connection.query_row(
            "SELECT sqlite_compileoption_used('ENABLE_FTS5')",
            [],
            |row| row.get(0),
        )?;
        if !foreign_keys || !fts {
            return Err(Error::UnsupportedFeatures);
        }
        migrate(&mut connection, MIGRATIONS)?;
        let journal: String =
            connection.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
        if !journal.eq_ignore_ascii_case("wal") {
            return Err(Error::UnsupportedFeatures);
        }
        Ok(Self(connection))
    }

    pub fn preferences(&self, legacy_theme: Option<Theme>) -> Result<Preferences, Error> {
        if let Some(theme) = legacy_theme {
            self.0.execute(
                "INSERT INTO settings VALUES ('app', 'theme', ?1, CAST(unixepoch('subsec') * 1000 AS INTEGER))
                 ON CONFLICT(scope, key) DO NOTHING",
                [serde_json::to_string(&theme)?],
            )?;
        }
        Ok(Preferences {
            app: self.theme(Scope::App)?,
            reader: self.theme(Scope::Reader)?,
            schema_version: MIGRATIONS.len(),
        })
    }

    pub fn set_theme(&self, scope: Scope, theme: Theme) -> Result<(), Error> {
        self.0.execute(
            UPSERT_THEME,
            (scope.as_str(), serde_json::to_string(&theme)?),
        )?;
        Ok(())
    }

    fn theme(&self, scope: Scope) -> Result<Theme, Error> {
        let json: Option<String> = self
            .0
            .query_row(
                "SELECT value_json FROM settings WHERE scope = ?1 AND key = 'theme'",
                [scope.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        Ok(json
            .as_deref()
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or_default())
    }
}

pub(crate) fn schema_version() -> usize {
    MIGRATIONS.len()
}
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub schema_version: u32,
    pub books: i64,
    pub notes: i64,
    pub bookmarks: i64,
    pub reading_sessions: i64,
}
type SchemaObject = (String, String, String, Option<String>);
fn schema_objects(connection: &Connection) -> rusqlite::Result<Vec<SchemaObject>> {
    let mut statement=connection.prepare("SELECT type,name,tbl_name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get::<_, Option<String>>(3)?
                .map(|sql| sql.replace("\r\n", "\n")),
        ))
    })?;
    rows.collect()
}
pub(crate) fn validate_snapshot(path: &Path) -> Result<SnapshotInfo, Error> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    crate::catalog::search::register_normalization(&connection)?;
    connection.pragma_update(None, "foreign_keys", true)?;
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let version = usize::try_from(version).map_err(|_| Error::IncompatibleSchema)?;
    if version == 0 || version > MIGRATIONS.len() {
        return Err(Error::IncompatibleSchema);
    }
    let integrity: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(Error::IncompatibleSchema);
    }
    {
        let mut check = connection.prepare("PRAGMA foreign_key_check")?;
        if check.query([])?.next()?.is_some() {
            return Err(Error::IncompatibleSchema);
        }
    }
    let mut expected = Connection::open_in_memory()?;
    migrate(&mut expected, &MIGRATIONS[..version])?;
    if schema_objects(&connection)? != schema_objects(&expected)? {
        return Err(Error::IncompatibleSchema);
    }
    let history = |db: &Connection| -> rusqlite::Result<Vec<(i64, String, String)>> {
        let mut query =
            db.prepare("SELECT version,name,checksum FROM schema_migrations ORDER BY version")?;
        let rows = query.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        rows.collect()
    };
    if history(&connection)? != history(&expected)? {
        return Err(Error::IncompatibleSchema);
    }
    let count = |table: &str| -> rusqlite::Result<i64> {
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name=?1)",
            [table],
            |r| r.get(0),
        )?;
        if exists {
            connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        } else {
            Ok(0)
        }
    };
    Ok(SnapshotInfo {
        schema_version: version as u32,
        books: count("books")?,
        notes: count("notes")?,
        bookmarks: count("bookmarks")?,
        reading_sessions: count("reading_sessions")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initializes_and_reopens_with_independent_preferences_and_integrity() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("test.sqlite3");
        let db = Database::open(&path).unwrap();
        assert_eq!(
            db.preferences(Some(Theme::Light)).unwrap().app,
            Theme::Light
        );
        db.set_theme(Scope::App, Theme::Dark).unwrap();
        db.set_theme(Scope::Reader, Theme::Light).unwrap();
        drop(db);

        let db = Database::open(&path).unwrap();
        let preferences = db.preferences(Some(Theme::System)).unwrap();
        assert_eq!(preferences.app, Theme::Dark);
        assert_eq!(preferences.reader, Theme::Light);
        assert_eq!(preferences.schema_version, MIGRATIONS.len());
        assert_eq!(
            db.0.pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
                .unwrap(),
            "wal"
        );
        assert!(db
            .0
            .pragma_query_value(None, "foreign_keys", |row| row.get::<_, bool>(0))
            .unwrap());
        assert_eq!(
            db.0.pragma_query_value(None, "integrity_check", |row| row.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        db.0.execute_batch(
            "CREATE VIRTUAL TABLE temp.test_fts USING fts5(title, tokenize = \"unicode61 categories 'L* N* Co M*'\");
             INSERT INTO test_fts VALUES ('বাংলা বই React');"
        ).unwrap();
        let matches: i64 =
            db.0.query_row(
                "SELECT count(*) FROM test_fts WHERE test_fts MATCH ?1",
                ["বাংলা AND react"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(matches, 1);
    }

    #[test]
    fn catalog_migration_backfills_legacy_author_text_without_merging_names() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        migrate(&mut connection, &MIGRATIONS[..3]).unwrap();
        connection
            .execute_batch(
                "INSERT INTO books(id,title,title_sort_key,author,created_at,updated_at) VALUES
             (4,'First','first','লেখক, আরেকজন',0,0),
             (5,'Second','second','লেখক, আরেকজন',0,0),
             (6,'Third','third',NULL,0,0);
             INSERT INTO metadata_overrides VALUES (4,'author','লেখক, আরেকজন',0);",
            )
            .unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        let db = Database(connection);
        let first = db.get_book(4).unwrap();
        let second = db.get_book(5).unwrap();
        assert_eq!(first.authors[0].display_name, "লেখক, আরেকজন");
        assert_eq!(second.authors[0].display_name, "লেখক, আরেকজন");
        assert_ne!(first.authors[0].id, second.authors[0].id);
        assert!(db.get_book(6).unwrap().authors.is_empty());
        assert_eq!(
            db.query_catalog(&crate::catalog::search::CatalogQuery {
                text: "লেখক".into(),
                ..Default::default()
            })
            .unwrap()
            .total,
            2
        );
        let override_value: String =
            db.0.query_row(
                "SELECT value FROM metadata_overrides WHERE book_id=4 AND field='author'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(override_value, "লেখক, আরেকজন");
    }

    #[test]
    fn text_extraction_migration_queues_existing_files_without_changing_catalog() {
        use crate::catalog::repository::{insert_book, insert_file};
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("source.pdf");
        std::fs::write(&path, b"%PDF-1.7\nfixture").unwrap();
        let stamp = crate::scan::files::stamp(&std::fs::File::open(&path).unwrap()).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        migrate(&mut connection, &MIGRATIONS[..6]).unwrap();
        let tx = connection.unchecked_transaction().unwrap();
        let book = insert_book(&tx, "Existing book").unwrap();
        let first = insert_file(&tx, book, path.to_str().unwrap(), &stamp).unwrap();
        let other_book = insert_book(&tx, "Unavailable book").unwrap();
        let second = insert_file(
            &tx,
            other_book,
            fixture.path().join("missing.pdf").to_str().unwrap(),
            &stamp,
        )
        .unwrap();
        tx.execute(
            "UPDATE book_files SET availability='missing' WHERE id=?1",
            [second],
        )
        .unwrap();
        tx.commit().unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        let mut db = Database(connection);
        assert_eq!(
            db.text_extraction_status(first).unwrap().unwrap().status,
            "queued"
        );
        assert_eq!(
            db.text_extraction_status(second)
                .unwrap()
                .unwrap()
                .error
                .as_deref(),
            Some("source_unavailable")
        );
        assert_eq!(db.get_book(book).unwrap().files.len(), 1);
        assert_eq!(db.get_book(other_book).unwrap().files.len(), 1);
        migrate(&mut db.0, MIGRATIONS).unwrap();
    }

    #[test]
    fn page_search_migration_indexes_existing_extraction_and_replays_safely() {
        use crate::{
            catalog::repository::{insert_book, insert_file},
            extraction::search::PageQuery,
        };
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture.path().join("source.pdf");
        std::fs::write(&path, b"%PDF-1.7\nfixture").unwrap();
        let stamp = crate::scan::files::stamp(&std::fs::File::open(&path).unwrap()).unwrap();
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        migrate(&mut connection, &MIGRATIONS[..7]).unwrap();
        let tx = connection.unchecked_transaction().unwrap();
        let book = insert_book(&tx, "Backfill").unwrap();
        let file = insert_file(&tx, book, path.to_str().unwrap(), &stamp).unwrap();
        tx.execute("INSERT INTO page_text(file_id,file_revision,page_number,text,normalized_text,extracted_at)
         VALUES(?1,1,7,'বাংলা Reader','বাংলা reader',0)",[file]).unwrap();
        tx.commit().unwrap();
        migrate(&mut connection, MIGRATIONS).unwrap();
        let mut db = Database(connection);
        let query = PageQuery {
            text: "বাংলা rea".into(),
            page: 0,
        };
        assert_eq!(db.search_page_text(&query).unwrap().hits[0].page_number, 7);
        migrate(&mut db.0, MIGRATIONS).unwrap();
        assert_eq!(db.search_page_text(&query).unwrap().total, 1);
        db.0.execute(
            "INSERT INTO page_text_fts(page_text_fts,rank) VALUES('integrity-check',1)",
            [],
        )
        .unwrap();
    }

    #[test]
    fn failed_migration_rolls_back_and_incompatible_history_is_rejected() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection, &MIGRATIONS[..1]).unwrap();
        let migrations = [
            MIGRATIONS[0],
            ("broken", "CREATE TABLE temporary_change(id); INVALID SQL;"),
        ];
        assert!(migrate(&mut connection, &migrations).is_err());
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            connection
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name = 'temporary_change'",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
        assert!(matches!(
            migrate(&mut connection, &[("different", MIGRATIONS[0].1)]),
            Err(Error::IncompatibleSchema)
        ));
        connection
            .pragma_update(None, "user_version", MIGRATIONS.len() as i64 + 1)
            .unwrap();
        assert!(matches!(
            migrate(&mut connection, MIGRATIONS),
            Err(Error::IncompatibleSchema)
        ));
    }
}
