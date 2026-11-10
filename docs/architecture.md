# LibraDesk architecture

LibraDesk runs as a single Windows desktop application. React renders the workspace in Tauri's WebView2 window. Rust owns AppData, SQLite, native folder selection, and the background scanner. The frontend is bundled in the executable. Future translation and PDF.js worker assets will also load locally.

## Frontend

`src/app/` owns startup, hash routing, providers, and the window shell. `src/features/` currently contains the library, reader preview, and settings. `src/shared/` contains reusable UI, the IPC client, and shared types. Components use Tailwind, shadcn/ui, Lucide, and Framer Motion. The overview animation respects reduced-motion preferences.

Use React local state for component interactions and Zustand for shared UI state such as view, filters, and reader controls. Read durable state through typed Rust commands. Validate forms and IPC responses with Zod where runtime parsing is useful, and validate writes independently in Rust. Use TanStack Table for list behavior and virtualize large lists or grids. Request bounded result pages with stable ordering and an ID tie-breaker. Keep reader settings and app appearance separate. Provide English and Bengali labels and preserve Unicode in metadata and paths.

The browser preview uses localStorage. The desktop app reads and writes SQLite through Rust and imports any valid bootstrap app preference in that webview only when a saved SQLite value is absent. App and reader themes are independent. The reader preview scopes its color tokens to its own surface.

## Native commands and background work

Keep Tauri command handlers thin. They decode request DTOs, dispatch to domain services, and return structured responses. Under `src-tauri/src/`, separate command handlers, domain services, SQLite repositories and migrations, and native file operations. Only Rust repositories issue SQL.

Use IDs for known books, files, scan roots, and jobs. A native picker supplies paths when adding roots or relinking. Return a stable error code and retryability alongside details suitable for the frontend. Map these codes to localized user messages. Keep paths and document contents out of ordinary logs. Distinguish missing files from denied access, corrupt PDFs, unsupported encryption, storage failures, and cancellation.

One background worker handles scans and imports on its own SQLite connection. The single-instance guard keeps a second window process from recovering active work. Discovery writes batches of up to 64 directory entries and resumes at pending directories. Confirmed imports revalidate and commit batches of up to 32 candidates. A job revision guards writes after pause, resume, or cancellation. Progress events prompt the UI to request a fresh snapshot. Recovery pauses interrupted discovery and resumes previously confirmed imports. Candidates remain separate from the library until confirmation.

## Windows storage and file identity

Resolve the Windows Local AppData known folder in Rust and initialize `%LOCALAPPDATA%\LibraDesk\` with `database/`, `thumbnails/`, `cache/`, `backups/`, `logs/`, and `temp/`. Report errors if the root is inaccessible. Use a single database writer and coordinate restore with app jobs. Treat cache directories as disposable, keep cleanup inside validated AppData directories, and use staging files for writes that must be promoted atomically.

Keep original PDF locations and open source files read-only. Store the supplied display path separately from an identity key derived from the resolved Windows path and filesystem case semantics. Preserve Unicode in paths and account for case-sensitive directories. Use size, timestamp, and file identity to detect changes, and stream SHA-256 when needed for duplicate or replacement verification. Recheck identity and containment when opening a handle after discovery. Skip reparse points during recursion. An unavailable drive remains a recoverable library state.

## PDF pipeline

Task 2.3 validates the signature and readability and records file attributes. Task 3.1 will select a Rust metadata parser using fixtures for malformed, encrypted, and large documents. Preserve extracted values separately from overrides. Task 3.5 introduces a small local PDF.js rendering adapter for covers. Task 5.1 extends it to the reader, and Task 6.1 reuses worker-based text extraction for background indexing.

Prototype a read-only Tauri custom protocol during cover integration. Rust issues a short-lived session token scoped to a registered file and version. The webview requests bytes by token, and Rust serves checked byte ranges with accurate lengths and status codes. Keep a bounded memory budget for large PDFs and release file handles when the session ends. Verify actual WebView2 range behavior with a packaged build before relying on this transport. Disable PDF scripting and automatic external navigation. Use PDF.js workers, lazy page rendering, cancellation, and deterministic cleanup on document change.

## SQLite, search, and recovery

Use the [database schema](database-schema.md) as the contract. Startup prepares and validates AppData and applies migrations off the UI thread. Commands share a serialized connection, and the scan worker has a dedicated connection. SQLite serializes their short write transactions. Rust enables foreign keys and verifies WAL and bundled FTS5 support. Treat FTS and thumbnails as rebuildable derivatives of saved records.

Normalize search text consistently at indexing and query time. Match English and Bengali, including mixed-language queries, while preserving the original metadata. Keep metadata and page text in separate FTS5 indexes. Search by the current file revision and return page references for full-text hits. Validate Bengali tokenization with fixtures before accepting a tokenizer configuration.

Back up through SQLite's backup API, with a versioned manifest and optional thumbnails. Restore through a staged integrity check, an explicit confirmation, and a safety snapshot. Preserve links to the original PDFs and rebuild indexes if required by the backup or schema version.

## Security boundary

Keep production CSP restrictive and development allowances tied to loopback. Register only required commands and grant Tauri capabilities to specific windows. Keep filesystem and SQL APIs behind Rust services. Validate every incoming ID, path, query, page, range, and mutation. Revalidate approved roots and selected files at the native boundary. Keep token scopes and lifetimes narrow. Use parameterized SQL and construct FTS expressions from escaped terms. Revisit capabilities, CSP, PDF links, restore paths, and destructive database actions during Task 8.5.

References

- [Tauri IPC](https://v2.tauri.app/concept/inter-process-communication/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [PDF.js API](https://mozilla.github.io/pdf.js/api/)
