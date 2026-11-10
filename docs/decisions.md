# LibraDesk architecture decisions

These decisions guide implementation. The targeted checks identify where a choice must be proven on Windows.

| ID | Decision | Reason and verification |
| --- | --- | --- |
| ADR-001 | Ship one offline Tauri desktop application. Bundle React, translation files, and PDF.js assets locally. | Supports offline use and a single installer. Verify the packaged app starts and core features work with networking disabled. |
| ADR-002 | Keep SQLite access and filesystem operations in Rust, using typed Tauri commands and scoped capabilities. | Provides a single place to enforce file permissions, invariants, and transactions. Test invalid DTOs, unauthorized file access, and error propagation. |
| ADR-003 | Resolve Windows Local AppData once and keep PDF originals in place. | Separates managed state from user documents. Test missing and denied drives, reparse points, case-sensitive directories, Unicode, and interrupted operations. |
| ADR-004 | Bundle SQLite with FTS5 through the Rust dependency selected in Task 1.7. Use WAL, connection-level foreign keys, and transactional migrations. | Makes availability repeatable across Windows installs. Probe FTS5 and verify foreign-key enforcement and integrity through the selected Rust build. |
| ADR-005 | Separate a book's metadata, file locations, and user overrides. Use a monotonic revision for each file. | Allows relinking and rescanning while preserving user edits and reader state. Invalidate derived records and reject stale worker results when a source revision changes. |
| ADR-006 | Use lopdf for bounded native extraction and PDF.js for cover rendering and metadata fallback. Feed the worker binary ranges through Tauri IPC tied to a queued file revision. | The cover worker can read local PDFs without exposing arbitrary filesystem paths or adding a local HTTP server. Verify the wider range and resource needs of the reader in Task 5.1. |
| ADR-007 | Use independent FTS5 indexes for metadata and page text. Normalize derived text and validate Bengali matches against fixtures. | Keeps fast catalog queries separate from background content indexing. Verify combining marks, canonical equivalence, mixed scripts, prefix matching, and ranking. |
| ADR-008 | Persist import and indexing checkpoints. Use the SQLite backup API and stage restores with a safety snapshot. | Supports recovery from interrupted work and produces a consistent snapshot under WAL. Exercise restart, repeated confirmation, failed restore, and index rebuild. |
| ADR-009 | Include page-linked personal notes in Task 5.7 alongside bookmarks. | Gives the existing notes requirement a delivery task and keeps it with reader annotations. Verify CRUD, restart, relink preservation, and backup/restore. |
