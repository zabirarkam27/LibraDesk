# Backup and recovery

Use Settings → Backup → Create backup. Choose generated thumbnails if desired. Manual thumbnails are database records and are always included. A complete backup is a folder containing manifest.json, library.sqlite3 and optionally thumbnails. Keep that structure intact. The database contains metadata, settings, categories/tags/collections, notes, bookmarks, progress, sessions and extracted text. Original PDFs are excluded.

Creation reads the live SQLite WAL through the online backup API inside a read snapshot. It validates database integrity, schema and migration history, writes a version-1 manifest with SHA-256/size entries, then promotes the completed folder. Do not make a live backup by copying only library.sqlite3 from AppData while the app runs.

Restore inspection copies only listed safe payload names into a private staging directory, verifies checksums and schema, checks app compatibility, migrates supported older schemas and rebuilds FTS. Inspection and cancellation do not replace the current library. A preview expires after 15 minutes.

Confirmation replaces the current local library and settings. A mandatory pre-restore safety database snapshot is saved under AppData/LibraDesk/backups. Workers are quiesced, in-flight old operations are rejected, and SQLite copies the staged database in place so idle worker connections remain coherent. A separate cover generation changes with the database. Reload on success.

If promotion fails, the safety database is copied back. If both restore and automatic recovery fail, stop using the app, preserve the safety folder and current database/WAL/SHM files, and seek recovery before further writes. Never delete the only remaining safety snapshot to retry.

To return to the previous library after a successful restore, select its safety backup through the same inspected restore flow. Safety backups exclude generated thumbnails; those covers can regenerate from accessible original PDFs. Manual thumbnails remain in the database snapshot. Restoring a backup from another disk or machine retains saved paths; relink unavailable files.

Checksums detect changed bytes, not authorship. Backups are not encrypted or signed. Treat library data and extracted text as private; share backups only deliberately. This version has manual snapshots, not scheduled backups or cloud synchronization.

## Source metadata is outside database rollback

Version 0.2.0 manual metadata edits write original PDFs without persistent PDF safety copies. An atomic sibling staging file is temporary output, not an old-file backup. Keep independent source-document backups if you need history. Restoring a library snapshot cannot undo metadata already written into a PDF.

Restore refuses while the current database has prepared PDF replacements. Finish those saves or use Stop in the affected book's PDF metadata panel before restoring. Restored write jobs/journals are cleared so importing an old snapshot never replays old metadata over today's PDFs. Manual covers survive restore even when generated thumbnails were excluded.
