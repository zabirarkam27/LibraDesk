# Testing strategy

Validated on Windows 11 Pro 10.0.26200, AMD Ryzen 5 8500G, approximately 31.2 GiB RAM, on 2026-11-14. Tests create generated fixtures; personal documents, databases and backups are excluded from Git.

## Repeatable checks

Run from the repository root:

~~~powershell
npm ci
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run test:backend
python scripts/make-reader-fixtures.py
npm run test:browser
~~~

The PDF generator requires Python with reportlab and Pillow. It uses the Windows Nirmala font for Bengali. Chrome must be installed at its standard Windows path. Browser runners use Node's built-in CDP/WebSocket support, bind fixture servers to loopback, create separate temporary profiles under target, and remove only their checked profile paths. No browser automation dependency is required. Large generated PDFs remain under ignored target.

The ten browser suites cover catalog routing, grid/list/compact virtualization, CRUD/forms, organization, filters, Ctrl+K, real PDF.js rendering, navigation, search/highlights, bookmarks/notes, reader preferences, close persistence, extraction resume, full-text snippets, recovery, duplicates, all settings areas, storage, backup, restore and analytics. Historical phase numbers identify their feature contracts. IPC is mocked in these browser suites; PDF parsing/rendering is real. They do not establish native filesystem correctness.

Rust tests exercise real SQLite migrations/FTS, transactions, files, hashes, source handles, extraction leases, paths, duplicates, recovery, archive validation and rollback. A cross-feature integration test runs interrupted discovery/reopen/confirmation/import, metadata, edits, reader state/note, backup, restore, relink and reopen, and compares original PDF bytes. SQL/FTS metacharacters and real Windows junctions have regression coverage.

## Performance and native IPC

~~~powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml measure_collection_sizes -- --ignored --nocapture
node tests/browser/performance-ui-check.mjs
cargo build --locked --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol
node scripts/test-native.mjs
~~~

The explicit benchmark is normally ignored to keep routine tests short. It measures real discovery/import and SQLite queries at 100, 1,000, 5,000 and 10,001 valid generated single-page PDFs. Its retained 10,001-record profile path is written to target/qa-profile-path.txt. Parsing queues are removed for the idle catalog native benchmark. The debug-only LIBRADESK_QA_DATA_DIR override centralizes database, settings, covers, storage and backup in that profile. It is compiled out of release builds.

The native runner opens an embedded-frontend debug build, checks actual IPC counts and backup, times startup and ten warm searches, and samples the total Windows working set of its process and descendant WebView2 processes. It asserts the agreed 3,000 ms / 500 ms / 500 MiB budgets. Remote debugging is enabled only through that launched process's QA environment; the normal application has no hardcoded debugging port. Close other LibraDesk instances before native QA because single-instance behavior otherwise redirects to the existing window.

Browser grid measurements are a separate fixture-IPC lab: first visible rows, 60 scroll frames, bounded rendered items and JavaScript heap. Heap is not total application memory. Large-document tests separately open 2,000 pages and a source over 300 MiB using bounded ranges.

## Installer

~~~powershell
npm run tauri build -- --bundles nsis -- --locked
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/test-installer.ps1
~~~

Installer QA reads the current version from Tauri configuration, refuses an existing registered installation, uses a unique workspace directory, validates version and all executable bytes after the expected three-byte Tauri NSIS bundle marker, silently uninstalls, and checks PDF bytes, a user-data subfolder and registry cleanup. It never recursively deletes the data folder. Version 0.1.1 bundles the full Microsoft standalone runtime; payload inspection and signature checks complement installation testing on the runtime-equipped test machine. No runtime is uninstalled or registry keys hidden to simulate an absent prerequisite.

Installed release startup smoke is a separate opt-in: -IncludeReleaseSmoke. That option launches the real release using the account's actual AppData library. Obtain the owner's authorization before using it on their account. It can run normal metadata/file checks. It does not restore or delete library data. The isolated native test above is the default alternative.

Raw reports are written under ignored target. Reviewed, nonpersonal snapshots and interpretation are in the performance and release reports. Tests on one Windows account do not prove behavior on every drive, permission setup, Windows version or fresh OS installation.
