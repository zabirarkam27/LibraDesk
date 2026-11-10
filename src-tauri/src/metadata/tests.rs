use std::{
    fs,
    io::{self, Cursor, Read},
    path::Path,
};

use lopdf::{dictionary, text_string, Document, Object};
use rusqlite::params;

use crate::{database::Database, scan::files};

use super::{
    cover,
    pdf::{filename_title, hash},
    process_next,
    repository::{Edit, Field},
};

fn make_pdf(path: &Path) {
    let mut doc = Document::with_version("1.4");
    let pages = doc.new_object_id();
    let page = doc.add_object(dictionary! {
        "Type" => "Page", "Parent" => pages,
        "MediaBox" => vec![0.into(), 0.into(), 144.into(), 144.into()],
    });
    doc.objects.insert(
        pages,
        Object::Dictionary(dictionary! {
            "Type" => "Pages", "Kids" => vec![Object::Reference(page)], "Count" => 1,
        }),
    );
    let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
    let info = doc.add_object(dictionary! {
        "Title" => text_string("বাংলা বই"),
        "Author" => text_string("  মূল লেখক  "),
        "Subject" => text_string("Original subject"),
        "Creator" => text_string("Fixture"),
        "Producer" => text_string("LibraDesk test"),
        "CreationDate" => text_string("D:20261111"),
    });
    doc.trailer.set("Root", catalog);
    doc.trailer.set("Info", info);
    doc.save(path).unwrap();
}

fn register(db: &Database, file: &Path) -> i64 {
    let canonical = fs::canonicalize(file).unwrap();
    let stamp = files::stamp(&fs::File::open(&canonical).unwrap()).unwrap();
    db.0.execute(
        "INSERT INTO books(title,title_sort_key,created_at,updated_at) VALUES ('Provisional','provisional',0,0)",
        [],
    ).unwrap();
    let book_id = db.0.last_insert_rowid();
    db.0.execute(
        "INSERT INTO book_files(book_id,original_path,path_key,size_bytes,modified_at,source_token)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![
            book_id,
            file.to_str().unwrap(),
            canonical.to_str().unwrap(),
            stamp.bytes,
            stamp.modified_at,
            stamp.token
        ],
    )
    .unwrap();
    book_id
}

#[test]
fn extracts_metadata_groups_exact_duplicates_and_preserves_overrides_after_refresh() {
    let sandbox = tempfile::tempdir().unwrap();
    let file = sandbox.path().join("বই_sample.pdf");
    let copy = sandbox.path().join("copy.pdf");
    make_pdf(&file);
    fs::copy(&file, &copy).unwrap();
    let original = fs::read(&file).unwrap();
    let db = Database::open(&sandbox.path().join("catalog.db")).unwrap();
    let first = register(&db, &file);
    let second = register(&db, &copy);
    assert!(process_next(&db).unwrap());
    assert!(process_next(&db).unwrap());
    assert!(!process_next(&db).unwrap());
    let rows = db.review_page(None, false).unwrap();
    assert_eq!(rows[0].title, "বাংলা বই");
    assert_eq!(rows[0].author.as_deref(), Some("মূল লেখক"));
    assert_eq!(rows[0].raw.author.as_deref(), Some("  মূল লেখক  "));
    assert_eq!(rows[0].raw.page_count, Some(1));
    assert_eq!(rows[0].raw.creator.as_deref(), Some("Fixture"));
    assert_eq!(rows[0].raw.producer.as_deref(), Some("LibraDesk test"));
    assert_eq!(rows[0].raw.creation_date_raw.as_deref(), Some("D:20261111"));
    assert!(db.review_page(None, true).unwrap().is_empty());
    let groups = db.duplicate_groups(None).unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].count, 2);
    assert!(!groups[0].reviewed);
    let digest = hash(&mut Cursor::new(&original)).unwrap();
    assert!(db.review_duplicate(&digest, true).unwrap());
    assert!(db.duplicate_groups(None).unwrap()[0].reviewed);

    assert!(db
        .save_metadata(
            &[first, second],
            &[Edit {
                field: Field::Author,
                value: "Edited".into()
            }]
        )
        .unwrap());
    let mut changed = original.clone();
    changed.extend_from_slice(b"\n% revision\n");
    fs::write(&file, &changed).unwrap();
    let stamp = files::stamp(&fs::File::open(&file).unwrap()).unwrap();
    db.0.execute(
        "UPDATE book_files SET revision=revision+1,size_bytes=?2,modified_at=?3,source_token=?4,sha256=NULL WHERE book_id=?1",
        params![first,stamp.bytes,stamp.modified_at,stamp.token],
    ).unwrap();
    assert!(process_next(&db).unwrap());
    assert!(db.duplicate_groups(None).unwrap().is_empty());
    let row = db
        .review_page(None, false)
        .unwrap()
        .into_iter()
        .find(|row| row.id == first)
        .unwrap();
    assert_eq!(row.author.as_deref(), Some("Edited"));
    assert_eq!(row.raw.author.as_deref(), Some("  মূল লেখক  "));
    assert_eq!(fs::read(file).unwrap(), changed);
    assert_eq!(fs::read(copy).unwrap(), original);
}

#[test]
fn streams_sha256_and_normalizes_filename_without_losing_bengali() {
    let digest = hash(&mut Cursor::new(b"abc")).unwrap();
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(filename_title(Path::new(" বাংলা__বই -  ০১.PDF")), "বাংলা বই ০১");
    assert_eq!(filename_title(Path::new("__--.pdf")), "Untitled PDF");

    struct Probe {
        data: io::Take<io::Repeat>,
        peak: usize,
    }
    impl Read for Probe {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.peak = self.peak.max(buffer.len());
            self.data.read(buffer)
        }
    }
    let mut large = Probe {
        data: io::repeat(0).take(34 * 1024 * 1024),
        peak: 0,
    };
    hash(&mut large).unwrap();
    assert_eq!(large.data.limit(), 0);
    assert_eq!(large.peak, 128 * 1024);
}

#[test]
fn hashes_a_password_protected_pdf_without_inventing_metadata() {
    let sandbox = tempfile::tempdir().unwrap();
    let path = sandbox.path().join("locked_book.pdf");
    make_pdf(&path);
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
        owner_password: "fixture-owner",
        user_password: "fixture-user",
        permissions: lopdf::Permissions::all(),
    })
    .unwrap();
    doc.encrypt(&state).unwrap();
    doc.save(&path).unwrap();
    let original = fs::read(&path).unwrap();
    let db = Database::open(&sandbox.path().join("catalog.db")).unwrap();
    register(&db, &path);
    assert!(process_next(&db).unwrap());
    let row = db.review_page(None, false).unwrap().pop().unwrap();
    assert_eq!(row.title, "locked book");
    assert!(row.raw.title.is_none() && row.author.is_none());
    let task = db.next_cover().unwrap().unwrap();
    assert!(task.needs_metadata);
    assert_eq!(fs::read(path).unwrap(), original);
}

#[test]
fn verifies_cover_bytes_and_rejects_out_of_bounds_ranges() {
    let sandbox = tempfile::tempdir().unwrap();
    fs::create_dir(sandbox.path().join("thumbnails")).unwrap();
    let path = sandbox.path().join("source.pdf");
    make_pdf(&path);
    let db = Database::open(&sandbox.path().join("catalog.db")).unwrap();
    register(&db, &path);
    assert!(process_next(&db).unwrap());
    let task = db.next_cover().unwrap().unwrap();
    let source = db
        .cover_source(task.file_id, task.revision)
        .unwrap()
        .unwrap();
    assert_eq!(
        cover::read_range(&source, 0, 8).unwrap(),
        fs::read(&path).unwrap()[..8]
    );
    assert!(cover::read_range(&source, task.length - 1, 2).is_err());
    assert!(cover::save_png(sandbox.path(), &source, b"not PNG").is_err());
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut encoded, 2, 2);
        encoder.set_color(png::ColorType::Rgba);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255; 16]).unwrap();
    }
    let cached = cover::save_png(sandbox.path(), &source, &encoded).unwrap();
    assert!(db.complete_cover(&source, Some(cached), None).unwrap());
    assert_eq!(
        db.cover_revision(task.file_id).unwrap(),
        Some(task.revision)
    );
    assert_eq!(
        cover::load_png(sandbox.path(), task.file_id, task.revision).unwrap(),
        encoded
    );
    db.requeue_cover(task.file_id, task.revision).unwrap();
    assert!(db.cover_revision(task.file_id).unwrap().is_none());
}
