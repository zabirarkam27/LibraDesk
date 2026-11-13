# LibraDesk 0.1.0 release report

Prepared 2026-11-14. Phase 7 is implemented and pushed through 2fb29ae. Phase 8 QA, packaging and documentation are implemented; installed-release launch on the owner's live AppData remains an explicitly permission-dependent smoke check. Final project acceptance has not been granted.

## Delivered behavior

Offline Windows PDF library with confirmed resumable import, metadata/covers, bilingual metadata/page search, categories/tags/collections, favorites/statuses, virtualized library views, details/editing, range-based PDF reader, notes/bookmarks/progress, background extraction, file verification/relinking, safe duplicate review, settings/storage cleanup, validated backup/restore and persisted reading analytics.

Restore inspects and stages before replacing anything, checks versions/hashes/schema, creates a mandatory safety snapshot, rebuilds indexes, isolates restored covers and rolls back on injected failure. Original PDFs remain outside the archive and are never deleted by library record cleanup.

## Verification

- Strict frontend production build passed.
- Rust formatting and Clippy with warnings denied passed.
- 46 ordinary Rust tests passed; the explicit four-size performance benchmark passed separately.
- Ten browser regression suites passed against the final production-built frontend, with mock native IPC and real PDF.js processing.
- A real SQLite cross-feature fixture covers interrupted import through metadata/editing, reader/note persistence, backup/restore, relink and reopen, with unchanged source bytes.
- A real embedded-frontend native QA profile contains 10,001 records and verifies IPC queries and a 10,001-book backup.
- Agreed native budgets passed: startup 705 ms, warm search maximum 91.91 ms and full app/WebView2 process-tree working set maximum 457.91 MiB. See [performance report](performance-report.md) for workload bounds and all four sizes.
- SQL/FTS metacharacter and Windows junction regressions passed. [Security review](security-review.md) records CSP, permissions, PDF actions, paths, backups and destructive operations.
- Production x64 executable and NSIS installer built successfully. Current-user install/uninstall exited 0, executable/version checks passed, original PDF hash and user-data subfolder survived, and the uninstall registry entry was removed.
- Installer tests used a clean directory on the existing Windows account. Isolated native profile QA used a debug build with embedded frontend. Fresh Windows/absent-WebView testing was not performed.

## Installer artifact

Local path: src-tauri/target/release/bundle/nsis/LibraDesk_0.1.0_x64-setup.exe

Size: 6,570,546 bytes.

SHA-256:

~~~text
31F6F0FF6BAD1C3850E8AED869E9A7075CFBAA8B8CA44864252859B640D175F3
~~~

The installer and executable are unsigned. WebView2 must be preinstalled. Source and QA reports are versioned; generated installers, databases, PDFs, caches and private backups are excluded from Git. No GitHub release upload is implied by a source push.

## Pending release smoke and acceptance

Automatic approval review rejected launching the installed release because release builds use the owner's actual %LOCALAPPDATA%\LibraDesk library; startup metadata/file checks can update that database. The debug-only QA override is intentionally unavailable in production. The rejected launch was not executed. The owner was asked to authorize that specific launch or keep the isolated checks only. No workaround was used.

All unaffected installer/preservation, native-isolation, backend, browser, performance, security and documentation work has proceeded. Once the owner decides, either execute the authorized installed smoke or record their acceptance of the stated isolated-test scope. Then request final project/release acceptance; completion approval is still outstanding.

## Documentation

[Installation](installation.md), [user guide](user-guide.md), [backup/recovery](backup-recovery.md), [troubleshooting](troubleshooting.md), [known limitations](known-limitations.md), [architecture](architecture.md), [database](database-schema.md), [testing strategy](testing-strategy.md), [performance](performance-report.md) and [security](security-review.md).
