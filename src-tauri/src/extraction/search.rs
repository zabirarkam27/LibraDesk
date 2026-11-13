use crate::{
    catalog::search::normalize,
    database::{Database, Error},
};
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PageQuery {
    pub text: String,
    pub page: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageHit {
    pub id: i64,
    pub book_id: i64,
    pub title: String,
    pub author: Option<String>,
    pub file_id: i64,
    pub revision: i64,
    pub path: String,
    pub page_number: i64,
    pub snippet: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCoverage {
    pub files: i64,
    pub completed_files: i64,
    pub failed_files: i64,
    pub extracted_pages: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageResults {
    pub hits: Vec<PageHit>,
    pub total: i64,
    pub page: u32,
    pub coverage: SearchCoverage,
}
const FROM: &str = "FROM page_text_fts
 JOIN page_text p ON p.id=page_text_fts.rowid
 JOIN book_files f ON f.id=p.file_id AND f.revision=p.file_revision
 JOIN books b ON b.id=f.book_id
 WHERE page_text_fts MATCH ?1 AND f.availability='available'";

impl Database {
    pub fn search_page_text(&self, input: &PageQuery) -> Result<PageResults, Error> {
        if input.text.len() > 1024
            || input.text.contains('\0')
            || input.text.split_whitespace().count() > 32
            || input.page > 1_000_000
        {
            return Err(Error::InvalidCatalogInput);
        }
        let text = normalize(&input.text);
        // Quoted literal terms prevent users' punctuation/operators becoming FTS syntax.
        let expression = text
            .split_whitespace()
            .filter(|_| text.chars().any(char::is_alphanumeric))
            .map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(" AND ");
        let tx = self.0.unchecked_transaction()?;
        let coverage=tx.query_row(
   "SELECT count(*),coalesce(sum(j.status='completed'),0),coalesce(sum(j.status='failed'),0),
    coalesce(sum(j.next_page-1),0) FROM text_extraction_jobs j
    JOIN book_files f ON f.id=j.file_id AND f.revision=j.file_revision WHERE f.availability='available'",
   [],|r|Ok(SearchCoverage{files:r.get(0)?,completed_files:r.get(1)?,failed_files:r.get(2)?,extracted_pages:r.get(3)?})
  )?;
        let mut hits = Vec::new();
        let mut total = 0;
        let mut page = 0;
        if !expression.is_empty() {
            total = tx.query_row(&format!("SELECT count(*) {FROM}"), [&expression], |r| {
                r.get::<_, i64>(0)
            })?;
            page = input.page.min(((total - 1).max(0) / 50) as u32);
            let mut statement = tx.prepare(&format!(
    "SELECT p.id,b.id,b.title,b.author,f.id,p.file_revision,f.original_path,p.page_number,
     snippet(page_text_fts,0,'','',' … ',32) {FROM}
     ORDER BY bm25(page_text_fts),b.id,f.id,p.page_number,p.id LIMIT 50 OFFSET ?2"
   ))?;
            hits = statement
                .query_map(params![expression, i64::from(page) * 50], |r| {
                    let snippet: String = r.get(8)?;
                    let mut excerpt: String = snippet.chars().take(600).collect();
                    if snippet.chars().nth(600).is_some() {
                        excerpt.push('…');
                    }
                    Ok(PageHit {
                        id: r.get(0)?,
                        book_id: r.get(1)?,
                        title: r.get(2)?,
                        author: r.get(3)?,
                        file_id: r.get(4)?,
                        revision: r.get(5)?,
                        path: r.get(6)?,
                        page_number: r.get(7)?,
                        snippet: excerpt,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
        }
        tx.commit()?;
        Ok(PageResults {
            hits,
            total,
            page,
            coverage,
        })
    }
}
