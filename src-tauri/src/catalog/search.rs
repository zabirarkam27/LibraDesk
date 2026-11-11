use rusqlite::{functions::FunctionFlags, params_from_iter, types::Value, Connection};
use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use super::repository::{summary_row, BookSummary, ReadingStatus, SUMMARY_COLUMNS, SUMMARY_JOINS};
use crate::database::{Database, Error};

type Result<T> = std::result::Result<T, Error>;

pub(crate) fn normalize(text: &str) -> String {
    text.nfc()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .nfc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn register_normalization(connection: &Connection) -> rusqlite::Result<()> {
    let flags = FunctionFlags::SQLITE_UTF8
        | FunctionFlags::SQLITE_DETERMINISTIC
        | FunctionFlags::SQLITE_INNOCUOUS;
    connection.create_scalar_function("catalog_normalize", 1, flags, |ctx| {
        Ok(normalize(
            ctx.get::<Option<String>>(0)?.as_deref().unwrap_or_default(),
        ))
    })?;
    connection.create_scalar_function("catalog_contains", 2, flags, |ctx| {
        let text = ctx.get::<String>(0)?;
        let query = ctx.get::<String>(1)?;
        Ok(query.split_whitespace().all(|term| text.contains(term)))
    })
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogSort {
    Title,
    Author,
    #[default]
    Added,
    Size,
    Year,
    Progress,
    Relevance,
    Collection,
}
#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Asc,
    #[default]
    Desc,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct CatalogQuery {
    pub text: String,
    pub page: u32,
    pub sort: CatalogSort,
    pub direction: Direction,
    pub category_id: Option<i64>,
    pub tag_id: Option<i64>,
    pub collection_id: Option<i64>,
    pub language: Option<String>,
    pub reading_status: Option<ReadingStatus>,
    pub favorites_only: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogPage {
    pub books: Vec<BookSummary>,
    pub total: i64,
    pub page: u32,
}

const SEARCH_CTE: &str = "
WITH RECURSIVE descendants(id) AS (
 SELECT ?3 UNION SELECT c.id FROM categories c JOIN descendants d ON c.parent_id=d.id
),
fts_hits AS MATERIALIZED (
 SELECT rowid AS book_id,bm25(book_search_fts,10.0,5.0,1.0,1.0,2.0) AS score
 FROM book_search_fts WHERE book_search_fts MATCH ?1
),
hits AS MATERIALIZED (
 SELECT book_id,score FROM fts_hits
 UNION ALL
 SELECT s.book_id,1000000.0 FROM book_search_text s
 WHERE ?2<>'' AND catalog_contains(s.title||' '||s.author||' '||s.subject||' '||s.publisher||' '||s.tags,?2)
 AND NOT EXISTS(SELECT 1 FROM fts_hits h WHERE h.book_id=s.book_id)
)";
const FILTERS: &str = "
 WHERE (?2='' OR h.book_id IS NOT NULL)
 AND (?3 IS NULL OR EXISTS(SELECT 1 FROM book_categories bc WHERE bc.book_id=b.id AND bc.category_id IN (SELECT id FROM descendants)))
 AND (?4 IS NULL OR EXISTS(SELECT 1 FROM book_tags bt WHERE bt.book_id=b.id AND bt.tag_id=?4))
 AND (?5 IS NULL OR cb.book_id IS NOT NULL)
 AND (?6 IS NULL OR b.language=?6)
 AND (?7 IS NULL OR b.reading_status=?7)
 AND (NOT ?8 OR b.is_favorite=1)";

impl Database {
    pub fn query_catalog(&self, input: &CatalogQuery) -> Result<CatalogPage> {
        if input.text.len() > 1024
            || input.text.contains('\0')
            || input.text.split_whitespace().count() > 32
            || input.page > 1_000_000
            || input
                .language
                .as_ref()
                .is_some_and(|language| language.len() > 4096)
        {
            return Err(Error::InvalidCatalogInput);
        }
        let text = normalize(&input.text);
        let fts = if text.is_empty() {
            "\"__libradesk_empty_query__\"".into()
        } else {
            text.split_whitespace()
                .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(" AND ")
        };
        let mut values: Vec<Value> = vec![
            fts.into(),
            text.into(),
            input.category_id.into(),
            input.tag_id.into(),
            input.collection_id.into(),
            input.language.clone().into(),
            input
                .reading_status
                .map(|status| status.key().to_owned())
                .into(),
            input.favorites_only.into(),
        ];
        let tx = self.0.unchecked_transaction()?;
        let from = format!(
            "FROM books b {SUMMARY_JOINS}
             LEFT JOIN reading_progress rp ON rp.book_id=b.id
             LEFT JOIN collection_books cb ON cb.book_id=b.id AND cb.collection_id=?5
             LEFT JOIN hits h ON h.book_id=b.id {FILTERS}"
        );
        let total: i64 = tx.query_row(
            &format!("{SEARCH_CTE} SELECT count(*) {from}"),
            params_from_iter(values.iter()),
            |row| row.get(0),
        )?;
        let page = input.page.min(((total - 1).max(0) / 50) as u32);
        let sort = match input.sort {
            CatalogSort::Title => "b.title_sort_key",
            CatalogSort::Author => "nullif(catalog_normalize(b.author),'')",
            CatalogSort::Added => "b.created_at",
            CatalogSort::Size => "f.size_bytes",
            CatalogSort::Year => "b.publication_year",
            CatalogSort::Progress => "CASE WHEN rp.total_pages IS NOT NULL THEN 1.0*rp.page_number/rp.total_pages END",
            CatalogSort::Relevance => "coalesce(h.score,0)",
            CatalogSort::Collection => "(SELECT CASE c.sort_mode WHEN 'title' THEN b.title_sort_key WHEN 'added' THEN cb.added_at ELSE cb.position END FROM collections c WHERE c.id=?5)",
        };
        let direction = match input.direction {
            Direction::Asc => "ASC",
            Direction::Desc => "DESC",
        };
        values.push((i64::from(page) * 50).into());
        let books = {
            let mut query = tx.prepare(&format!(
                "{SEARCH_CTE} SELECT {SUMMARY_COLUMNS} {from} ORDER BY {sort} {direction} NULLS LAST,b.id {direction} LIMIT 50 OFFSET ?9"
            ))?;
            let rows = query.query_map(params_from_iter(values.iter()), summary_row)?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        tx.commit()?;
        Ok(CatalogPage { books, total, page })
    }

    pub fn dashboard(&self) -> Result<Dashboard> {
        let tx = self.0.unchecked_transaction()?;
        let (total_books,favorites,reading): (i64,i64,i64) = tx.query_row(
            "SELECT count(*),coalesce(sum(is_favorite),0),coalesce(sum(reading_status='reading'),0) FROM books",
            [],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?)),
        )?;
        let total_storage = tx.query_row(
            "SELECT coalesce(sum(size_bytes),0) FROM book_files",
            [],
            |row| row.get(0),
        )?;
        let total_pages = tx.query_row(
            "SELECT coalesce(sum(page_count),0) FROM book_files WHERE is_primary=1",
            [],
            |row| row.get(0),
        )?;
        let mut lists = Vec::new();
        for order in [
            "ORDER BY b.created_at DESC,b.id DESC",
            "JOIN reading_progress rp ON rp.book_id=b.id ORDER BY rp.last_read_at DESC,b.id DESC",
            "WHERE b.is_favorite=1 ORDER BY b.updated_at DESC,b.id DESC",
            "WHERE b.reading_status='reading' ORDER BY b.updated_at DESC,b.id DESC",
        ] {
            let mut query = tx.prepare(&format!(
                "SELECT {SUMMARY_COLUMNS} FROM books b {SUMMARY_JOINS} {order} LIMIT 6"
            ))?;
            lists.push(
                query
                    .query_map([], summary_row)?
                    .collect::<rusqlite::Result<Vec<_>>>()?,
            );
        }
        let categories = {
            let mut query = tx.prepare(
                "SELECT c.id,c.name,count(bc.book_id) FROM categories c LEFT JOIN book_categories bc ON bc.category_id=c.id
                 GROUP BY c.id ORDER BY count(bc.book_id) DESC,c.name_key,c.id",
            )?;
            let rows = query.query_map([], |row| {
                Ok(CategoryCount {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    count: row.get(2)?,
                })
            })?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut lists = lists.into_iter();
        let dashboard = Dashboard {
            total_books,
            total_storage,
            total_pages,
            favorites,
            reading,
            categories,
            recently_added: lists.next().unwrap_or_default(),
            recently_read: lists.next().unwrap_or_default(),
            favorite_books: lists.next().unwrap_or_default(),
            currently_reading: lists.next().unwrap_or_default(),
        };
        tx.commit()?;
        Ok(dashboard)
    }
}

#[derive(Serialize)]
pub struct CategoryCount {
    id: i64,
    name: String,
    count: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    total_books: i64,
    total_storage: i64,
    total_pages: i64,
    favorites: i64,
    reading: i64,
    categories: Vec<CategoryCount>,
    recently_added: Vec<BookSummary>,
    recently_read: Vec<BookSummary>,
    favorite_books: Vec<BookSummary>,
    currently_reading: Vec<BookSummary>,
}
