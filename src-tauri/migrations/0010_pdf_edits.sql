ALTER TABLE books ADD COLUMN genre TEXT CHECK(genre IS NULL OR length(genre)<=4096);
CREATE TABLE custom_covers(
 book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
 revision INTEGER NOT NULL CHECK(revision>0),
 png BLOB NOT NULL CHECK(length(png)>0 AND length(png)<=524288),
 updated_at INTEGER NOT NULL
) STRICT;
CREATE TABLE pdf_write_jobs(
 file_id INTEGER PRIMARY KEY REFERENCES book_files(id) ON DELETE CASCADE,
 generation INTEGER NOT NULL DEFAULT 1 CHECK(generation>0),
 status TEXT NOT NULL CHECK(status IN ('queued','running','completed','failed')),
 error_code TEXT,
 updated_at INTEGER NOT NULL
) STRICT;
CREATE INDEX pdf_write_jobs_ready ON pdf_write_jobs(status,file_id);
CREATE TABLE pdf_write_attempts(
 file_id INTEGER PRIMARY KEY REFERENCES book_files(id) ON DELETE CASCADE,
 generation INTEGER NOT NULL,
 source_revision INTEGER NOT NULL,
 source_path TEXT NOT NULL,
 source_token TEXT NOT NULL,
 original_sha256 BLOB NOT NULL CHECK(length(original_sha256)=32),
 prepared_sha256 BLOB NOT NULL CHECK(length(prepared_sha256)=32),
 stage_path TEXT NOT NULL,
 created_at INTEGER NOT NULL
) STRICT;
CREATE TRIGGER prevent_removal_during_pdf_write BEFORE DELETE ON book_files
WHEN EXISTS(SELECT 1 FROM pdf_write_attempts WHERE file_id=OLD.id)
BEGIN SELECT RAISE(ABORT,'PDF metadata save in progress; retry removal'); END;

DROP VIEW book_search_source;
CREATE VIEW book_search_source AS
 SELECT b.id AS book_id,catalog_normalize(b.title) AS title,catalog_normalize(b.author) AS author,
 catalog_normalize(coalesce(b.subject,'')||' '||coalesce(b.isbn,'')||' '||coalesce(b.genre,'')) AS subject,
 catalog_normalize(b.publisher) AS publisher,
 catalog_normalize((SELECT group_concat(name,' ') FROM
   (SELECT t.name FROM book_tags bt JOIN tags t ON t.id=bt.tag_id WHERE bt.book_id=b.id ORDER BY t.id))) AS tags
 FROM books b;
DROP TRIGGER books_search_update;
CREATE TRIGGER books_search_update AFTER UPDATE OF title,author,subject,publisher,isbn,genre ON books BEGIN
 INSERT INTO book_search_text SELECT * FROM book_search_source WHERE book_id=new.id
 ON CONFLICT(book_id) DO UPDATE SET title=excluded.title,author=excluded.author,subject=excluded.subject,publisher=excluded.publisher,tags=excluded.tags;
END;
UPDATE book_search_text SET subject=(SELECT subject FROM book_search_source WHERE book_id=book_search_text.book_id);

CREATE TRIGGER book_categories_pdf_insert AFTER INSERT ON book_categories
BEGIN
 INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at)
 SELECT id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files WHERE book_id=NEW.book_id
 ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at;
END;

CREATE TRIGGER book_categories_pdf_delete AFTER DELETE ON book_categories
BEGIN
 INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at)
 SELECT id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files WHERE book_id=OLD.book_id
 ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at;
END;

CREATE TRIGGER book_tags_pdf_insert AFTER INSERT ON book_tags
BEGIN
 INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at)
 SELECT id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files WHERE book_id=NEW.book_id
 ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at;
END;

CREATE TRIGGER book_tags_pdf_delete AFTER DELETE ON book_tags
BEGIN
 INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at)
 SELECT id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files WHERE book_id=OLD.book_id
 ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at;
END;

CREATE TRIGGER collection_books_pdf_insert AFTER INSERT ON collection_books
BEGIN
 INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at)
 SELECT id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files WHERE book_id=NEW.book_id
 ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at;
END;

CREATE TRIGGER collection_books_pdf_delete AFTER DELETE ON collection_books
BEGIN
 INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at)
 SELECT id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files WHERE book_id=OLD.book_id
 ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at;
END;

CREATE TRIGGER categories_pdf_update AFTER UPDATE OF name,parent_id ON categories
BEGIN
 INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at)
 SELECT f.id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files f
 WHERE f.book_id IN (SELECT book_id FROM book_categories WHERE category_id IN (WITH RECURSIVE children(id) AS (SELECT NEW.id UNION SELECT c.id FROM categories c JOIN children d ON c.parent_id=d.id) SELECT id FROM children))
 ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at;
END;

CREATE TRIGGER tags_pdf_update AFTER UPDATE OF name ON tags
BEGIN
 INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at)
 SELECT f.id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files f
 WHERE f.book_id IN (SELECT book_id FROM book_tags WHERE tag_id=NEW.id)
 ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at;
END;

CREATE TRIGGER collections_pdf_update AFTER UPDATE OF name ON collections
BEGIN
 INSERT INTO pdf_write_jobs(file_id,generation,status,updated_at)
 SELECT f.id,1,'queued',CAST(unixepoch('subsec')*1000 AS INTEGER) FROM book_files f
 WHERE f.book_id IN (SELECT book_id FROM collection_books WHERE collection_id=NEW.id)
 ON CONFLICT(file_id) DO UPDATE SET generation=generation+1,status='queued',error_code=NULL,updated_at=excluded.updated_at;
END;
