use crate::{
    catalog::{repository::BookInput, search::CatalogQuery},
    database::Database,
};
#[test]
fn sql_and_fts_metacharacters_are_literal_and_cannot_mutate_catalog() {
    let temp = tempfile::tempdir().unwrap();
    let db = Database::open(&temp.path().join("library.sqlite3")).unwrap();
    db.create_book(&BookInput {
        title: "Safe বাংলা title".into(),
        ..Default::default()
    })
    .unwrap();
    for text in [
        "' OR 1=1 --",
        "\"; DROP TABLE books; --",
        "NEAR(foo bar)",
        "<script>alert(1)</script>",
        "* : ( )",
    ] {
        let result = db
            .query_catalog(&CatalogQuery {
                text: text.into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(result.total, 0);
    }
    assert_eq!(db.query_catalog(&CatalogQuery::default()).unwrap().total, 1);
    assert_eq!(
        db.0.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}
#[cfg(windows)]
#[test]
fn junctions_are_rejected_for_storage_and_retained_during_cleanup() {
    use std::{fs, os::windows::process::CommandExt, process::Command};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("AppData");
    crate::storage::initialize(&root).unwrap();
    let target = temp.path().join("outside");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("original.pdf"), b"%PDF-source").unwrap();
    let link = root.join("cache").join("junction");
    let output = Command::new("cmd.exe")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(&target)
        .creation_flags(0x08000000)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(crate::storage_management::directory(&link).is_err());
    assert!(crate::storage::initialize(&link).is_err());
    let before = crate::storage_management::usage(&root).unwrap();
    assert!(before.skipped >= 1);
    crate::storage_management::clean(&root, crate::storage_management::CacheKind::Cache).unwrap();
    assert_eq!(
        fs::read(target.join("original.pdf")).unwrap(),
        b"%PDF-source"
    );
    assert!(link.exists());
    // Removing a junction itself is nonrecursive and never removes the target.
    fs::remove_dir(link).unwrap();
}
