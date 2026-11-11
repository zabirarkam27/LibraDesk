CREATE TABLE categories (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 parent_id INTEGER REFERENCES categories(id) ON DELETE RESTRICT,
 name TEXT NOT NULL CHECK(trim(name)<>''),
 name_key TEXT NOT NULL,
 revision INTEGER NOT NULL DEFAULT 1
) STRICT;
CREATE UNIQUE INDEX categories_root_name ON categories(name_key) WHERE parent_id IS NULL;
CREATE UNIQUE INDEX categories_sibling_name ON categories(parent_id,name_key) WHERE parent_id IS NOT NULL;
CREATE TABLE book_categories (
 book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
 category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
 PRIMARY KEY(book_id,category_id)
) STRICT;
CREATE INDEX book_categories_category ON book_categories(category_id,book_id);

CREATE TABLE tags (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 name TEXT NOT NULL CHECK(trim(name)<>''),
 name_key TEXT NOT NULL UNIQUE,
 revision INTEGER NOT NULL DEFAULT 1
) STRICT;
CREATE TABLE book_tags (
 book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
 tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
 PRIMARY KEY(book_id,tag_id)
) STRICT;
CREATE INDEX book_tags_tag ON book_tags(tag_id,book_id);

CREATE TABLE collections (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 name TEXT NOT NULL CHECK(trim(name)<>''),
 name_key TEXT NOT NULL UNIQUE,
 description TEXT,
 sort_mode TEXT NOT NULL DEFAULT 'manual' CHECK(sort_mode IN ('manual','title','added')),
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL,
 revision INTEGER NOT NULL DEFAULT 1
) STRICT;
CREATE TABLE collection_books (
 collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
 book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
 position INTEGER NOT NULL CHECK(position>=0),
 added_at INTEGER NOT NULL,
 PRIMARY KEY(collection_id,book_id),
 UNIQUE(collection_id,position)
) STRICT;
CREATE INDEX collection_books_book ON collection_books(book_id,collection_id);

CREATE TABLE reading_progress (
 book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
 file_id INTEGER REFERENCES book_files(id) ON DELETE SET NULL,
 file_revision INTEGER NOT NULL CHECK(file_revision>0),
 page_number INTEGER NOT NULL CHECK(page_number>0),
 total_pages INTEGER CHECK(total_pages>0 AND page_number<=total_pages),
 last_read_at INTEGER NOT NULL
) STRICT;
CREATE INDEX reading_progress_recent ON reading_progress(last_read_at,book_id);
CREATE INDEX books_status ON books(reading_status,id);
CREATE INDEX books_language ON books(language,id);
CREATE INDEX books_favorite ON books(is_favorite,id);
UPDATE books SET title_sort_key=catalog_normalize(title);
UPDATE authors SET name_sort_key=catalog_normalize(display_name);

CREATE TABLE book_search_text (
 book_id INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
 title TEXT NOT NULL,
 author TEXT NOT NULL,
 subject TEXT NOT NULL,
 publisher TEXT NOT NULL,
 tags TEXT NOT NULL
) STRICT;
CREATE VIRTUAL TABLE book_search_fts USING fts5(
 title,author,subject,publisher,tags,
 content='book_search_text',content_rowid='book_id',
 tokenize="unicode61 remove_diacritics 2 categories 'L* N* Co M*'",
 prefix='2 3 4'
);
CREATE TRIGGER search_insert AFTER INSERT ON book_search_text BEGIN
 INSERT INTO book_search_fts(rowid,title,author,subject,publisher,tags)
 VALUES(new.book_id,new.title,new.author,new.subject,new.publisher,new.tags);
END;
CREATE TRIGGER search_delete AFTER DELETE ON book_search_text BEGIN
 INSERT INTO book_search_fts(book_search_fts,rowid,title,author,subject,publisher,tags)
 VALUES('delete',old.book_id,old.title,old.author,old.subject,old.publisher,old.tags);
END;
CREATE TRIGGER search_update AFTER UPDATE ON book_search_text BEGIN
 INSERT INTO book_search_fts(book_search_fts,rowid,title,author,subject,publisher,tags)
 VALUES('delete',old.book_id,old.title,old.author,old.subject,old.publisher,old.tags);
 INSERT INTO book_search_fts(rowid,title,author,subject,publisher,tags)
 VALUES(new.book_id,new.title,new.author,new.subject,new.publisher,new.tags);
END;
CREATE VIEW book_search_source AS
 SELECT b.id AS book_id,catalog_normalize(b.title) AS title,catalog_normalize(b.author) AS author,
 catalog_normalize(coalesce(b.subject,'')||' '||coalesce(b.isbn,'')) AS subject,
 catalog_normalize(b.publisher) AS publisher,
 catalog_normalize((SELECT group_concat(name,' ') FROM
   (SELECT t.name FROM book_tags bt JOIN tags t ON t.id=bt.tag_id WHERE bt.book_id=b.id ORDER BY t.id))) AS tags
 FROM books b;
INSERT INTO book_search_text SELECT * FROM book_search_source;

CREATE TRIGGER books_search_insert AFTER INSERT ON books BEGIN INSERT INTO book_search_text SELECT * FROM book_search_source WHERE book_id=new.id
 ON CONFLICT(book_id) DO UPDATE SET title=excluded.title,author=excluded.author,
 subject=excluded.subject,publisher=excluded.publisher,tags=excluded.tags; END;
CREATE TRIGGER books_search_update AFTER UPDATE OF title,author,subject,publisher,isbn ON books BEGIN INSERT INTO book_search_text SELECT * FROM book_search_source WHERE book_id=new.id
 ON CONFLICT(book_id) DO UPDATE SET title=excluded.title,author=excluded.author,
 subject=excluded.subject,publisher=excluded.publisher,tags=excluded.tags; END;
CREATE TRIGGER tags_search_update AFTER UPDATE OF name ON tags BEGIN
 INSERT INTO book_search_text SELECT * FROM book_search_source WHERE book_id IN (SELECT book_id FROM book_tags WHERE tag_id=new.id)
 ON CONFLICT(book_id) DO UPDATE SET title=excluded.title,author=excluded.author,
 subject=excluded.subject,publisher=excluded.publisher,tags=excluded.tags; END;
CREATE TRIGGER book_tags_search_insert AFTER INSERT ON book_tags BEGIN INSERT INTO book_search_text SELECT * FROM book_search_source WHERE book_id=new.book_id
 ON CONFLICT(book_id) DO UPDATE SET title=excluded.title,author=excluded.author,
 subject=excluded.subject,publisher=excluded.publisher,tags=excluded.tags; END;
CREATE TRIGGER book_tags_search_delete AFTER DELETE ON book_tags BEGIN INSERT INTO book_search_text SELECT * FROM book_search_source WHERE book_id=old.book_id
 ON CONFLICT(book_id) DO UPDATE SET title=excluded.title,author=excluded.author,
 subject=excluded.subject,publisher=excluded.publisher,tags=excluded.tags; END;
