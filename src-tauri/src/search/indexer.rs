//! Writing notes into the search index.
//!
//! Indexing always happens *after* the Markdown write has succeeded, in its
//! own transaction. An index failure costs a stale search result; it must
//! never cost a save.

use std::path::Path;

use rusqlite::{params, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::database::Db;
use crate::error::AppError;
use crate::notes::model::NoteId;
use crate::notes::{frontmatter, paths, store};
use crate::search::extract;

/// How many notes to index per transaction during a rebuild. Large enough that
/// commit overhead disappears, small enough that progress moves visibly.
pub const REBUILD_BATCH: usize = 200;

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// What the index already knows about a note, for reconciliation.
#[derive(Debug, Clone)]
pub struct IndexedNote {
    pub path: String,
    pub modified_at: i64,
    pub size_bytes: i64,
}

/// Read one note and write it into the index, replacing whatever was there.
pub fn index_note(db: &Db, root: &Path, id: &NoteId) -> Result<(), AppError> {
    let path = paths::resolve(root, id.as_str())?;

    let bytes = std::fs::read(&path).map_err(|source| AppError::NoteRead {
        path: path.clone(),
        source,
    })?;
    let hash = sha256_hex(&bytes);

    // A file we cannot read as text is skipped rather than indexed as mojibake.
    let content =
        String::from_utf8(bytes).map_err(|_| AppError::NoteNotText { path: path.clone() })?;

    let fs_meta = std::fs::metadata(&path).map_err(|source| AppError::NoteRead {
        path: path.clone(),
        source,
    })?;
    let modified = store::file_mtime(&path);

    let split = frontmatter::split(&content);
    let title = store::derive_title(id, &split.frontmatter, &split.body);
    let extracted = extract::extract(&split.body);

    let id_str = id.as_str().to_string();
    let folder = id.folder().to_string();
    let size = fs_meta.len() as i64;
    let indexed_at = now_unix();

    db.transaction("indexing a note", |tx| {
        // Replace rather than update: the passage set changes shape on every
        // edit, and ON DELETE CASCADE plus the FTS triggers clean up for us.
        if let Some(old_id) = tx
            .query_row(
                "SELECT id FROM notes WHERE path = ?1",
                params![id_str],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
        {
            tx.execute("DELETE FROM passages WHERE note_id = ?1", params![old_id])?;
            tx.execute("DELETE FROM notes_fts WHERE rowid = ?1", params![old_id])?;
            tx.execute("DELETE FROM notes WHERE id = ?1", params![old_id])?;
        }

        tx.execute(
            "INSERT INTO notes
               (path, title, folder, created_at, modified_at, size_bytes, content_hash, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id_str,
                title,
                folder,
                Option::<i64>::None,
                modified,
                size,
                hash,
                indexed_at
            ],
        )?;
        let note_row = tx.last_insert_rowid();

        tx.execute(
            "INSERT INTO notes_fts(rowid, title, body) VALUES (?1, ?2, ?3)",
            params![note_row, title, extracted.plain],
        )?;

        {
            let mut insert = tx.prepare(
                "INSERT INTO passages
                   (note_id, ordinal, heading_path, line_start, line_end, text)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;
            for passage in &extracted.passages {
                insert.execute(params![
                    note_row,
                    passage.ordinal as i64,
                    passage.heading_path,
                    passage.line_start as i64,
                    passage.line_end as i64,
                    passage.text,
                ])?;
            }
        }

        Ok(())
    })?;

    Ok(())
}

/// Drop a note from the index. Passages and their FTS rows follow by cascade.
pub fn remove_note(db: &Db, id: &NoteId) -> Result<(), AppError> {
    let id_str = id.as_str().to_string();
    db.transaction("removing a note from the index", |tx| {
        if let Some(row) = tx
            .query_row(
                "SELECT id FROM notes WHERE path = ?1",
                params![id_str],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
        {
            tx.execute("DELETE FROM passages WHERE note_id = ?1", params![row])?;
            tx.execute("DELETE FROM notes_fts WHERE rowid = ?1", params![row])?;
            tx.execute("DELETE FROM notes WHERE id = ?1", params![row])?;
        }
        Ok(())
    })
}

/// Re-key a note after a rename or move.
///
/// The content did not change, so re-reading the file would be wasted work —
/// but the path, folder, and title all did.
pub fn rename_note(db: &Db, root: &Path, from: &NoteId, to: &NoteId) -> Result<(), AppError> {
    let from_str = from.as_str().to_string();
    let existed = db.with("looking up a renamed note", |conn| {
        conn.query_row(
            "SELECT count(*) FROM notes WHERE path = ?1",
            params![from_str],
            |r| r.get::<_, i64>(0),
        )
    })?;

    if existed == 0 {
        // Never indexed: treat the rename as a first index.
        return index_note(db, root, to);
    }

    // Title is derived from the file, which the rename rewrote, so re-read.
    index_note(db, root, to)?;
    remove_note(db, from)
}

/// Everything the index currently holds, for reconciliation against disk.
pub fn indexed_notes(db: &Db) -> Result<Vec<IndexedNote>, AppError> {
    db.with("listing indexed notes", |conn| {
        let mut stmt = conn.prepare("SELECT path, modified_at, size_bytes FROM notes")?;
        let rows = stmt.query_map([], |row| {
            Ok(IndexedNote {
                path: row.get(0)?,
                modified_at: row.get(1)?,
                size_bytes: row.get(2)?,
            })
        })?;
        rows.collect()
    })
}

pub fn note_count(db: &Db) -> Result<i64, AppError> {
    db.with("counting notes", |conn| {
        conn.query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
    })
}

pub fn passage_count(db: &Db) -> Result<i64, AppError> {
    db.with("counting passages", |conn| {
        conn.query_row("SELECT count(*) FROM passages", [], |r| r.get(0))
    })
}

pub fn set_state(db: &Db, key: &str, value: &str) -> Result<(), AppError> {
    let (key, value) = (key.to_string(), value.to_string());
    db.with("recording index state", |conn| {
        conn.execute(
            "INSERT INTO index_state(key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    })
}

pub fn get_state(db: &Db, key: &str) -> Result<Option<String>, AppError> {
    let key = key.to_string();
    db.with("reading index state", |conn| {
        conn.query_row(
            "SELECT value FROM index_state WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
    })
}

/// Clear the whole index, for a rebuild from scratch.
pub fn clear(db: &Db) -> Result<(), AppError> {
    db.transaction("clearing the index", |tx| {
        tx.execute("DELETE FROM passages", [])?;
        tx.execute("DELETE FROM notes_fts", [])?;
        tx.execute("DELETE FROM notes", [])?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::cache;

    fn setup() -> (Db, tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        cache::bootstrap(&root).unwrap();
        let db = Db::open_in_memory().unwrap();
        (db, dir, root)
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    fn fts_passage_hits(db: &Db, term: &str) -> i64 {
        let term = term.to_string();
        db.with("count", |conn| {
            conn.query_row(
                "SELECT count(*) FROM passages_fts WHERE passages_fts MATCH ?1",
                params![term],
                |r| r.get(0),
            )
        })
        .unwrap()
    }

    #[test]
    fn indexes_a_note_into_both_tables() {
        let (db, _dir, root) = setup();
        write(
            &root,
            "work/gpu.md",
            "# GPU Infrastructure\n\nThe orchestrator stops the node pool too early sometimes.\n",
        );

        index_note(&db, &root, &NoteId::new("work/gpu.md")).unwrap();

        assert_eq!(note_count(&db).unwrap(), 1);
        assert!(passage_count(&db).unwrap() >= 1);
        assert_eq!(fts_passage_hits(&db, "orchestrator"), 1);
    }

    #[test]
    fn reindexing_replaces_rather_than_duplicates() {
        let (db, _dir, root) = setup();
        let id = NoteId::new("work/n.md");
        write(
            &root,
            "work/n.md",
            "# One\n\nthe first version of this note.\n",
        );
        index_note(&db, &root, &id).unwrap();

        write(
            &root,
            "work/n.md",
            "# Two\n\nthe second version of this note.\n",
        );
        index_note(&db, &root, &id).unwrap();

        assert_eq!(note_count(&db).unwrap(), 1, "note was duplicated");
        assert_eq!(
            fts_passage_hits(&db, "first"),
            0,
            "stale text still matches"
        );
        assert_eq!(fts_passage_hits(&db, "second"), 1);
    }

    #[test]
    fn removing_a_note_also_removes_its_passages_from_search() {
        let (db, _dir, root) = setup();
        let id = NoteId::new("work/gone.md");
        write(
            &root,
            "work/gone.md",
            "# Doomed\n\nthis text mentions orchestrator and should disappear.\n",
        );
        index_note(&db, &root, &id).unwrap();
        assert_eq!(fts_passage_hits(&db, "orchestrator"), 1);

        remove_note(&db, &id).unwrap();

        assert_eq!(note_count(&db).unwrap(), 0);
        assert_eq!(passage_count(&db).unwrap(), 0);
        // The real risk with external-content FTS5: rows that survive their
        // source and keep matching forever.
        assert_eq!(
            fts_passage_hits(&db, "orchestrator"),
            0,
            "deleted passages must not still be searchable"
        );
    }

    #[test]
    fn renaming_moves_the_row_without_leaving_the_old_path() {
        let (db, _dir, root) = setup();
        let from = NoteId::new("work/old.md");
        write(
            &root,
            "work/old.md",
            "# Old\n\ncontent that mentions orchestrator here.\n",
        );
        index_note(&db, &root, &from).unwrap();

        // The rename has already happened on disk by the time we are called.
        std::fs::rename(root.join("work/old.md"), root.join("ideas/new.md")).unwrap();
        let to = NoteId::new("ideas/new.md");
        rename_note(&db, &root, &from, &to).unwrap();

        assert_eq!(note_count(&db).unwrap(), 1);
        let folder: String = db
            .with("folder", |conn| {
                conn.query_row("SELECT folder FROM notes", [], |r| r.get(0))
            })
            .unwrap();
        assert_eq!(folder, "ideas");
        assert_eq!(fts_passage_hits(&db, "orchestrator"), 1);
    }

    #[test]
    fn non_text_files_are_refused_not_indexed_as_rubbish() {
        let (db, _dir, root) = setup();
        std::fs::write(root.join("bin.md"), [0xff, 0xfe, 0x00]).unwrap();

        assert!(matches!(
            index_note(&db, &root, &NoteId::new("bin.md")),
            Err(AppError::NoteNotText { .. })
        ));
        assert_eq!(note_count(&db).unwrap(), 0);
    }

    #[test]
    fn clear_empties_everything_including_search() {
        let (db, _dir, root) = setup();
        write(
            &root,
            "a.md",
            "# A\n\nsome text about orchestrator processes.\n",
        );
        index_note(&db, &root, &NoteId::new("a.md")).unwrap();

        clear(&db).unwrap();

        assert_eq!(note_count(&db).unwrap(), 0);
        assert_eq!(passage_count(&db).unwrap(), 0);
        assert_eq!(fts_passage_hits(&db, "orchestrator"), 0);
    }

    #[test]
    fn index_state_round_trips() {
        let (db, _dir, _root) = setup();
        assert!(get_state(&db, "last_full_rebuild").unwrap().is_none());
        set_state(&db, "last_full_rebuild", "2026-09-16T10:00:00Z").unwrap();
        assert_eq!(
            get_state(&db, "last_full_rebuild").unwrap().as_deref(),
            Some("2026-09-16T10:00:00Z")
        );
        set_state(&db, "last_full_rebuild", "later").unwrap();
        assert_eq!(
            get_state(&db, "last_full_rebuild").unwrap().as_deref(),
            Some("later")
        );
    }

    #[test]
    fn indexed_notes_reports_what_reconciliation_needs() {
        let (db, _dir, root) = setup();
        write(
            &root,
            "a.md",
            "# A\n\nenough text here to make a passage.\n",
        );
        index_note(&db, &root, &NoteId::new("a.md")).unwrap();

        let rows = indexed_notes(&db).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "a.md");
        assert!(rows[0].size_bytes > 0);
        assert!(rows[0].modified_at > 0);
    }
}
