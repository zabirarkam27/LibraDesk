ALTER TABLE books ADD COLUMN author TEXT;
ALTER TABLE book_files ADD COLUMN source_token TEXT;

CREATE TABLE pdf_metadata (
    file_id INTEGER PRIMARY KEY REFERENCES book_files(id) ON DELETE CASCADE,
    file_revision INTEGER NOT NULL,
    title TEXT,
    author TEXT,
    subject TEXT,
    creator TEXT,
    producer TEXT,
    creation_date_raw TEXT,
    page_count INTEGER CHECK (page_count > 0),
    extracted_at INTEGER NOT NULL
) STRICT;

CREATE TABLE metadata_overrides (
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    field TEXT NOT NULL CHECK (field IN ('title','author','subject')),
    value TEXT,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (book_id, field)
) STRICT;

CREATE TABLE processing_jobs (
    file_id INTEGER NOT NULL REFERENCES book_files(id) ON DELETE CASCADE,
    file_revision INTEGER NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('analysis','cover')),
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued','completed','failed')),
    error_code TEXT,
    PRIMARY KEY (file_id, file_revision, kind)
) STRICT;
CREATE INDEX processing_ready ON processing_jobs(kind, status, file_id, file_revision);

CREATE TABLE thumbnails (
    file_id INTEGER PRIMARY KEY REFERENCES book_files(id) ON DELETE CASCADE,
    file_revision INTEGER NOT NULL,
    relative_path TEXT NOT NULL UNIQUE,
    width INTEGER NOT NULL CHECK (width BETWEEN 1 AND 240),
    height INTEGER NOT NULL CHECK (height BETWEEN 1 AND 320),
    size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE duplicate_reviews (
    sha256 BLOB NOT NULL CHECK (length(sha256) = 32),
    file_id INTEGER NOT NULL REFERENCES book_files(id) ON DELETE CASCADE,
    decision TEXT NOT NULL CHECK (decision = 'keep'),
    reviewed_at INTEGER NOT NULL,
    PRIMARY KEY (sha256, file_id)
) STRICT;
CREATE INDEX duplicate_reviews_file ON duplicate_reviews(file_id);
CREATE INDEX book_files_sha256 ON book_files(sha256) WHERE sha256 IS NOT NULL;

INSERT INTO processing_jobs(file_id,file_revision,kind)
SELECT id,revision,'analysis' FROM book_files;

CREATE TRIGGER book_files_queue_analysis AFTER INSERT ON book_files
BEGIN
    INSERT INTO processing_jobs(file_id,file_revision,kind) VALUES (NEW.id,NEW.revision,'analysis');
END;

CREATE TRIGGER book_files_refresh_analysis AFTER UPDATE OF revision ON book_files
WHEN NEW.revision <> OLD.revision
BEGIN
    DELETE FROM processing_jobs WHERE file_id=NEW.id;
    DELETE FROM duplicate_reviews WHERE file_id=NEW.id;
    INSERT INTO processing_jobs(file_id,file_revision,kind) VALUES (NEW.id,NEW.revision,'analysis');
END;
