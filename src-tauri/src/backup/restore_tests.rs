use super::{archive, restore};
use crate::database::Database;
use std::{fs, path::Path};
fn setup() -> (tempfile::TempDir, std::path::PathBuf, Database) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("LibraDesk");
    let db = Database::open(&crate::storage::initialize(&root).unwrap()).unwrap();
    let pdf = dir.path().join("original.pdf");
    fs::write(&pdf, b"%PDF-1.7\noriginal").unwrap();
    let pdf = fs::canonicalize(pdf).unwrap();
    let stamp = crate::scan::files::stamp(&fs::File::open(&pdf).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    let book = crate::catalog::repository::insert_book(&tx, "Backup title").unwrap();
    let file =
        crate::catalog::repository::insert_file(&tx, book, pdf.to_str().unwrap(), &stamp).unwrap();
    tx.execute("INSERT INTO notes(book_id,file_id,file_revision,page_number,body,client_key,created_at,updated_at)
  VALUES(?1,?2,1,1,'Keep this note','note',0,0)",[book,file]).unwrap();
    tx.execute("INSERT INTO reading_progress(book_id,file_id,file_revision,page_number,total_pages,last_read_at)
  VALUES(?1,?2,1,1,1,0)",[book,file]).unwrap();
    tx.execute(
        "INSERT INTO page_text(file_id,file_revision,page_number,text,normalized_text,extracted_at)
  VALUES(?1,1,1,'বাংলা restore','বাংলা restore',0)",
        [file],
    )
    .unwrap();
    tx.commit().unwrap();
    db.set_theme(crate::database::Scope::App, crate::database::Theme::Dark)
        .unwrap();
    (dir, root, db)
}
fn title(db: &Database) -> String {
    db.0.query_row("SELECT title FROM books WHERE id=1", [], |r| r.get(0))
        .unwrap()
}
#[test]
fn restore_cancel_safety_rollback_and_success_with_idle_connections() {
    let (dir, root, mut db) = setup();
    let backup = archive::create(&db, &root, &root.join("backups"), false).unwrap();
    db.0.execute("UPDATE books SET title='Current title' WHERE id=1", [])
        .unwrap();
    db.set_theme(crate::database::Scope::App, crate::database::Theme::Light)
        .unwrap();
    let observer = Database::open(&root.join("database/library.sqlite3")).unwrap();
    let draft = restore::prepare(&root, Path::new(&backup.path), 1).unwrap();
    let stage = draft.stage.path().to_path_buf();
    let cache = draft.cache.path().to_path_buf();
    assert_eq!(title(&db), "Current title");
    drop(draft);
    assert!(!stage.exists());
    assert!(!cache.exists());
    assert_eq!(title(&db), "Current title");
    let draft = restore::prepare(&root, Path::new(&backup.path), 2).unwrap();
    assert!(restore::apply(&mut db, &root, draft, true).is_err());
    assert_eq!(title(&db), "Current title");
    assert_eq!(
        db.preferences(None).unwrap().app,
        crate::database::Theme::Light
    );
    let draft = restore::prepare(&root, Path::new(&backup.path), 3).unwrap();
    let result = restore::apply(&mut db, &root, draft, false).unwrap();
    assert!(Path::new(&result.safety_backup)
        .join("manifest.json")
        .exists());
    assert_eq!(title(&db), "Backup title");
    assert_eq!(title(&observer), "Backup title");
    assert_eq!(
        db.preferences(None).unwrap().app,
        crate::database::Theme::Dark
    );
    assert_eq!(
        db.reader_annotations(1).unwrap().notes[0].text,
        "Keep this note"
    );
    assert_eq!(
        db.0.query_row(
            "SELECT page_number FROM reading_progress WHERE book_id=1",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
    assert_eq!(
        db.search_page_text(&crate::extraction::search::PageQuery {
            text: "বাংলা".into(),
            page: 0
        })
        .unwrap()
        .total,
        1
    );
    assert_eq!(
        fs::read(dir.path().join("original.pdf")).unwrap(),
        b"%PDF-1.7\noriginal"
    );
    assert!(db
        .cover_cache_generation()
        .unwrap()
        .unwrap()
        .starts_with("covers-"));
}
#[test]
fn restore_rejects_newer_versions_corruption_changed_sources_and_staged_changes() {
    let (_dir, root, mut db) = setup();
    let backup = archive::create(&db, &root, &root.join("backups"), false).unwrap();
    let folder = Path::new(&backup.path);
    let original = archive::manifest(folder).unwrap();
    let mut manifest = original.clone();
    manifest.database.schema_version += 1;
    fs::write(
        folder.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(restore::prepare(&root, folder, 1).is_err());
    manifest = original.clone();
    manifest.app_version = "999.0.0".into();
    fs::write(
        folder.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    assert!(restore::prepare(&root, folder, 2).is_err());
    fs::write(
        folder.join("manifest.json"),
        serde_json::to_vec(&original).unwrap(),
    )
    .unwrap();
    let draft = restore::prepare(&root, folder, 3).unwrap();
    let mut bytes = fs::read(folder.join("manifest.json")).unwrap();
    bytes.push(b' ');
    fs::write(folder.join("manifest.json"), bytes).unwrap();
    assert!(restore::apply(&mut db, &root, draft, false).is_err());
    assert_eq!(title(&db), "Backup title");
    fs::write(
        folder.join("manifest.json"),
        serde_json::to_vec(&original).unwrap(),
    )
    .unwrap();
    let draft = restore::prepare(&root, folder, 4).unwrap();
    let altered = rusqlite::Connection::open(draft.stage.path().join("library.sqlite3")).unwrap();
    altered
        .execute(
            "UPDATE settings SET value_json='null' WHERE key='theme'",
            [],
        )
        .unwrap();
    drop(altered);
    assert!(restore::apply(&mut db, &root, draft, false).is_err());
    fs::write(folder.join("library.sqlite3"), b"corrupt").unwrap();
    assert!(restore::prepare(&root, folder, 5).is_err());
    assert_eq!(title(&db), "Backup title");
}
#[test]
fn restore_uses_a_new_cache_generation_and_preserves_old_cache() {
    let (_dir, root, mut db) = setup();
    let backup = archive::create(&db, &root, &root.join("backups"), false).unwrap();
    fs::write(root.join("thumbnails/1-1.png"), b"old library cover").unwrap();
    let draft = restore::prepare(&root, Path::new(&backup.path), 1).unwrap();
    restore::apply(&mut db, &root, draft, false).unwrap();
    let generation = db.cover_cache_generation().unwrap().unwrap();
    let selected = crate::metadata::resolve_cache_root(&root, Some(&generation)).unwrap();
    assert!(!selected.join("thumbnails/1-1.png").exists());
    assert_eq!(
        fs::read(root.join("thumbnails/1-1.png")).unwrap(),
        b"old library cover"
    );
    assert!(crate::metadata::resolve_cache_root(&root, Some("../escape")).is_err());
    let next = archive::create(&db, &root, &root.join("backups"), true).unwrap();
    assert_eq!(next.thumbnails, 0);
}

#[test]
fn custom_thumbnails_survive_cache_cleanup_and_restore_without_replaying_pdf_edits() {
    let (dir, root, mut db) = setup();
    let png = |color: [u8; 3]| {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&color).unwrap();
        }
        bytes
    };
    let first = png([120, 30, 20]);
    let second = png([20, 30, 120]);
    let original = fs::read(dir.path().join("original.pdf")).unwrap();
    crate::metadata::process_next(&db).unwrap();
    db.save_custom_cover(1, db.get_book(1).unwrap().revision, Some(&first))
        .unwrap();
    let revision = db.get_book(1).unwrap().custom_cover_revision.unwrap();
    assert_eq!(db.cover_revision(1).unwrap(), Some(-revision));
    assert!(db.next_cover().unwrap().is_none());
    assert_eq!(db.custom_cover(1, -revision).unwrap().unwrap(), first);
    assert!(db.save_custom_cover(1, 1, Some(b"bad PNG")).is_err());
    let book = db.get_book(1).unwrap();
    db.update_book(
        1,
        book.revision,
        &crate::catalog::repository::BookInput {
            title: book.title,
            genre: Some("Restored genre".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let saved = archive::create(&db, &root, &root.join("backups"), false).unwrap();
    db.save_custom_cover(1, db.get_book(1).unwrap().revision, Some(&second))
        .unwrap();
    let revision2 = db.get_book(1).unwrap().custom_cover_revision.unwrap();
    assert!(revision2 > revision);
    crate::storage_management::clean(&root, crate::storage_management::CacheKind::Thumbnails)
        .unwrap();
    assert_eq!(db.custom_cover(1, -revision2).unwrap().unwrap(), second);
    let draft = restore::prepare(&root, Path::new(&saved.path), 1).unwrap();
    restore::apply(&mut db, &root, draft, false).unwrap();
    assert_eq!(db.custom_cover(1, -revision).unwrap().unwrap(), first);
    assert_eq!(
        db.get_book(1).unwrap().genre.as_deref(),
        Some("Restored genre")
    );
    assert_eq!(db.pdf_write_status(1).unwrap()[0].status, "not_requested");
    assert_eq!(fs::read(dir.path().join("original.pdf")).unwrap(), original);
    db.save_custom_cover(1, db.get_book(1).unwrap().revision, None)
        .unwrap();
    assert!(db.get_book(1).unwrap().custom_cover_revision.is_none());
    assert!(db.next_cover().unwrap().is_some());
}

#[test]
fn restore_refuses_unreconciled_pdf_writes_before_replacing_library_data() {
    let (dir, root, mut db) = setup();
    let backup = archive::create(&db, &root, &root.join("backups"), false).unwrap();
    let draft = restore::prepare(&root, Path::new(&backup.path), 1).unwrap();
    let original = fs::read(dir.path().join("original.pdf")).unwrap();
    db.0.execute("INSERT INTO pdf_write_attempts(file_id,generation,source_revision,source_path,source_token,original_sha256,prepared_sha256,stage_path,created_at) SELECT id,1,revision,path_key,coalesce(source_token,''),zeroblob(32),zeroblob(32),path_key||'.pending',0 FROM book_files WHERE id=1",[]).unwrap();
    let result = restore::apply(&mut db, &root, draft, false);
    assert!(result.is_err());
    assert!(result
        .err()
        .unwrap()
        .message()
        .contains("PDF metadata saves"));
    assert_eq!(title(&db), "Backup title");
    assert_eq!(fs::read(dir.path().join("original.pdf")).unwrap(), original);
    assert_eq!(fs::read_dir(root.join("backups")).unwrap().count(), 1);
}
