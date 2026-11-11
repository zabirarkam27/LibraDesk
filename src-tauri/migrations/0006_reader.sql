CREATE TABLE reading_sessions (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
 file_id INTEGER REFERENCES book_files(id) ON DELETE SET NULL,
 file_revision INTEGER NOT NULL CHECK(file_revision>0),
 started_at INTEGER NOT NULL,
 ended_at INTEGER,
 last_checkpoint_at INTEGER NOT NULL,
 active_ms INTEGER NOT NULL DEFAULT 0 CHECK(active_ms>=0),
 pages_read INTEGER NOT NULL DEFAULT 0 CHECK(pages_read>=0),
 sequence INTEGER NOT NULL DEFAULT 0 CHECK(sequence>=0),
 total_pages INTEGER CHECK(total_pages>0)
) STRICT;
CREATE INDEX reading_sessions_started ON reading_sessions(started_at,book_id);
CREATE TABLE reader_session_pages (
 session_id INTEGER NOT NULL REFERENCES reading_sessions(id) ON DELETE CASCADE,
 page_number INTEGER NOT NULL CHECK(page_number>0),
 PRIMARY KEY(session_id,page_number)
) STRICT;
ALTER TABLE reading_progress ADD COLUMN session_id INTEGER REFERENCES reading_sessions(id) ON DELETE SET NULL;

CREATE TABLE bookmarks (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
 file_id INTEGER REFERENCES book_files(id) ON DELETE SET NULL,
 file_revision INTEGER NOT NULL CHECK(file_revision>0),
 page_number INTEGER NOT NULL CHECK(page_number>0),
 label TEXT NOT NULL,
 client_key TEXT NOT NULL UNIQUE,
 revision INTEGER NOT NULL DEFAULT 1,
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
) STRICT;
CREATE INDEX bookmarks_page ON bookmarks(book_id,page_number,id);
CREATE TABLE notes (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
 file_id INTEGER REFERENCES book_files(id) ON DELETE SET NULL,
 file_revision INTEGER NOT NULL CHECK(file_revision>0),
 page_number INTEGER NOT NULL CHECK(page_number>0),
 body TEXT NOT NULL CHECK(trim(body)<>''),
 client_key TEXT NOT NULL UNIQUE,
 revision INTEGER NOT NULL DEFAULT 1,
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL
) STRICT;
CREATE INDEX notes_page ON notes(book_id,page_number,id);
