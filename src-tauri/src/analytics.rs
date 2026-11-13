use crate::{
    commands::{with_database, CommandError},
    database::{Database, Error},
};
use serde::Serialize;
use tauri::AppHandle;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    day: String,
    sessions: i64,
    pages: i64,
    active_ms: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    id: i64,
    book_id: i64,
    title: String,
    started_at: i64,
    pages: i64,
    active_ms: i64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analytics {
    completed: i64,
    reading: i64,
    pages: i64,
    active_ms: i64,
    sessions: i64,
    activity: Vec<Activity>,
    history: Vec<History>,
    page: i64,
    has_more: bool,
}
impl Database {
    pub fn analytics(&self, page: i64, now: i64) -> Result<Analytics, Error> {
        if !(1..=50_000).contains(&page) {
            return Err(Error::InvalidCatalogInput);
        }
        let tx = self.0.unchecked_transaction()?;
        let (completed,reading)=tx.query_row("SELECT coalesce(sum(reading_status='completed'),0),coalesce(sum(reading_status='reading'),0) FROM books",[],|r|Ok((r.get(0)?,r.get(1)?)))?;
        let (pages,active_ms,sessions)=tx.query_row("SELECT coalesce(sum(pages_read),0),coalesce(sum(active_ms),0),count(*) FROM reading_sessions",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?;
        let activity = {
            let mut stmt=tx.prepare("SELECT date(started_at/1000,'unixepoch','localtime') day,count(*),sum(pages_read),sum(active_ms) FROM reading_sessions
              WHERE started_at>=?1 AND started_at<=?2 GROUP BY day ORDER BY day")?;
            let rows = stmt.query_map([now.saturating_sub(30 * 86400000), now], |r| {
                Ok(Activity {
                    day: r.get(0)?,
                    sessions: r.get(1)?,
                    pages: r.get(2)?,
                    active_ms: r.get(3)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let mut history = {
            let mut stmt=tx.prepare("SELECT s.id,b.id,b.title,s.started_at,s.pages_read,s.active_ms FROM reading_sessions s JOIN books b ON b.id=s.book_id ORDER BY s.started_at DESC,s.id DESC LIMIT 21 OFFSET ?1")?;
            let rows = stmt.query_map([(page - 1) * 20], |r| {
                Ok(History {
                    id: r.get(0)?,
                    book_id: r.get(1)?,
                    title: r.get(2)?,
                    started_at: r.get(3)?,
                    pages: r.get(4)?,
                    active_ms: r.get(5)?,
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        let has_more = history.len() > 20;
        history.truncate(20);
        tx.commit()?;
        Ok(Analytics {
            completed,
            reading,
            pages,
            active_ms,
            sessions,
            activity,
            history,
            page,
            has_more,
        })
    }
}
#[tauri::command]
pub async fn reading_analytics(app: AppHandle, page: i64) -> Result<Analytics, CommandError> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64;
    with_database(app, move |db| db.analytics(page, now)).await
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::repository::BookInput;
    #[test]
    fn analytics_reconcile_persisted_sessions_statuses_dates_and_history() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("library.sqlite3");
        let db = Database::open(&path).unwrap();
        let now = 1_800_000_000_000i64;
        let empty = db.analytics(1, now).unwrap();
        assert_eq!(empty.sessions, 0);
        assert!(empty.activity.is_empty());
        let book = db
            .create_book(&BookInput {
                title: "বাংলা Reading".into(),
                ..Default::default()
            })
            .unwrap()
            .id;
        db.0.execute(
            "UPDATE books SET reading_status='completed' WHERE id=?1",
            [book],
        )
        .unwrap();
        for index in 0..23 {
            db.0.execute("INSERT INTO reading_sessions(book_id,file_revision,started_at,last_checkpoint_at,pages_read,active_ms) VALUES(?1,1,?2,?2,3,60000)",rusqlite::params![book,now-index*86400000]).unwrap();
        }
        let data = db.analytics(1, now).unwrap();
        assert_eq!(
            (
                data.completed,
                data.reading,
                data.pages,
                data.active_ms,
                data.sessions
            ),
            (1, 0, 69, 1380000, 23)
        );
        assert_eq!(data.activity.iter().map(|d| d.pages).sum::<i64>(), 69);
        assert_eq!(
            data.activity.iter().map(|d| d.active_ms).sum::<i64>(),
            1380000
        );
        assert_eq!(data.history.len(), 20);
        assert!(data.has_more);
        let second = db.analytics(2, now).unwrap();
        assert_eq!(second.history.len(), 3);
        assert!(!second.has_more);
        assert_ne!(data.history[19].id, second.history[0].id);
        assert!(db.analytics(0, now).is_err());
        assert!(db.analytics(50_001, now).is_err());
        db.0.execute("INSERT INTO reading_sessions(book_id,file_revision,started_at,last_checkpoint_at,pages_read) VALUES(?1,1,?2,?2,4)",rusqlite::params![book,now-40*86400000]).unwrap();
        drop(db);
        let db = Database::open(&path).unwrap();
        let data = db.analytics(1, now).unwrap();
        assert_eq!(data.pages, 73);
        assert_eq!(data.activity.iter().map(|d| d.pages).sum::<i64>(), 69);
        db.0.execute("DELETE FROM books WHERE id=?1", [book])
            .unwrap();
        assert_eq!(db.analytics(1, now).unwrap().sessions, 0);
    }
}
