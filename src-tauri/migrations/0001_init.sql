-- Mushroom search index.
--
-- Everything here is derived from the Markdown files and can be rebuilt from
-- them. Nothing the user typed exists only in this database.

CREATE TABLE notes (
    id            INTEGER PRIMARY KEY,
    -- Relative to the notes root, '/' separated. The note's identity.
    path          TEXT NOT NULL UNIQUE,
    title         TEXT NOT NULL,
    folder        TEXT NOT NULL,
    created_at    INTEGER,
    -- Filesystem mtime; drives reconciliation and list ordering.
    modified_at   INTEGER NOT NULL,
    size_bytes    INTEGER NOT NULL,
    -- sha256 of the file bytes, so an unchanged file can be skipped for sure.
    content_hash  TEXT NOT NULL,
    indexed_at    INTEGER NOT NULL
);

CREATE INDEX idx_notes_folder ON notes(folder);
CREATE INDEX idx_notes_modified ON notes(modified_at DESC);

-- A passage is a heading-delimited section with its line range. Retrieval
-- returns these rather than whole files, which is what lets a search result
-- open at the right line and what milestone 5 will cite.
CREATE TABLE passages (
    id           INTEGER PRIMARY KEY,
    note_id      INTEGER NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
    ordinal      INTEGER NOT NULL,
    heading_path TEXT NOT NULL,
    line_start   INTEGER NOT NULL,
    line_end     INTEGER NOT NULL,
    text         TEXT NOT NULL
);

CREATE INDEX idx_passages_note ON passages(note_id);

-- Note-level index, for ranking whole notes. An ordinary FTS5 table keyed by
-- notes.id: it keeps its own copy of the text, which costs a little disk but
-- makes deletes a plain DELETE rather than FTS5's 'delete' incantation.
CREATE VIRTUAL TABLE notes_fts USING fts5(
    title,
    body,
    tokenize='porter unicode61 remove_diacritics 2'
);

-- Passage-level index, external-content over `passages` so passage text is
-- stored once.
CREATE VIRTUAL TABLE passages_fts USING fts5(
    heading_path,
    text,
    content='passages',
    content_rowid='id',
    tokenize='porter unicode61 remove_diacritics 2'
);

-- External-content FTS5 does not track its source table by itself. Triggers
-- are the only way to stay in sync through ON DELETE CASCADE, which happens
-- inside SQLite where application code never runs: deleting a note would
-- otherwise leave its passages searchable forever.
CREATE TRIGGER passages_ai AFTER INSERT ON passages BEGIN
    INSERT INTO passages_fts(rowid, heading_path, text)
    VALUES (new.id, new.heading_path, new.text);
END;

CREATE TRIGGER passages_ad AFTER DELETE ON passages BEGIN
    INSERT INTO passages_fts(passages_fts, rowid, heading_path, text)
    VALUES ('delete', old.id, old.heading_path, old.text);
END;

CREATE TRIGGER passages_au AFTER UPDATE ON passages BEGIN
    INSERT INTO passages_fts(passages_fts, rowid, heading_path, text)
    VALUES ('delete', old.id, old.heading_path, old.text);
    INSERT INTO passages_fts(rowid, heading_path, text)
    VALUES (new.id, new.heading_path, new.text);
END;

CREATE TABLE index_state (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
