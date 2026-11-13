# LibraDesk user guide

LibraDesk manages PDFs where they already live. Categories, tags, collections, edits, notes and progress are stored locally; importing never copies, renames or moves source PDFs.

## Import and organize

In Import, add one or more folders and scan an enabled root. Review new, existing, invalid and inaccessible entries before confirming import. Pause or cancel a scan when needed. An interrupted discovery resumes from its saved checkpoint after reopening; a confirmed import resumes without adding duplicate paths. Disabling or retiring a scan root retains its library records.

Metadata and covers process in the background. Metadata lets you review incomplete fields and edit one or several books. Manual fields survive rescanning. Same-size files are only duplicate candidates; exact duplicate groups use SHA-256.

Library offers Grid, List and Compact views, title/author/date/size/year/progress sorting and combined filters. Open a book for details, manual metadata, file availability, reading status and membership. Organize manages nested categories, tags and collections. Collections support manual ordering and reading queues. Deleting a category or collection does not delete its books.

Favorites and Unread, Reading, Completed and On Hold are explicit choices. Opening a final page never marks a book completed.

## Search and read

Ctrl+K opens search. Choose metadata for titles/authors/tags or PDF contents for indexed page text. Bengali and English can be combined. Results include only indexed pages of the current source revision. The app reports indexing coverage; refresh results as work progresses. Image-only scans need OCR, which this version does not provide.

Read opens the integrated PDF.js viewer. Use previous/next, direct page entry, fit width/page, zoom, contents and lazy thumbnails. Ctrl+F searches embedded text; arrows/Page Up/Page Down navigate, Home/End reach the first/last page, +/− zoom and F11 toggles fullscreen. Settings can disable shortcuts. Escape exits fullscreen.

The right sidebar contains bookmarks, page-linked personal notes and reader preferences. Notes remain local. Progress saves after navigation, on checkpoints and before close. Reopening resumes the saved page; a search-result link opens its target instead. A storage failure leaves close available for retry rather than silently discarding the latest progress. Passwords entered for encrypted reader documents are not stored; background extraction cannot unlock those files.

Overview displays real totals, categories, recent books and analytics. Page views count distinct saved pages within each session, including repeat views in later sessions. Active time counts ready, focused, visible reading. Thirty-day activity groups sessions by their local start date. Completed counts come from manual status.

## Recover files and review duplicates

File recovery lists missing, inaccessible, invalid and changed sources. Startup/hourly checks run in the background; Check files now requests a pass. A disconnected disk can appear unavailable without removing any records.

Relink PDF selects and compares a replacement. Review its metadata/hash and confirm. Cancellation preserves the old link. Notes, bookmarks and progress retain their book identity; replacing the content may leave stale page anchors that need review.

In Metadata → Exact duplicates, inspect copies, choose a preferred record, ignore/reopen a group or explicitly remove a library record. Removal rehashes the kept source and requires acknowledgement. Removing a book's last file also removes that book's notes, bookmarks, progress and memberships from the database. Its PDF remains on disk.

## Settings and backup

General chooses the startup page. Appearance controls the app theme; Reader has its own appearance, zoom and layout. Library chooses the default view. Storage reports local usage and confirms cache cleanup. Temporary files must be at least 24 hours old; linked folders, database/backups and PDF content are protected. Restored covers use separate cache generations, which the thumbnail cleanup can clear.

In Backup, create a manual snapshot with optional thumbnails and a chosen destination, or use managed backups. Keep the whole generated folder and manifest. Original PDFs are excluded and backups are unencrypted. Store source PDFs separately using your own backup method.

To restore, select a complete backup folder, inspect its date/counts and acknowledge replacement. LibraDesk creates a safety snapshot of the current database, validates and restores the selected library, rebuilds indexes and asks you to reload. A failed replacement attempts automatic rollback. Newer/incompatible or corrupted backups are rejected. Paths on another computer may need relinking.

See [backup and recovery](backup-recovery.md), [troubleshooting](troubleshooting.md) and [known limitations](known-limitations.md).
