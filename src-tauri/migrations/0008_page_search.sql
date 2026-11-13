CREATE VIRTUAL TABLE page_text_fts USING fts5(
 normalized_text,
 content='page_text',content_rowid='id',
 tokenize="unicode61 remove_diacritics 2 categories 'L* N* Co M*'",
 prefix='2 3 4'
);
CREATE TRIGGER page_text_search_insert AFTER INSERT ON page_text BEGIN
 INSERT INTO page_text_fts(rowid,normalized_text) VALUES(new.id,new.normalized_text);
END;
CREATE TRIGGER page_text_search_delete AFTER DELETE ON page_text BEGIN
 INSERT INTO page_text_fts(page_text_fts,rowid,normalized_text)
 VALUES('delete',old.id,old.normalized_text);
END;
CREATE TRIGGER page_text_search_update AFTER UPDATE ON page_text BEGIN
 INSERT INTO page_text_fts(page_text_fts,rowid,normalized_text)
 VALUES('delete',old.id,old.normalized_text);
 INSERT INTO page_text_fts(rowid,normalized_text) VALUES(new.id,new.normalized_text);
END;
INSERT INTO page_text_fts(page_text_fts) VALUES('rebuild');
INSERT INTO page_text_fts(page_text_fts,rank) VALUES('integrity-check',1);
