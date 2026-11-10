CREATE TABLE authors (
    id INTEGER PRIMARY KEY,
    display_name TEXT NOT NULL CHECK (trim(display_name) <> ''),
    name_sort_key TEXT NOT NULL
) STRICT;
CREATE INDEX authors_name ON authors(name_sort_key, id);

CREATE TABLE book_authors (
    book_id INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    author_id INTEGER NOT NULL REFERENCES authors(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    PRIMARY KEY (book_id, author_id),
    UNIQUE (book_id, position)
) STRICT;
CREATE INDEX book_authors_author ON book_authors(author_id, book_id);

-- Legacy text does not identify a person across books.
INSERT INTO authors(id, display_name, name_sort_key)
SELECT id, author, lower(author) FROM books WHERE trim(author) <> '';
INSERT INTO book_authors(book_id, author_id, position)
SELECT id, id, 0 FROM authors;

-- A removed record must not be confused with a future book or cached file.
CREATE TABLE catalog_sequences (
    kind TEXT PRIMARY KEY,
    last_id INTEGER NOT NULL
) STRICT;
INSERT INTO catalog_sequences VALUES
    ('book', (SELECT coalesce(max(id), 0) FROM books)),
    ('file', (SELECT coalesce(max(id), 0) FROM book_files));
