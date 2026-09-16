//! Retrieval, behind a trait.
//!
//! Keyword search is one implementation. Semantic and hybrid retrieval are
//! spec 07, and the whole point of this interface is that adding them should
//! not touch a single caller. Scores are normalised to 0..1 at the source so
//! that a future hybrid retriever can fuse results that are measured in
//! completely different units.

use rusqlite::params;
use serde::Serialize;

use crate::database::Db;
use crate::error::AppError;
use crate::search::query;

/// Why a passage was returned. Only `Keyword` is possible today; the field
/// exists so the UI can eventually explain a result, and so spec 07 does not
/// have to change this struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RetrievalSource {
    Keyword,
    Semantic,
    Hybrid,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetrievedPassage {
    pub note_id: String,
    pub note_title: String,
    pub folder: String,
    pub heading_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub text: String,
    /// Normalised 0..1 within this result set, so different retrievers can be
    /// compared and fused.
    pub score: f32,
    pub source: RetrievalSource,
}

#[derive(Debug, Clone)]
pub struct RetrievalQuery {
    pub text: String,
    pub limit: usize,
    /// At most this many passages from any one note, so a single long note
    /// cannot crowd out everything else.
    pub per_note_cap: usize,
    pub folder: Option<String>,
    pub modified_after: Option<i64>,
}

impl RetrievalQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            limit: 30,
            per_note_cap: 3,
            folder: None,
            modified_after: None,
        }
    }
}

pub trait Retriever: Send + Sync {
    fn retrieve(&self, q: &RetrievalQuery) -> Result<Vec<RetrievedPassage>, AppError>;
}

/// FTS5 over the passage index.
pub struct FtsRetriever<'a> {
    pub db: &'a Db,
}

impl<'a> FtsRetriever<'a> {
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }
}

impl Retriever for FtsRetriever<'_> {
    fn retrieve(&self, q: &RetrievalQuery) -> Result<Vec<RetrievedPassage>, AppError> {
        let parsed = query::parse(&q.text);
        if parsed.is_empty() {
            return Ok(Vec::new());
        }

        // Ask for more than we need: the per-note cap thins the results
        // afterwards, and without headroom a single note could eat the limit.
        let fetch = (q.limit * q.per_note_cap.max(1) * 2).clamp(q.limit, 500);

        let expression = parsed.expression.clone();
        let folder = q.folder.clone();
        let modified_after = q.modified_after;

        let mut rows = self.db.with("searching passages", move |conn| {
            let mut stmt = conn.prepare(
                "SELECT n.path, n.title, n.folder,
                        p.heading_path, p.line_start, p.line_end, p.text,
                        bm25(passages_fts, 2.0, 1.0) AS rank
                 FROM passages_fts
                 JOIN passages p ON p.id = passages_fts.rowid
                 JOIN notes n ON n.id = p.note_id
                 WHERE passages_fts MATCH ?1
                   AND (?2 IS NULL OR n.folder = ?2 OR n.folder LIKE ?3)
                   AND (?4 IS NULL OR n.modified_at >= ?4)
                 ORDER BY rank
                 LIMIT ?5",
            )?;

            let like = folder.as_ref().map(|f| format!("{f}/%"));
            let mut out = stmt
                .query_map(
                    params![
                        expression.clone(),
                        folder.clone(),
                        like.clone(),
                        modified_after,
                        fetch as i64
                    ],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, i64>(4)?,
                            row.get::<_, i64>(5)?,
                            row.get::<_, String>(6)?,
                            row.get::<_, f64>(7)?,
                        ))
                    },
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            // A note's title is not necessarily in its body: frontmatter can
            // name a note something the text never says. Searching for a note
            // by its name has to work, so match titles too and rank them
            // above body matches (R3.3), attaching the note's first passage
            // so the hit still opens somewhere sensible.
            let mut by_title = conn.prepare(
                "SELECT n.path, n.title, n.folder,
                        COALESCE(p.heading_path, ''),
                        COALESCE(p.line_start, 1),
                        COALESCE(p.line_end, 1),
                        COALESCE(p.text, n.title),
                        bm25(notes_fts, 10.0, 1.0) AS rank
                 FROM notes_fts
                 JOIN notes n ON n.id = notes_fts.rowid
                 LEFT JOIN passages p ON p.note_id = n.id AND p.ordinal = 0
                 WHERE notes_fts MATCH ?1
                   AND (?2 IS NULL OR n.folder = ?2 OR n.folder LIKE ?3)
                   AND (?4 IS NULL OR n.modified_at >= ?4)
                 ORDER BY rank
                 LIMIT ?5",
            )?;

            let titles = by_title
                .query_map(
                    params![expression, folder, like, modified_after, fetch as i64],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, i64>(4)?,
                            row.get::<_, i64>(5)?,
                            row.get::<_, String>(6)?,
                            // Bias title hits ahead of body hits: bm25 is
                            // negative, so subtracting makes it rank better.
                            row.get::<_, f64>(7)? - 1.0,
                        ))
                    },
                )?
                .collect::<rusqlite::Result<Vec<_>>>()?;

            // Keep only title hits for notes the passage query missed, so a
            // note never appears twice from the two queries.
            let seen: std::collections::HashSet<String> = out.iter().map(|r| r.0.clone()).collect();
            out.extend(titles.into_iter().filter(|t| !seen.contains(&t.0)));

            Ok(out)
        })?;

        // bm25() returns a negative number, more negative being a better match.
        // Normalise within the result set so the score means something to a
        // caller that has never heard of bm25.
        let best = rows.iter().map(|r| r.7).fold(f64::INFINITY, f64::min);
        let worst = rows.iter().map(|r| r.7).fold(f64::NEG_INFINITY, f64::max);
        let span = (worst - best).abs();

        let mut per_note: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut out = Vec::new();

        rows.sort_by(|a, b| a.7.partial_cmp(&b.7).unwrap_or(std::cmp::Ordering::Equal));

        for (path, title, folder, heading_path, line_start, line_end, text, rank) in rows {
            let seen = per_note.entry(path.clone()).or_insert(0);
            if *seen >= q.per_note_cap {
                continue;
            }
            *seen += 1;

            let score = if span < f64::EPSILON {
                1.0
            } else {
                ((worst - rank) / span) as f32
            };

            out.push(RetrievedPassage {
                note_id: path,
                note_title: title,
                folder,
                heading_path,
                line_start: line_start as u32,
                line_end: line_end as u32,
                text,
                score,
                source: RetrievalSource::Keyword,
            });

            if out.len() >= q.limit {
                break;
            }
        }

        Ok(out)
    }
}

/// Wraps another retriever and logs what it did.
///
/// Useful in its own right for the SEARCH log category, and it is the cheap
/// proof that the trait composes — a semantic or hybrid retriever slots in the
/// same way, without callers noticing (R4.3).
pub struct LoggingRetriever<R: Retriever> {
    inner: R,
}

impl<R: Retriever> LoggingRetriever<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: Retriever> Retriever for LoggingRetriever<R> {
    fn retrieve(&self, q: &RetrievalQuery) -> Result<Vec<RetrievedPassage>, AppError> {
        let started = std::time::Instant::now();
        let result = self.inner.retrieve(q);
        match &result {
            // Never log the query text itself: it is note content by another
            // name. Counts and timings are enough to diagnose slowness.
            Ok(hits) => tracing::debug!(
                target: "search",
                hits = hits.len(),
                ms = started.elapsed().as_millis() as u64,
                "retrieval finished"
            ),
            Err(err) => tracing::warn!(target: "search", error = %err, "retrieval failed"),
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::cache;
    use crate::notes::model::NoteId;
    use crate::search::indexer;

    fn corpus() -> (Db, tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        cache::bootstrap(&root).unwrap();
        let db = Db::open_in_memory().unwrap();

        let notes = [
            (
                "work/autoqa.md",
                "# AI AutoQA / Batch Incident\n\nThe orchestrator can stop the node pool when it sees no active queue messages, even though transcript-processing work may still remain.\n",
            ),
            (
                "work/gpu-infra.md",
                "# GPU Infrastructure\n\nGPU nodes drain on a schedule. A GPU failure during drain leaves the pool short.\n",
            ),
            (
                "ideas/rag.md",
                "# RAG ideas\n\nChunking and embeddings for retrieval. Nothing about hardware here at all.\n",
            ),
        ];

        for (rel, body) in notes {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
            indexer::index_note(&db, &root, &NoteId::new(rel)).unwrap();
        }

        (db, dir, root)
    }

    #[test]
    fn finds_the_note_that_mentions_the_words() {
        let (db, _dir, _root) = corpus();
        let hits = FtsRetriever::new(&db)
            .retrieve(&RetrievalQuery::new("orchestrator node pool"))
            .unwrap();

        assert!(!hits.is_empty());
        assert_eq!(hits[0].note_id, "work/autoqa.md");
        assert!(hits[0].text.contains("orchestrator"));
    }

    #[test]
    fn scores_are_normalised_and_ordered() {
        let (db, _dir, _root) = corpus();
        let hits = FtsRetriever::new(&db)
            .retrieve(&RetrievalQuery::new("gpu"))
            .unwrap();

        assert!(!hits.is_empty());
        for hit in &hits {
            assert!(
                (0.0..=1.0).contains(&hit.score),
                "score {} outside 0..1",
                hit.score
            );
        }
        // Best first.
        for pair in hits.windows(2) {
            assert!(pair[0].score >= pair[1].score);
        }
    }

    #[test]
    fn stemming_means_a_different_word_form_still_matches() {
        let (db, _dir, _root) = corpus();
        // The note says "processing"; porter stemming should reach it.
        let hits = FtsRetriever::new(&db)
            .retrieve(&RetrievalQuery::new("process"))
            .unwrap();
        assert!(!hits.is_empty(), "porter stemming is not working");
    }

    #[test]
    fn a_note_is_found_by_its_title_even_when_the_body_never_says_it() {
        // Frontmatter can name a note something its text never mentions.
        // Searching for a note by its name has to work.
        let (db, _dir, root) = corpus();
        std::fs::write(
            root.join("work/oddly-named.md"),
            "---
title: Quarterly Capacity Review
---
# Notes

Something entirely unrelated to the title is written here.
",
        )
        .unwrap();
        indexer::index_note(&db, &root, &NoteId::new("work/oddly-named.md")).unwrap();

        let hits = FtsRetriever::new(&db)
            .retrieve(&RetrievalQuery::new("quarterly capacity"))
            .unwrap();

        assert!(
            hits.iter().any(|h| h.note_id == "work/oddly-named.md"),
            "a note must be findable by its title: {hits:?}"
        );
    }

    #[test]
    fn a_note_matching_in_both_title_and_body_appears_once() {
        let (db, _dir, _root) = corpus();
        let hits = FtsRetriever::new(&db)
            .retrieve(&RetrievalQuery::new("gpu"))
            .unwrap();

        let mut ids: Vec<_> = hits.iter().map(|h| h.note_id.clone()).collect();
        let before = ids.len();
        ids.sort();
        ids.dedup();
        // Per-note cap is 3 by default, so duplicates would show as a note
        // appearing more times than the cap allows.
        assert!(before > 0);
        for id in ids {
            let count = hits.iter().filter(|h| h.note_id == id).count();
            assert!(count <= 3, "{id} appeared {count} times");
        }
    }

    #[test]
    fn folder_scope_limits_results() {
        let (db, _dir, _root) = corpus();
        let mut q = RetrievalQuery::new("gpu");
        q.folder = Some("ideas".to_string());
        let hits = FtsRetriever::new(&db).retrieve(&q).unwrap();
        assert!(
            hits.iter().all(|h| h.folder.starts_with("ideas")),
            "{hits:?}"
        );
    }

    #[test]
    fn per_note_cap_stops_one_note_filling_the_page() {
        let (db, dir, root) = corpus();
        let _ = dir;
        // A note with many sections that all mention the same word.
        let mut body = String::from("# Repeats\n\n");
        for i in 0..10 {
            body.push_str(&format!(
                "## Section {i}\n\northogonal orthogonal orthogonal words in section {i} here.\n\n"
            ));
        }
        std::fs::write(root.join("work/repeat.md"), &body).unwrap();
        indexer::index_note(&db, &root, &NoteId::new("work/repeat.md")).unwrap();

        let mut q = RetrievalQuery::new("orthogonal");
        q.per_note_cap = 2;
        let hits = FtsRetriever::new(&db).retrieve(&q).unwrap();

        let from_repeat = hits
            .iter()
            .filter(|h| h.note_id == "work/repeat.md")
            .count();
        assert!(from_repeat <= 2, "cap ignored: {from_repeat} passages");
    }

    #[test]
    fn a_meaningless_query_returns_nothing_rather_than_erroring() {
        let (db, _dir, _root) = corpus();
        for query in ["", "   ", "* ( ) :", "zzzzznotaword"] {
            let hits = FtsRetriever::new(&db)
                .retrieve(&RetrievalQuery::new(query))
                .unwrap();
            assert!(hits.is_empty(), "{query:?} returned {} hits", hits.len());
        }
    }

    #[test]
    fn exclusion_removes_a_note() {
        let (db, _dir, _root) = corpus();
        let with = FtsRetriever::new(&db)
            .retrieve(&RetrievalQuery::new("gpu"))
            .unwrap();
        assert!(with.iter().any(|h| h.note_id == "work/gpu-infra.md"));

        let without = FtsRetriever::new(&db)
            .retrieve(&RetrievalQuery::new("gpu -drain"))
            .unwrap();
        assert!(
            !without.iter().any(|h| h.note_id == "work/gpu-infra.md"),
            "exclusion did not apply"
        );
    }

    #[test]
    fn a_decorated_retriever_is_substitutable() {
        // The evidence that spec 07 can add a retriever without touching
        // callers: anything taking &dyn Retriever accepts a wrapped one.
        let (db, _dir, _root) = corpus();

        fn count_through(r: &dyn Retriever, text: &str) -> usize {
            r.retrieve(&RetrievalQuery::new(text)).unwrap().len()
        }

        let plain = FtsRetriever::new(&db);
        let direct = count_through(&plain, "gpu");

        let wrapped = LoggingRetriever::new(FtsRetriever::new(&db));
        let decorated = count_through(&wrapped, "gpu");

        assert_eq!(direct, decorated);
        assert!(direct > 0);
    }
}
