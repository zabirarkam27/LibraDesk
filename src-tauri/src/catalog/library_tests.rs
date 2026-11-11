use rusqlite::params;
use unicode_normalization::UnicodeNormalization;

use super::{
    organization::{GroupInput, GroupKind, MemberAction},
    repository::{BookInput, ReadingStatus},
    search::{CatalogQuery, CatalogSort, Direction},
};
use crate::database::{Database, Error};

fn input(title: &str) -> BookInput {
    BookInput {
        title: title.into(),
        ..BookInput::default()
    }
}
fn group(db: &Database, kind: GroupKind, name: &str, parent: Option<i64>) -> i64 {
    db.save_group(
        kind,
        &GroupInput {
            name: name.into(),
            parent_id: parent,
            ..GroupInput::default()
        },
    )
    .unwrap()
}

#[test]
fn organizes_books_transactionally_and_rejects_category_cycles() {
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("library.db");
    let db = Database::open(&path).unwrap();
    let root = group(&db, GroupKind::Category, "বই", None);
    let child = group(&db, GroupKind::Category, "Science", Some(root));
    let tag = group(&db, GroupKind::Tag, "বাংলা", None);
    let collection = group(&db, GroupKind::Collection, "Reading list", None);
    assert!(matches!(
        db.save_group(
            GroupKind::Category,
            &GroupInput {
                id: Some(root),
                revision: Some(1),
                name: "বই".into(),
                parent_id: Some(child),
                ..GroupInput::default()
            }
        ),
        Err(Error::OrganizationConflict)
    ));
    assert!(matches!(
        db.delete_group(GroupKind::Category, root, 1),
        Err(Error::CategoryHasChildren)
    ));
    assert!(matches!(
        db.save_group(
            GroupKind::Tag,
            &GroupInput {
                name: "বাংলা".into(),
                ..GroupInput::default()
            }
        ),
        Err(Error::OrganizationConflict)
    ));

    let book = db
        .create_book(&BookInput {
            category_ids: Some(vec![root, child]),
            tag_ids: Some(vec![tag]),
            collection_ids: Some(vec![collection]),
            author_names: Some(vec!["প্রথম লেখক".into(), "Second author".into()]),
            is_favorite: Some(true),
            reading_status: Some(ReadingStatus::Reading),
            language: Some("bn".into()),
            publication_year: Some(2026),
            ..input("Science বাংলা")
        })
        .unwrap();
    let filtered = CatalogQuery {
        category_id: Some(root),
        tag_id: Some(tag),
        collection_id: Some(collection),
        favorites_only: true,
        reading_status: Some(ReadingStatus::Reading),
        language: Some("bn".into()),
        ..CatalogQuery::default()
    };
    let dashboard = serde_json::to_value(db.dashboard().unwrap()).unwrap();
    assert_eq!(dashboard["categories"][0]["count"], 1);
    assert_eq!(dashboard["favorites"], 1);
    assert_eq!(db.query_catalog(&filtered).unwrap().books[0].id, book.id);
    assert!(db
        .update_book(
            book.id,
            book.revision,
            &BookInput {
                category_ids: Some(vec![root]),
                tag_ids: Some(vec![tag + 99]),
                ..input("Must roll back")
            }
        )
        .is_err());
    assert_eq!(
        db.get_book(book.id).unwrap().category_ids,
        vec![root, child]
    );
    assert_eq!(db.get_book(book.id).unwrap().title, "Science বাংলা");
    db.set_book_flags(
        book.id,
        book.revision,
        Some(false),
        Some(ReadingStatus::OnHold),
    )
    .unwrap();
    assert!(matches!(
        db.set_book_flags(book.id, book.revision, Some(true), None),
        Err(Error::CatalogConflict)
    ));
    drop(db);
    let db = Database::open(&path).unwrap();
    let reopened = db.get_book(book.id).unwrap();
    assert!(!reopened.is_favorite);
    assert_eq!(reopened.reading_status, "on_hold");
    assert_eq!(reopened.authors.len(), 2);
    db.set_book_flags(
        book.id,
        reopened.revision,
        None,
        Some(ReadingStatus::Completed),
    )
    .unwrap();
    assert_eq!(db.get_book(book.id).unwrap().reading_status, "completed");
    db.delete_group(GroupKind::Tag, tag, 1).unwrap();
    db.delete_group(GroupKind::Category, child, 1).unwrap();
    let current = db.organization().unwrap().collections.remove(0);
    db.delete_group(GroupKind::Collection, collection, current.revision)
        .unwrap();
    assert!(db.get_book(book.id).unwrap().tag_ids.is_empty());
    assert_eq!(db.get_book(book.id).unwrap().category_ids, vec![root]);
    assert!(db.get_book(book.id).unwrap().collection_ids.is_empty());
}

#[test]
fn bilingual_search_preserves_text_and_tracks_metadata_tags_and_deletes() {
    let fixture = tempfile::tempdir().unwrap();
    let db = Database::open(&fixture.path().join("search.db")).unwrap();
    let title = "বাংলা বইয়ের নাম — English CAFÉ যুক্তাক্ষর। কাল";
    let tag = group(&db, GroupKind::Tag, "গ্রন্থাগার", None);
    let book = db
        .create_book(&BookInput {
            tag_ids: Some(vec![tag]),
            author_names: Some(vec!["বাঙালি লেখক".into()]),
            ..input(title)
        })
        .unwrap();
    for term in [
        "বাংলা বই",
        "ENGLISH",
        "বাঙালি লেখক",
        "English বাংলা",
        "যুক্তাক্ষর",
        "বইয়ের",
        "গ্রন্থাগার",
        "ন্থাগা",
        "cafe\u{301}",
        &title.nfd().collect::<String>(),
    ] {
        let result = db
            .query_catalog(&CatalogQuery {
                text: term.into(),
                ..CatalogQuery::default()
            })
            .unwrap();
        assert_eq!(result.total, 1, "query {term}");
    }
    assert_eq!(
        db.query_catalog(&CatalogQuery {
            text: "কল".into(),
            ..CatalogQuery::default()
        })
        .unwrap()
        .total,
        0
    );
    for term in ["\" OR *", "NEAR()", "%_[]", "।", "\u{200d}", "🔖"] {
        assert!(
            db.query_catalog(&CatalogQuery {
                text: term.into(),
                ..CatalogQuery::default()
            })
            .is_ok(),
            "{term}"
        );
    }
    assert_eq!(db.get_book(book.id).unwrap().title, title);
    db.save_group(
        GroupKind::Tag,
        &GroupInput {
            id: Some(tag),
            revision: Some(1),
            name: "নতুনট্যাগ".into(),
            ..GroupInput::default()
        },
    )
    .unwrap();
    assert_eq!(
        db.query_catalog(&CatalogQuery {
            text: "নতুনট্যাগ".into(),
            ..CatalogQuery::default()
        })
        .unwrap()
        .total,
        1
    );
    assert_eq!(
        db.query_catalog(&CatalogQuery {
            text: "গ্রন্থাগার".into(),
            ..CatalogQuery::default()
        })
        .unwrap()
        .total,
        0
    );
    let revision = db.get_book(book.id).unwrap().revision;
    db.update_book(book.id, revision, &input("Replacement"))
        .unwrap();
    assert_eq!(
        db.query_catalog(&CatalogQuery {
            text: "বাংলা বই".into(),
            ..CatalogQuery::default()
        })
        .unwrap()
        .total,
        0
    );
    let revision = db.get_book(book.id).unwrap().revision;
    db.delete_book(book.id, revision).unwrap();
    assert_eq!(
        db.query_catalog(&CatalogQuery {
            text: "Replacement".into(),
            ..CatalogQuery::default()
        })
        .unwrap()
        .total,
        0
    );
    db.0.execute(
        "INSERT INTO book_search_fts(book_search_fts,rank) VALUES ('integrity-check',1)",
        [],
    )
    .unwrap();
}

#[test]
fn collection_order_sorting_pagination_and_dashboard_use_saved_data() {
    let fixture = tempfile::tempdir().unwrap();
    let db = Database::open(&fixture.path().join("catalog.db")).unwrap();
    let collection = group(&db, GroupKind::Collection, "Queue", None);
    let a = db
        .create_book(&BookInput {
            publication_year: Some(1990),
            ..input("Zulu")
        })
        .unwrap();
    let b = db
        .create_book(&BookInput {
            publication_year: Some(2020),
            ..input("Alpha")
        })
        .unwrap();
    db.set_collection_books(collection, 1, &[a.id, b.id])
        .unwrap();
    let mut query = CatalogQuery {
        collection_id: Some(collection),
        sort: CatalogSort::Collection,
        direction: Direction::Asc,
        ..CatalogQuery::default()
    };
    let result = db.query_catalog(&query).unwrap();
    assert_eq!(result.books[0].id, a.id);
    db.edit_collection_member(collection, 2, b.id, MemberAction::Up)
        .unwrap();
    assert_eq!(db.query_catalog(&query).unwrap().books[0].id, b.id);
    assert!(matches!(
        db.set_collection_books(collection, 2, &[a.id]),
        Err(Error::CatalogConflict)
    ));
    assert!(db
        .set_collection_books(collection, 3, &[a.id, 99999])
        .is_err());
    assert_eq!(db.query_catalog(&query).unwrap().total, 2);
    db.edit_collection_member(collection, 3, b.id, MemberAction::Remove)
        .unwrap();
    assert!(db.get_book(b.id).is_ok());
    assert_eq!(db.query_catalog(&query).unwrap().total, 1);
    query.collection_id = None;
    query.sort = CatalogSort::Year;
    assert_eq!(db.query_catalog(&query).unwrap().books[0].id, a.id);
    query.sort = CatalogSort::Title;
    assert_eq!(db.query_catalog(&query).unwrap().books[0].id, b.id);
    for sort in [
        CatalogSort::Title,
        CatalogSort::Author,
        CatalogSort::Added,
        CatalogSort::Size,
        CatalogSort::Year,
        CatalogSort::Progress,
        CatalogSort::Relevance,
    ] {
        query.sort = sort;
        assert_eq!(db.query_catalog(&query).unwrap().total, 2);
    }
    for index in 0..55 {
        db.create_book(&input(&format!("Book {index}"))).unwrap();
    }
    let first = db.query_catalog(&CatalogQuery::default()).unwrap();
    let second = db
        .query_catalog(&CatalogQuery {
            page: 1,
            ..CatalogQuery::default()
        })
        .unwrap();
    assert_eq!(first.books.len(), 50);
    assert_eq!(second.books.len(), 7);
    assert!(first
        .books
        .iter()
        .all(|a| second.books.iter().all(|b| a.id != b.id)));
    db.0.execute(
        "INSERT INTO reading_progress(book_id,file_revision,page_number,total_pages,last_read_at) VALUES (?1,1,20,100,99)",
        [b.id],
    ).unwrap();
    let progress = db
        .query_catalog(&CatalogQuery {
            sort: CatalogSort::Progress,
            ..CatalogQuery::default()
        })
        .unwrap();
    assert_eq!(progress.books[0].id, b.id);
    let current = db.get_book(a.id).unwrap();
    db.set_book_flags(
        a.id,
        current.revision,
        Some(true),
        Some(ReadingStatus::Reading),
    )
    .unwrap();
    let dashboard = serde_json::to_value(db.dashboard().unwrap()).unwrap();
    assert_eq!(dashboard["totalBooks"], 57);
    assert_eq!(dashboard["favorites"], 1);
    assert_eq!(dashboard["reading"], 1);
    assert_eq!(dashboard["recentlyRead"][0]["id"], b.id);
    assert_eq!(dashboard["totalStorage"], 0);
    assert_eq!(dashboard["totalPages"], 0);
    let stale = db
        .query_catalog(&CatalogQuery {
            page: 999,
            ..CatalogQuery::default()
        })
        .unwrap();
    assert_eq!(stale.page, 1);
    let fk_count: i64 =
        db.0.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(fk_count, 0);
    let title_key: String =
        db.0.query_row(
            "SELECT title_sort_key FROM books WHERE id=?1",
            params![a.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(title_key, "zulu");
}

#[test]
fn queries_a_ten_thousand_book_library_with_bounded_results_and_weighted_search() {
    let fixture = tempfile::tempdir().unwrap();
    let db = Database::open(&fixture.path().join("large.db")).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    for index in 0..10_000 {
        super::repository::insert_book(&tx, &format!("বাংলা Library book {index}")).unwrap();
    }
    tx.commit().unwrap();
    let last = db
        .query_catalog(&CatalogQuery {
            page: 199,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(last.total, 10_000);
    assert_eq!(last.books.len(), 50);
    let partial = db
        .query_catalog(&CatalogQuery {
            text: "brar".into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(partial.total, 10_000);
    assert_eq!(partial.books.len(), 50);
    let title = db.create_book(&input("UniqueRankingWord")).unwrap();
    db.create_book(&BookInput {
        subject: Some("UniqueRankingWord".into()),
        ..input("Unrelated title")
    })
    .unwrap();
    let ranked = db
        .query_catalog(&CatalogQuery {
            text: "UniqueRankingWord".into(),
            sort: CatalogSort::Relevance,
            direction: Direction::Asc,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(ranked.books[0].id, title.id);
}
