# LibraDesk database schema

This is the schema contract for `%LOCALAPPDATA%\LibraDesk\database\library.sqlite3`. Migration 0001 creates `settings` alongside the migration history table. The remaining feature tables are introduced by their owning tasks.

## Conventions

Use integer primary keys for records, composite primary keys for link tables, UTC Unix milliseconds for timestamps, and integers with `CHECK (value IN (0, 1))` for booleans. Required fields are `NOT NULL`. Optional values are identified below. Persist normalized name or search keys separately from display values. Use an integer revision for optimistic concurrency where multiple requests can update the same record. Keep database paths, original file paths, and cache keys distinct. Cache paths are relative to their AppData subdirectory.

Foreign keys below are enforced on every connection. Cascade only through dependent database records. No database action may remove, rename, or rewrite a source PDF. Keep writes short and transactional, use WAL on a verified local volume, and configure a bounded busy timeout. Roll back a batch or retain an explicit checkpoint when interrupted.

## Catalog and organization

| Table | Fields and relationships | Constraints and indexes |
| --- | --- | --- |
| `books` | `id` PK, `title`, optional `subject`, `publisher`, `isbn`, `publication_year`, `language`, `is_favorite`, `reading_status`, `created_at`, `updated_at`, `revision`, `title_sort_key` | Nonblank title. Status is unread, reading, completed, or on_hold. Index `(title_sort_key, id)`, `(created_at, id)`, and frequently used status, favorite, language, and year filters. |
| `book_files` | `id` PK, `book_id` FK to books with cascade, `original_path`, `path_key`, optional `windows_file_id`, `size_bytes`, `modified_at`, `last_verified_at`, `sha256`, `page_count`, `availability`, `revision`, `is_primary` | Unique path key based on resolved Windows identity and case semantics. At most one primary file per book via a partial unique index. Nonnegative byte size and positive known page count. Availability is available, missing, inaccessible, or invalid. Index `(book_id, id)`, `(availability, id)`, and non-null hashes and file sizes. Hash is 32 bytes, valid for the current revision. |
| `pdf_metadata` | `file_id` PK and FK to book_files with cascade, `file_revision`, optional extracted `title`, `author`, `subject`, `creator`, `producer`, `creation_date_raw`, `page_count`, `extracted_at` | One raw extraction per current file revision. Retain the original PDF date string and nullable fields. |
| `metadata_overrides` | `book_id` PK and FK to books with cascade, `edited_fields`, `updated_at` | Store the set of fields explicitly edited, including intentional empty values. Effective values remain typed in catalog columns and author links. Extraction may refresh only fields outside this set. Update effective metadata and overrides together. |
| `authors` | `id` PK, `display_name`, `name_sort_key` | Nonblank display name. Index `(name_sort_key, id)`. Permit distinct people with identical names. |
| `book_authors` | PK `(book_id, author_id)`, FKs to books and authors with cascade, `position` | Nonnegative position and unique `(book_id, position)`. Index `(author_id, book_id)`. |
| `categories` | `id` PK, nullable `parent_id` FK to categories with restrict, `name`, `name_key` | Partial unique indexes for root names and sibling names. Reject cycles in the service transaction. Move or delete descendants explicitly before deleting a parent. |
| `book_categories` | PK `(book_id, category_id)`, FKs to books and categories with cascade | Index `(category_id, book_id)` for filtering. |
| `tags` | `id` PK, `name`, `name_key` | Unique normalized name key. |
| `book_tags` | PK `(book_id, tag_id)`, FKs to books and tags with cascade | Index `(tag_id, book_id)`. |
| `collections` | `id` PK, `name`, `name_key`, optional `description`, `sort_mode`, `created_at`, `updated_at` | Unique normalized name key. Validate the sort mode. Collections can serve as user-defined reading lists. |
| `collection_books` | PK `(collection_id, book_id)`, FKs to collections and books with cascade, `position`, `added_at` | Unique `(collection_id, position)` for manual order. Index `(book_id, collection_id)`. |

Create a distinct book and primary file per imported path unless an explicit workflow attaches it to an existing record. Hash equality identifies duplicate candidates and does not merge records. A relink updates the existing file row after confirmation, preserving the book ID and its user data. Keep alternate path spellings as display input where useful, and update path keys only after validation.

## Import, processing, and recovery

| Table | Fields and relationships | Constraints and indexes |
| --- | --- | --- |
| `scan_roots` | `id` PK, `display_path`, `path_key`, `is_enabled`, `created_at`, optional `retired_at` | Unique path key. Retire roots to preserve scan history and leave their books intact. |
| `scan_jobs` | `id` PK, `root_id` FK to scan_roots with restrict, `status`, `revision`, `created_at`, optional `started_at`, `finished_at`, `error_code`, `discovered_count`, `imported_count`, `failed_count` | One nonterminal job per root via a partial unique index. Statuses are queued, running, paused, awaiting_confirmation, importing, completed, cancelled, or failed. Index `(root_id, created_at, id)` for history. |
| `scan_frontier` | PK `(job_id, directory_key)`, `job_id` FK to scan_jobs with cascade, `display_path`, `status` | States pending, in_progress, done, or inaccessible. Index `(job_id, status)`. Revisit incomplete directories after restart and upsert candidates by key. |
| `scan_candidates` | PK `(job_id, path_key)`, `job_id` FK with cascade, `display_path`, `classification`, `import_state`, optional `size_bytes`, `modified_at`, `error_code`, `file_id` FK to book_files with set null | Classify new, existing, duplicate_candidate, invalid, or inaccessible. Record import outcomes as pending, imported, skipped, or failed. Index `(job_id, classification, import_state)`. Revalidate the confirmed file version at import time. |
| `processing_jobs` | `id` PK, `file_id` FK to book_files with cascade, `file_revision`, `kind`, `status`, `attempt_count`, optional `last_completed_page`, `error_code`, `updated_at` | Unique `(file_id, file_revision, kind)`. Kinds include metadata, hash, cover, and page_text. Index `(status, kind, updated_at)`. Commit each page batch and its checkpoint together. |
| `thumbnails` | `file_id` PK and FK to book_files with cascade, `file_revision`, `relative_path`, `width`, `height`, `size_bytes`, `created_at` | Positive dimensions, nonnegative size, and unique validated cache path. Use a versioned filename. Commit the pointer after the generated image has been promoted into place. |
| `duplicate_reviews` | PK `(sha256, file_id)`, `file_id` FK to book_files with cascade, `decision`, `reviewed_at` | Groups are derived from current non-null hashes shared by at least two files. Decisions are keep or ignore. Use the current hash when applying decisions and discard obsolete reviews when a file changes. |

Scan confirmation advances a persisted job revision before importing. Process a candidate and its outcome in the same transaction, with the file path unique key protecting retries and concurrent jobs. Keep history counters in sync with committed outcomes. Mark processed files only after their result has been committed. After a file changes, bump its revision and invalidate stale hash, metadata derivatives, covers, and indexed text. Leave original paths and user edits intact.

## Reader, settings, and analytics

| Table | Fields and relationships | Constraints and indexes |
| --- | --- | --- |
| `reading_progress` | `book_id` PK and FK to books with cascade, optional `file_id` FK to book_files with set null, `file_revision`, `page_number`, optional `total_pages`, `last_read_at` | Page numbers are 1-based and bounded by total pages when known. Index `(last_read_at, book_id)`. Derive percentage from the stored counts. |
| `bookmarks` | `id` PK, `book_id` FK to books with cascade, optional `file_id` FK to book_files with set null, `file_revision`, `page_number`, `label`, `created_at`, `updated_at` | Positive page number and index `(book_id, page_number, id)`. |
| `notes` | `id` PK, `book_id` FK to books with cascade, optional `file_id` FK to book_files with set null, `file_revision`, `page_number`, `body`, `created_at`, `updated_at` | Positive page and nonblank note. Index `(book_id, page_number, id)`. |
| `reading_sessions` | `id` PK, `book_id` FK to books with cascade, `started_at`, optional `ended_at`, `active_ms`, `pages_read` | Nonnegative active time and page count. Index start time and `(book_id, started_at)`. Count recorded reading activity using a documented session rule, independent of the saved page. |
| `settings` | PK `(scope, key)`, `value_json`, `updated_at` | Separate app and reader scopes. Validate each key and payload before writing. Reject unknown keys at the command boundary and use defaults for absent values. |

Require reader file references to belong to the same book in the service transaction. Preserve anchors after relinking or source revision changes. Warn about stale anchors and avoid navigating beyond the replacement's page count. Persist reading status independently of progress.

## Search indexes

- `book_search_text` uses `book_id` as PK and FK with cascade. It stores normalized title, author, subject, publisher, and tag text derived from current catalog rows. `book_search_fts` is an FTS5 external-content index keyed by `book_id`. Recompute the projection after relevant metadata or link changes.
- `page_text` uses `id` as PK, references `book_files` with cascade, and has unique `(file_id, file_revision, page_number)`. Store positive page numbers and extracted normalized text. `page_text_fts` is an FTS5 external-content index keyed by `page_text.id`. Scope matches to the current file revision and return the book, file, page, and snippet.
- Use insert, update, and delete triggers on each projection/content table to keep its FTS index synchronized in the same transaction. Rebuild after adding an index over existing content, or if integrity checks require it. Check FTS integrity separately from SQLite's database checks.
- Start with NFC normalization and the tested tokenizer configuration, `unicode61 remove_diacritics 2 categories 'L* N* Co M*'`. Use bound query values with literal terms safely quoted, controlled prefix matching, and BM25 with title and author weights for catalog results. For a bounded catalog fallback, escape wildcard characters before matching normalized text. Verify Bengali vowel signs, যুক্তাক্ষর, equivalent spellings under Unicode normalization, mixed scripts, punctuation, and empty queries. Keep stored display titles unchanged.

## Migrations and recovery

Use `schema_migrations(version INTEGER PRIMARY KEY, name, checksum, applied_at)` to track ordered, immutable migrations. Cross-check `PRAGMA user_version` with the highest applied version. Before applying upgrades, validate versions and checksums and take a consistent snapshot when changing an existing library. Apply schema, data changes, and version updates in a single transaction per migration. Roll back and stop on failure. Refuse writes to a newer or incompatible schema. Test upgrades from every supported prior release using fixture databases.

Create backup snapshots through SQLite's backup API, including app and reader settings stored in the database. Include schema and manifest versions, checksums, and optional thumbnails. Restore into a staging directory, validate manifest paths and checksums, run `integrity_check` and `foreign_key_check`, and verify the schema can be opened or migrated. Show the user the backup contents and obtain confirmation before replacing live state. Quiesce jobs and connections, write a safety backup, and promote the validated staged database without mixing WAL or SHM sidecars. Reopen and verify the restored database, or roll back from the safety snapshot. Rebuild derived indexes and missing thumbnails as needed.

References

- [SQLite FTS5](https://sqlite.org/fts5.html)
- [SQLite foreign keys](https://sqlite.org/foreignkeys.html)
- [SQLite online backup API](https://sqlite.org/backup.html)
