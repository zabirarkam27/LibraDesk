use super::{
    probe,
    repository::{hex, RegisteredFile},
};
use crate::{
    catalog::repository::{insert_book, insert_file},
    database::Database,
    metadata,
    scan::files,
};
use lopdf::{dictionary, text_string, Document, Object};
use std::{fs, path::Path};

fn pdf(path: &Path, title: &str) {
    let mut doc = Document::with_version("1.4");
    let pages = doc.new_object_id();
    let page=doc.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),144.into(),144.into()]});
    doc.objects.insert(
        pages,
        Object::Dictionary(
            dictionary! {"Type"=>"Pages","Kids"=>vec![Object::Reference(page)],"Count"=>1},
        ),
    );
    let catalog = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages});
    let info = doc.add_object(
        dictionary! {"Title"=>text_string(title),"Author"=>text_string("Original author")},
    );
    doc.trailer.set("Root", catalog);
    doc.trailer.set("Info", info);
    doc.save(path).unwrap();
}
fn register(db: &Database, path: &Path) -> (i64, i64) {
    let canonical = fs::canonicalize(path).unwrap();
    let stamp = files::stamp(&fs::File::open(&canonical).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    let book = insert_book(&tx, "Saved title").unwrap();
    let file = insert_file(&tx, book, canonical.to_str().unwrap(), &stamp).unwrap();
    tx.commit().unwrap();
    (book, file)
}
fn fixture() -> (tempfile::TempDir, Database, i64, i64) {
    let dir = tempfile::tempdir().unwrap();
    pdf(&dir.path().join("source.pdf"), "Original");
    let db = Database::open(&dir.path().join("library.db")).unwrap();
    let (book, file) = register(&db, &dir.path().join("source.pdf"));
    assert!(metadata::process_next(&db).unwrap());
    (dir, db, book, file)
}
fn verify(db: &Database, file: i64) {
    let source = db.recovery_file(file).unwrap();
    assert!(db
        .apply_verification(&source, probe::inspect(&source))
        .unwrap());
}
fn anchors(db: &Database, book: i64, file: i64) {
    db.0.execute("INSERT INTO notes(book_id,file_id,file_revision,page_number,body,client_key,created_at,updated_at) VALUES(?1,?2,1,1,'Personal note','note',0,0)",[book,file]).unwrap();
    db.0.execute("INSERT INTO bookmarks(book_id,file_id,file_revision,page_number,label,client_key,created_at,updated_at) VALUES(?1,?2,1,1,'Bookmark','mark',0,0)",[book,file]).unwrap();
    db.0.execute("INSERT INTO reading_progress(book_id,file_id,file_revision,page_number,total_pages,last_read_at) VALUES(?1,?2,1,1,1,0)",[book,file]).unwrap();
}
#[test]
fn background_verification_classifies_missing_denied_invalid_and_resumes() {
    let (dir, db, _, file) = fixture();
    db.start_verification().unwrap();
    let source = db.verification_next().unwrap().unwrap();
    assert!(probe::inspect(&source).analysis.is_none());
    let bytes = fs::read(dir.path().join("source.pdf")).unwrap();
    fs::remove_file(dir.path().join("source.pdf")).unwrap();
    assert_eq!(probe::inspect(&source).status, "missing");
    verify(&db, file);
    assert_eq!(
        db.recovery_files(0, true).unwrap()[0].availability,
        "missing"
    );
    let saved_hash = db.recovery_file(file).unwrap().hash;
    assert!(saved_hash.is_some());
    assert_eq!(db.recovery_snapshot().unwrap().checked, 1);
    drop(db);
    let db = Database::open(&dir.path().join("library.db")).unwrap();
    assert!(db.verification_next().unwrap().is_none());
    assert!(db.finish_verification().unwrap());
    fs::write(dir.path().join("source.pdf"), &bytes).unwrap();
    db.start_verification().unwrap();
    verify(&db, file);
    assert_eq!(db.recovery_file(file).unwrap().source.revision, 1);
    assert_eq!(db.recovery_file(file).unwrap().availability, "available");
    assert_eq!(db.recovery_file(file).unwrap().hash, saved_hash);
    // On Windows a held deny-sharing handle deterministically models an inaccessible file.
    let held = std::fs::OpenOptions::new()
        .write(true)
        .open(dir.path().join("source.pdf"))
        .unwrap();
    assert_eq!(
        probe::inspect(&db.recovery_file(file).unwrap()).status,
        "inaccessible"
    );
    drop(held);
    fs::write(dir.path().join("source.pdf"), b"not a PDF").unwrap();
    verify(&db, file);
    assert_eq!(db.recovery_file(file).unwrap().availability, "invalid");
}
#[test]
fn changed_files_rehash_and_invalidate_derivatives_without_losing_manual_metadata() {
    let (dir, db, book, file) = fixture();
    db.0.execute(
        "INSERT INTO metadata_overrides VALUES(?1,'title','My title',0)",
        [book],
    )
    .unwrap();
    db.0.execute(
        "UPDATE books SET title='My title',title_sort_key='my title' WHERE id=?1",
        [book],
    )
    .unwrap();
    anchors(&db, book, file);
    db.0.execute("INSERT INTO page_text(file_id,file_revision,page_number,text,normalized_text,extracted_at) VALUES(?1,1,1,'obsolete','obsolete',0)",[file]).unwrap();
    let prior = db.recovery_file(file).unwrap();
    let file_handle = fs::OpenOptions::new()
        .write(true)
        .open(dir.path().join("source.pdf"))
        .unwrap();
    file_handle
        .set_times(
            fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1700000000)),
        )
        .unwrap();
    drop(file_handle);
    let observation = probe::inspect(&prior);
    assert!(observation.analysis.is_some());
    db.apply_verification(&prior, observation).unwrap();
    assert_eq!(db.recovery_file(file).unwrap().source.revision, 1);
    let before = db.recovery_file(file).unwrap();
    pdf(
        &dir.path().join("source.pdf"),
        "Changed document with longer metadata",
    );
    let observation = probe::inspect(&before);
    assert!(observation.analysis.is_some());
    db.apply_verification(&before, observation).unwrap();
    assert_eq!(db.recovery_file(file).unwrap().source.revision, 2);
    assert_ne!(db.recovery_file(file).unwrap().hash, before.hash);
    assert_eq!(db.get_book(book).unwrap().title, "My title");
    assert_eq!(
        db.0.query_row("SELECT count(*) FROM page_text", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.reader_annotations(book).unwrap().notes[0].text,
        "Personal note"
    );
    assert_eq!(
        db.0.query_row(
            "SELECT file_revision FROM reading_progress WHERE book_id=?1",
            [book],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert!(!db
        .apply_verification(&before, probe::inspect(&before))
        .unwrap());
}
#[test]
fn relink_preview_cancel_confirm_preserve_identity_anchors_and_original_bytes() {
    let (dir, db, book, file) = fixture();
    anchors(&db, book, file);
    let original = fs::read(dir.path().join("source.pdf")).unwrap();
    fs::copy(
        dir.path().join("source.pdf"),
        dir.path().join("replacement.pdf"),
    )
    .unwrap();
    let source = db.recovery_file(file).unwrap();
    let (candidate, analysis) = probe::replacement(&dir.path().join("replacement.pdf")).unwrap();
    let preview = db
        .create_relink_draft(&source, &candidate, &analysis)
        .unwrap();
    assert!(preview.identical);
    assert_eq!(preview.new_pages, Some(1));
    db.cancel_relink(preview.draft).unwrap();
    assert!(db.relink_draft(preview.draft).is_err());
    assert_eq!(
        db.recovery_file(file).unwrap().source.path,
        source.source.path
    );
    let preview = db
        .create_relink_draft(&source, &candidate, &analysis)
        .unwrap();
    let draft = db.relink_draft(preview.draft).unwrap();
    db.confirm_relink(&draft).unwrap();
    assert_eq!(db.recovery_file(file).unwrap().source.revision, 1);
    assert_eq!(db.recovery_file(file).unwrap().source.path, candidate.path);
    assert_eq!(db.reader_annotations(book).unwrap().notes.len(), 1);
    assert_eq!(db.reader_annotations(book).unwrap().bookmarks.len(), 1);
    assert_eq!(
        db.0.query_row(
            "SELECT page_number FROM reading_progress WHERE book_id=?1",
            [book],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(fs::read(dir.path().join("source.pdf")).unwrap(), original);
    assert!(db.relink_draft(preview.draft).is_err());
    // Different replacement preserves records but marks page anchors stale by advancing revision.
    pdf(&dir.path().join("different.pdf"), "Different book");
    let (candidate, analysis) = probe::replacement(&dir.path().join("different.pdf")).unwrap();
    let current = db.recovery_file(file).unwrap();
    let preview = db
        .create_relink_draft(&current, &candidate, &analysis)
        .unwrap();
    assert!(!preview.identical);
    db.confirm_relink(&db.relink_draft(preview.draft).unwrap())
        .unwrap();
    assert_eq!(db.recovery_file(file).unwrap().source.revision, 2);
    while metadata::process_next(&db).unwrap() {}
    assert_eq!(db.get_book(book).unwrap().title, "Original");
    assert_eq!(
        db.reader_annotations(book).unwrap().notes[0].file_revision,
        1
    );
    assert_eq!(
        fs::read(dir.path().join("replacement.pdf")).unwrap(),
        original
    );
}
#[test]
fn relink_rejects_malformed_registered_expired_and_changed_candidates() {
    let (dir, db, _, file) = fixture();
    fs::write(dir.path().join("fake.pdf"), b"%PDF-1.7\nnot real\n%%EOF").unwrap();
    assert!(probe::replacement(&dir.path().join("fake.pdf")).is_err());
    assert!(probe::replacement(Path::new("../escape.pdf")).is_err());
    let (same, analysis) = probe::replacement(&dir.path().join("source.pdf")).unwrap();
    assert!(db
        .create_relink_draft(&db.recovery_file(file).unwrap(), &same, &analysis)
        .is_err());
    pdf(&dir.path().join("other.pdf"), "Other");
    let (candidate, analysis) = probe::replacement(&dir.path().join("other.pdf")).unwrap();
    let p = db
        .create_relink_draft(&db.recovery_file(file).unwrap(), &candidate, &analysis)
        .unwrap();
    db.0.execute(
        "UPDATE relink_drafts SET created_at=0 WHERE id=?1",
        [p.draft],
    )
    .unwrap();
    assert!(db.relink_draft(p.draft).is_err());
    let p = db
        .create_relink_draft(&db.recovery_file(file).unwrap(), &candidate, &analysis)
        .unwrap();
    pdf(&dir.path().join("other.pdf"), "Changed after preview");
    assert!(db.relink_draft(p.draft).unwrap().candidate.open().is_err());
}
#[test]
fn duplicate_preference_ignore_and_record_removal_preserve_pdf_sources() {
    let (dir, db, first_book, first) = fixture();
    fs::copy(dir.path().join("source.pdf"), dir.path().join("copy.pdf")).unwrap();
    let (second_book, second) = register(&db, &dir.path().join("copy.pdf"));
    while metadata::process_next(&db).unwrap() {}
    anchors(&db, second_book, second);
    let keep = db.recovery_file(first).unwrap();
    let hash = hex(keep.hash.as_deref().unwrap());
    assert_eq!(db.duplicate_files(&hash, 0).unwrap().len(), 2);
    db.prefer_duplicate(&hash, first, 1).unwrap();
    assert!(db.duplicate_files(&hash, 0).unwrap()[0].preferred);
    assert!(db
        .review_duplicate(keep.hash.as_deref().unwrap(), true)
        .unwrap());
    assert!(db.remove_duplicate(first, 1, &keep).is_err());
    let bad = RegisteredFile {
        source: keep.source.clone(),
        hash: Some(vec![0; 32]),
        availability: "available".into(),
    };
    assert!(db.remove_duplicate(second, 1, &bad).is_err());
    db.remove_duplicate(second, 1, &keep).unwrap();
    assert!(db.get_book(second_book).is_err());
    assert!(db.get_book(first_book).is_ok());
    assert!(dir.path().join("copy.pdf").is_file());
    assert_eq!(
        fs::read(dir.path().join("copy.pdf")).unwrap(),
        fs::read(dir.path().join("source.pdf")).unwrap()
    );
    assert_eq!(db.duplicate_files(&hash, 0).unwrap().len(), 1);
    assert!(db.duplicate_files("bad hash", 0).is_err());
    assert!(db.remove_duplicate(second, 1, &keep).is_err());
    db.0.execute(
        "INSERT INTO page_text_fts(page_text_fts,rank) VALUES('integrity-check',1)",
        [],
    )
    .unwrap();
}

#[test]
fn relink_digest_guard_rejects_same_stamp_changes_and_holds_read_lock() {
    let (dir, db, _, file) = fixture();
    pdf(&dir.path().join("candidate.pdf"), "Preview A");
    let (candidate, analysis) = probe::replacement(&dir.path().join("candidate.pdf")).unwrap();
    let preview = db
        .create_relink_draft(&db.recovery_file(file).unwrap(), &candidate, &analysis)
        .unwrap();
    let guard = probe::verified_handle(&candidate, &analysis.digest).unwrap();
    assert!(fs::OpenOptions::new()
        .write(true)
        .open(dir.path().join("candidate.pdf"))
        .is_err());
    drop(guard);
    let old = fs::read(dir.path().join("candidate.pdf")).unwrap();
    let mut changed = old.clone();
    let position = changed.windows(9).position(|w| w == b"Preview A").unwrap();
    changed[position + 8] = b'B';
    fs::write(dir.path().join("candidate.pdf"), changed).unwrap();
    let nanos: u64 = candidate.token.as_ref().unwrap().parse().unwrap();
    let handle = fs::OpenOptions::new()
        .write(true)
        .open(dir.path().join("candidate.pdf"))
        .unwrap();
    handle
        .set_times(
            fs::FileTimes::new()
                .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_nanos(nanos)),
        )
        .unwrap();
    drop(handle);
    assert!(candidate.open().is_ok());
    assert!(probe::verified_handle(&candidate, &analysis.digest).is_err());
    db.cancel_relink(preview.draft).unwrap();
    assert_eq!(db.recovery_file(file).unwrap().source.revision, 1);
}

#[test]
fn removing_one_of_multiple_book_files_preserves_book_and_anchors() {
    let (dir, db, book, first) = fixture();
    anchors(&db, book, first);
    fs::copy(
        dir.path().join("source.pdf"),
        dir.path().join("alternate.pdf"),
    )
    .unwrap();
    let path = fs::canonicalize(dir.path().join("alternate.pdf")).unwrap();
    let stamp = files::stamp(&fs::File::open(&path).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    tx.execute("UPDATE book_files SET is_primary=0 WHERE id=?1", [first])
        .unwrap();
    let second = insert_file(&tx, book, path.to_str().unwrap(), &stamp).unwrap();
    tx.commit().unwrap();
    while metadata::process_next(&db).unwrap() {}
    let keep = db.recovery_file(first).unwrap();
    db.remove_duplicate(second, 1, &keep).unwrap();
    assert_eq!(db.get_book(book).unwrap().files.len(), 1);
    assert_eq!(db.reader_annotations(book).unwrap().notes.len(), 1);
    assert_eq!(db.reader_annotations(book).unwrap().bookmarks.len(), 1);
    assert!(db
        .0
        .query_row(
            "SELECT is_primary FROM book_files WHERE id=?1",
            [first],
            |r| r.get::<_, bool>(0)
        )
        .unwrap());
    assert!(dir.path().join("alternate.pdf").exists());
}
