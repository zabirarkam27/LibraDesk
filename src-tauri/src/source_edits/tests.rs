use super::{pdf, repository, worker, xmp};
use crate::{
    catalog::{
        organization::{GroupInput, GroupKind},
        repository::BookInput,
    },
    database::Database,
};
use lopdf::{dictionary, text_string, Document, Object, Stream};
use std::{fs, path::Path};
fn make_pdf(path: &Path) {
    let mut doc = Document::with_version("1.7");
    let pages = doc.new_object_id();
    let content = doc.add_object(Stream::new(dictionary! {}, b"q\nQ".to_vec()));
    let page=doc.add_object(dictionary!{"Type"=>"Page","Parent"=>pages,"MediaBox"=>vec![0.into(),0.into(),144.into(),144.into()],"Contents"=>content});
    doc.objects.insert(
        pages,
        Object::Dictionary(
            dictionary! {"Type"=>"Pages","Kids"=>vec![Object::Reference(page)],"Count"=>1},
        ),
    );
    let xmp=doc.add_object(Stream::new(dictionary!{"Type"=>"Metadata","Subtype"=>"XML"},br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:other="urn:other" dc:title="Old title" other:Keep="keep attribute"><dc:creator><rdf:Seq><rdf:li>Old author</rdf:li></rdf:Seq></dc:creator><other:Keep>Keep unrelated XMP</other:Keep></rdf:Description></rdf:RDF></x:xmpmeta>"#.to_vec()));
    let root = doc.add_object(dictionary! {"Type"=>"Catalog","Pages"=>pages,"Metadata"=>xmp});
    let info=doc.add_object(dictionary!{"Title"=>text_string("Original title"),"Creator"=>text_string("Original creator"),"CustomKeep"=>text_string("preserve")});
    doc.trailer.set("Root", root);
    doc.trailer.set("Info", info);
    doc.save(path).unwrap();
}
fn fixture() -> (tempfile::TempDir, std::path::PathBuf, Database, i64, i64) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("AppData");
    let path = crate::storage::initialize(&root).unwrap();
    let db = Database::open(&path).unwrap();
    let pdf = dir.path().join("original.pdf");
    make_pdf(&pdf);
    let pdf = fs::canonicalize(pdf).unwrap();
    let stamp = crate::scan::files::stamp(&fs::File::open(&pdf).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    let book = crate::catalog::repository::insert_book(&tx, "Original title").unwrap();
    let file =
        crate::catalog::repository::insert_file(&tx, book, pdf.to_str().unwrap(), &stamp).unwrap();
    tx.commit().unwrap();
    (dir, root, db, book, file)
}
fn text(path: &Path, key: &[u8]) -> String {
    let doc = Document::load(path).unwrap();
    let id = doc.trailer.get(b"Info").unwrap().as_reference().unwrap();
    lopdf::decode_text_string(doc.get_dictionary(id).unwrap().get(key).unwrap()).unwrap()
}
fn edit(db: &Database, book: i64, title: &str) {
    let current = db.get_book(book).unwrap();
    db.update_book(
        book,
        current.revision,
        &BookInput {
            title: title.into(),
            subject: Some("Subject বাংলা & <safe>".into()),
            genre: Some("উপন্যাস".into()),
            author_names: Some(vec!["লেখক এক".into(), "Author two".into()]),
            publisher: Some("Publisher".into()),
            isbn: Some("9780000000002".into()),
            language: Some("bn".into()),
            publication_year: Some(2026),
            ..Default::default()
        },
    )
    .unwrap();
}
fn prepared_attempt(db: &Database, file: i64) -> repository::Attempt {
    let source = repository::source(&db.0, file).unwrap();
    let book: i64 =
        db.0.query_row("SELECT book_id FROM book_files WHERE id=?1", [file], |r| {
            r.get(0)
        })
        .unwrap();
    let data = repository::snapshot(&db.0, book).unwrap();
    let gen =
        db.0.query_row(
            "SELECT generation FROM pdf_write_jobs WHERE file_id=?1",
            [file],
            |r| r.get(0),
        )
        .unwrap();
    let prepared = pdf::prepare(&source, &data).unwrap();
    assert!(repository::stage_attempt(
        db,
        &source,
        gen,
        prepared.temporary.path(),
        &prepared.original,
        &prepared.digest,
        &prepared.token
    )
    .unwrap());
    let (handle, _) = prepared.temporary.keep().unwrap();
    drop(handle);
    repository::attempt(&db.0, Some(file)).unwrap().unwrap()
}
#[test]
fn atomic_metadata_roundtrip_preserves_pages_xmp_anchors_and_has_no_safety_backup() {
    let (dir, root, db, book, file) = fixture();
    let path = dir.path().join("original.pdf");
    let original = fs::read(&path).unwrap();
    let before = Document::load(&path).unwrap();
    let pages = before.get_pages();
    let content = before.get_page_content(pages[&1]);
    let parent = db
        .save_group(
            GroupKind::Category,
            &GroupInput {
                name: "বই".into(),
                ..Default::default()
            },
        )
        .unwrap();
    let child = db
        .save_group(
            GroupKind::Category,
            &GroupInput {
                name: "Fiction".into(),
                parent_id: Some(parent),
                ..Default::default()
            },
        )
        .unwrap();
    edit(&db, book, "নতুন Title");
    let current = db.get_book(book).unwrap();
    db.update_book(
        book,
        current.revision,
        &BookInput {
            title: current.title,
            genre: Some("উপন্যাস".into()),
            subject: Some("Subject বাংলা & <safe>".into()),
            category_ids: Some(vec![child]),
            author_names: Some(vec!["লেখক এক".into(), "Author two".into()]),
            ..Default::default()
        },
    )
    .unwrap();
    db.0.execute("INSERT INTO notes(book_id,file_id,file_revision,page_number,body,client_key,created_at,updated_at) VALUES(?1,?2,1,1,'note','key',0,0)",rusqlite::params![book,file]).unwrap();
    db.0.execute("INSERT INTO reading_progress(book_id,file_id,file_revision,page_number,total_pages,last_read_at) VALUES(?1,?2,1,1,1,0)",rusqlite::params![book,file]).unwrap();
    assert!(worker::process_next(&db, |_| {}).unwrap());
    assert_eq!(db.pdf_write_status(book).unwrap()[0].status, "completed");
    assert_eq!(text(&path, b"Title"), "নতুন Title");
    assert_eq!(text(&path, b"Author"), "লেখক এক; Author two");
    assert_eq!(text(&path, b"Genre"), "উপন্যাস");
    assert_eq!(text(&path, b"Creator"), "Original creator");
    assert_eq!(text(&path, b"CustomKeep"), "preserve");
    assert_eq!(text(&path, b"LibraDeskCategories"), r#"["বই / Fiction"]"#);
    let after = Document::load(&path).unwrap();
    assert_eq!(after.get_pages(), pages);
    assert_eq!(after.get_page_content(pages[&1]), content);
    assert!(fs::read(&path).unwrap().starts_with(&original));
    let root_dict = after.catalog().unwrap();
    let xmp_id = root_dict.get(b"Metadata").unwrap().as_reference().unwrap();
    let xml = String::from_utf8(
        after
            .get_object(xmp_id)
            .unwrap()
            .as_stream()
            .unwrap()
            .content
            .clone(),
    )
    .unwrap();
    assert!(xml.contains("Keep unrelated XMP") && xml.contains("keep attribute"));
    assert!(!xml.contains("Old title") && !xml.contains("Old author"));
    assert_eq!(
        db.reader_annotations(book).unwrap().notes[0].file_revision,
        2
    );
    assert!(!db.open_reader(book, None).unwrap().stale_progress);
    assert_eq!(fs::read_dir(root.join("backups")).unwrap().count(), 0);
    assert!(!fs::read_dir(dir.path()).unwrap().any(|p| p
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".libradesk-edit-")));
    assert_eq!(
        db.query_catalog(&crate::catalog::search::CatalogQuery {
            text: "উপন্যাস".into(),
            ..Default::default()
        })
        .unwrap()
        .total,
        1
    );
    let parent_group = db
        .organization()
        .unwrap()
        .categories
        .into_iter()
        .find(|g| g.id == parent)
        .unwrap();
    db.save_group(
        GroupKind::Category,
        &GroupInput {
            id: Some(parent),
            revision: Some(parent_group.revision),
            name: "Renamed".into(),
            ..Default::default()
        },
    )
    .unwrap();
    // Closing the fixture reader allows the next source write.
    db.0.execute(
        "UPDATE reading_sessions SET ended_at=last_checkpoint_at",
        [],
    )
    .unwrap();
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(
        text(&path, b"LibraDeskCategories"),
        r#"["Renamed / Fiction"]"#
    );
}
#[test]
fn prepared_write_reconciles_after_replacement_and_newer_edits_are_not_lost() {
    let (dir, root, db, book, file) = fixture();
    let path = dir.path().join("original.pdf");
    edit(&db, book, "First edit");
    let attempt = prepared_attempt(&db, file);
    assert!(worker::apply(&db, &attempt, true).is_err());
    assert_eq!(text(&path, b"Title"), "First edit");
    assert_eq!(repository::source(&db.0, file).unwrap().revision, 1);
    drop(db);
    let db = Database::open(&root.join("database/library.sqlite3")).unwrap();
    edit(&db, book, "Second edit");
    assert!(worker::process_next(&db, |_| {}).unwrap());
    assert_eq!(db.pdf_write_status(book).unwrap()[0].status, "queued");
    assert!(worker::process_next(&db, |_| {}).unwrap());
    assert_eq!(text(&path, b"Title"), "Second edit");
    assert_eq!(db.pdf_write_status(book).unwrap()[0].status, "completed");
    assert!(repository::attempt(&db.0, Some(file)).unwrap().is_none());
}
#[test]
fn readonly_changed_signed_and_cancelled_sources_are_not_overwritten() {
    let (dir, _root, db, book, file) = fixture();
    let path = dir.path().join("original.pdf");
    let original = fs::read(&path).unwrap();
    edit(&db, book, "Desired");
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions.clone()).unwrap();
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(db.pdf_write_status(book).unwrap()[0].status, "failed");
    #[allow(clippy::permissions_set_readonly_false)]
    {
        permissions.set_readonly(false);
    }
    fs::set_permissions(&path, permissions).unwrap();
    db.0.execute("UPDATE pdf_write_jobs SET status='queued'", [])
        .unwrap();
    let attempt = prepared_attempt(&db, file);
    let mut changed = original.clone();
    changed.extend_from_slice(b"\n% external modification");
    fs::write(&path, &changed).unwrap();
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(fs::read(&path).unwrap(), changed);
    assert!(repository::attempt(&db.0, Some(file)).unwrap().is_none());
    assert!(!Path::new(&attempt.stage).exists());
    worker::cancel(&db, file).unwrap();
    assert_eq!(
        db.pdf_write_status(book).unwrap()[0].status,
        "not_requested"
    );
    let mut doc = Document::load_mem(&original).unwrap();
    doc.add_object(dictionary!{"Type"=>"Sig","ByteRange"=>vec![0.into(),1.into()],"Contents"=>Object::String(vec![0],lopdf::StringFormat::Hexadecimal)});
    doc.save(&path).unwrap();
    let stamp = crate::scan::files::stamp(&fs::File::open(&path).unwrap()).unwrap();
    db.0.execute(
        "UPDATE book_files SET size_bytes=?2,modified_at=?3,source_token=?4 WHERE id=?1",
        rusqlite::params![file, stamp.bytes, stamp.modified_at, stamp.token],
    )
    .unwrap();
    let signed = fs::read(&path).unwrap();
    edit(&db, book, "Signed");
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(fs::read(&path).unwrap(), signed);
    assert!(db.pdf_write_status(book).unwrap()[0]
        .error
        .as_ref()
        .unwrap()
        .contains("signed"));
    db.delete_book(book, db.get_book(book).unwrap().revision)
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), signed);
}
#[test]
fn xmp_preserves_unrelated_namespaces_and_rejects_unsafe_packets() {
    let (_dir, _root, db, book, _file) = fixture();
    let snapshot = repository::snapshot(&db.0, book).unwrap();
    assert!(xmp::merge(
        Some(b"<!DOCTYPE x [<!ENTITY x SYSTEM 'file:///x'>]><x/>"),
        &snapshot
    )
    .is_err());
    assert!(xmp::merge(Some(b"<broken>"), &snapshot).is_err());
    let xml = xmp::merge(None, &snapshot).unwrap();
    let twice = xmp::merge(Some(&xml), &snapshot).unwrap();
    assert_eq!(
        String::from_utf8(twice)
            .unwrap()
            .matches("<dc:title>")
            .count(),
        1
    );
}

#[test]
fn locked_replacement_retries_from_stage_and_tampering_can_be_cancelled() {
    let (dir, _root, db, book, file) = fixture();
    let path = dir.path().join("original.pdf");
    let original = fs::read(&path).unwrap();
    edit(&db, book, "Locked update");
    let locked = crate::scan::files::open_regular_readonly(&path).unwrap();
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(db.pdf_write_status(book).unwrap()[0].status, "failed");
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(repository::attempt(&db.0, Some(file)).unwrap().is_some());
    drop(locked);
    db.0.execute(
        "UPDATE pdf_write_jobs SET status='queued' WHERE file_id=?1",
        [file],
    )
    .unwrap();
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(text(&path, b"Title"), "Locked update");
    edit(&db, book, "Next update");
    let attempt = prepared_attempt(&db, file);
    fs::write(&attempt.stage, b"tampered").unwrap();
    let current = fs::read(&path).unwrap();
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(fs::read(&path).unwrap(), current);
    assert!(db.pdf_write_status(book).unwrap()[0]
        .error
        .as_ref()
        .unwrap()
        .contains("staged"));
    assert!(db
        .delete_book(book, db.get_book(book).unwrap().revision)
        .is_err());
    worker::cancel(&db, file).unwrap();
    assert!(!Path::new(&attempt.stage).exists());
    db.delete_book(book, db.get_book(book).unwrap().revision)
        .unwrap();
    assert_eq!(fs::read(&path).unwrap(), current);
}
#[test]
fn encrypted_and_oversized_pdfs_fail_without_replacing_the_source() {
    let (dir, _root, db, book, file) = fixture();
    let path = dir.path().join("original.pdf");
    let mut doc = Document::load(&path).unwrap();
    doc.trailer.set(
        "ID",
        Object::Array(vec![
            Object::string_literal(vec![1; 16]),
            Object::string_literal(vec![2; 16]),
        ]),
    );
    let state = lopdf::EncryptionState::try_from(lopdf::EncryptionVersion::V1 {
        document: &doc,
        owner_password: "owner",
        user_password: "password",
        permissions: lopdf::Permissions::all(),
    })
    .unwrap();
    doc.encrypt(&state).unwrap();
    doc.save(&path).unwrap();
    let stamp = crate::scan::files::stamp(&fs::File::open(&path).unwrap()).unwrap();
    db.0.execute(
        "UPDATE book_files SET size_bytes=?2,modified_at=?3,source_token=?4 WHERE id=?1",
        rusqlite::params![file, stamp.bytes, stamp.modified_at, stamp.token],
    )
    .unwrap();
    let original = fs::read(&path).unwrap();
    edit(&db, book, "Encrypted edit");
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(db.pdf_write_status(book).unwrap()[0].status, "failed");
    worker::cancel(&db, file).unwrap();
    let handle = fs::OpenOptions::new().write(true).open(&path).unwrap();
    handle.set_len((pdf::LIMIT + 1) as u64).unwrap();
    drop(handle);
    let stamp = crate::scan::files::stamp(&fs::File::open(&path).unwrap()).unwrap();
    db.0.execute(
        "UPDATE book_files SET size_bytes=?2,modified_at=?3,source_token=?4 WHERE id=?1",
        rusqlite::params![file, stamp.bytes, stamp.modified_at, stamp.token],
    )
    .unwrap();
    edit(&db, book, "Too large");
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(fs::metadata(&path).unwrap().len(), (pdf::LIMIT + 1) as u64);
    assert!(db.pdf_write_status(book).unwrap()[0]
        .error
        .as_ref()
        .unwrap()
        .contains("256 MiB"));
}

#[test]
fn an_open_reader_defers_source_writes_until_its_session_closes() {
    let (dir, _root, db, book, _file) = fixture();
    let path = dir.path().join("original.pdf");
    let original = fs::read(&path).unwrap();
    let reader = db.open_reader(book, None).unwrap();
    edit(&db, book, "After reader closes");
    assert!(!worker::process_next(&db, |_| {}).unwrap());
    assert_eq!(fs::read(&path).unwrap(), original);
    db.save_reader_progress(&crate::reader::ProgressInput {
        session_id: reader.session_id,
        sequence: 1,
        page: 1,
        total: 1,
        active_ms: 0,
        close: true,
    })
    .unwrap();
    assert!(worker::process_next(&db, |_| {}).unwrap());
    assert_eq!(text(&path, b"Title"), "After reader closes");
}
#[test]
#[ignore = "Explicit native QA fixture preparation"]
fn prepare_native_edit_fixture() {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let dir = tempfile::Builder::new()
        .prefix("pdf-edit-profile-")
        .tempdir_in(&target)
        .unwrap();
    let root = dir.path().join("AppData");
    let path = crate::storage::initialize(&root).unwrap();
    let db = Database::open(&path).unwrap();
    let source = dir.path().join("original.pdf");
    make_pdf(&source);
    fs::write(dir.path().join("baseline.bin"), fs::read(&source).unwrap()).unwrap();
    let source = fs::canonicalize(source).unwrap();
    let stamp = crate::scan::files::stamp(&fs::File::open(&source).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    let book = crate::catalog::repository::insert_book(&tx, "Original title").unwrap();
    let file = crate::catalog::repository::insert_file(&tx, book, source.to_str().unwrap(), &stamp)
        .unwrap();
    tx.execute("INSERT INTO notes(book_id,file_id,file_revision,page_number,body,client_key,created_at,updated_at) VALUES(?1,?2,1,1,'Native note','native-note',0,0)",rusqlite::params![book,file]).unwrap();
    tx.execute("INSERT INTO reading_progress(book_id,file_id,file_revision,page_number,total_pages,last_read_at) VALUES(?1,?2,1,1,1,0)",rusqlite::params![book,file]).unwrap();
    tx.commit().unwrap();
    drop(db);
    fs::write(
        target.join("native-edit-profile.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"root":root,"book":book,"file":file,"pdf":source}),
        )
        .unwrap(),
    )
    .unwrap();
    let _ = dir.keep();
    println!("Prepared isolated native metadata/cover fixture");
}
#[test]
#[ignore = "Explicit native QA on-disk verification"]
fn verify_native_edit_fixture() {
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(target.join("native-edit-profile.json")).unwrap())
            .unwrap();
    let path = Path::new(value["pdf"].as_str().unwrap());
    let root = Path::new(value["root"].as_str().unwrap());
    assert!(fs::canonicalize(root)
        .unwrap()
        .starts_with(fs::canonicalize(&target).unwrap()));
    let baseline = fs::read(path.parent().unwrap().join("baseline.bin")).unwrap();
    let current = fs::read(path).unwrap();
    assert!(current.starts_with(&baseline));
    assert_eq!(text(path, b"Title"), "Native বাংলা title");
    assert_eq!(text(path, b"Genre"), "উপন্যাস");
    assert_eq!(text(path, b"Author"), "লেখক; Native author");
    assert_eq!(text(path, b"Publisher"), "Native publisher");
    assert_eq!(text(path, b"ISBN"), "9780000000002");
    assert_eq!(text(path, b"Language"), "bn");
    assert_eq!(text(path, b"PublicationYear"), "2026");
    assert_eq!(text(path, b"LibraDeskCategories"), r#"["বাংলা / Fiction"]"#);
    assert_eq!(text(path, b"LibraDeskTags"), r#"["Native tag"]"#);
    assert_eq!(
        text(path, b"LibraDeskCollections"),
        r#"["Native collection"]"#
    );
    assert_eq!(text(path, b"LibraDeskFavorite"), "true");
    assert_eq!(text(path, b"LibraDeskReadingStatus"), "reading");
    let db = Database::open(&root.join("database/library.sqlite3")).unwrap();
    let book = value["book"].as_i64().unwrap();
    let file = value["file"].as_i64().unwrap();
    let current = Document::load(path).unwrap();
    let before = Document::load_mem(&baseline).unwrap();
    assert_eq!(current.get_pages(), before.get_pages());
    assert_eq!(
        current.get_page_content(current.get_pages()[&1]),
        before.get_page_content(before.get_pages()[&1])
    );
    assert_eq!(
        db.reader_annotations(book).unwrap().notes[0].text,
        "Native note"
    );
    let revision = db.get_book(book).unwrap().custom_cover_revision.unwrap();
    assert!(db.custom_cover(file, -revision).unwrap().unwrap().len() > 50);
    assert_eq!(db.pdf_write_status(book).unwrap()[0].status, "completed");
    assert_eq!(
        db.0.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    println!("Native on-disk PDF Info/genre/categories/tags/collections, original page content, note, custom PNG and integrity: passed");
}

#[cfg(windows)]
#[test]
fn source_access_rules_and_internet_origin_mark_survive_replacement() {
    use std::{os::windows::process::CommandExt, process::Command};
    let (dir, _root, db, book, file) = fixture();
    let path = dir.path().join("original.pdf");
    let powershell = |script: &str| {
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", script])
            .env("LIBRADESK_QA_SOURCE", &path)
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Fixture ACL operation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    };
    powershell("$ErrorActionPreference='Stop';$acl=[System.IO.File]::GetAccessControl($env:LIBRADESK_QA_SOURCE);$acl.SetAccessRuleProtection($true,$false);$sid=[System.Security.Principal.WindowsIdentity]::GetCurrent().User;$rule=[System.Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','Allow');$acl.AddAccessRule($rule);[System.IO.File]::SetAccessControl($env:LIBRADESK_QA_SOURCE,$acl)");
    let acl_before =
        powershell("([System.IO.File]::GetAccessControl($env:LIBRADESK_QA_SOURCE)).Sddl");
    let mut zone = path.as_os_str().to_os_string();
    zone.push(":Zone.Identifier");
    fs::write(&zone, b"[ZoneTransfer]\r\nZoneId=3\r\n").unwrap();
    let stamp = crate::scan::files::stamp(&fs::File::open(&path).unwrap()).unwrap();
    db.0.execute(
        "UPDATE book_files SET size_bytes=?2,modified_at=?3,source_token=?4 WHERE id=?1",
        rusqlite::params![file, stamp.bytes, stamp.modified_at, stamp.token],
    )
    .unwrap();
    edit(&db, book, "Preserved permissions");
    worker::process_next(&db, |_| {}).unwrap();
    assert_eq!(db.pdf_write_status(book).unwrap()[0].status, "completed");
    assert_eq!(text(&path, b"Title"), "Preserved permissions");
    assert_eq!(fs::read(&zone).unwrap(), b"[ZoneTransfer]\r\nZoneId=3\r\n");
    // Windows can normalize the informational AutoInherited flag; access rules and protection remain identical.
    let acl_after =
        powershell("([System.IO.File]::GetAccessControl($env:LIBRADESK_QA_SOURCE)).Sddl");
    assert!(
        acl_after.replace("D:PAI(", "D:P(") == acl_before.replace("D:PAI(", "D:P("),
        "Windows ACL rules/protection changed"
    );
}
