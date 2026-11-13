# LibraDesk architecture

LibraDesk runs as a single Windows desktop application. React renders the workspace in Tauri's WebView2 window. Rust owns AppData, SQLite, native folder selection, and the background scanner. The frontend, PDF.js worker, fonts, CMaps, and decoders are bundled in the executable.

## Frontend

`src/app/` owns startup, hash routing, providers, and the window shell. `src/features/` contains the catalog, book details, organization, collections, dashboard, scanner, metadata tools, reader, and settings. `src/shared/` contains reusable UI, the IPC client, and shared types. Components use Tailwind, shadcn/ui, and Lucide. Book details, organization, collections, and the reader load as separate local chunks.

Use React local state for component interactions and Zustand for shared UI state such as view, filters, and reader controls. Read durable state through typed Rust commands. Validate forms and IPC responses with Zod where runtime parsing is useful, and validate writes independently in Rust. Use TanStack Table for list behavior and virtualize large lists or grids. Request bounded result pages with stable ordering and an ID tie-breaker. Keep reader settings and app appearance separate. Provide English and Bengali labels and preserve Unicode in metadata and paths.

The browser preview uses localStorage. The desktop app reads and writes SQLite through Rust and imports any valid bootstrap app preference in that webview only when a saved SQLite value is absent. App and reader themes are independent. Reader appearance is scoped to its surface.

Catalog search, filters, sorting, and page selection live in the URL. A counted query returns at most 50 books from one SQLite snapshot, and the grid, list, and compact views render only visible rows with overscan. Library mutations emit a catalog event that refreshes active queries. Editors retain their original revision and reject concurrent changes.

## Native commands and background work

Keep Tauri command handlers thin. They decode request DTOs, dispatch to domain services, and return structured responses. Under `src-tauri/src/`, separate command handlers, domain services, SQLite repositories and migrations, and native file operations. Only Rust repositories issue SQL.

Use IDs for known books, files, scan roots, and jobs. A native picker supplies paths when adding roots or relinking. Return a stable error code and retryability alongside details suitable for the frontend. Map these codes to localized user messages. Keep paths and document contents out of ordinary logs. Distinguish missing files from denied access, corrupt PDFs, unsupported encryption, storage failures, and cancellation.

One background worker handles scans and imports on its own SQLite connection. The single-instance guard keeps a second window process from recovering active work. Discovery writes batches of up to 64 directory entries and resumes at pending directories. Confirmed imports revalidate and commit batches of up to 32 candidates. A job revision guards writes after pause, resume, or cancellation. Progress events prompt the UI to request a fresh snapshot. Recovery pauses interrupted discovery and resumes previously confirmed imports. Candidates remain separate from the library until confirmation.

The file-recovery worker uses a separate native thread and SQLite connection. Startup schedules or resumes a persisted verification pass without walking source paths on the UI thread. A cursor and upper file ID bound each pass; completed passes repeat hourly or on explicit request. One file is inspected at a time, with throttled progress events. Missing paths/unavailable drives, denied or sharing-blocked files, and invalid signatures have distinct states. Last known hashes remain available for recovery comparisons. Size or timestamp changes trigger bounded-buffer hashing; identical content retains its revision, while changed content invalidates derivatives and preserves manual overrides.

The lazy File recovery route lists at most 50 files per page and links to a native replacement picker. Relink preview and confirmation are separate commands. Previews compare metadata, page counts, and hashes, expire after 15 minutes, and never update the original link. Confirmation revalidates and rehashes the replacement outside the shared database lock, keeps a read-only handle open through the transaction, and preserves the existing book/file identity and annotations. Background checks also retain read handles through persistence so Windows sharing rules prevent writes between inspection and commit.

Exact duplicate review presents ten copies at a time within each bounded group page, including metadata, paths, availability, and annotation counts. Users can save a preferred record, ignore/reopen a group, or acknowledge removal of one library record. Removal freshly verifies a distinct kept copy and runs only SQL mutations. PDFs are never removed. A book with remaining files keeps its book-level annotations and progress; removing its last file removes its associated library data after the UI warning.

## Windows storage and file identity

Resolve the Windows Local AppData known folder in Rust and initialize `%LOCALAPPDATA%\LibraDesk\` with `database/`, `thumbnails/`, `cache/`, `backups/`, `logs/`, and `temp/`. Report errors if the root is inaccessible. Use a single database writer and coordinate restore with app jobs. Treat cache directories as disposable, keep cleanup inside validated AppData directories, and use staging files for writes that must be promoted atomically.

Keep original PDF locations and open source files read-only. Store the supplied display path separately from an identity key derived from the resolved Windows path and filesystem case semantics. Preserve Unicode in paths and account for case-sensitive directories. Use size, timestamp, and file identity to detect changes, and stream SHA-256 when needed for duplicate or replacement verification. Recheck identity and containment when opening a handle after discovery. Skip reparse points during recursion. An unavailable drive remains a recoverable library state.

## PDF pipeline

Rust hashes each queued source in 128 KiB chunks and uses lopdf to extract document info and page count from PDFs up to 32 MiB. Larger PDFs and files the native parser cannot read can use the local PDF.js worker for metadata. Extracted values stay separate from catalog overrides. Rescanning a changed source invalidates its old digest and cover and queues the new revision.

The cover adapter uses PDF.js range transport over binary Tauri IPC. Rust checks the registered file and queued revision on each request and serves up to 256 KiB per IPC call. It validates the resulting PNG and writes it atomically to the thumbnail cache. The worker renders page one at up to 240 × 320 pixels and uses a placeholder when it cannot render. Source files larger than 256 MiB retain streamed hashes and use the placeholder.

The reader opens a session tied to a registered book, file, and revision. Range requests resolve that session in Rust and reuse the source identity checks. The frontend assembles each requested PDF.js range from IPC chunks of at most 256 KiB. PDFViewer manages visible page rendering and its canvas cache, while a separate serialized queue renders virtualized sidebar thumbnails. PDF.js text layers and the find controller handle selection, search, and highlights. Outline navigation ignores late asynchronous destinations. Cleanup aborts observers and range requests, cancels rendering, and destroys the PDF worker.

PDF fonts, CMaps, and decoders are emitted by the Vite resource plugin and served locally. The production CSP permits bundled WebAssembly decoders. PDFViewer runs with scripting and external PDF links disabled. Source handles use read-only access.

Task 6.1 adds a separate background text coordinator in `src/features/extraction/`. It loads one PDF.js worker and streams one page at a time over the shared, bounded range transport. Rust claims a revision-bound extraction lease for 120 seconds, renewed every 15 seconds. At most four contiguous pages and their next-page checkpoint commit together. Raw text and NFC/lowercase/whitespace-normalized text remain separate. Each page is limited to 256 KiB of UTF-8 text; oversized pages fail explicitly rather than being truncated.

The coordinator yields between chunks and pages, pauses while the reader is active or the window is hidden, and releases a job after at most 32 pages or roughly ten seconds between pages. A 90-second task deadline handles stalled extraction. Cleanup destroys the PDF worker, including interrupted text streams. Expired leases recover the saved checkpoint after restart. A new file revision clears old page text and queues fresh work; old lease holders cannot write to it. Book details show progress, failure reasons, and manual retry. Empty pages are saved, including image-only pages. OCR and password entry for background extraction are excluded; encrypted PDFs report a password-required failure.

## SQLite, search, and recovery

Use the [database schema](database-schema.md) as the contract. Startup prepares and validates AppData and applies migrations off the UI thread. Commands share a serialized connection, and the scan worker has a dedicated connection. SQLite serializes their short write transactions. Rust enables foreign keys and verifies WAL and bundled FTS5 support. Treat FTS and thumbnails as rebuildable derivatives of saved records.

Normalize search text consistently at indexing and query time. Match English and Bengali, including mixed-language queries, while preserving the original metadata. Keep metadata and page text in separate FTS5 indexes. Search by the current file revision and return page references for full-text hits. Validate Bengali tokenization with fixtures before accepting a tokenizer configuration.

Phase 4 implements the metadata index with a normalized projection and triggers for books and tags. Rust registers NFC/lowercase normalization on each connection before migrations. Literal prefix terms use FTS5 and weighted BM25, and literal substring matching supplies partial results. Reader checkpoints populate progress used by the dashboard and catalog sorting.

Task 6.2 adds a separate FTS5 page index over normalized extracted text. Migration 0008 backfills existing pages, checks integrity, and installs transactional synchronization triggers. The typed search command returns exact counts and at most 50 stable BM25-ranked page hits from one snapshot. Results identify the book, file, source revision, page, and bounded plain-text snippet. Only available files and current revisions qualify. The lazy `/search` route is reachable from the sidebar and the Ctrl+K search scope. Results refresh on explicit request or catalog changes rather than re-running a large search for every extraction batch. The UI reports extraction coverage to make partial results visible.

Reader result links include a target page and expected file revision. The reader validates both after opening the registered source, rejects a changed revision or unavailable page, and otherwise starts at the target instead of the saved reading position. Normal book opens continue to resume saved progress. Snippets render as React text, with no injected HTML. Original page text stays separate from the normalized search derivative.

Reader writes carry a session ID and increasing sequence. Progress saves after a 700 ms navigation debounce, on ten-second checkpoints, and when the reader or window closes. Opening and closing readers share a frontend queue. Window close awaits a final save and remains available for retry if storage fails. Active session time counts ready, focused, visible reading. Distinct saved page positions determine the session page count. Reading status remains an independent user choice.

Back up through SQLite's backup API, with a versioned manifest and optional thumbnails. Restore through a staged integrity check, an explicit confirmation, and a safety snapshot. Preserve links to the original PDFs and rebuild indexes if required by the backup or schema version.

## Security boundary

Keep production CSP restrictive and development allowances tied to loopback. Register only required commands and grant Tauri capabilities to specific windows. Keep filesystem and SQL APIs behind Rust services. Validate every incoming ID, path, query, page, range, and mutation. Revalidate approved roots and selected files at the native boundary. Keep token scopes and lifetimes narrow. Use parameterized SQL and construct FTS expressions from escaped terms. Revisit capabilities, CSP, PDF links, restore paths, and destructive database actions during Task 8.5.

References

- [Tauri IPC](https://v2.tauri.app/concept/inter-process-communication/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [PDF.js API](https://mozilla.github.io/pdf.js/api/)
