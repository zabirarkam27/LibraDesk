use super::archive;
use crate::database::Database;
use std::fs;
#[test]
fn backup_snapshot_manifest_excludes_originals_preserves_settings_and_detects_tampering() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("LibraDesk");
    let db = Database::open(&crate::storage::initialize(&root).unwrap()).unwrap();
    db.set_theme(crate::database::Scope::App, crate::database::Theme::Dark)
        .unwrap();
    db.0.execute("INSERT INTO books(title,title_sort_key,created_at,updated_at) VALUES('বাংলা backup','বাংলা backup',0,0)",[]).unwrap();
    fs::write(root.join("cache/private.pdf"), b"%PDF-do not copy").unwrap();
    let summary = archive::create(&db, &root, &root.join("backups"), false).unwrap();
    assert_eq!(summary.books, 1);
    assert_eq!(summary.thumbnails, 0);
    let folder = std::path::Path::new(&summary.path);
    let manifest = archive::manifest(folder).unwrap();
    assert!(!manifest.originals_included);
    assert_eq!(manifest.files.len(), 1);
    archive::verify_files(folder, &manifest).unwrap();
    let staged = tempfile::tempdir().unwrap();
    archive::copy_verified(folder, staged.path(), &manifest).unwrap();
    let restored = Database::open(&staged.path().join("library.sqlite3")).unwrap();
    assert_eq!(
        restored.preferences(None).unwrap().app,
        crate::database::Theme::Dark
    );
    assert_eq!(
        restored
            .0
            .query_row("SELECT count(*) FROM books", [], |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(root.join("cache/private.pdf").exists());
    let mut bad = manifest.clone();
    bad.files[0].path = "../escape".into();
    fs::write(
        folder.join("manifest.json"),
        serde_json::to_vec(&bad).unwrap(),
    )
    .unwrap();
    assert!(archive::manifest(folder).is_err());
    fs::write(
        folder.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(folder.join("library.sqlite3"), b"corrupt").unwrap();
    assert!(archive::verify_files(folder, &manifest).is_err());
}

#[test]
fn optional_thumbnails_validate_and_unexpected_schema_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("LibraDesk");
    let db = Database::open(&crate::storage::initialize(&root).unwrap()).unwrap();
    let original = dir.path().join("original.pdf");
    fs::write(&original, b"%PDF-1.7\noriginal").unwrap();
    let path = fs::canonicalize(&original).unwrap();
    let stamp = crate::scan::files::stamp(&fs::File::open(&path).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    let book = crate::catalog::repository::insert_book(&tx, "Thumbnail fixture").unwrap();
    let file =
        crate::catalog::repository::insert_file(&tx, book, path.to_str().unwrap(), &stamp).unwrap();
    tx.commit().unwrap();
    let mut png = Vec::new();
    {
        let encoder = png::Encoder::new(&mut png, 1, 1);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[0]).unwrap();
    }
    fs::write(root.join("thumbnails").join(format!("{file}-1.png")), &png).unwrap();
    db.0.execute(
        "INSERT INTO thumbnails VALUES(?1,1,?2,1,1,?3,0)",
        rusqlite::params![file, format!("{file}-1.png"), png.len() as i64],
    )
    .unwrap();
    let backup = archive::create(&db, &root, &root.join("backups"), true).unwrap();
    assert_eq!(backup.thumbnails, 1);
    let folder = std::path::Path::new(&backup.path);
    let manifest = archive::manifest(folder).unwrap();
    assert_eq!(manifest.files.len(), 2);
    archive::verify_files(folder, &manifest).unwrap();
    assert_eq!(fs::read(&original).unwrap(), b"%PDF-1.7\noriginal");
    let staged = tempfile::tempdir().unwrap();
    archive::copy_verified(folder, staged.path(), &manifest).unwrap();
    let connection = rusqlite::Connection::open(staged.path().join("library.sqlite3")).unwrap();
    connection
        .execute_batch("CREATE TABLE unexpected(secret TEXT)")
        .unwrap();
    drop(connection);
    assert!(crate::database::validate_snapshot(&staged.path().join("library.sqlite3")).is_err());
}
