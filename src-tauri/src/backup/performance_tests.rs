use crate::{catalog::search::CatalogQuery, database::Database, scan};
use std::{fs, time::Instant};
#[test]
#[ignore = "Explicit performance run writes a machine-specific report under target"]
fn measure_collection_sizes() {
    let mut reports = Vec::new();
    for count in [100, 1000, 5000, 10001] {
        let temp = tempfile::Builder::new()
            .prefix("performance-profile-")
            .tempdir_in(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target"))
            .unwrap();
        let sources = temp.path().join("sources");
        fs::create_dir(&sources).unwrap();
        let template = temp.path().join("template.pdf");
        super::integration_tests::pdf(&template);
        let pdf_bytes = fs::read(template).unwrap();
        for index in 0..count {
            fs::write(sources.join(format!("বই-{index}.pdf")), &pdf_bytes).unwrap();
        }
        let root = temp.path().join("AppData");
        let path = crate::storage::initialize(&root).unwrap();
        let db = Database::open(&path).unwrap();
        db.register_roots(&[scan::files::root_key(&sources)
            .unwrap()
            .to_string_lossy()
            .into_owned()])
            .unwrap();
        let root_id =
            db.0.query_row("SELECT id FROM scan_roots", [], |r| r.get::<_, i64>(0))
                .unwrap();
        let start = Instant::now();
        db.queue_scan(root_id).unwrap();
        while let Some(work) = db.next_work().unwrap() {
            scan::step(&db, &work, || {}).unwrap();
        }
        let discovery_ms = start.elapsed().as_secs_f64() * 1000.;
        let job =
            db.0.query_row("SELECT id FROM scan_jobs", [], |r| r.get::<_, i64>(0))
                .unwrap();
        let start = Instant::now();
        db.control_job(job, scan::Action::Confirm).unwrap();
        while let Some(work) = db.next_work().unwrap() {
            scan::step(&db, &work, || {}).unwrap();
        }
        let import_ms = start.elapsed().as_secs_f64() * 1000.;
        drop(db);
        let start = Instant::now();
        let db = Database::open(&path).unwrap();
        let catalog = db.query_catalog(&CatalogQuery::default()).unwrap();
        assert_eq!(catalog.total, count);
        let initial_ms = start.elapsed().as_secs_f64() * 1000.;
        let mut searches = Vec::new();
        for text in ["বই", "বই-99", "' OR 1=1 --"] {
            let query = CatalogQuery {
                text: text.into(),
                ..Default::default()
            };
            db.query_catalog(&query).unwrap();
            let mut times = Vec::new();
            for _ in 0..10 {
                let start = Instant::now();
                let result = db.query_catalog(&query).unwrap();
                assert!(result.books.len() <= 50);
                times.push(start.elapsed().as_secs_f64() * 1000.);
            }
            times.sort_by(f64::total_cmp);
            searches.push(serde_json::json!({"query":text,"p50Ms":times[5],"maxMs":times[9]}));
        }
        db.0.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
        let usage = crate::storage_management::usage(&root).unwrap();
        reports.push(serde_json::json!({"records":count,"discoveryMs":discovery_ms,"importMs":import_ms,"openAndFirstQueryMs":initial_ms,"searches":searches,"databaseBytes":usage.database,"thumbnailBytes":usage.thumbnails,"fixture":"Generated valid single-page PDFs; metadata/reader benchmarks are separate"}));
        if count == 10001 {
            // Idle catalog benchmark: asynchronous document processing is deliberately excluded.
            db.0.execute_batch("DELETE FROM processing_jobs;DELETE FROM text_extraction_jobs;")
                .unwrap();
            drop(db);
            let kept = temp.keep();
            fs::write(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/qa-profile-path.txt"),
                kept.join("AppData").to_string_lossy().as_bytes(),
            )
            .unwrap();
        }
    }
    let out =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/performance-backend.json");
    fs::write(&out, serde_json::to_vec_pretty(&reports).unwrap()).unwrap();
    println!("Performance report: {}", out.display());
    println!("{}", serde_json::to_string_pretty(&reports).unwrap());
}
