use super::repository::*;
use crate::{
    catalog::repository::{insert_book, insert_file},
    database::Database,
    scan::files,
};
use std::fs;

fn setup() -> (tempfile::TempDir, Database, i64, i64) {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("source.pdf");
    fs::write(&path, b"%PDF-1.7\ntext fixture").unwrap();
    let path = fs::canonicalize(path).unwrap();
    let db = Database::open(&fixture.path().join("library.db")).unwrap();
    let stamp = files::stamp(&fs::File::open(&path).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    let book = insert_book(&tx, "Extraction fixture").unwrap();
    let file = insert_file(&tx, book, path.to_str().unwrap(), &stamp).unwrap();
    tx.commit().unwrap();
    (fixture, db, book, file)
}
fn key(task: &TextTask) -> JobKey {
    JobKey {
        file_id: task.file_id,
        revision: task.revision,
        lease: task.lease,
    }
}
fn page(number: i64, text: &str) -> PageText {
    PageText {
        number,
        text: text.into(),
    }
}
fn count(db: &Database) -> i64 {
    db.0.query_row("SELECT count(*) FROM page_text", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn checkpoint_reopen_expired_lease_idempotency_and_unicode() {
    let (fixture, db, _, file) = setup();
    let task = db.claim_text_extraction().unwrap().unwrap();
    assert!(db.claim_text_extraction().unwrap().is_none());
    assert!(db
        .commit_page_text(key(&task), 3, &[page(1, "বাংলা  TEXT\n"), page(2, "")])
        .unwrap());
    assert!(db
        .commit_page_text(key(&task), 3, &[page(1, "বাংলা  TEXT\n"), page(2, "")])
        .unwrap());
    assert_eq!(count(&db), 2);
    assert!(db
        .commit_page_text(key(&task), 3, &[page(1, "wrong")])
        .is_err());
    let normalized: String =
        db.0.query_row(
            "SELECT normalized_text FROM page_text WHERE page_number=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(normalized, "বাংলা text");
    drop(db);
    let db = Database::open(&fixture.path().join("library.db")).unwrap();
    db.0.execute("UPDATE text_extraction_jobs SET lease_expires_at=0", [])
        .unwrap();
    let resumed = db.claim_text_extraction().unwrap().unwrap();
    assert_eq!(resumed.next_page, 3);
    assert!(resumed.lease > task.lease);
    assert!(!db
        .commit_page_text(key(&task), 3, &[page(3, "stale")])
        .unwrap());
    assert!(db
        .commit_page_text(key(&resumed), 3, &[page(3, "final")])
        .unwrap());
    assert!(db
        .commit_page_text(key(&resumed), 3, &[page(3, "final")])
        .unwrap());
    let status = db.text_extraction_status(file).unwrap().unwrap();
    assert_eq!(status.status, "completed");
    assert_eq!(status.completed_pages, 3);
    assert_eq!(status.pages_with_text, 2);
    assert_eq!(count(&db), 3);
    assert!(db.claim_text_extraction().unwrap().is_none());
    assert_eq!(
        fs::read(fixture.path().join("source.pdf")).unwrap(),
        b"%PDF-1.7\ntext fixture"
    );
}
#[test]
fn invalid_batches_are_atomic_and_source_changes_are_rejected() {
    let (fixture, db, _, _) = setup();
    let task = db.claim_text_extraction().unwrap().unwrap();
    for pages in [
        vec![page(1, "valid"), page(3, "gap")],
        vec![page(1, "valid"), page(2, &"x".repeat(MAX_PAGE_BYTES + 1))],
        vec![page(1, "nul\0")],
        (1..=5).map(|n| page(n, "text")).collect(),
    ] {
        assert!(db.commit_page_text(key(&task), 5, &pages).is_err());
        assert_eq!(count(&db), 0);
    }
    assert!(!db
        .commit_page_text(key(&task), 5, &[page(2, "out of order")])
        .unwrap());
    fs::write(fixture.path().join("source.pdf"), b"%PDF-changed size").unwrap();
    assert!(db
        .commit_page_text(key(&task), 5, &[page(1, "obsolete")])
        .is_err());
    assert_eq!(count(&db), 0);
}
#[test]
fn failure_retry_release_revision_invalidation_and_cascade() {
    let (_fixture, db, book, file) = setup();
    let task = db.claim_text_extraction().unwrap().unwrap();
    assert!(db
        .commit_page_text(key(&task), 3, &[page(1, "saved")])
        .unwrap());
    db.fail_text_extraction(key(&task), Failure::ExtractionFailed)
        .unwrap();
    assert_eq!(
        db.text_extraction_status(file)
            .unwrap()
            .unwrap()
            .error
            .as_deref(),
        Some("extraction_failed")
    );
    assert!(db.claim_text_extraction().unwrap().is_none());
    assert!(!db.retry_text_extraction(file, 99).unwrap());
    assert!(db.retry_text_extraction(file, 1).unwrap());
    let retry = db.claim_text_extraction().unwrap().unwrap();
    assert_eq!(retry.next_page, 2);
    db.fail_text_extraction(key(&task), Failure::InvalidPdf)
        .unwrap();
    assert_eq!(
        db.text_extraction_status(file).unwrap().unwrap().status,
        "running"
    );
    db.release_text_extraction(key(&retry)).unwrap();
    let retry = db.claim_text_extraction().unwrap().unwrap();
    assert_eq!(retry.next_page, 2);
    db.0.execute(
        "UPDATE book_files SET revision=revision+1 WHERE id=?1",
        [file],
    )
    .unwrap();
    assert_eq!(count(&db), 0);
    assert!(!db.renew_text_extraction(key(&retry)).unwrap());
    assert!(!db
        .commit_page_text(key(&retry), 3, &[page(2, "obsolete")])
        .unwrap());
    let fresh = db.claim_text_extraction().unwrap().unwrap();
    assert_eq!(fresh.next_page, 1);
    assert_eq!(fresh.revision, 2);
    db.0.execute("DELETE FROM books WHERE id=?1", [book])
        .unwrap();
    assert!(db.text_extraction_status(file).unwrap().is_none());
    assert!(db.extraction_source(key(&fresh)).unwrap().is_none());
}
