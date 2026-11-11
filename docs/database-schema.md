# LibraDesk database schema

This is the schema contract for `%LOCALAPPDATA%\LibraDesk\database\library.sqlite3`. Migration 0001 creates settings, migration 0002 adds the catalog and scanner, migration 0003 adds PDF metadata, processing jobs, covers, and duplicate reviews, migration 0004 adds author relationships and catalog ID sequences, migration 0005 adds organization, metadata search, and reading-progress storage, and migration 0006 adds reading sessions, bookmarks, notes, and progress ordering. Later features introduce the remaining tables.

## Conventions

Use integer primary keys for records, composite primary keys for link tables, UTC Unix milliseconds for timestamps, and integers with `CHECK (value IN (0, 1))` for booleans. Required fields are `NOT NULL`. Optional values are identified below. Persist normalized name or search keys separately from display values. Use an integer revision for optimistic concurrency where multiple requests can update the same record. Keep database paths, original file paths, and cache keys distinct. Cache paths are relative to their AppData subdirectory.

Foreign keys below are enforced on every connection. Cascade only through dependent database records. No database action may remove, rename, or rewrite a source PDF. Keep writes short and transactional, use WAL on a verified local volume, and configure a bounded busy timeout. Roll back a batch or retain an explicit checkpoint when interrupted.

## Catalog and organization

| Table | Fields and relationships | Constraints and indexes |
| --- | --- | --- |
| `books` | `id` PK, `title`, optional `author`, `subject`, `publisher`, `isbn`, `publication_year`, `language`, `is_favorite`, `reading_status`, `created_at`, `updated_at`, `revision`, `title_sort_key` | Nonblank title. Status is unread, reading, completed, or on_hold. Index `(title_sort_key, id)` and `(created_at, id)`. The effective `author` text is a display value for the ordered author relationships. |
| `book_files` | `id` PK, `book_id` FK to books with cascade, `original_path`, `path_key`, `size_bytes`, `modified_at`, optional `windows_file_id`, `source_token`, `last_verified_at`, `sha256`, `page_count`, `availability`, `revision`, `is_primary` | Unique path key based on resolved Windows identity and case semantics. At most one primary file per book via a partial unique index. Nonnegative size and positive known page count. The full-precision modification token and size identify unchanged sources on rescan. A 32-byte digest belongs to the current revision. |
| `pdf_metadata` | `file_id` PK and FK to book_files with cascade, `file_revision`, optional extracted `title`, `author`, `subject`, `creator`, `producer`, `creation_date_raw`, `page_count`, `extracted_at` | One raw extraction per current file revision. Retain the original PDF date string and nullable fields. |
| `metadata_overrides` | PK `(book_id, field)`, FK to books with cascade, nullable `value`, `updated_at` | Records explicit edits to title, author, and subject. A null author or subject records an intentional clear. Update effective catalog fields and overrides in the same transaction. |
| `authors` | `id` PK, `display_name`, `name_sort_key` | Nonblank display name. Index `(name_sort_key, id)`. Permit distinct people with identical names. |
| `book_authors` | PK `(book_id, author_id)`, FKs to books and authors with cascade, `position` | Nonnegative position and unique `(book_id, position)`. Index `(author_id, book_id)`. |
| `catalog_sequences` | `kind` PK, `last_id` | Tracks book and file IDs across deletions. Scanner imports and catalog writes allocate IDs inside their write transactions. |
| `categories` | `id` PK, nullable `parent_id` FK to categories with restrict, `name`, `name_key` | Partial unique indexes for root names and sibling names. Reject cycles in the service transaction. Move or delete descendants explicitly before deleting a parent. |
| `book_categories` | PK `(book_id, category_id)`, FKs to books and categories with cascade | Index `(category_id, book_id)` for filtering. |
| `tags` | `id` PK, `name`, `name_key` | Unique normalized name key. |
| `book_tags` | PK `(book_id, tag_id)`, FKs to books and tags with cascade | Index `(tag_id, book_id)`. |
| `collections` | `id` PK, `name`, `name_key`, optional `description`, `sort_mode`, `created_at`, `updated_at` | Unique normalized name key. Validate the sort mode. Collections can serve as user-defined reading lists. |
| `collection_books` | PK `(collection_id, book_id)`, FKs to collections and books with cascade, `position`, `added_at` | Unique `(collection_id, position)` for manual order. Index `(book_id, collection_id)`. |

Create a distinct book and primary file per imported path unless an explicit workflow attaches it to an existing record. Hash equality identifies duplicate candidates and does not merge records. A relink updates the existing file row after confirmation, preserving the book ID and its user data. Keep alternate path spellings as display input where useful, and update path keys only after validation.

Migration 0004 links each existing nonblank author string as one name. The PDF extractor and the free-text metadata editor use the same convention. Assign author IDs through the catalog service to represent separate contributors or reuse a known identity.

Catalog creation supports books without a source file. Catalog updates replace editable metadata and require the current revision. Omitting optional scalar fields clears them. Omitting author IDs preserves the current links, and an empty list clears them. Deletion also requires the current revision and removes dependent database rows through foreign keys.

The Phase 4 editor can supply ordered author names instead of IDs. Existing identities at unchanged positions are retained. Favorite, status, category, tag, and collection inputs are optional and preserve their stored values when omitted. Assignment changes share the metadata transaction. Organization records have revisions, and category, tag, and collection IDs use AUTOINCREMENT. A category with children requires moving or deleting those children before removal. Collection positions can have gaps after membership removal.

## Import, processing, and recovery

| Table | Fields and relationships | Constraints and indexes |
| --- | --- | --- |
| `scan_roots` | `id` PK, `display_path`, `path_key`, `is_enabled`, `created_at`, optional `retired_at` | Unique path key. Retire roots to preserve scan history and leave their books intact. |
| `scan_jobs` | `id` PK, `root_id` FK to scan_roots with restrict, `status`, `revision`, `created_at`, optional `started_at`, `finished_at`, `error_code` | One nonterminal job per root via a partial unique index. Statuses are queued, running, paused, awaiting_confirmation, importing, completed, cancelled, or failed. History counts are derived from candidate and frontier outcomes. |
| `scan_frontier` | PK `(job_id, directory_key)`, `job_id` FK to scan_jobs with cascade, `status` | States pending, done, or inaccessible. Index `(job_id, status, directory_key)`. Revisit incomplete directories after restart and upsert candidates by key. |
| `scan_candidates` | PK `(job_id, path_key)`, `job_id` FK with cascade, `display_path`, `classification`, `import_state`, optional `size_bytes`, `modified_at`, `modified_token`, `error_code`, `file_id` FK to book_files with set null | Classify new, existing, duplicate_candidate, invalid, or inaccessible. Record import outcomes as pending, imported, skipped, or failed. Index `(job_id, import_state, classification, path_key)`. Revalidate size and the full-precision modification token at import time. |
| `processing_jobs` | PK `(file_id, file_revision, kind)`, FK to book_files with cascade, `status`, optional `error_code` | Analysis jobs stream hashes and extract metadata. Cover jobs render page one. Queued jobs remain pending until their output commits. Index `(kind, status, file_id, file_revision)`. |
| `thumbnails` | `file_id` PK and FK to book_files with cascade, `file_revision`, `relative_path`, `width`, `height`, `size_bytes`, `created_at` | Positive dimensions, nonnegative size, and unique validated cache path. Use a versioned filename. Commit the pointer after the generated image has been promoted into place. |
| `duplicate_reviews` | PK `(sha256, file_id)`, `file_id` FK to book_files with cascade, `decision`, `reviewed_at` | Groups use current hashes shared by at least two files. Reviewing a group records a keep decision for each current member. Reopening clears the review and a changed revision invalidates its prior decision. |

Scan confirmation advances a persisted job revision before importing. The preview flags equal-size files as possible duplicates. Import keeps each eligible distinct path and uses its filename stem as a provisional title. Each candidate and its outcome commit in the same transaction, with the file path unique key protecting retries and overlapping roots. In the planned change-detection feature, bump the file revision and invalidate stale hash, metadata derivatives, covers, and indexed text when a file changes.

## Reader, settings, and analytics

| Table | Fields and relationships | Constraints and indexes |
| --- | --- | --- |
| `reading_progress` | `book_id` PK and FK to books with cascade, optional `file_id` FK to book_files with set null, `file_revision`, `page_number`, optional `total_pages`, `last_read_at`, optional `session_id` FK to reading_sessions with set null | Page numbers are 1-based and bounded by total pages when known. Index `(last_read_at, book_id)`. Derive percentage from the stored counts. |
| `bookmarks` | `id` PK, `book_id` FK to books with cascade, optional `file_id` FK to book_files with set null, `file_revision`, `page_number`, `label`, `client_key`, `revision`, `created_at`, `updated_at` | Positive page number, unique creation key, and index `(book_id, page_number, id)`. |
| `notes` | `id` PK, `book_id` FK to books with cascade, optional `file_id` FK to book_files with set null, `file_revision`, `page_number`, `body`, `client_key`, `revision`, `created_at`, `updated_at` | Positive page, nonblank note, and unique creation key. Index `(book_id, page_number, id)`. |
| `reading_sessions` | `id` PK, `book_id` FK to books with cascade, optional `file_id` FK with set null, `file_revision`, `started_at`, optional `ended_at`, `last_checkpoint_at`, `active_ms`, `pages_read`, `sequence`, optional `total_pages` | Nonnegative active time, sequence, and page count. Index `(started_at, book_id)`. IDs use AUTOINCREMENT. |
| `reader_session_pages` | PK `(session_id, page_number)`, FK to reading_sessions with cascade | Tracks distinct saved page positions within a session. |
| `settings` | PK `(scope, key)`, `value_json`, `updated_at` | Separate app and reader scopes. Validate each key and payload before writing. Reject unknown keys at the command boundary and use defaults for absent values. |

Require reader file references to belong to the same book in the service transaction. Preserve anchors after relinking or source revision changes. Warn about stale anchors and avoid navigating beyond the replacement's page count. Persist reading status independently of progress.

Migration 0006 adds reader state. Checkpoints reject stale sequences and file revisions, bound the current page by the loaded page count, and update progress and session statistics together. A newer session owns the saved position. Opening a reader closes earlier interrupted sessions at their last checkpoint. Active milliseconds count ready, visible, focused reading time. Session page counts use distinct checkpointed positions. Annotation creation keys make retries idempotent, and revisions protect edits and deletion.

Reader zoom, layout, fullscreen, and shortcut preferences use the `reader/viewer` settings key. Reader theme uses the existing `reader/theme` key. These settings are independent of app appearance.

## Search indexes

- `book_search_text` uses `book_id` as PK and FK with cascade. It stores normalized title, author, subject, publisher, and tag text derived from current catalog rows. `book_search_fts` is an FTS5 external-content index keyed by `book_id`. Recompute the projection after relevant metadata or link changes.
- Migration 0007 implements `page_text` with an AUTOINCREMENT `id`, a cascading `book_files` reference, and unique `(file_id, file_revision, page_number)`. Positive page numbers identify the source page. Store raw `text`, derived `normalized_text`, and `extracted_at`, including rows for empty pages. Task 6.2 will add `page_text_fts` keyed by `page_text.id`, revision-scoped hits, and snippets.
- Use insert, update, and delete triggers on each projection/content table to keep its FTS index synchronized in the same transaction. Rebuild after adding an index over existing content, or if integrity checks require it. Check FTS integrity separately from SQLite's database checks.
- Start with NFC normalization and the tested tokenizer configuration, `unicode61 remove_diacritics 2 categories 'L* N* Co M*'`. Use bound query values with literal terms safely quoted, controlled prefix matching, and BM25 with title and author weights for catalog results. For a bounded catalog fallback, escape wildcard characters before matching normalized text. Verify Bengali vowel signs, যুক্তাক্ষর, equivalent spellings under Unicode normalization, mixed scripts, punctuation, and empty queries. Keep stored display titles unchanged.

Migration 0005 implements the metadata projection, including ISBN in its subject text. Each application connection registers deterministic normalization and literal substring functions before migration. Quoted prefix terms use FTS5, and unmatched literal substrings use the normalized projection. Search and catalog filters share a read transaction for counts and data. The query returns at most 50 rows, places missing sort values last, and uses book ID to break ties. Metadata and tag triggers update the projection and index atomically. Dashboard storage sums all registered files, while known pages sum primary files.

## Migrations and recovery

Migration 0007 also adds `text_extraction_jobs`, one cascading row per file, with `file_revision`, queued/running/completed/failed status, 1-based `next_page`, optional `total_pages`, monotonically increasing `lease`, expiry, error code, and update time. Existing files are backfilled and new files queue automatically. Revision triggers delete old text, reset the checkpoint, and invalidate the lease. Each immediate transaction verifies lease ownership and the current source, inserts one to four contiguous pages (at most 1 MiB), and advances the checkpoint. Replaying an identical committed batch succeeds without duplicates. Expired running jobs can be reclaimed; manual retry preserves the last committed checkpoint for the same revision.

Use `schema_migrations(version INTEGER PRIMARY KEY, name, checksum, applied_at)` to track ordered, immutable migrations. Cross-check `PRAGMA user_version` with the highest applied version. Before applying upgrades, validate versions and checksums and take a consistent snapshot when changing an existing library. Apply schema, data changes, and version updates in a single transaction per migration. Roll back and stop on failure. Refuse writes to a newer or incompatible schema. Test upgrades from every supported prior release using fixture databases.

Create backup snapshots through SQLite's backup API, including app and reader settings stored in the database. Include schema and manifest versions, checksums, and optional thumbnails. Restore into a staging directory, validate manifest paths and checksums, run `integrity_check` and `foreign_key_check`, and verify the schema can be opened or migrated. Show the user the backup contents and obtain confirmation before replacing live state. Quiesce jobs and connections, write a safety backup, and promote the validated staged database without mixing WAL or SHM sidecars. Reopen and verify the restored database, or roll back from the safety snapshot. Rebuild derived indexes and missing thumbnails as needed.

References

- [SQLite FTS5](https://sqlite.org/fts5.html)
- [SQLite foreign keys](https://sqlite.org/foreignkeys.html)
- [SQLite online backup API](https://sqlite.org/backup.html)
