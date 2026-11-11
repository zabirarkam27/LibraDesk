# LibraDesk architecture

LibraDesk runs as a single Windows desktop application. React renders the workspace in Tauri's WebView2 window. Rust owns AppData, SQLite, native folder selection, and the background scanner. The frontend is bundled in the executable. Future translation and PDF.js worker assets will also load locally.

## Frontend

`src/app/` owns startup, hash routing, providers, and the window shell. `src/features/` contains the catalog, book details, organization, collections, dashboard, scanner, metadata tools, reader preview, and settings. `src/shared/` contains reusable UI, the IPC client, and shared types. Components use Tailwind, shadcn/ui, and Lucide. Book details, organization, and collection pages load as separate local chunks.

Use React local state for component interactions and Zustand for shared UI state such as view, filters, and reader controls. Read durable state through typed Rust commands. Validate forms and IPC responses with Zod where runtime parsing is useful, and validate writes independently in Rust. Use TanStack Table for list behavior and virtualize large lists or grids. Request bounded result pages with stable ordering and an ID tie-breaker. Keep reader settings and app appearance separate. Provide English and Bengali labels and preserve Unicode in metadata and paths.

The browser preview uses localStorage. The desktop app reads and writes SQLite through Rust and imports any valid bootstrap app preference in that webview only when a saved SQLite value is absent. App and reader themes are independent. The reader preview scopes its color tokens to its own surface.

Catalog search, filters, sorting, and page selection live in the URL. A counted query returns at most 50 books from one SQLite snapshot, and the grid, list, and compact views render only visible rows with overscan. Library mutations emit a catalog event that refreshes active queries. Editors retain their original revision and reject concurrent changes.

## Native commands and background work

Keep Tauri command handlers thin. They decode request DTOs, dispatch to domain services, and return structured responses. Under `src-tauri/src/`, separate command handlers, domain services, SQLite repositories and migrations, and native file operations. Only Rust repositories issue SQL.

Use IDs for known books, files, scan roots, and jobs. A native picker supplies paths when adding roots or relinking. Return a stable error code and retryability alongside details suitable for the frontend. Map these codes to localized user messages. Keep paths and document contents out of ordinary logs. Distinguish missing files from denied access, corrupt PDFs, unsupported encryption, storage failures, and cancellation.

One background worker handles scans and imports on its own SQLite connection. The single-instance guard keeps a second window process from recovering active work. Discovery writes batches of up to 64 directory entries and resumes at pending directories. Confirmed imports revalidate and commit batches of up to 32 candidates. A job revision guards writes after pause, resume, or cancellation. Progress events prompt the UI to request a fresh snapshot. Recovery pauses interrupted discovery and resumes previously confirmed imports. Candidates remain separate from the library until confirmation.

## Windows storage and file identity

Resolve the Windows Local AppData known folder in Rust and initialize `%LOCALAPPDATA%\LibraDesk\` with `database/`, `thumbnails/`, `cache/`, `backups/`, `logs/`, and `temp/`. Report errors if the root is inaccessible. Use a single database writer and coordinate restore with app jobs. Treat cache directories as disposable, keep cleanup inside validated AppData directories, and use staging files for writes that must be promoted atomically.

Keep original PDF locations and open source files read-only. Store the supplied display path separately from an identity key derived from the resolved Windows path and filesystem case semantics. Preserve Unicode in paths and account for case-sensitive directories. Use size, timestamp, and file identity to detect changes, and stream SHA-256 when needed for duplicate or replacement verification. Recheck identity and containment when opening a handle after discovery. Skip reparse points during recursion. An unavailable drive remains a recoverable library state.

## PDF pipeline

Rust hashes each queued source in 128 KiB chunks and uses lopdf to extract document info and page count from PDFs up to 32 MiB. Larger PDFs and files the native parser cannot read can use the local PDF.js worker for metadata. Extracted values stay separate from catalog overrides. Rescanning a changed source invalidates its old digest and cover and queues the new revision.

The cover adapter uses PDF.js range transport over binary Tauri IPC. Rust checks the registered file and queued revision on each request and serves up to 256 KiB per IPC call. It validates the resulting PNG and writes it atomically to the thumbnail cache. The worker renders page one at up to 240 × 320 pixels and uses a placeholder when it cannot render. Source files larger than 256 MiB retain streamed hashes and use the placeholder. Tasks 5.1 and 6.1 will extend local PDF access for reading and indexing.

## SQLite, search, and recovery

Use the [database schema](database-schema.md) as the contract. Startup prepares and validates AppData and applies migrations off the UI thread. Commands share a serialized connection, and the scan worker has a dedicated connection. SQLite serializes their short write transactions. Rust enables foreign keys and verifies WAL and bundled FTS5 support. Treat FTS and thumbnails as rebuildable derivatives of saved records.

Normalize search text consistently at indexing and query time. Match English and Bengali, including mixed-language queries, while preserving the original metadata. Keep metadata and page text in separate FTS5 indexes. Search by the current file revision and return page references for full-text hits. Validate Bengali tokenization with fixtures before accepting a tokenizer configuration.

Phase 4 implements the metadata index with a normalized projection and triggers for books and tags. Rust registers NFC/lowercase normalization on each connection before migrations. Literal prefix terms use FTS5 and weighted BM25, and literal substring matching supplies partial results. Reader progress has storage and read queries for the dashboard and sorting. The reader will write it in Task 5.6.

Back up through SQLite's backup API, with a versioned manifest and optional thumbnails. Restore through a staged integrity check, an explicit confirmation, and a safety snapshot. Preserve links to the original PDFs and rebuild indexes if required by the backup or schema version.

## Security boundary

Keep production CSP restrictive and development allowances tied to loopback. Register only required commands and grant Tauri capabilities to specific windows. Keep filesystem and SQL APIs behind Rust services. Validate every incoming ID, path, query, page, range, and mutation. Revalidate approved roots and selected files at the native boundary. Keep token scopes and lifetimes narrow. Use parameterized SQL and construct FTS expressions from escaped terms. Revisit capabilities, CSP, PDF links, restore paths, and destructive database actions during Task 8.5.

References

- [Tauri IPC](https://v2.tauri.app/concept/inter-process-communication/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [PDF.js API](https://mozilla.github.io/pdf.js/api/)
