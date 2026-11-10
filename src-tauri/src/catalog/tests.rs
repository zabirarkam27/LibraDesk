use std::fs;

use rusqlite::{Transaction, TransactionBehavior};

use crate::{
    database::{Database, Error},
    scan::files,
};

use super::repository::{insert_book, insert_file, BookInput};

#[test]
fn creates_updates_and_removes_books_without_touching_source_files() {
    let fixture = tempfile::tempdir().unwrap();
    let database = fixture.path().join("library.db");
    let db = Database::open(&database).unwrap();
    let file = fixture.path().join("book.pdf");
    let original = b"%PDF-1.7\n";
    fs::write(&file, original).unwrap();
    let stamp = files::stamp(&fs::File::open(&file).unwrap()).unwrap();
    let path = file.to_str().unwrap();

    let first = db.create_author("  লেখক  ").unwrap();
    let second = db.create_author("লেখক").unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(db.list_authors(Some(first.id)).unwrap()[0].id, second.id);
    let created = db
        .create_book(&BookInput {
            title: " বই ".into(),
            subject: Some("  বিষয় ".into()),
            publisher: Some("Press".into()),
            isbn: Some("9780000000002".into()),
            publication_year: Some(2026),
            language: Some("bn".into()),
            author_ids: Some(vec![second.id, first.id]),
        })
        .unwrap();
    assert_eq!(created.title, "বই");
    assert_eq!(created.author.as_deref(), Some("লেখক, লেখক"));
    assert_eq!(created.authors[0].id, second.id);
    assert_eq!(created.authors[1].id, first.id);
    assert_eq!(db.list_books(None, true).unwrap().len(), 1);
    assert!(db.list_books(None, false).unwrap().is_empty());
    assert!(db.list_books(Some(created.id), true).unwrap().is_empty());

    let tx = Transaction::new_unchecked(&db.0, TransactionBehavior::Immediate).unwrap();
    let file_id = insert_file(&tx, created.id, path, &stamp).unwrap();
    tx.commit().unwrap();
    drop(db);
    let db = Database::open(&database).unwrap();
    let book = db.get_book(created.id).unwrap();
    assert_eq!(book.subject.as_deref(), Some("বিষয়"));
    assert_eq!(book.publisher.as_deref(), Some("Press"));
    assert_eq!(book.isbn.as_deref(), Some("9780000000002"));
    assert_eq!(book.publication_year, Some(2026));
    assert_eq!(book.language.as_deref(), Some("bn"));
    assert_eq!(book.files[0].id, file_id);
    assert_eq!(db.get_book_file(file_id).unwrap().id, file_id);

    let input = BookInput {
        title: "New title".into(),
        ..BookInput::default()
    };
    let updated = db.update_book(book.id, book.revision, &input).unwrap();
    assert_eq!(updated.title, "New title");
    assert_eq!(updated.revision, book.revision + 1);
    assert_eq!(updated.authors[0].id, second.id);
    assert!(matches!(
        db.update_book(book.id, book.revision, &input),
        Err(Error::CatalogConflict)
    ));
    assert!(matches!(
        db.delete_book(book.id, book.revision),
        Err(Error::CatalogConflict)
    ));

    let cleared = db
        .update_book(
            book.id,
            updated.revision,
            &BookInput {
                author_ids: Some(Vec::new()),
                ..input
            },
        )
        .unwrap();
    assert!(cleared.authors.is_empty());
    assert_eq!(cleared.author, None);
    db.delete_book(book.id, cleared.revision).unwrap();
    assert!(matches!(db.get_book(book.id), Err(Error::CatalogNotFound)));
    assert!(matches!(
        db.get_book_file(file_id),
        Err(Error::CatalogNotFound)
    ));
    assert_eq!(db.list_authors(None).unwrap().len(), 2);
    assert_eq!(fs::read(&file).unwrap(), original);

    let tx = Transaction::new_unchecked(&db.0, TransactionBehavior::Immediate).unwrap();
    let next_book = insert_book(&tx, "Imported again").unwrap();
    let next_file = insert_file(&tx, next_book, path, &stamp).unwrap();
    tx.commit().unwrap();
    assert!(next_book > book.id);
    assert!(next_file > file_id);
}

#[test]
fn rejects_invalid_metadata_and_rolls_back_failed_author_assignments() {
    let fixture = tempfile::tempdir().unwrap();
    let db = Database::open(&fixture.path().join("library.db")).unwrap();
    let author = db.create_author("Author").unwrap();
    let mut input = BookInput {
        title: "  ".into(),
        ..BookInput::default()
    };
    assert!(matches!(
        db.create_book(&input),
        Err(Error::InvalidCatalogInput)
    ));
    input.title = "Original".into();
    input.author_ids = Some(vec![author.id, author.id]);
    assert!(matches!(
        db.create_book(&input),
        Err(Error::InvalidCatalogInput)
    ));
    input.author_ids = Some(vec![author.id, author.id + 1]);
    assert!(matches!(db.create_book(&input), Err(Error::UnknownAuthor)));
    assert!(db.list_books(None, true).unwrap().is_empty());

    input.author_ids = Some(vec![author.id]);
    let book = db.create_book(&input).unwrap();
    input.title = "Failed update".into();
    input.author_ids = Some(vec![author.id + 1]);
    assert!(matches!(
        db.update_book(book.id, book.revision, &input),
        Err(Error::UnknownAuthor)
    ));
    let unchanged = db.get_book(book.id).unwrap();
    assert_eq!(unchanged.title, "Original");
    assert_eq!(unchanged.revision, book.revision);
    assert_eq!(unchanged.authors[0].id, author.id);
    let override_title: String =
        db.0.query_row(
            "SELECT value FROM metadata_overrides WHERE book_id=?1 AND field='title'",
            [book.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(override_title, "Original");
}
