CREATE TABLE text_extraction_jobs (
 file_id INTEGER PRIMARY KEY REFERENCES book_files(id) ON DELETE CASCADE,
 file_revision INTEGER NOT NULL CHECK(file_revision>0),
 status TEXT NOT NULL CHECK(status IN ('queued','running','completed','failed')),
 next_page INTEGER NOT NULL DEFAULT 1 CHECK(next_page>0),
 total_pages INTEGER CHECK(total_pages>0),
 lease INTEGER NOT NULL DEFAULT 0 CHECK(lease>=0),
 lease_expires_at INTEGER,
 error_code TEXT,
 updated_at INTEGER NOT NULL,
 CHECK(total_pages IS NULL OR next_page<=total_pages+1)
) STRICT;
CREATE INDEX text_extraction_ready ON text_extraction_jobs(status,updated_at,file_id);

CREATE TABLE page_text (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 file_id INTEGER NOT NULL REFERENCES book_files(id) ON DELETE CASCADE,
 file_revision INTEGER NOT NULL CHECK(file_revision>0),
 page_number INTEGER NOT NULL CHECK(page_number>0),
 text TEXT NOT NULL,
 normalized_text TEXT NOT NULL,
 extracted_at INTEGER NOT NULL,
 UNIQUE(file_id,file_revision,page_number)
) STRICT;

INSERT INTO text_extraction_jobs(file_id,file_revision,status,error_code,updated_at)
 SELECT id,revision,CASE WHEN availability='available' THEN 'queued' ELSE 'failed' END,
 CASE WHEN availability='available' THEN NULL ELSE 'source_unavailable' END,
 CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files;

CREATE TRIGGER book_files_queue_text AFTER INSERT ON book_files BEGIN
 INSERT INTO text_extraction_jobs(file_id,file_revision,status,updated_at)
 VALUES(new.id,new.revision,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER));
END;
CREATE TRIGGER book_files_refresh_text AFTER UPDATE OF revision ON book_files
 WHEN new.revision<>old.revision BEGIN
 DELETE FROM page_text WHERE file_id=new.id;
 UPDATE text_extraction_jobs SET file_revision=new.revision,next_page=1,total_pages=NULL,
 status=CASE WHEN new.availability='available' THEN 'queued' ELSE 'failed' END,
 error_code=CASE WHEN new.availability='available' THEN NULL ELSE 'source_unavailable' END,
 lease=lease+1,lease_expires_at=NULL,updated_at=CAST(unixepoch('subsec')*1000 AS INTEGER)
 WHERE file_id=new.id;
END;
