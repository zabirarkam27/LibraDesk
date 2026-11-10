CREATE TABLE books (
    id INTEGER PRIMARY KEY,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    title_sort_key TEXT NOT NULL,
    subject TEXT,
    publisher TEXT,
    isbn TEXT,
    publication_year INTEGER,
    language TEXT,
    is_favorite INTEGER NOT NULL DEFAULT 0 CHECK (is_favorite IN (0, 1)),
    reading_status TEXT NOT NULL DEFAULT 'unread' CHECK (reading_status IN ('unread', 'reading', 'completed', 'on_hold')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0)
) STRICT;
CREATE INDEX books_title ON books(title_sort_key, id);
CREATE INDEX books_created ON books(created_at, id);

CREATE TABLE book_files (
    id INTEGER PRIMARY KEY,
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    original_path TEXT NOT NULL,
    path_key TEXT NOT NULL UNIQUE,
    windows_file_id TEXT,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    modified_at INTEGER NOT NULL,
    last_verified_at INTEGER,
    sha256 BLOB CHECK (sha256 IS NULL OR length(sha256) = 32),
    page_count INTEGER CHECK (page_count > 0),
    availability TEXT NOT NULL DEFAULT 'available' CHECK (availability IN ('available', 'missing', 'inaccessible', 'invalid')),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    is_primary INTEGER NOT NULL DEFAULT 1 CHECK (is_primary IN (0, 1))
) STRICT;
CREATE UNIQUE INDEX book_files_primary ON book_files(book_id) WHERE is_primary = 1;
CREATE INDEX book_files_book ON book_files(book_id, id);
CREATE INDEX book_files_size ON book_files(size_bytes);

CREATE TABLE scan_roots (
    id INTEGER PRIMARY KEY,
    display_path TEXT NOT NULL,
    path_key TEXT NOT NULL UNIQUE,
    is_enabled INTEGER NOT NULL DEFAULT 1 CHECK (is_enabled IN (0, 1)),
    created_at INTEGER NOT NULL,
    retired_at INTEGER
) STRICT;

CREATE TABLE scan_jobs (
    id INTEGER PRIMARY KEY,
    root_id INTEGER NOT NULL REFERENCES scan_roots(id) ON DELETE RESTRICT,
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'paused', 'awaiting_confirmation', 'importing', 'completed', 'cancelled', 'failed')),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at INTEGER NOT NULL,
    started_at INTEGER,
    finished_at INTEGER,
    error_code TEXT
) STRICT;
CREATE UNIQUE INDEX scan_jobs_active ON scan_jobs(root_id) WHERE status IN ('queued', 'running', 'paused', 'awaiting_confirmation', 'importing');
CREATE INDEX scan_jobs_root ON scan_jobs(root_id, id DESC);

CREATE TABLE scan_frontier (
    job_id INTEGER NOT NULL REFERENCES scan_jobs(id) ON DELETE CASCADE,
    directory_key TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'done', 'inaccessible')),
    PRIMARY KEY (job_id, directory_key)
) STRICT;
CREATE INDEX scan_frontier_pending ON scan_frontier(job_id, status, directory_key);

CREATE TABLE scan_candidates (
    job_id INTEGER NOT NULL REFERENCES scan_jobs(id) ON DELETE CASCADE,
    path_key TEXT NOT NULL,
    display_path TEXT NOT NULL,
    classification TEXT NOT NULL CHECK (classification IN ('new', 'existing', 'duplicate_candidate', 'invalid', 'inaccessible')),
    import_state TEXT NOT NULL DEFAULT 'pending' CHECK (import_state IN ('pending', 'imported', 'skipped', 'failed')),
    size_bytes INTEGER CHECK (size_bytes >= 0),
    modified_at INTEGER,
    modified_token TEXT,
    file_id INTEGER REFERENCES book_files(id) ON DELETE SET NULL,
    error_code TEXT,
    PRIMARY KEY (job_id, path_key)
) STRICT;
CREATE INDEX scan_candidates_import ON scan_candidates(job_id, import_state, classification, path_key);
CREATE INDEX scan_candidates_size ON scan_candidates(job_id, size_bytes) WHERE classification IN ('new', 'duplicate_candidate');
