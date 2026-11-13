use super::{archive, restore};
use crate::{
    database::Database,
    metadata,
    reader::{AnnotationInput, AnnotationKind, ProgressInput},
    scan,
};
use std::{fs, path::Path};
fn drain(db: &Database) {
    for _ in 0..100 {
        let Some(work) = db.next_work().unwrap() else {
            return;
        };
        scan::step(db, &work, || {}).unwrap();
    }
    panic!("fixture scan did not finish");
}
pub(super) fn pdf(path: &Path) {
    use lopdf::{dictionary, text_string, Document, Object};
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
    let info = doc.add_object(dictionary! {"Title"=>text_string("বাংলা integration")});
    doc.trailer.set("Root", catalog);
    doc.trailer.set("Info", info);
    doc.save(path).unwrap();
}
#[test]
fn interrupted_import_reader_backup_restore_and_relink_preserve_sources() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("AppData");
    let sources = temp.path().join("বই");
    fs::create_dir(&sources).unwrap();
    let original = sources.join("source.pdf");
    pdf(&original);
    let bytes = fs::read(&original).unwrap();
    let path = crate::storage::initialize(&root).unwrap();
    let db = Database::open(&path).unwrap();
    db.register_roots(&[scan::files::root_key(&sources)
        .unwrap()
        .to_string_lossy()
        .into_owned()])
        .unwrap();
    let root_id =
        db.0.query_row("SELECT id FROM scan_roots LIMIT 1", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
    db.queue_scan(root_id).unwrap();
    let work = db.next_work().unwrap().unwrap();
    scan::step(&db, &work, || {}).unwrap();
    drop(db);
    let mut db = Database::open(&path).unwrap();
    db.recover_scans().unwrap();
    let snapshot = serde_json::to_value(db.scan_snapshot(None).unwrap()).unwrap();
    if snapshot["jobs"][0]["status"] == "paused" {
        db.control_job(
            snapshot["jobs"][0]["id"].as_i64().unwrap(),
            scan::Action::Resume,
        )
        .unwrap();
    }
    drain(&db);
    let job = serde_json::to_value(db.scan_snapshot(None).unwrap()).unwrap()["jobs"][0]["id"]
        .as_i64()
        .unwrap();
    assert!(db.list_books(None, false).unwrap().is_empty());
    db.control_job(job, scan::Action::Confirm).unwrap();
    drain(&db);
    while metadata::process_next(&db).unwrap() {}
    let book = db.list_books(None, false).unwrap()[0].id;
    let details = db.get_book(book).unwrap();
    db.update_book(
        book,
        details.revision,
        &crate::catalog::repository::BookInput {
            title: "Manually edited বাংলা".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let opened = db.open_reader(book, None).unwrap();
    db.save_reader_progress(&ProgressInput {
        session_id: opened.session_id,
        sequence: 1,
        page: 1,
        total: 1,
        active_ms: 0,
        close: false,
    })
    .unwrap();
    db.save_reader_annotation(
        AnnotationKind::Note,
        &AnnotationInput {
            session_id: opened.session_id,
            id: None,
            revision: None,
            page: 1,
            text: "Persistent note".into(),
            client_key: "integration-note".into(),
        },
    )
    .unwrap();
    db.save_reader_progress(&ProgressInput {
        session_id: opened.session_id,
        sequence: 2,
        page: 1,
        total: 1,
        active_ms: 0,
        close: true,
    })
    .unwrap();
    let backup = archive::create(&db, &root, &root.join("backups"), false).unwrap();
    db.0.execute("DELETE FROM notes", []).unwrap();
    let draft = restore::prepare(&root, Path::new(&backup.path), 1).unwrap();
    restore::apply(&mut db, &root, draft, false).unwrap();
    assert_eq!(db.get_book(book).unwrap().title, "Manually edited বাংলা");
    assert_eq!(
        db.reader_annotations(book).unwrap().notes[0].text,
        "Persistent note"
    );
    let file = opened.file_id;
    let replacement = temp.path().join("replacement.pdf");
    fs::write(&replacement, &bytes).unwrap();
    let source = db.recovery_file(file).unwrap();
    let (candidate, analysis) = crate::recovery::probe::replacement(&replacement).unwrap();
    let preview = db
        .create_relink_draft(&source, &candidate, &analysis)
        .unwrap();
    db.confirm_relink(&db.relink_draft(preview.draft).unwrap())
        .unwrap();
    drop(db);
    let db = Database::open(&path).unwrap();
    assert_eq!(db.open_reader(book, None).unwrap().resume_page, 1);
    assert_eq!(db.reader_annotations(book).unwrap().notes.len(), 1);
    assert_eq!(fs::read(original).unwrap(), bytes);
    assert_eq!(fs::read(replacement).unwrap(), bytes);
    assert_eq!(
        db.0.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}
