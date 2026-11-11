use super::repository::*;
use crate::{
    catalog::repository::{insert_book, insert_file},
    database::{Database, Error},
    scan::files,
};
use rusqlite::params;
use std::fs;

#[test]
fn reader_ranges_progress_annotations_and_preferences_survive_reopen() {
    let fixture = tempfile::tempdir().unwrap();
    let file = fixture.path().join("source.pdf");
    fs::write(&file, b"%PDF-1.7\nfixture").unwrap();
    let file = fs::canonicalize(file).unwrap();
    let original = fs::read(&file).unwrap();
    let dbpath = fixture.path().join("library.db");
    let db = Database::open(&dbpath).unwrap();
    let stamp = files::stamp(&fs::File::open(&file).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    let book = insert_book(&tx, "Reader fixture").unwrap();
    let file_id = insert_file(&tx, book, file.to_str().unwrap(), &stamp).unwrap();
    tx.commit().unwrap();
    let opened = db.open_reader(book, None).unwrap();
    let source = db.reader_source(opened.session_id).unwrap();
    assert_eq!(source.read_range(0, 5).unwrap(), b"%PDF-");
    assert!(source.read_range(-1, 5).is_err());
    assert!(source.read_range(0, 256 * 1024 + 1).is_err());
    assert!(source.read_range(i64::MAX, 1).is_err());
    assert!(db.open_reader(book, Some(file_id + 99)).is_err());
    let progress = ProgressInput {
        session_id: opened.session_id,
        sequence: 1,
        page: 2,
        total: 3,
        active_ms: 0,
        close: false,
    };
    db.save_reader_progress(&progress).unwrap();
    db.save_reader_progress(&ProgressInput {
        page: 1,
        ..progress.clone()
    })
    .unwrap();
    let annotation = AnnotationInput {
        session_id: opened.session_id,
        id: None,
        revision: None,
        page: 2,
        text: "বাংলা note".into(),
        client_key: "first-note".into(),
    };
    let note = db
        .save_reader_annotation(AnnotationKind::Note, &annotation)
        .unwrap();
    assert_eq!(
        db.save_reader_annotation(AnnotationKind::Note, &annotation)
            .unwrap(),
        note
    );
    assert_eq!(db.reader_annotations(book).unwrap().notes.len(), 1);
    db.save_reader_annotation(
        AnnotationKind::Note,
        &AnnotationInput {
            id: Some(note),
            revision: Some(1),
            text: "Edited".into(),
            ..annotation
        },
    )
    .unwrap();
    db.save_reader_preferences(&ReaderPreferences {
        zoom: Zoom::Custom,
        scale: 175,
        layout: Layout::Single,
        fullscreen: false,
        shortcuts: false,
    })
    .unwrap();
    db.save_reader_progress(&ProgressInput {
        sequence: 2,
        page: 3,
        close: true,
        ..progress.clone()
    })
    .unwrap();
    assert!(db.reader_source(opened.session_id).is_err());
    assert_eq!(db.get_book(book).unwrap().reading_status, "unread");
    drop(db);
    let db = Database::open(&dbpath).unwrap();
    let reopened = db.open_reader(book, None).unwrap();
    assert_eq!(reopened.resume_page, 3);
    assert_eq!(reopened.preferences.scale, 175);
    assert!(!reopened.preferences.shortcuts);
    assert_eq!(db.reader_annotations(book).unwrap().notes[0].text, "Edited");
    db.delete_reader_annotation(AnnotationKind::Note, reopened.session_id, note, 2)
        .unwrap();
    assert!(db.reader_annotations(book).unwrap().notes.is_empty());
    let (visits, active): (i64, i64) =
        db.0.query_row(
            "SELECT pages_read,active_ms FROM reading_sessions WHERE id=?1",
            [opened.session_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(visits, 2);
    assert_eq!(active, 0);
    db.save_reader_progress(&ProgressInput {
        sequence: 99,
        page: 1,
        ..progress
    })
    .unwrap();
    let saved: i64 =
        db.0.query_row(
            "SELECT page_number FROM reading_progress WHERE book_id=?1",
            [book],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(saved, 3);
    assert_eq!(fs::read(file).unwrap(), original);
}
#[test]
fn source_changes_and_invalid_anchors_are_rejected_and_old_annotations_are_retained() {
    let fixture = tempfile::tempdir().unwrap();
    let file = fixture.path().join("source.pdf");
    fs::write(&file, b"%PDF-1.7\nfixture").unwrap();
    let file = fs::canonicalize(file).unwrap();
    let db = Database::open(&fixture.path().join("library.db")).unwrap();
    let stamp = files::stamp(&fs::File::open(&file).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    let book = insert_book(&tx, "First").unwrap();
    let other = insert_book(&tx, "Other").unwrap();
    let id = insert_file(&tx, book, file.to_str().unwrap(), &stamp).unwrap();
    tx.commit().unwrap();
    assert!(db.open_reader(other, Some(id)).is_err());
    let opened = db.open_reader(book, None).unwrap();
    let source = db.reader_source(opened.session_id).unwrap();
    let progress = ProgressInput {
        session_id: opened.session_id,
        sequence: 1,
        page: 1,
        total: 2,
        active_ms: 0,
        close: false,
    };
    assert!(db
        .save_reader_progress(&ProgressInput {
            page: 3,
            ..progress.clone()
        })
        .is_err());
    assert!(db
        .save_reader_progress(&ProgressInput {
            active_ms: i64::MAX,
            ..progress.clone()
        })
        .is_err());
    db.save_reader_progress(&progress).unwrap();
    let bookmark = AnnotationInput {
        session_id: opened.session_id,
        id: None,
        revision: None,
        page: 1,
        text: "Chapter".into(),
        client_key: "bookmark-1".into(),
    };
    db.save_reader_annotation(AnnotationKind::Bookmark, &bookmark)
        .unwrap();
    fs::write(&file, b"%PDF-1.7\nchanged").unwrap();
    assert!(source.read_range(0, 5).is_err());
    let stamp = files::stamp(&fs::File::open(&file).unwrap()).unwrap();
    db.0.execute(
        "UPDATE book_files SET revision=revision+1,size_bytes=?2,modified_at=?3,source_token=?4 WHERE id=?1",
        params![id,stamp.bytes,stamp.modified_at,stamp.token],
    ).unwrap();
    assert!(matches!(
        db.reader_source(opened.session_id),
        Err(Error::ReaderSourceUnavailable)
    ));
    assert!(db
        .save_reader_progress(&ProgressInput {
            sequence: 2,
            ..progress
        })
        .is_err());
    let next = db.open_reader(book, None).unwrap();
    assert!(next.stale_progress);
    assert_eq!(
        db.reader_annotations(book).unwrap().bookmarks[0].file_revision,
        1
    );
    let dashboard = serde_json::to_value(db.dashboard().unwrap()).unwrap();
    assert_eq!(dashboard["recentlyRead"][0]["id"], book);
    db.close_reader(next.session_id).unwrap();
    assert!(db.reader_source(next.session_id).is_err());
}
