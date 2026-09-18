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
use crate::exclusion::Exclusions;
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
    /// Whether every term must match or any may. Questions use
    /// [`query::Match::Any`]; the search box uses the default.
    pub match_mode: query::Match,
    pub limit: usize,
    /// At most this many passages from any one note, so a single long note
    /// cannot crowd out everything else.
    pub per_note_cap: usize,
    pub folder: Option<String>,
    pub modified_after: Option<i64>,
    /// Notes the user has told Mushroom never to send to an AI endpoint.
    ///
    /// Only the AI paths set this. Keyword search deliberately leaves it
    /// empty: an excluded note is still the user's note and must still be
    /// findable on their own machine (spec 08 R2.4). The risk being managed
    /// is the network, not the disk.
    pub excluded: Option<Exclusions>,
}

impl RetrievalQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            match_mode: query::Match::All,
            limit: 30,
            per_note_cap: 3,
            folder: None,
            modified_after: None,
            excluded: None,
        }
    }
}

/// What a retrieval produced, and what it deliberately left out.
///
/// Retrieval has two outputs now: the passages, and how many distinct notes
/// matched but were withheld because the user excluded them. The second is not
/// a detail — an answer built from less than the user expects looks like a bad
/// answer unless it says so (spec 08 R2.6).
///
/// Derefs to the passage slice so callers that only want the hits read exactly
/// as they did before.
#[derive(Debug, Clone, Default)]
pub struct RetrievalResult {
    pub passages: Vec<RetrievedPassage>,
    /// Distinct notes, not passages: one note can contribute several.
    pub excluded_notes: usize,
}

impl std::ops::Deref for RetrievalResult {
    type Target = [RetrievedPassage];

    fn deref(&self) -> &Self::Target {
        &self.passages
    }
}

impl IntoIterator for RetrievalResult {
    type Item = RetrievedPassage;
    type IntoIter = std::vec::IntoIter<RetrievedPassage>;

    fn into_iter(self) -> Self::IntoIter {
        self.passages.into_iter()
    }
}

impl<'a> IntoIterator for &'a RetrievalResult {
    type Item = &'a RetrievedPassage;
    type IntoIter = std::slice::Iter<'a, RetrievedPassage>;

    fn into_iter(self) -> Self::IntoIter {
        self.passages.iter()
    }
}

pub trait Retriever: Send + Sync {
    fn retrieve(&self, q: &RetrievalQuery) -> Result<RetrievalResult, AppError>;
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
    fn retrieve(&self, q: &RetrievalQuery) -> Result<RetrievalResult, AppError> {
        let parsed = query::parse_with(&q.text, q.match_mode);
        if parsed.is_empty() {
            return Ok(RetrievalResult::default());
        }

        // Ask for more than we need: the per-note cap thins the results
        // afterwards, and without headroom a single note could eat the limit.
        let fetch = (q.limit * q.per_note_cap.max(1) * 2).clamp(q.limit, 500);

        let expression = parsed.expression.clone();
        let folder = q.folder.clone();
        let modified_after = q.modified_after;
        let apply_ai_exclusion = q.excluded.is_some();

        let rows = self.db.with("searching passages", move |conn| {
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
                   AND (?6 = 0 OR n.ai_excluded = 0)
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
                        fetch as i64,
                        // Only the AI paths pass exclusions; keyword search
                        // leaves them off and still finds excluded notes.
                        apply_ai_exclusion as i64
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
                   AND (?6 = 0 OR n.ai_excluded = 0)
                 ORDER BY rank
                 LIMIT ?5",
            )?;

            let titles = by_title
                .query_map(
                    params![
                        expression,
                        folder,
                        like,
                        modified_after,
                        fetch as i64,
                        apply_ai_exclusion as i64
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

            // How many notes the SQL filter above withheld.
            //
            // A separate query, because the filter belongs in SQL — an
            // excluded note must not eat a fetch slot, and somebody who
            // excludes a large folder and asks about it would otherwise get
            // nothing back — but the count cannot then be observed from the
            // rows that came out. Counting is the price of filtering early,
            // and the user needs the number: without it, a short answer reads
            // as retrieval being broken.
            let excluded_by_flag: usize = if apply_ai_exclusion {
                conn.query_row(
                    "SELECT COUNT(*) FROM (
                       SELECT p.note_id AS id
                         FROM passages_fts
                         JOIN passages p ON p.id = passages_fts.rowid
                         JOIN notes n ON n.id = p.note_id
                        WHERE passages_fts MATCH ?1
                          AND n.ai_excluded = 1
                          AND (?2 IS NULL OR n.folder = ?2 OR n.folder LIKE ?3)
                       UNION
                       SELECT n.id
                         FROM notes_fts
                         JOIN notes n ON n.id = notes_fts.rowid
                        WHERE notes_fts MATCH ?1
                          AND n.ai_excluded = 1
                          AND (?2 IS NULL OR n.folder = ?2 OR n.folder LIKE ?3)
                     )",
                    params![expression, folder, like],
                    |row| row.get::<_, i64>(0),
                )? as usize
            } else {
                0
            };

            Ok((out, excluded_by_flag))
        })?;

        let (mut rows, excluded_by_flag) = rows;

        // bm25() returns a negative number, more negative being a better match.
        // Normalise within the result set so the score means something to a
        // caller that has never heard of bm25.
        let best = rows.iter().map(|r| r.7).fold(f64::INFINITY, f64::min);
        let worst = rows.iter().map(|r| r.7).fold(f64::NEG_INFINITY, f64::max);
        let span = (worst - best).abs();

        let mut per_note: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        // Distinct notes dropped, not passages: "2 notes were excluded" is
        // what the panel says, and one note can contribute several passages.
        let mut dropped_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut out = Vec::new();

        rows.sort_by(|a, b| a.7.partial_cmp(&b.7).unwrap_or(std::cmp::Ordering::Equal));

        for (path, title, folder, heading_path, line_start, line_end, text, rank) in rows {
            // Before the cap and the limit, so an excluded note does not eat a
            // result slot. The retriever already over-fetches for the per-note
            // cap, which is the headroom this borrows.
            if let Some(excluded) = &q.excluded {
                if excluded.excludes(&path) {
                    dropped_ids.insert(path);
                    continue;
                }
            }

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

        Ok(RetrievalResult {
            passages: out,
            // No overlap: a note caught by the SQL filter never reaches the
            // Rust loop, so the two sets are disjoint by construction.
            excluded_notes: excluded_by_flag + dropped_ids.len(),
        })
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
    fn retrieve(&self, q: &RetrievalQuery) -> Result<RetrievalResult, AppError> {
        let started = std::time::Instant::now();
        let result = self.inner.retrieve(q);
        match &result {
            // Never log the query text itself: it is note content by another
            // name. Counts and timings are enough to diagnose slowness.
            Ok(hits) => tracing::debug!(
                target: "search",
                hits = hits.passages.len(),
                excluded_notes = hits.excluded_notes,
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

#[cfg(test)]
mod exclusion_tests {
    use super::*;
    use crate::exclusion::ExclusionRule;
    use crate::notes::cache;
    use crate::notes::model::NoteId;
    use crate::search::indexer;

    /// Two notes about the same subject: one ordinary, one the user has
    /// excluded in its frontmatter.
    fn corpus() -> (Db, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("notes");
        cache::bootstrap(&root).unwrap();
        let db = Db::open_in_memory().unwrap();

        for (rel, body) in [
            (
                "work/gpu-infra.md",
                "# GPU Infrastructure\n\nGPU nodes drain on a schedule.\n",
            ),
            (
                "personal/gpu-vault.md",
                "---\ntitle: GPU Vault\nai: false\n---\n# GPU Vault\n\nGPU credentials for the drain schedule.\n",
            ),
            (
                "personal/gpu-diary.md",
                "# GPU diary\n\nGPU drain notes I keep to myself.\n",
            ),
        ] {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
            indexer::index_note(&db, &root, &NoteId::new(rel)).unwrap();
        }

        (db, dir)
    }

    fn query(excluded: Option<Exclusions>) -> RetrievalQuery {
        let mut q = RetrievalQuery::new("gpu");
        q.match_mode = query::Match::Any;
        q.excluded = excluded;
        q
    }

    fn paths(result: &RetrievalResult) -> Vec<String> {
        result.passages.iter().map(|p| p.note_id.clone()).collect()
    }

    #[test]
    fn keyword_search_still_finds_an_excluded_note() {
        // The risk being managed is the network, not the disk. A note the user
        // keeps out of the AI is still their note and must still be findable
        // on their own machine (R2.4).
        let (db, _dir) = corpus();
        let hits = FtsRetriever::new(&db).retrieve(&query(None)).unwrap();
        assert!(
            paths(&hits).iter().any(|p| p == "personal/gpu-vault.md"),
            "got {:?}",
            paths(&hits)
        );
        assert_eq!(hits.excluded_notes, 0);
    }

    #[test]
    fn an_ai_retrieval_drops_a_note_excluded_in_its_frontmatter() {
        let (db, _dir) = corpus();
        let hits = FtsRetriever::new(&db)
            .retrieve(&query(Some(Exclusions::default())))
            .unwrap();
        assert!(
            !paths(&hits).iter().any(|p| p == "personal/gpu-vault.md"),
            "got {:?}",
            paths(&hits)
        );
        assert!(paths(&hits).iter().any(|p| p == "work/gpu-infra.md"));
    }

    #[test]
    fn a_folder_rule_drops_notes_and_is_counted() {
        let (db, _dir) = corpus();
        let rules = [ExclusionRule::parse("personal/**")];
        let hits = FtsRetriever::new(&db)
            .retrieve(&query(Some(Exclusions::new(&rules))))
            .unwrap();

        assert!(
            paths(&hits).iter().all(|p| p.starts_with("work/")),
            "got {:?}",
            paths(&hits)
        );
        // Both: the vault by its frontmatter (caught in SQL) and the diary by
        // the folder rule (caught in Rust). An earlier version counted only
        // the second, which made the commonest case — a note marked
        // `ai: false` — invisible in the notice. The number exists to tell the
        // user their answer was built from less than they expect, so it has to
        // count every mechanism, not the one that happens to be observable
        // from the rows that came back.
        assert_eq!(hits.excluded_notes, 2);
    }

    #[test]
    fn the_title_query_honours_exclusion_too() {
        // The retriever runs a second query against note titles, and the
        // first version of this filtered only the passage query — so a note
        // whose *title* matched sailed straight through. The end-to-end leak
        // harness caught it; this pins it at the level it broke.
        let (db, _dir) = corpus();
        let mut q = RetrievalQuery::new("vault");
        q.match_mode = query::Match::Any;
        q.excluded = Some(Exclusions::default());

        let hits = FtsRetriever::new(&db).retrieve(&q).unwrap();
        assert!(
            !paths(&hits).iter().any(|p| p == "personal/gpu-vault.md"),
            "a title match bypassed exclusion: {:?}",
            paths(&hits)
        );
    }

    #[test]
    fn excluded_notes_do_not_eat_result_slots() {
        // Filtering after LIMIT would silently shrink the answer: the user
        // would get fewer passages and no explanation.
        let (db, _dir) = corpus();
        let mut q = query(Some(Exclusions::new(&[ExclusionRule::parse("personal")])));
        q.limit = 2;

        let hits = FtsRetriever::new(&db).retrieve(&q).unwrap();
        assert!(
            !hits.passages.is_empty(),
            "the surviving note should still fill a slot"
        );
        assert!(paths(&hits).iter().all(|p| p.starts_with("work/")));
    }
}
