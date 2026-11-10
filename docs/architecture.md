# LibraDesk architecture

LibraDesk runs as a single Windows desktop application. React renders the library in Tauri's WebView2 window. Rust owns local files, SQLite, and background jobs. The production frontend, fonts, translations, and PDF.js worker ship with the app and load locally. The current scaffold only opens the window and demonstrates a React-to-Rust call.

## Frontend

`src/app/` will own app startup, routing, providers, and the window shell. `src/features/` will group library, import, search, reader, and settings code by feature. `src/shared/` will contain the reusable UI, IPC client, and shared types. Components use Tailwind, shadcn/ui, Lucide, and Framer Motion as their approved tasks introduce them. Keep animations optional under reduced-motion preferences.

Use React local state for component interactions and Zustand for shared UI state such as view, filters, and reader controls. Read durable state through typed Rust commands. Validate forms and IPC responses with Zod where runtime parsing is useful, and validate writes independently in Rust. Use TanStack Table for list behavior and virtualize large lists or grids. Request bounded result pages with stable ordering and an ID tie-breaker. Keep reader settings and app appearance separate. Provide English and Bengali labels and preserve Unicode in metadata and paths.

Task 1.5 can persist the appearance preference in the webview until SQLite is available in Task 1.7. Import the saved preference once into typed app settings before switching reads and writes to Rust.

## Native commands and background work

Keep Tauri command handlers thin. They decode request DTOs, dispatch to domain services, and return structured responses. Under `src-tauri/src/`, separate command handlers, domain services, SQLite repositories and migrations, and native file operations. Only Rust repositories issue SQL.

Use IDs for known books, files, scan roots, and jobs. A native picker supplies paths when adding roots or relinking. Return a stable error code and retryability alongside details suitable for the frontend. Map these codes to localized user messages. Keep paths and document contents out of ordinary logs. Distinguish missing files from denied access, corrupt PDFs, unsupported encryption, storage failures, and cancellation.

Run scanning, hashing, and database work outside the UI thread. Use bounded queues for disk reads and worker processing. Persist job state and committed file or page checkpoints. Events report progress with a job ID and revision. The UI reads a fresh snapshot after reconnecting and discards events older than that snapshot. Recovery requeues interrupted work and retries idempotently. Workers commit results only if the file version still matches. Keep scan candidates separate from the library until the user confirms import.

## Windows storage and file identity

Resolve the Windows Local AppData known folder in Rust and initialize `%LOCALAPPDATA%\LibraDesk\` with `database/`, `thumbnails/`, `cache/`, `backups/`, `logs/`, and `temp/`. Report errors if the root is inaccessible. Use a single database writer and coordinate restore with app jobs. Treat cache directories as disposable, keep cleanup inside validated AppData directories, and use staging files for writes that must be promoted atomically.

Keep original PDF locations and open source files read-only. Store the supplied display path separately from an identity key derived from the resolved Windows path and filesystem case semantics. Preserve Unicode in paths and account for case-sensitive directories. Use size, timestamp, and file identity to detect changes, and stream SHA-256 when needed for duplicate or replacement verification. Recheck identity and containment when opening a handle after discovery. Skip reparse points during recursion. An unavailable drive remains a recoverable library state.

## PDF pipeline

Task 2.3 validates the signature and readability and records file attributes. Task 3.1 will select a Rust metadata parser using fixtures for malformed, encrypted, and large documents. Preserve extracted values separately from overrides. Task 3.5 introduces a small local PDF.js rendering adapter for covers. Task 5.1 extends it to the reader, and Task 6.1 reuses worker-based text extraction for background indexing.

Prototype a read-only Tauri custom protocol during cover integration. Rust issues a short-lived session token scoped to a registered file and version. The webview requests bytes by token, and Rust serves checked byte ranges with accurate lengths and status codes. Keep a bounded memory budget for large PDFs and release file handles when the session ends. Verify actual WebView2 range behavior with a packaged build before relying on this transport. Disable PDF scripting and automatic external navigation. Use PDF.js workers, lazy page rendering, cancellation, and deterministic cleanup on document change.

## SQLite, search, and recovery

Use the [database schema](database-schema.md) as the contract. Rust enables foreign keys on every connection and uses WAL for the live database on the local disk after verifying support. Keep writes transactional and batch background work. Treat FTS and thumbnails as rebuildable derivatives of saved records.

Normalize search text consistently at indexing and query time. Match English and Bengali, including mixed-language queries, while preserving the original metadata. Keep metadata and page text in separate FTS5 indexes. Search by the current file revision and return page references for full-text hits. Validate Bengali tokenization with fixtures before accepting a tokenizer configuration.

Back up through SQLite's backup API, with a versioned manifest and optional thumbnails. Restore through a staged integrity check, an explicit confirmation, and a safety snapshot. Preserve links to the original PDFs and rebuild indexes if required by the backup or schema version.

## Security boundary

Keep production CSP restrictive and development allowances tied to loopback. Register only required commands and grant Tauri capabilities to specific windows. Keep filesystem and SQL APIs behind Rust services. Validate every incoming ID, path, query, page, range, and mutation. Revalidate approved roots and selected files at the native boundary. Keep token scopes and lifetimes narrow. Use parameterized SQL and construct FTS expressions from escaped terms. Revisit capabilities, CSP, PDF links, restore paths, and destructive database actions during Task 8.5.

References

- [Tauri IPC](https://v2.tauri.app/concept/inter-process-communication/)
- [Tauri capabilities](https://v2.tauri.app/security/capabilities/)
- [PDF.js API](https://mozilla.github.io/pdf.js/api/)
