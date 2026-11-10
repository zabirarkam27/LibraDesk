# LibraDesk architecture decisions

These decisions guide implementation. The targeted checks identify where a choice must be proven on Windows.

| ID | Decision | Reason and verification |
| --- | --- | --- |
| ADR-001 | Ship one offline Tauri desktop application. Bundle React, translation files, and PDF.js assets locally. | Supports offline use and a single installer. Verify the packaged app starts and core features work with networking disabled. |
| ADR-002 | Keep SQLite access and filesystem operations in Rust, using typed Tauri commands and scoped capabilities. | Provides a single place to enforce file permissions, invariants, and transactions. Test invalid DTOs, unauthorized file access, and error propagation. |
| ADR-003 | Resolve Windows Local AppData once and keep PDF originals in place. | Separates managed state from user documents. Test missing and denied drives, reparse points, case-sensitive directories, Unicode, and interrupted operations. |
| ADR-004 | Bundle SQLite with FTS5 through the Rust dependency selected in Task 1.7. Use WAL, connection-level foreign keys, and transactional migrations. | Makes availability repeatable across Windows installs. Probe FTS5 and verify foreign-key enforcement and integrity through the selected Rust build. |
| ADR-005 | Separate a book's metadata, file locations, and user overrides. Use a monotonic revision for each file. | Allows relinking and rescanning while preserving user edits and reader state. Invalidate derived records and reject stale worker results when a source revision changes. |
| ADR-006 | Start with PDF.js for rendering and text extraction, and select a Rust metadata parser in Task 3.1. Prototype the Rust-owned PDF range transport in Task 3.5. | Reuses rendering across covers and the reader. Test byte ranges, cancellation, worker cleanup, malformed files, and large files in packaged WebView2 before committing to the transport. |
| ADR-007 | Use independent FTS5 indexes for metadata and page text. Normalize derived text and validate Bengali matches against fixtures. | Keeps fast catalog queries separate from background content indexing. Verify combining marks, canonical equivalence, mixed scripts, prefix matching, and ranking. |
| ADR-008 | Persist import and indexing checkpoints. Use the SQLite backup API and stage restores with a safety snapshot. | Supports recovery from interrupted work and produces a consistent snapshot under WAL. Exercise restart, repeated confirmation, failed restore, and index rebuild. |
| ADR-009 | Include page-linked personal notes in Task 5.7 alongside bookmarks. | Gives the existing notes requirement a delivery task and keeps it with reader annotations. Verify CRUD, restart, relink preservation, and backup/restore. |
