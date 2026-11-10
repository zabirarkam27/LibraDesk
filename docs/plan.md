# LibraDesk implementation roadmap

Use this checklist to track task status. The [detailed plan](../plan.md) defines scope, acceptance, and approval rules. Run tasks in the listed order unless the owner approves a change. Dependencies below list the main technical prerequisites in addition to the approval gates. A task is complete when its exit check is supported by evidence in the [development log](development-log.md).

## Phase 0 · Repository inspection and planning

| Task | Depends on | Exit check |
| --- | --- | --- |
| [x] 0.1 Repository inspection | — | Inspect the root, source, configuration, tests, README, and Git. Record gaps and conflicts. |
| [x] 0.2 Development environment audit | 0.1 | Record Windows and toolchain versions. Confirm WebView2, MSVC and SDK, and build the scaffold. |
| [x] 0.3 Architecture documentation | 0.2 | Review process boundaries, storage, PDF transport, search, and security decisions against the plan. |
| [x] 0.4 Database documentation | 0.3 | Review entities, keys, constraints, migrations and recovery. Probe Bengali FTS5 and snapshot support. |
| [x] 0.5 Implementation roadmap | 0.1–0.4 | Cross-check all task IDs, dependencies and exit checks. Publish the checklist and dated log. |

## Phase 1 · Desktop foundation

| Task | Depends on | Exit check |
| --- | --- | --- |
| [ ] 1.1 Initialize Tauri 2 | 0.5 | Verify the scaffold's identifier and window settings, launch the Windows app, and exercise a real native command. |
| [ ] 1.2 Frontend architecture | 1.1 | Build strict TypeScript, open each route through the app entry, and verify error handling and import boundaries. |
| [ ] 1.3 UI design system | 1.2 | Check representative shared controls for focus, disabled and loading states, keyboard operation, and desktop widths. |
| [ ] 1.4 Application layout | 1.2, 1.3 | Navigate every shell route and resize the window. Verify sidebar, header, and search placeholder. |
| [ ] 1.5 Theme system | 1.3, 1.4 | Switch light, dark, and system modes. Restart to verify persistence and reader isolation. |
| [ ] 1.6 AppData initialization | 1.1 | Verify first and repeated startup, all six local directories, and a denied path. |
| [ ] 1.7 SQLite initialization | 1.6 | Create and reopen a database, verify FK and FTS5 support, and test a failed migration rollback and integrity checks. |

## Phase 2 · PDF scanner and importer

| Task | Depends on | Exit check |
| --- | --- | --- |
| [ ] 2.1 Native folder picker | 1.4, 1.7 | Register multiple roots, cancel the picker, and disable or remove a root without losing its books. |
| [ ] 2.2 Recursive PDF discovery | 2.1 | Discover nested and Bengali paths, handle denied locations, and skip reparse points and traversal attempts. |
| [ ] 2.3 PDF validation | 2.2 | Classify valid, unreadable, missing, and renamed non-PDF fixtures. Record size and modification time. |
| [ ] 2.4 Scan jobs | 2.3, 1.7 | Pause, resume, and cancel under load. Restart mid-scan and verify checkpoints and duplicate job prevention. |
| [ ] 2.5 Import preview | 2.4 | Reconcile preview counts and byte totals with fixtures. Verify cancelling creates zero books. |
| [ ] 2.6 Database import | 2.5 | Confirm twice and rescan overlapping roots. Inject a batch failure and verify safe recovery and unchanged PDF bytes. |
| [ ] 2.7 Scan history | 2.6 | Compare persisted counts, root, times, and terminal statuses after restart. |

## Phase 3 · Metadata, hashing, and thumbnails

| Task | Depends on | Exit check |
| --- | --- | --- |
| [ ] 3.1 PDF metadata extraction | 2.6 | Extract the specified fields from real fixtures and preserve raw, missing, malformed, and encrypted metadata cases. |
| [ ] 3.2 Filename fallback | 3.1 | Verify empty, punctuated, and Bengali filenames produce sensible provisional titles. |
| [ ] 3.3 SHA-256 hashing | 2.6 | Compare streamed hashes with known digests. Measure memory for large PDFs and check unchanged files skip rehashing. |
| [ ] 3.4 Duplicate detection | 3.3 | Group matching digests and keep separate records and paths. Verify review state persists. |
| [ ] 3.5 Cover thumbnails | 3.1, 1.6 | Verify bundled PDF.js rendering and range transport in WebView2, cover caching, revision invalidation, and corrupt-file fallback. |
| [ ] 3.6 Metadata review | 3.1, 3.2 | Test save, cancel, and bulk edit. Rescan and confirm manual fields survive. |

## Phase 4 · Library management

| Task | Depends on | Exit check |
| --- | --- | --- |
| [ ] 4.1 Book repositories and services | 1.7, 2.6, 3.6 | Verify typed CRUD, transactions, and repository errors with fixture data. |
| [ ] 4.2 All Books | 1.4, 3.5, 4.1 | Test grid, list, compact, loading, empty, and error states. Verify bounded rendering with a large library. |
| [ ] 4.3 Book details | 4.1, 4.2 | Open real records, check file and metadata fields, save and reload edits, and handle missing records. |
| [ ] 4.4 Categories | 4.1 | Verify nested CRUD, cycle prevention, multiple assignments, filtering, and book retention after category removal. |
| [ ] 4.5 Tags | 4.1 | Verify tag CRUD, multiple assignments, and filtering. |
| [ ] 4.6 Favorites and reading status | 4.1, 4.3 | Toggle favorites and each status, then reopen the app to verify persistence. |
| [ ] 4.7 Collections | 4.1 | Create collections and reading lists. Verify membership, manual or configured sorting, and book retention after removal. |
| [ ] 4.8 Sorting and filtering | 4.2, 4.4–4.7 | Combine filters and compare stable ordering across pages, including missing values. Verify progress ordering again in 5.6. |
| [ ] 4.9 Bilingual metadata search | 4.1, 4.8 | Exercise English, Bengali, mixed queries, combining marks, punctuation, partial matches, ranking, and filters. Verify updates and deletions reach the index. |
| [ ] 4.10 Dashboard | 4.1, 4.4, 4.6 | Reconcile displayed statistics against persisted fixtures and empty states. Recheck recently read data in 5.6. |

## Phase 5 · Built-in PDF reader

| Task | Depends on | Exit check |
| --- | --- | --- |
| [ ] 5.1 PDF.js integration | 3.5, 4.3 | Open a real local PDF in the packaged app offline. Verify worker lifecycle, errors, and cleanup. |
| [ ] 5.2 Reader layout | 5.1 | Check canvas, toolbar, sidebars, and resizing at supported desktop dimensions. |
| [ ] 5.3 PDF navigation | 5.2 | Verify page limits, direct navigation, zoom, fit modes, and fullscreen. |
| [ ] 5.4 Contents and thumbnails | 5.2, 5.3 | Navigate nested outline entries and confirm visible thumbnails load lazily. |
| [ ] 5.5 Reader text search | 5.1, 5.3 | Find, navigate, and highlight hits across pages, including mixed-language text and zero matches. |
| [ ] 5.6 Reading progress | 4.6, 5.3 | Close and restart to verify resume and debounced persistence. Recheck dashboard and sorting. Opening the last page preserves manual reading status. |
| [ ] 5.7 Bookmarks and notes | 5.3, 5.6 | Create, edit, delete, and navigate bookmarks and personal notes. Verify persistence after restart. |
| [ ] 5.8 Reader preferences | 1.5, 5.3 | Persist reader theme, zoom, layout, and shortcuts separately. Check keyboard conflicts. |
| [ ] 5.9 Reader performance | 5.1–5.8 | Measure large-document load, rapid paging, book switching, memory cleanup, and shutdown. |

## Phase 6 · Advanced search and file recovery

| Task | Depends on | Exit check |
| --- | --- | --- |
| [ ] 6.1 PDF text extraction | 2.4, 5.1 | Index bounded page batches, simulate interruption, and resume without duplicate text or blocking the UI. |
| [ ] 6.2 Full-text search index | 4.9, 6.1 | Verify page-aware hits, snippets, navigation, index maintenance, and stale revision exclusion. |
| [ ] 6.3 Missing file detection | 4.1, 2.3 | Disconnect a source drive and verify responsive startup, background detection, and distinction from denied access. |
| [ ] 6.4 File relinking | 3.3, 5.7, 6.3 | Validate and compare replacements, test cancel and confirm, and retain notes, bookmarks, and progress. |
| [ ] 6.5 Changed file detection | 3.3, 3.6, 6.3 | Simulate size and timestamp changes, rehash when needed, and preserve manual metadata. |
| [ ] 6.6 Duplicate management | 3.4, 4.1 | Verify review, keep, ignore, and database-only removal. Confirm source bytes remain unchanged. |

## Phase 7 · Settings, analytics, and backup

| Task | Depends on | Exit check |
| --- | --- | --- |
| [ ] 7.1 Settings | 1.5, 5.8 | Open each settings area and verify applicable preferences persist. |
| [ ] 7.2 Storage management | 1.6, 3.5, 7.1 | Reconcile storage totals and clear only validated cache entries. Verify records remain intact. |
| [ ] 7.3 Backup creation | 1.7, 7.1 | Validate a live database snapshot and versioned manifest, with and without thumbnails. Verify PDFs stay outside the default backup. |
| [ ] 7.4 Backup restore | 6.2, 7.3 | Test corrupt and incompatible backups, cancellation, failed promotion, rollback, and successful restore with derived index rebuild. |
| [ ] 7.5 Reading analytics | 4.6, 5.6, 7.1 | Compare completion, reading activity, page counts, category totals, and history with persisted fixtures. |

## Phase 8 · Quality assurance and release

| Task | Depends on | Exit check |
| --- | --- | --- |
| [ ] 8.1 Rust backend testing | 2.7, 6.6, 7.4 | Pass scanner, path, parsing, hash, duplicate, database, and recovery tests on Windows. |
| [ ] 8.2 Frontend testing | 4.10, 5.9, 7.5 | Pass routing, search, filter, form, reader, annotation, and settings tests. |
| [ ] 8.3 Integration testing | 8.1, 8.2 | Exercise interrupted import through restore, relink, and reader use on a clean profile. |
| [ ] 8.4 Large collection performance | 8.3 | Record startup, search, scroll, import, memory, database, and thumbnail metrics at 100, 1,000, 5,000, and 10,000+ records. Agree on budgets before testing. |
| [ ] 8.5 Security and data safety | 8.3 | Audit capabilities, CSP, PDF scripting, paths, input, SQL, backups, and destructive actions. Verify source PDFs stay intact. |
| [ ] 8.6 Windows release build | 8.4, 8.5 | Verify version, icon, installer, and offline launch. Install and uninstall on a clean profile and check data preservation. |
| [ ] 8.7 Final documentation | 8.6 | Review setup, guide, recovery, limitations, and release report against the final acceptance checklist. Obtain release approval. |
