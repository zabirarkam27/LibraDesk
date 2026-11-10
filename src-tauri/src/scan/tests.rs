use std::{fs, path::Path};

use crate::database::Database;

use super::{files, repository::Action, worker::step};

fn write_pdf(path: &Path) {
    fs::write(path, b"%PDF-1.7\n%%EOF\n").unwrap();
}

fn drain(db: &Database) {
    for _ in 0..30 {
        let Some(work) = db.next_work().unwrap() else {
            return;
        };
        step(db, &work, || {}).unwrap();
    }
    panic!("scan worker did not finish");
}

#[test]
fn previews_then_imports_once_and_retiring_root_preserves_catalog_and_history() {
    let sandbox = tempfile::tempdir().unwrap();
    let folder = sandbox.path().join("source");
    let nested = folder.join("পাঠ");
    fs::create_dir_all(&nested).unwrap();
    let original = nested.join("বই.PdF");
    write_pdf(&original);
    write_pdf(&folder.join("Copy.pdf"));
    fs::write(folder.join("broken.pdf"), b"not a PDF").unwrap();
    fs::write(folder.join("ignore.txt"), b"%PDF-1.7").unwrap();
    let root = files::root_key(&folder)
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();

    let db = Database::open(&sandbox.path().join("test.sqlite3")).unwrap();
    db.register_roots(&[root.clone(), root]).unwrap();
    let snapshot = db.scan_snapshot(None).unwrap();
    assert_eq!(snapshot.roots.len(), 1);
    let root_id = snapshot.roots[0].id;
    assert!(db.update_root(root_id, Some(false)).unwrap());
    assert!(!db.queue_scan(root_id).unwrap());
    assert!(db.update_root(root_id, Some(true)).unwrap());
    assert!(db.queue_scan(root_id).unwrap());
    assert!(!db.queue_scan(root_id).unwrap());
    drain(&db);

    let snapshot = db.scan_snapshot(None).unwrap();
    let job = &snapshot.jobs[0];
    assert_eq!(job.status, "awaiting_confirmation");
    assert_eq!((job.found, job.invalid, job.duplicates), (3, 1, 2));
    assert!(db.list_books(None, false).unwrap().is_empty());
    assert!(db.control_job(job.id, Action::Cancel).unwrap());
    drain(&db);
    assert!(db.list_books(None, false).unwrap().is_empty());
    assert!(db.queue_scan(root_id).unwrap());
    drain(&db);
    let snapshot = db.scan_snapshot(None).unwrap();
    let job = &snapshot.jobs[0];
    assert!(db.control_job(job.id, Action::Confirm).unwrap());
    assert!(!db.control_job(job.id, Action::Confirm).unwrap());
    db.0.execute_batch(
        "CREATE TEMP TRIGGER reject_fixture BEFORE INSERT ON books
         WHEN NEW.title='বই' BEGIN SELECT RAISE(ABORT, 'fixture rollback'); END;",
    )
    .unwrap();
    let importing = db.next_work().unwrap().unwrap();
    assert!(step(&db, &importing, || {}).is_err());
    assert!(db.list_books(None, false).unwrap().is_empty());
    db.0.execute_batch("DROP TRIGGER reject_fixture").unwrap();
    drain(&db);
    assert_eq!(db.list_books(None, false).unwrap().len(), 2);
    assert_eq!(fs::read(&original).unwrap(), b"%PDF-1.7\n%%EOF\n");

    let overlap = files::root_key(&nested)
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    db.register_roots(&[overlap]).unwrap();
    let overlap_id = db
        .scan_snapshot(None)
        .unwrap()
        .roots
        .iter()
        .find(|root| root.id != root_id)
        .unwrap()
        .id;
    assert!(db.queue_scan(overlap_id).unwrap());
    drain(&db);
    let history = db.scan_snapshot(None).unwrap();
    assert_eq!(history.jobs[0].existing, 1);
    assert!(db.control_job(history.jobs[0].id, Action::Cancel).unwrap());
    assert!(db.update_root(root_id, None).unwrap());
    assert_eq!(db.scan_snapshot(None).unwrap().roots.len(), 1);
    assert_eq!(db.scan_snapshot(None).unwrap().jobs[1].imported, 2);
    assert_eq!(db.list_books(None, false).unwrap().len(), 2);
    assert_eq!(
        db.0.pragma_query_value(None, "integrity_check", |row| row.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn interrupted_scan_resumes_from_frontier_and_changed_pdf_is_rejected() {
    let sandbox = tempfile::tempdir().unwrap();
    let folder = sandbox.path().join("root");
    fs::create_dir_all(folder.join("nested")).unwrap();
    let changed = folder.join("changed.pdf");
    write_pdf(&changed);
    write_pdf(&folder.join("nested/valid.pdf"));
    for index in 0..32 {
        write_pdf(&folder.join(format!("nested/extra-{index}.pdf")));
    }
    let root = files::root_key(&folder)
        .unwrap()
        .into_os_string()
        .into_string()
        .unwrap();
    let database_path = sandbox.path().join("test.sqlite3");
    let db = Database::open(&database_path).unwrap();
    db.register_roots(&[root]).unwrap();
    let root_id = db.scan_snapshot(None).unwrap().roots[0].id;
    assert!(db.queue_scan(root_id).unwrap());
    let work = db.next_work().unwrap().unwrap();
    step(&db, &work, || {}).unwrap();
    assert_eq!(db.scan_snapshot(None).unwrap().jobs[0].found, 1);
    assert!(db.control_job(work.id, Action::Pause).unwrap());
    step(&db, &work, || {}).unwrap();
    assert_eq!(db.scan_snapshot(None).unwrap().jobs[0].found, 1);
    assert!(db.control_job(work.id, Action::Resume).unwrap());
    let _ = db.next_work().unwrap().unwrap();
    drop(db);

    let db = Database::open(&database_path).unwrap();
    db.recover_scans().unwrap();
    assert_eq!(db.scan_snapshot(None).unwrap().jobs[0].status, "paused");
    assert!(db.control_job(work.id, Action::Resume).unwrap());
    drain(&db);
    assert_eq!(db.scan_snapshot(None).unwrap().jobs[0].found, 34);
    fs::write(&changed, b"%PDF-1.7\nchanged size\n%%EOF\n").unwrap();
    assert!(db.control_job(work.id, Action::Confirm).unwrap());
    let importing = db.next_work().unwrap().unwrap();
    step(&db, &importing, || {}).unwrap();
    assert_eq!(db.import_candidates(work.id, 32).unwrap().len(), 2);
    drop(db);

    let db = Database::open(&database_path).unwrap();
    db.recover_scans().unwrap();
    drain(&db);
    let snapshot = db.scan_snapshot(None).unwrap();
    let job = &snapshot.jobs[0];
    assert_eq!(
        (job.status.as_str(), job.imported, job.failed),
        ("completed", 33, 1)
    );
    assert_eq!(db.list_books(None, false).unwrap().len(), 33);
}

#[test]
fn inaccessible_root_fails_and_file_validation_rejects_paths_outside_it() {
    let sandbox = tempfile::tempdir().unwrap();
    let folder = sandbox.path().join("root");
    fs::create_dir(&folder).unwrap();
    let outside = sandbox.path().join("outside.pdf");
    write_pdf(&outside);
    let root = files::root_key(&folder).unwrap();
    let outside = fs::canonicalize(outside).unwrap();
    assert!(matches!(
        files::validate(&root, &outside),
        files::Pdf::Inaccessible
    ));

    let db = Database::open(&sandbox.path().join("test.sqlite3")).unwrap();
    db.register_roots(&[files::key(&root).unwrap().to_owned()])
        .unwrap();
    let root_id = db.scan_snapshot(None).unwrap().roots[0].id;
    fs::remove_dir(folder).unwrap();
    assert!(db.queue_scan(root_id).unwrap());
    drain(&db);
    let snapshot = db.scan_snapshot(None).unwrap();
    let job = &snapshot.jobs[0];
    assert_eq!(job.status, "failed");
    assert_eq!(job.error.as_deref(), Some("root_unavailable"));
}
