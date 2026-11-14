# Troubleshooting

| Symptom | Action |
| --- | --- |
| App will not open | Use the Windows 11 x64 setup.exe, which includes the missing WebView2 prerequisite. Keep an existing runtime updated, close another running LibraDesk instance and reinstall without removing AppData. Check free space and organization policy if runtime installation fails. |
| Storage/database unavailable | Check free space and write access to %LOCALAPPDATA%\LibraDesk. Close other tools that lock its files, then restart. Preserve database/WAL/SHM before manual recovery. |
| Build cannot replace libradesk.exe | Close that executable, then rerun the failed build. Do not delete your library database. |
| Missing books after drive removal | Reconnect the disk and run file verification. Relink if the source path changed. |
| Invalid or inaccessible PDF | Check access and open the original in a trusted viewer. A header-valid scan entry can still contain malformed PDF structures. Rescan after correcting the source. |
| Search misses words | Confirm metadata versus PDF-contents scope, indexing coverage and current file revision. Image-only pages need OCR. Background extraction pauses in the reader or hidden window. |
| Encrypted PDF not indexed | Open it manually with its password for reading. Background indexing does not request or retain passwords. |
| Metadata stays incomplete | Review parser errors in Metadata and enter manual fields. Large/complex files may use the bounded cover-worker metadata fallback. |
| Cover is absent | Ensure the source is available. Clear thumbnail cache if necessary; visible covers regenerate. |
| Note/bookmark has a stale anchor | The source content/revision changed. Review the saved page against the new document. |
| Reader close reports save failure | Retry while storage is available; keep the window open until the checkpoint succeeds. |
| Backup checksum/schema/version error | Keep the original folder intact, select a complete copy, and use a compatible application version. Do not edit a manifest to bypass validation. |
| Restore failed | The app attempts rollback. Preserve the safety snapshot. If rollback also failed, stop further writes and retain all database files for recovery. |
| Native QA opens the wrong window | Close other LibraDesk instances. Use the embedded debug build and generated QA profile; release builds ignore the QA data override. |
| Installer QA refuses an installation | It protects an existing registered copy. Review/uninstall that copy yourself before testing; do not delete registry entries to force the test. |

Report a reproducible sequence, app/Windows version, and safe error text. Avoid posting personal paths, documents, notes, extracted text, databases or backups in public issues. See the [test strategy](testing-strategy.md) for generated fixtures.

## PDF metadata syncs and manual covers

- Catalog saved but PDF unchanged: open Book details → PDF metadata sync. Queued/writing are pending; Saved to PDF confirms completion. A failure notification has View book. Close the reader/other PDF editors, check write access and staging space, then Retry.
- Signed, encrypted/EFS, malformed or over-256-MiB PDFs: source writing is refused; the catalog edit remains. Use an appropriate external document workflow if needed. Do not remove signatures/passwords automatically.
- Source changed: inspect/reverify the current file before retrying. Stop cancels pending work without undoing a replacement already completed.
- Restore/removal says a PDF save is in progress: finish or Stop the affected prepared saves first. Restoring the database does not restore previous PDF metadata.
- Manual cover rejected: select PNG/JPEG/WebP up to 10 MiB and 32 million pixels. Preview before saving. Change thumbnail → Use automatic thumbnail resets the choice; clearing generated caches does not remove manual covers.
