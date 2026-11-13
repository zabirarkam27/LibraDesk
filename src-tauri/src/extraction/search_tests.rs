use super::{
    search::PageQuery,
    tests::{key, page, setup},
};
use crate::database::Error;
use rusqlite::params;

fn query(text: &str) -> PageQuery {
    PageQuery {
        text: text.into(),
        page: 0,
    }
}

#[test]
fn bilingual_page_search_snippets_updates_stale_revisions_and_deletion() {
    let (_fixture, db, book, file) = setup();
    let task = db.claim_text_extraction().unwrap().unwrap();
    let long = format!(
        "{} বাংলা যুক্তাক্ষর বড় Reader Needle café end",
        "preface ".repeat(100)
    );
    assert!(db
        .commit_page_text(
            key(&task),
            3,
            &[
                page(1, &long),
                page(2, "different needle second"),
                page(3, "")
            ]
        )
        .unwrap());
    let result = db.search_page_text(&query("বাংলা rea")).unwrap();
    assert_eq!(result.total, 1);
    let hit = &result.hits[0];
    assert_eq!(
        (hit.book_id, hit.file_id, hit.revision, hit.page_number),
        (book, file, 1, 1)
    );
    assert_eq!(hit.title, "Extraction fixture");
    assert!(hit.path.ends_with("source.pdf"));
    assert!(hit.snippet.contains("বাংলা"));
    assert!(hit.snippet.chars().count() <= 601);
    assert_eq!(db.search_page_text(&query("বড়")).unwrap().total, 1);
    assert_eq!(db.search_page_text(&query("যুক্তাক্ষর")).unwrap().total, 1);
    assert_eq!(db.search_page_text(&query("cafe")).unwrap().total, 1);
    assert_eq!(db.search_page_text(&query("needle")).unwrap().total, 2);
    assert_eq!(
        db.search_page_text(&query("needle")).unwrap().hits[0].page_number,
        2
    );
    assert_eq!(result.coverage.completed_files, 1);
    assert_eq!(result.coverage.extracted_pages, 3);
    assert_eq!(db.search_page_text(&query("")).unwrap().total, 0);
    for text in ["*", "(", "\" OR \"", "needle NOT nonexistent", "\u{301}"] {
        assert_eq!(
            db.search_page_text(&query(text)).unwrap().total,
            0,
            "{text:?}"
        );
    }
    db.0.execute(
        "UPDATE page_text SET text='Replacement',normalized_text='replacement' WHERE page_number=1",
        [],
    )
    .unwrap();
    assert_eq!(db.search_page_text(&query("বাংলা")).unwrap().total, 0);
    assert_eq!(db.search_page_text(&query("replacement")).unwrap().total, 1);
    // Even a residual old derivative cannot appear as a current revision hit.
    db.0.execute("INSERT INTO page_text(file_id,file_revision,page_number,text,normalized_text,extracted_at) VALUES(?1,99,4,'staleonly','staleonly',0)",[file]).unwrap();
    assert_eq!(db.search_page_text(&query("staleonly")).unwrap().total, 0);
    db.0.execute(
        "UPDATE book_files SET availability='missing' WHERE id=?1",
        [file],
    )
    .unwrap();
    assert_eq!(db.search_page_text(&query("needle")).unwrap().total, 0);
    db.0.execute(
        "UPDATE book_files SET availability='available',revision=revision+1 WHERE id=?1",
        [file],
    )
    .unwrap();
    assert_eq!(db.search_page_text(&query("replacement")).unwrap().total, 0);
    let fresh = db.claim_text_extraction().unwrap().unwrap();
    db.commit_page_text(key(&fresh), 1, &[page(1, "current needle")])
        .unwrap();
    assert_eq!(
        db.search_page_text(&query("needle")).unwrap().hits[0].revision,
        2
    );
    let alternate_path = _fixture.path().join("alternate.pdf");
    std::fs::write(&alternate_path, b"%PDF-1.7\nalternate").unwrap();
    let alternate_path = std::fs::canonicalize(alternate_path).unwrap();
    let stamp = crate::scan::files::stamp(&std::fs::File::open(&alternate_path).unwrap()).unwrap();
    let tx = db.0.unchecked_transaction().unwrap();
    tx.execute("UPDATE book_files SET is_primary=0 WHERE id=?1", [file])
        .unwrap();
    let alternate = crate::catalog::repository::insert_file(
        &tx,
        book,
        alternate_path.to_str().unwrap(),
        &stamp,
    )
    .unwrap();
    tx.commit().unwrap();
    let task = db.claim_text_extraction().unwrap().unwrap();
    db.commit_page_text(key(&task), 1, &[page(1, "alternate needle")])
        .unwrap();
    let hit = db
        .search_page_text(&query("alternate"))
        .unwrap()
        .hits
        .remove(0);
    assert_eq!((hit.book_id, hit.file_id), (book, alternate));
    assert_ne!(hit.book_id, hit.file_id);
    {
        let tx = db.0.unchecked_transaction().unwrap();
        tx.execute(
            "UPDATE page_text SET normalized_text='rolledback' WHERE file_id=?1",
            [alternate],
        )
        .unwrap();
    }
    assert_eq!(db.search_page_text(&query("rolledback")).unwrap().total, 0);
    assert_eq!(db.search_page_text(&query("alternate")).unwrap().total, 1);
    db.0.execute("DELETE FROM books WHERE id=?1", [book])
        .unwrap();
    assert_eq!(db.search_page_text(&query("needle")).unwrap().total, 0);
    db.0.execute(
        "INSERT INTO page_text_fts(page_text_fts,rank) VALUES('integrity-check',1)",
        [],
    )
    .unwrap();
}

#[test]
fn page_search_is_bounded_stable_and_validates_untrusted_input() {
    let (_fixture, db, _, file) = setup();
    let tx = db.0.unchecked_transaction().unwrap();
    for number in 1..=10_000 {
        tx.execute("INSERT INTO page_text(file_id,file_revision,page_number,text,normalized_text,extracted_at)
   VALUES(?1,1,?2,'Needle বাংলা','needle বাংলা',0)",params![file,number]).unwrap();
    }
    tx.commit().unwrap();
    let first = db.search_page_text(&query("needle")).unwrap();
    assert_eq!(first.total, 10_000);
    assert_eq!(first.hits.len(), 50);
    assert_eq!(first.hits[0].page_number, 1);
    let second = db
        .search_page_text(&PageQuery {
            page: 1,
            ..query("needle")
        })
        .unwrap();
    assert_eq!(second.hits[0].page_number, 51);
    assert_eq!(second.hits[49].page_number, 100);
    let last = db
        .search_page_text(&PageQuery {
            page: 999,
            ..query("needle")
        })
        .unwrap();
    assert_eq!(last.page, 199);
    assert_eq!(last.hits.last().unwrap().page_number, 10_000);
    assert_eq!(
        db.search_page_text(&query("needle")).unwrap().hits[0].id,
        first.hits[0].id
    );
    for input in [
        query(&"x".repeat(1025)),
        query("bad\0query"),
        query(&"term ".repeat(33)),
        PageQuery {
            page: 1_000_001,
            ..query("needle")
        },
    ] {
        assert!(matches!(
            db.search_page_text(&input),
            Err(Error::InvalidCatalogInput)
        ));
    }
    db.0.execute(
        "INSERT INTO page_text_fts(page_text_fts) VALUES('rebuild')",
        [],
    )
    .unwrap();
    assert_eq!(db.search_page_text(&query("বাংলা")).unwrap().total, 10_000);
    db.0.execute(
        "INSERT INTO page_text_fts(page_text_fts,rank) VALUES('integrity-check',1)",
        [],
    )
    .unwrap();
}
