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
