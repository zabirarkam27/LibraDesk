# LibraDesk development log

## 2026-11-10 · Phase 0

| Task | Status | Work and verification |
| --- | --- | --- |
| 0.1 Repository inspection | Completed | The workspace initially contained only the root plan. The inspection recorded absent source, tooling, tests and Git, plus unassigned work for the frontend scaffold and notes. The owner then requested the scaffold and public repository. Scaffold commit `276b379` passed the frontend and Windows builds and the desktop IPC smoke check. |
| 0.2 Development environment audit | Completed | [Audit](environment-audit.md) records the installed toolchain and build checks. The restricted shell prevented Tauri's optional upstream version lookups. Rust compilation completed with a sandbox cache hard-link warning. No setup blockers were found. |
| 0.3 Architecture documentation | Completed | [Architecture](architecture.md) and [decisions](decisions.md) cover the frontend, Rust IPC, local storage, PDF processing, search and security. Reviewed against the scaffold and product plan. The PDF range transport and metadata parser have implementation-time verification gates. |
| 0.4 Database documentation | Completed | [Schema](database-schema.md) documents catalog, import, recovery, reader and search records. An in-memory SQLite 3.50.4 probe passed Bengali FTS5 queries, FK cascade and backup integrity. The app's bundled SQLite and actual migrations are verified in Task 1.7. |
| 0.5 Implementation roadmap | Completed | [Roadmap](plan.md) covers all 62 tasks with dependencies and exit checks. Only the five Phase 0 tasks are marked complete. Checked task IDs, dependency ordering, document links, and status consistency. |

Phase 0 documentation commit `261fabf` contains this log.

## 2026-11-10 · Phase 1

| Task | Status | Work and verification |
| --- | --- | --- |
| 1.1 Tauri application | Completed | Ran the Vite-backed development app and the standalone Windows debug build. Settings reported a successful native database connection and schema version. |
| 1.2 Frontend architecture | Completed | Strict TypeScript and Vite built the feature modules, hash router, providers, and typed preference client. Opened each workspace route. |
| 1.3 UI design system | Completed | Integrated Tailwind, shadcn/ui, Lucide, Framer Motion, and toast feedback. Verified the disabled search field, dialog keyboard operation and focus return, dropdown and tooltip, and appearance controls. |
| 1.4 Application layout | Completed | Verified sidebar and menu navigation, the Ctrl+K placeholder, and the layout at default and 640-pixel window widths. |
| 1.5 Theme system | Completed | Changed app appearance through Light, Dark, and System. Verified an independent dark reader surface inside the light app and confirmed both settings persisted after closing and reopening. Restored the original System choices. |
| 1.6 AppData initialization | Completed | Verified all six directories under Local AppData. Rust tests covered repeat initialization, existing-file preservation, unsuitable paths, and temporary-file cleanup. |
| 1.7 SQLite initialization | Completed | Migration 0001 creates scoped settings and records its checksum and version. Rust tests passed for reopening, WAL, foreign keys, Bengali FTS5, failed migration rollback, version rejection, and integrity. Checked the live database read-only after saving preferences through the UI. |

Phase 1 commit `65312a7` contains this entry.

## 2026-11-11 · Phase 2

| Task | Status | Work and verification |
| --- | --- | --- |
| 2.1 Native folder picker | Completed | Selected two roots in one Windows dialog. Cancellation left the list empty. Automated tests covered disabling and retiring roots while preserving books and history. |
| 2.2 Recursive discovery | Completed | The Windows scan found a nested Bengali `.PDF` and skipped a junction pointing at an unselected folder. Automated tests rejected paths outside the root. |
| 2.3 PDF validation | Completed | Read-only header validation flagged a plain-text `.pdf`. Tests covered unavailable roots and an import candidate changed after preview. |
| 2.4 Scan jobs | Completed | Added persisted frontier and outcome checkpoints, throttled progress events, and pause, resume, and cancel controls. Tests covered a stale worker, duplicate start prevention, and interrupted scan and import recovery. |
| 2.5 Import preview | Completed | The Windows preview reported three paths, two possible duplicates, and one invalid file. A read-only SQLite check found zero catalog entries before confirmation. Equal file size is advisory until hashing is implemented. |
| 2.6 Database import | Completed | Confirmation added two entries, including the Bengali file. A rescan classified both as existing. Tests covered repeated confirmation, overlap, batch rollback, changed files, and source preservation. |
| 2.7 Scan history | Completed | The Windows app displayed the root, start and finish times, and a completed result of three found, two imported, and one failed. Verified the stored result after reopening. |

Seven Rust tests, Clippy, the TypeScript and Vite build, and the standalone Windows debug build passed. Migration 0002 adds the catalog and scan tables. Phase 2 commit `906cbe7` contains this entry.

## 2026-11-11 · Phase 3

| Task | Status | Work and verification |
| --- | --- | --- |
| 3.1 PDF metadata extraction | Completed | Preserved original title, author, subject, creator, producer, creation date, and page count. Verified Bengali metadata, safe fallback for a password-protected fixture, and PDF.js range extraction for a 34 MiB document. |
| 3.2 Filename fallback | Completed | Verified a Bengali filename with hyphens, underscores, whitespace, and digits. Missing authors remain unset until edited. |
| 3.3 SHA-256 hashing | Completed | Compared five persisted digests to independent SHA-256 values and measured a 128 KiB peak read buffer for a 34 MiB input. The Windows rescan refreshed only the changed fixture. |
| 3.4 Duplicate detection | Completed | Verified two copies in one exact digest group, saved a keep decision, and confirmed the decision persisted after restart. |
| 3.5 Cover thumbnails | Completed | Verified four first-page PNGs at 240 × 320 and a placeholder for a malformed PDF. Checked a new cover revision after rescan and corrected cache reads under Windows AppData redirection. |
| 3.6 Metadata review | Completed | Verified missing-title and author review, cancellation of the single-book editor, and a two-book bulk author save. Manual values survived restart and rescan, and raw fields remained separate. |

Eleven Rust tests, Clippy, the frontend build, and the standalone Windows debug build passed. SQLite integrity and foreign-key checks passed. Migration 0003 adds persistent metadata and analysis queues for existing and new PDFs. The cover renderer limits source size to 256 MiB. Larger files still receive a streamed SHA-256 digest and use the cover placeholder.

## 2026-11-11 · Phase 4

| Task | Status | Work and verification |
| --- | --- | --- |
| 4.1 Book repositories and services | Completed | Added revision-checked book CRUD, file and author repositories, metadata overrides, author relationships, and typed Tauri services. Fixture tests covered migration, persistence after reopen, ordered and same-name authors, rollback, stale revisions, file and book ID reuse prevention, and source PDF preservation. |

Fourteen Rust tests, Clippy, and the TypeScript and Vite build passed.

### Task 4.2 · All Books

The Library page opens in Grid view and also offers List and Compact views. List shows the source path. Each page contains up to 50 books, and each view mounts only visible rows and up to two extra rows on either side.

Headless Chromium verified all three views with a mocked Tauri catalog of 10,000 records. At 1140 × 760, the top and bottom of a full page mounted 7 to 18 cards across the tested modes. Checks also covered resize to 520 pixels, cursor pagination, cover and placeholder rendering, Bengali metadata, loading, empty catalog, error and retry, and browser preview. The TypeScript/Vite build and Windows debug build passed.

### Tasks 4.3–4.10 · Phase 4 completion

The owner approved all remaining Phase 4 tasks together.

| Task | Status | Work and verification |
| --- | --- | --- |
| 4.3 Book details | Completed | Added book routes, cover or placeholder, bibliographic metadata, source files, organization, and a transactional editor with stale-write detection. Browser checks saved and cancelled edits, and the Windows app opened a real record. |
| 4.4 Categories | Completed | Added nested CRUD, parent selection, cycle rejection, multiple assignments, descendant filtering, and deletion with book retention. A parent with children must have its children moved or removed first. |
| 4.5 Tags | Completed | Added CRUD, multiple assignments, filters, and transactional search refresh after rename or removal. |
| 4.6 Favorites and reading status | Completed | Added card favorite toggles and persisted Unread, Reading, Completed, and On Hold settings in the editor. Fixtures verified storage after reopen and rejection of stale revisions. |
| 4.7 Collections | Completed | Added descriptions, collection details, membership, reading lists, manual movement, and title or added-date order. Removal preserves books. |
| 4.8 Sorting and filtering | Completed | Added title, author, added date, file size, year, and saved-progress sorts with an ID tie-breaker, plus category, tag, collection, language, status, and favorite filters. The UI now requests counted pages of at most 50 books and retains row virtualization. |
| 4.9 Bilingual metadata search | Completed | Migration 0005 backfills an external-content FTS5 index and transactional projection triggers. Rust normalizes derived text with NFC and lowercase. Fixtures covered English, Bengali, mixed scripts, vowel signs, যুক্তাক্ষর, canonical equivalence, punctuation, literal substrings, title-weighted ranking, metadata edits, tag renames, and deletion. Ctrl+K now opens working search. |
| 4.10 Dashboard | Completed | Added SQLite-backed book, storage, known-page, favorite, and reading totals, recent and favorite lists, and category distribution. Saved-progress fixtures verified recently read and progress order. The reader will produce this data in Task 5.6. |

Eighteen Rust tests, Clippy, strict TypeScript/Vite, and the standalone Windows debug build passed. A database fixture verified bounded pages and ranked search with 10,000 books. Headless Chromium exercised all catalog views, pagination, responsive layout, editing, organization, collection movement and removal, combined filters, favorites, Ctrl+K search, error recovery, missing records, and empty states through a mocked Tauri bridge. Native Windows verification opened the existing library of 1,817 records, read a real book, and returned a real Bengali title through search.

## 2026-11-11 · Phase 5

The owner approved all Phase 5 tasks together.

| Task | Status | Work and verification |
| --- | --- | --- |
| 5.1 PDF.js integration | Completed | Added session-bound binary range IPC, bundled worker/font/CMap/decoder assets, password handling, source error recovery, and document cleanup. Verified actual PDF rendering in the production-protocol Windows executable. |
| 5.2 Reader layout | Completed | Added a toolbar, PDF canvas and text layers, independent reader appearance, navigation sidebar, annotation/settings sidebar, and responsive layout. Checked a 680-pixel browser viewport. |
| 5.3 Navigation | Completed | Verified first, previous, next, last, direct page entry, custom and fit zoom, fullscreen, and keyboard navigation. Page limits follow the loaded PDF. |
| 5.4 Contents and thumbnails | Completed | Added lazy nested contents and virtualized, serialized thumbnail rendering. Rapid outline navigation ignores older asynchronous destinations. |
| 5.5 Text search | Completed | PDF.js find controller searches embedded English and Bengali text, highlights matches, reports counts, and moves between matches. Verified zero results and corrected duplicate form submission from next/previous buttons. |
| 5.6 Progress | Completed | Added 700 ms debounce, ten-second activity checkpoints, resume, final reader/window-close flush, and session history. Tests rejected stale sequences and source revisions and preserved manual status on the last page. Native close persisted page 2/167 and an ended session. Database integrity and foreign-key checks passed. |
| 5.7 Bookmarks and notes | Completed | Added creation, editing, confirmed deletion, navigation, idempotent creation keys, revision checks, and retained anchors after a file revision changes. Storage fixtures verified persistence after reopening. |
| 5.8 Preferences | Completed | Added independent reader light/dark/system appearance, default zoom, continuous/single-page layout, remembered fullscreen, and configurable keyboard shortcuts. Fullscreen uses the full reader surface. |
| 5.9 Performance | Completed | Checked a 2,000-page PDF, rapid paging, book switching, a source larger than 300 MiB, a multi-megabyte embedded image, worker cleanup, and Windows shutdown. Corrected merged-range delivery after testing a real PDF. |

Twenty Rust tests, Clippy, strict TypeScript/Vite, and the standard Windows debug build passed. Browser checks used real locally generated PDFs over a mocked Tauri persistence/range bridge. The 300+ MiB sparse fixture opened with 212,733 range bytes in the measured run. Main-page canvases stayed below 20 and mounted sidebar thumbnails below 12 in the long-document check. Leaving the reader removed all PDF workers. After garbage collection, the measured main-thread heap was 6,712,716 bytes. These measurements describe the test fixtures and viewport.

Rendering caps each page canvas at 12 million pixels and each source image at 32 million pixels. Reader data ranges use IPC chunks of at most 256 KiB and assemble the complete requested range before notifying PDF.js. Original PDF bytes remain under read-only handles. Tests covered password retry, unavailable sources, and recoverable progress-save failures during window close.

The standard executable at `src-tauri/target/debug/libradesk.exe` now contains the reader, replacing the earlier debug version. User data, PDF fixtures, browser profiles, and screenshots are excluded from the commit.


## 2026-11-11 · Task 6.1 · Background PDF text extraction

The owner approved Task 6.1. Added a serial background PDF.js coordinator, revision-bound binary range IPC, migration 0007, per-page raw and normalized text, transactional checkpoints, lease recovery, and per-file progress and retry on Book details. The worker runs while the app is visible and the reader is closed. It yields between streamed text chunks/pages and releases work after up to 32 pages or about ten seconds between pages. Native source access stays read-only.

Each transaction stores at most four contiguous pages and advances the checkpoint atomically. Text is bounded to 256 KiB UTF-8 per page and 1 MiB per batch. Repeating a committed batch cannot duplicate rows; an expired or replaced lease cannot advance the checkpoint. File revision changes clear derived text, reset the checkpoint, and queue the new source. Failures retain a reason and the committed position for manual retry. A 120-second lease, 15-second heartbeat, and 90-second task deadline cover interrupted or stalled workers.

Verification:

- All 24 Rust tests passed, including migration from schema 6 with existing available/missing files, migration replay, checkpoint persistence after reopen, expired lease recovery, stale-owner rejection, idempotent batches, atomic invalid-batch rejection, Unicode normalization, empty-page retention, physical source changes, retry, revision invalidation, and cascading deletion.
- Strict TypeScript/Vite, Clippy with warnings denied, and the standard production-protocol Windows debug build passed.
- Headless Chrome loaded actual local PDF fixtures over a mocked Tauri persistence/range bridge. A 30-page English/Bengali PDF produced ordered text rows and complete UI status. A 2,000-page fixture saved eight pages, reloaded, and resumed at page nine with exactly 32 new pages. Foreground reading and hidden-window checks paused extraction, reader navigation remained responsive, and idle extraction resumed.
- Blank pages persisted as empty rows. Encrypted, corrupt, and oversized-text fixtures produced explicit failures, including manual retry for encryption and no silent truncation. These checks exposed a PDF.js queued-stream cancellation race; task-owned worker destruction fixed it, and the final browser run had zero unhandled errors.
- The real bundled Windows app upgraded the existing 1,817-book library to schema 7. A read-only SQLite observation during extraction found 37 completed jobs, 472 pages in completed jobs, 8,192 checkpointed pages in queued jobs, and no duplicate page keys. Book details displayed the native extraction status. These are an observation during ongoing background processing, not a claim that the entire library has finished.

Background extraction does not request or retain passwords. Password-protected sources require an unencrypted copy for this worker. Image-only PDFs produce empty text; OCR is excluded. Page-content FTS, snippets, and cross-book search remain unchecked for Task 6.2. User data, PDFs, generated browser profiles/harnesses, build products, and screenshots stay outside the commit.
