//! The search use cases: keeping the index current, and querying it.
//!
//! Owns the database handle. Indexing failures are reported but never
//! propagated into a save — a stale index is an inconvenience, a lost note is
//! not.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::RwLock;

use serde::Serialize;

use crate::database::Db;
use crate::error::AppError;
use crate::notes::model::{NoteId, NoteMeta};
use crate::search::indexer::{self, REBUILD_BATCH};
use crate::search::query;
use crate::search::retriever::{FtsRetriever, LoggingRetriever, RetrievalQuery, Retriever};

/// One note in a search result, with the passage that matched.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: String,
    pub title: String,
    pub folder: String,
    pub modified: i64,
    /// The best-matching passage, for the snippet.
    pub snippet: String,
    pub heading_path: String,
    pub line_start: u32,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    /// What we actually searched for, so the UI can say so on an empty result.
    pub terms: Vec<String>,
    pub stale: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    pub note_count: i64,
    pub passage_count: i64,
    pub database_bytes: u64,
    pub last_full_rebuild: Option<String>,
    pub skipped: usize,
    pub stale: bool,
    pub indexing: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub done: usize,
    pub total: usize,
    pub skipped: usize,
}

pub struct SearchService {
    db: RwLock<Option<Db>>,
    /// Set when indexing failed, so the UI can offer a rebuild (R5.1).
    stale: AtomicBool,
    indexing: AtomicBool,
    cancel: AtomicBool,
    skipped: AtomicU64,
}

impl SearchService {
    pub fn new() -> Self {
        Self {
            db: RwLock::new(None),
            stale: AtomicBool::new(false),
            indexing: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            skipped: AtomicU64::new(0),
        }
    }

    fn lock_err() -> AppError {
        AppError::Internal("search index lock poisoned".into())
    }

    /// Open the index. A failure here leaves search unavailable but must not
    /// stop the application (R5.3).
    pub fn open(&self, path: &Path) -> Result<(), AppError> {
        let db = Db::open(path)?;
        *self.db.write().map_err(|_| Self::lock_err())? = Some(db);
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.db.read().map(|d| d.is_some()).unwrap_or(false)
    }

    pub fn mark_stale(&self) {
        self.stale.store(true, Ordering::Relaxed);
    }

    pub fn is_stale(&self) -> bool {
        self.stale.load(Ordering::Relaxed)
    }

    pub fn cancel_rebuild(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    fn with_db<T>(&self, f: impl FnOnce(&Db) -> Result<T, AppError>) -> Result<T, AppError> {
        let guard = self.db.read().map_err(|_| Self::lock_err())?;
        let db = guard.as_ref().ok_or(AppError::IndexNotOpen)?;
        f(db)
    }

    /// Index one note. Called after a successful Markdown write.
    ///
    /// Returns `Ok` even when indexing fails: the caller has already saved the
    /// user's work, and failing here would misreport that (R2.7).
    pub fn index_note_best_effort(&self, root: &Path, id: &NoteId) {
        let outcome = self.with_db(|db| indexer::index_note(db, root, id));
        if let Err(err) = outcome {
            tracing::warn!(target: "db", note = %id, error = %err, "note could not be indexed");
            self.mark_stale();
        }
    }

    pub fn remove_note_best_effort(&self, id: &NoteId) {
        if let Err(err) = self.with_db(|db| indexer::remove_note(db, id)) {
            tracing::warn!(target: "db", note = %id, error = %err, "note could not be de-indexed");
            self.mark_stale();
        }
    }

    pub fn rename_note_best_effort(&self, root: &Path, from: &NoteId, to: &NoteId) {
        if let Err(err) = self.with_db(|db| indexer::rename_note(db, root, from, to)) {
            tracing::warn!(target: "db", error = %err, "renamed note could not be re-indexed");
            self.mark_stale();
        }
    }

    /// Bring the index in line with what is on disk.
    ///
    /// Compares by modified time and size: anything new, changed, or missing is
    /// handled, and unchanged notes are skipped without being read (R2.2).
    pub fn reconcile(&self, root: &Path, on_disk: &[NoteMeta]) -> Result<IndexProgress, AppError> {
        let indexed = self.with_db(indexer::indexed_notes)?;
        let known: HashMap<&str, &indexer::IndexedNote> =
            indexed.iter().map(|n| (n.path.as_str(), n)).collect();

        let mut done = 0usize;
        let mut skipped = 0usize;

        for note in on_disk {
            let path = note.id.as_str();
            let unchanged = known.get(path).is_some_and(|row| {
                row.modified_at == note.modified && row.size_bytes == note.size_bytes as i64
            });

            if unchanged {
                continue;
            }

            match self.with_db(|db| indexer::index_note(db, root, &note.id)) {
                Ok(()) => done += 1,
                Err(err) => {
                    tracing::warn!(target: "db", note = %note.id, error = %err, "skipped");
                    skipped += 1;
                }
            }
        }

        // Anything the index knows about that is no longer on disk.
        let present: std::collections::HashSet<&str> =
            on_disk.iter().map(|n| n.id.as_str()).collect();
        for row in &indexed {
            if !present.contains(row.path.as_str()) {
                let _ = self.with_db(|db| indexer::remove_note(db, &NoteId::new(&row.path)));
            }
        }

        self.skipped.store(skipped as u64, Ordering::Relaxed);
        if skipped == 0 {
            self.stale.store(false, Ordering::Relaxed);
        }

        tracing::info!(
            target: "db",
            reindexed = done,
            skipped,
            total = on_disk.len(),
            "index reconciled"
        );
        Ok(IndexProgress {
            done,
            total: on_disk.len(),
            skipped,
        })
    }

    /// Rebuild from scratch, reporting progress.
    ///
    /// Reads every note again. Never touches the Markdown — the dialog that
    /// starts this says so, and this is why that promise is true.
    pub fn rebuild(
        &self,
        root: &Path,
        on_disk: &[NoteMeta],
        mut progress: impl FnMut(IndexProgress),
    ) -> Result<IndexProgress, AppError> {
        self.indexing.store(true, Ordering::Relaxed);
        self.cancel.store(false, Ordering::Relaxed);

        let result = (|| {
            self.with_db(indexer::clear)?;

            let mut done = 0usize;
            let mut skipped = 0usize;

            for chunk in on_disk.chunks(REBUILD_BATCH) {
                if self.cancel.load(Ordering::Relaxed) {
                    tracing::info!(target: "db", done, "rebuild cancelled");
                    break;
                }
                for note in chunk {
                    match self.with_db(|db| indexer::index_note(db, root, &note.id)) {
                        Ok(()) => done += 1,
                        Err(_) => skipped += 1,
                    }
                }
                progress(IndexProgress {
                    done,
                    total: on_disk.len(),
                    skipped,
                });
            }

            self.with_db(|db| {
                indexer::set_state(db, "last_full_rebuild", &crate::notes::store::now_rfc3339())
            })?;

            self.skipped.store(skipped as u64, Ordering::Relaxed);
            self.stale.store(skipped > 0, Ordering::Relaxed);

            Ok(IndexProgress {
                done,
                total: on_disk.len(),
                skipped,
            })
        })();

        self.indexing.store(false, Ordering::Relaxed);
        result
    }

    /// Search notes, returning the best passage per note as a snippet.
    pub fn search(
        &self,
        text: &str,
        folder: Option<String>,
        modified_after: Option<i64>,
        limit: usize,
    ) -> Result<SearchResults, AppError> {
        let parsed = query::parse(text);
        if parsed.is_empty() {
            return Ok(SearchResults {
                hits: Vec::new(),
                terms: parsed.terms,
                stale: self.is_stale(),
            });
        }

        let mut request = RetrievalQuery::new(text);
        // One passage per note: the list shows notes, not fragments.
        request.per_note_cap = 1;
        request.limit = limit;
        request.folder = folder;
        request.modified_after = modified_after;

        let passages = self.with_db(|db| {
            // Wrapped so the SEARCH log records timings, and to keep the
            // decorator on the real path rather than only in a test.
            let retriever = LoggingRetriever::new(FtsRetriever::new(db));
            retriever.retrieve(&request)
        })?;

        let modified: HashMap<String, i64> = self.with_db(|db| {
            db.with("reading note times", |conn| {
                let mut stmt = conn.prepare("SELECT path, modified_at FROM notes")?;
                let rows = stmt.query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })?;
                rows.collect()
            })
        })?;

        let hits = passages
            .into_iter()
            .map(|p| SearchHit {
                modified: modified.get(&p.note_id).copied().unwrap_or(0),
                id: p.note_id,
                title: p.note_title,
                folder: p.folder,
                snippet: snippet(&p.text, &parsed.terms),
                heading_path: p.heading_path,
                line_start: p.line_start,
                score: p.score,
            })
            .collect();

        Ok(SearchResults {
            hits,
            terms: parsed.terms,
            stale: self.is_stale(),
        })
    }

    pub fn stats(&self) -> IndexStats {
        let mut stats = IndexStats {
            stale: self.is_stale(),
            indexing: self.indexing.load(Ordering::Relaxed),
            skipped: self.skipped.load(Ordering::Relaxed) as usize,
            ..Default::default()
        };

        let _ = self.with_db(|db| {
            stats.note_count = indexer::note_count(db)?;
            stats.passage_count = indexer::passage_count(db)?;
            stats.last_full_rebuild = indexer::get_state(db, "last_full_rebuild")?;
            stats.database_bytes = std::fs::metadata(&db.path).map(|m| m.len()).unwrap_or(0);
            Ok(())
        });

        stats
    }

    pub fn database_path(&self) -> Option<PathBuf> {
        self.db
            .read()
            .ok()
            .and_then(|d| d.as_ref().map(|db| db.path.clone()))
    }
}

impl Default for SearchService {
    fn default() -> Self {
        Self::new()
    }
}

/// Trim a passage to a window around the first matching term.
///
/// Done here rather than with FTS5's `snippet()` because the retriever already
/// has the passage text, and this keeps the snippet identical whichever
/// retriever produced the passage — which matters once semantic search exists.
fn snippet(text: &str, terms: &[String]) -> String {
    const WINDOW: usize = 220;

    if text.chars().count() <= WINDOW {
        return text.to_string();
    }

    let lower = text.to_lowercase();
    let at = terms
        .iter()
        .filter_map(|t| lower.find(&t.to_lowercase()))
        .min()
        .unwrap_or(0);

    // Work in chars, not bytes: slicing a multi-byte character in half panics.
    let chars: Vec<char> = text.chars().collect();
    let char_at = text[..at].chars().count();

    let start = char_at.saturating_sub(WINDOW / 3);
    let end = (start + WINDOW).min(chars.len());
    let start = end.saturating_sub(WINDOW);

    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(&chars[start..end]);
    if end < chars.len() {
        out.push('…');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::cache;
    use crate::notes::store;

    fn service() -> (SearchService, tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        cache::bootstrap(&root).unwrap();

        let svc = SearchService::new();
        svc.open(&dir.path().join("mushroom.db")).unwrap();

        for (rel, body) in [
            (
                "work/autoqa.md",
                "# AI AutoQA / Batch Incident\n\nThe orchestrator can stop the node pool when it sees no active queue messages, even though transcript-processing work may still remain.\n",
            ),
            (
                "work/gpu-infra.md",
                "# GPU Infrastructure\n\nGPU nodes drain on a schedule. A GPU failure during drain leaves the pool short of capacity.\n",
            ),
            (
                "ideas/rag.md",
                "# RAG ideas\n\nChunking and embeddings for retrieval, nothing about hardware in this note.\n",
            ),
        ] {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, body).unwrap();
        }

        (svc, dir, root)
    }

    fn scan(root: &Path) -> Vec<NoteMeta> {
        store::scan(root).notes
    }

    #[test]
    fn reconcile_indexes_everything_then_skips_unchanged_work() {
        let (svc, _dir, root) = service();
        let notes = scan(&root);

        let first = svc.reconcile(&root, &notes).unwrap();
        assert_eq!(first.done, 3);
        assert_eq!(first.skipped, 0);

        // Nothing changed on disk, so nothing should be re-read.
        let second = svc.reconcile(&root, &notes).unwrap();
        assert_eq!(second.done, 0, "unchanged notes were re-indexed");
    }

    #[test]
    fn reconcile_removes_notes_that_are_gone() {
        let (svc, _dir, root) = service();
        svc.reconcile(&root, &scan(&root)).unwrap();
        assert_eq!(svc.stats().note_count, 3);

        std::fs::remove_file(root.join("ideas/rag.md")).unwrap();
        svc.reconcile(&root, &scan(&root)).unwrap();

        assert_eq!(svc.stats().note_count, 2);
        assert!(
            svc.search("chunking", None, None, 10)
                .unwrap()
                .hits
                .is_empty(),
            "a deleted note is still searchable"
        );
    }

    #[test]
    fn search_finds_the_expected_note_with_a_snippet() {
        let (svc, _dir, root) = service();
        svc.reconcile(&root, &scan(&root)).unwrap();

        let results = svc.search("GPU failure", None, None, 10).unwrap();

        assert!(!results.hits.is_empty(), "no hits for the brief's example");
        assert_eq!(results.hits[0].id, "work/gpu-infra.md");
        assert!(results.hits[0].snippet.to_lowercase().contains("gpu"));
        assert!(results.hits[0].line_start >= 1);
    }

    #[test]
    fn each_note_appears_at_most_once() {
        let (svc, _dir, root) = service();
        svc.reconcile(&root, &scan(&root)).unwrap();

        let results = svc.search("pool", None, None, 10).unwrap();
        let mut ids: Vec<_> = results.hits.iter().map(|h| h.id.clone()).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "a note appeared twice in the list");
    }

    #[test]
    fn an_empty_query_returns_nothing_and_does_not_error() {
        let (svc, _dir, root) = service();
        svc.reconcile(&root, &scan(&root)).unwrap();

        for q in ["", "   ", "*()"] {
            let results = svc.search(q, None, None, 10).unwrap();
            assert!(results.hits.is_empty(), "{q:?}");
        }
    }

    #[test]
    fn rebuild_reports_progress_and_records_when_it_happened() {
        let (svc, _dir, root) = service();
        let notes = scan(&root);

        let mut seen = Vec::new();
        let done = svc.rebuild(&root, &notes, |p| seen.push(p.done)).unwrap();

        assert_eq!(done.done, 3);
        assert!(!seen.is_empty(), "progress was never reported");
        assert!(svc.stats().last_full_rebuild.is_some());
    }

    #[test]
    fn rebuild_never_touches_the_markdown() {
        let (svc, _dir, root) = service();
        let notes = scan(&root);

        let before: Vec<_> = notes
            .iter()
            .map(|n| std::fs::read(root.join(n.id.as_str())).unwrap())
            .collect();

        svc.rebuild(&root, &notes, |_| {}).unwrap();

        for (note, original) in notes.iter().zip(before) {
            let after = std::fs::read(root.join(note.id.as_str())).unwrap();
            assert_eq!(after, original, "{} was modified by a rebuild", note.id);
        }
    }

    #[test]
    fn stats_describe_the_index() {
        let (svc, _dir, root) = service();
        svc.reconcile(&root, &scan(&root)).unwrap();

        let stats = svc.stats();
        assert_eq!(stats.note_count, 3);
        assert!(stats.passage_count >= 3);
        assert!(stats.database_bytes > 0);
        assert!(!stats.indexing);
    }

    #[test]
    fn indexing_failure_marks_stale_without_erroring() {
        let (svc, _dir, root) = service();
        // A file that is not text cannot be indexed.
        std::fs::write(root.join("bad.md"), [0xff, 0xfe]).unwrap();

        svc.index_note_best_effort(&root, &NoteId::new("bad.md"));

        assert!(svc.is_stale(), "a failed index should mark the index stale");
    }

    #[test]
    fn snippet_windows_around_the_match_without_splitting_characters() {
        let long = format!(
            "{} café 日本語 needle {}",
            "x ".repeat(300),
            "y ".repeat(300)
        );
        let out = snippet(&long, &["needle".to_string()]);

        assert!(out.contains("needle"));
        assert!(out.chars().count() < long.chars().count());
        assert!(out.starts_with('…') || out.ends_with('…'));
    }
}
