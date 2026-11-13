# Security and data safety review · 2026-11-14

Scope: LibraDesk 0.1.0 source, locked dependencies, packaged CSP/capabilities and generated Windows fixtures. This is an application review, not an independent penetration test or assurance against a privileged local attacker.

| Boundary | Reviewed behavior and evidence |
| --- | --- |
| Tauri capabilities | Only main-window event listen/unlisten and destroy/fullscreen/is-fullscreen permissions. No frontend filesystem, SQL, shell, HTTP, arbitrary opener or remote-origin capability. File dialogs are invoked through constrained Rust commands. |
| CSP | Production uses self assets, self workers, IPC connections and wasm-unsafe-eval for PDF decoders. No remote script source or unsafe-eval. Development WebSocket allowance is loopback-only. Inline styles are needed by the UI/PDF renderer. |
| PDF actions | Reader creates PDFViewer without a scripting manager, disables external links/autolinking and XFA. Covers disable XFA too. No code executes document JavaScript actions. PDF.js 6.5.438 does not expose the obsolete isEvalSupported option; unsupported options were not retained. |
| Source files | Canonical registered paths, reparse rejection, opened-handle final-path checks, size/full-precision timestamp checks and read-only handles. Windows sharing denies writes/deletion during reads. Import, hashing, reader and relink do not write PDFs. |
| Bounded work | Native metadata parsing has a size bound, hashes stream, IPC reads are capped at 256 KiB, extraction uses leases/checkpoints/text limits, and rendering uses image/canvas limits. These reduce exposure; arbitrary complex PDFs can still consume resources or fail. |
| SQL/FTS | Bind parameters for user values; enum-controlled sort fragments; literal escaped FTS terms. Metacharacter/injection regression leaves the catalog and integrity intact. |
| Paths and caches | Validated fixed AppData categories, directory handles, no reparse traversal during deletion, direct generated-file cleanup only, PDF extension/magic protection and 24-hour temp age. Real Windows junction regression preserves its external PDF target. |
| Backup | Snapshot API includes live WAL consistently. Strict manifest paths/layout, size/SHA-256 checks, schema objects and migration checksums, integrity/FK checks, staged copies and bounded PNG validation. Corruption, traversal and unexpected schemas are rejected in tests. |
| Restore | Version compatibility, expiring inspection token, explicit acknowledgement, revalidation, mandatory safety snapshot, index rebuild and rollback. Maintenance locks/epochs reject old queued work. Cover generation changes with restored data. Fault injection verifies recovery and idle SQLite connection consistency. |
| Record removal | Explicit UI acknowledgement and freshly verified kept duplicate. SQL-only deletion, with documented annotation cascade for a book's last source. Fixture byte comparisons preserve original PDFs. |
| Installation | Current-user NSIS, downgrade prevention, expected bundle marker verified byte-for-byte, default silent uninstall preserves data subfolders and original fixture PDF. Release ignores the debug-only QA profile override by construction. |
| Privacy | Local SQLite/notes/text/backups, no account/cloud/telemetry integration. Backups are unencrypted. Normal launch performs local metadata/file verification; it can update the database. |

Tests: 46 ordinary Rust tests passed; one explicit scale benchmark passed separately; ten browser suites passed. Native IPC/backup/performance used an isolated 10,001-record debug profile. Installer install/uninstall passed in a clean destination. Installed release launch on the owner's actual AppData is an additional permission-dependent check, tracked in the release report.

Residual limits: checksums do not authenticate backup authorship; an attacker with the account's filesystem access can alter data. Native PDF parsing is not a separate process sandbox. File cleanup assumes the app's private cache is not being adversarially rewritten by another local process. Code signing and fresh-OS/absent-WebView testing were not performed. These are documented limits, not claims of unresolved source-file deletion or SQL injection.
