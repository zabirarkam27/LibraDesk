CREATE TABLE file_health (
 file_id INTEGER PRIMARY KEY REFERENCES book_files(id) ON DELETE CASCADE,
 result TEXT NOT NULL,
 checked_at INTEGER NOT NULL,
 changed_at INTEGER
) STRICT;
CREATE TABLE verification_state (
 id INTEGER PRIMARY KEY CHECK(id=1),
 status TEXT NOT NULL CHECK(status IN ('running','completed')),
 cursor INTEGER NOT NULL DEFAULT 0,
 upper_id INTEGER NOT NULL,
 checked INTEGER NOT NULL DEFAULT 0,
 started_at INTEGER NOT NULL,
 finished_at INTEGER
) STRICT;
CREATE TABLE relink_drafts (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 file_id INTEGER NOT NULL REFERENCES book_files(id) ON DELETE CASCADE,
 file_revision INTEGER NOT NULL,
 old_path TEXT NOT NULL,
 old_hash BLOB,
 candidate_path TEXT NOT NULL,
 candidate_bytes INTEGER NOT NULL,
 candidate_modified INTEGER NOT NULL,
 candidate_token TEXT NOT NULL,
 candidate_hash BLOB NOT NULL CHECK(length(candidate_hash)=32),
 metadata_json TEXT,
 created_at INTEGER NOT NULL
) STRICT;
CREATE TABLE duplicate_preferences (
 sha256 BLOB PRIMARY KEY CHECK(length(sha256)=32),
 file_id INTEGER NOT NULL REFERENCES book_files(id) ON DELETE CASCADE
) STRICT;
CREATE TRIGGER duplicate_preference_refresh AFTER UPDATE OF revision ON book_files
 WHEN new.revision<>old.revision BEGIN
 DELETE FROM duplicate_preferences WHERE file_id=new.id;
 DELETE FROM relink_drafts WHERE file_id=new.id;
END;
CREATE INDEX book_files_availability ON book_files(availability,id);
