//! Search behaviour against a real notes folder and a real database file.
//!
//! The promise being tested is that the index is disposable: delete it and
//! nothing is lost but time.

use std::path::{Path, PathBuf};

use mushroom_lib::notes::{cache, store};
use mushroom_lib::search::service::SearchService;

fn corpus() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("notes");
    let db_path = dir.path().join("mushroom.db");
    cache::bootstrap(&root).unwrap();

    for (rel, body) in [
        (
            "work/autoqa.md",
            "---\ntitle: AI AutoQA / Batch Incident\n---\n# AI AutoQA\n\nThe orchestrator can stop the node pool when it sees no active queue messages, even though transcript-processing work may still remain.\n\n## Root cause\n\nA GPU failure during drain leaves the batch half finished.\n",
        ),
        (
            "work/gpu-infra.md",
            "# GPU Infrastructure\n\nGPU nodes drain on a schedule. Capacity planning assumes no GPU failure during the drain window.\n",
        ),
        (
            "work/realtime.md",
            "# Real-Time Guidance\n\nLatency budget for guidance. Mentions the GPU pool only in passing, as a dependency.\n",
        ),
        (
            "ideas/rag.md",
            "# RAG ideas\n\nChunking, embeddings, hybrid retrieval. Nothing about hardware in this note at all.\n",
        ),
    ] {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    (dir, root, db_path)
}

fn indexed(root: &Path, db_path: &Path) -> SearchService {
    let svc = SearchService::new();
    svc.open(db_path).unwrap();
    svc.reconcile(root, &store::scan(root).notes).unwrap();
    svc
}

/// Hash every note so a test can prove the Markdown was not touched.
fn fingerprint(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = walkdir::WalkDir::new(root)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            (
                e.path().strip_prefix(root).unwrap().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Task 23: the example from the brief has to work.
#[test]
fn the_briefs_own_example_finds_the_right_notes() {
    let (_dir, root, db_path) = corpus();
    let svc = indexed(&root, &db_path);

    let results = svc.search("GPU failure", None, None, 10).unwrap();

    assert!(!results.hits.is_empty(), "no results for 'GPU failure'");

    let ids: Vec<&str> = results.hits.iter().map(|h| h.id.as_str()).collect();
    assert!(
        ids.contains(&"work/gpu-infra.md") && ids.contains(&"work/autoqa.md"),
        "expected the two GPU notes, got {ids:?}"
    );
    assert!(
        !ids.contains(&"ideas/rag.md"),
        "the note about chunking should not match a hardware query: {ids:?}"
    );

    // Every hit carries a usable snippet and a line to open at.
    for hit in &results.hits {
        assert!(!hit.snippet.trim().is_empty(), "empty snippet for {}", hit.id);
        assert!(hit.line_start >= 1, "no line for {}", hit.id);
    }
}

#[test]
fn a_phrase_query_is_narrower_than_the_same_loose_words() {
    let (_dir, root, db_path) = corpus();
    let svc = indexed(&root, &db_path);

    let loose = svc.search("node pool", None, None, 10).unwrap();
    let phrase = svc.search("\"node pool\"", None, None, 10).unwrap();

    assert!(!phrase.hits.is_empty(), "the exact phrase exists in a note");
    assert!(
        phrase.hits.len() <= loose.hits.len(),
        "a phrase should not match more widely than its words"
    );
}

#[test]
fn search_can_be_scoped_to_a_folder() {
    let (_dir, root, db_path) = corpus();
    let svc = indexed(&root, &db_path);

    let all = svc.search("retrieval", None, None, 10).unwrap();
    assert!(!all.hits.is_empty());

    let scoped = svc
        .search("retrieval", Some("work".to_string()), None, 10)
        .unwrap();
    assert!(
        scoped.hits.iter().all(|h| h.folder.starts_with("work")),
        "folder scope leaked: {:?}",
        scoped.hits.iter().map(|h| &h.folder).collect::<Vec<_>>()
    );
}

/// Task 24: the index is a cache. Deleting it must lose nothing but time.
#[test]
fn deleting_the_index_loses_nothing_and_rebuilds() {
    let (_dir, root, db_path) = corpus();

    let before = {
        let svc = indexed(&root, &db_path);
        let hits = svc.search("orchestrator", None, None, 10).unwrap();
        assert!(!hits.hits.is_empty());
        fingerprint(&root)
    };

    // Simulate the user (or a crash) deleting the database between runs.
    std::fs::remove_file(&db_path).unwrap();
    for suffix in ["-wal", "-shm"] {
        let _ = std::fs::remove_file(PathBuf::from(format!("{}{suffix}", db_path.display())));
    }
    assert!(!db_path.exists());

    let svc = indexed(&root, &db_path);

    let after = fingerprint(&root);
    assert_eq!(before, after, "rebuilding the index modified the notes");

    let hits = svc.search("orchestrator", None, None, 10).unwrap();
    assert!(!hits.hits.is_empty(), "search did not come back after a rebuild");
    assert_eq!(svc.stats().note_count, 4);
}

#[test]
fn a_corrupt_index_recovers_without_losing_notes() {
    let (_dir, root, db_path) = corpus();
    indexed(&root, &db_path);
    let before = fingerprint(&root);

    // Scribble over the database while the app is not running.
    std::fs::write(&db_path, b"not a database at all").unwrap();

    let svc = indexed(&root, &db_path);

    assert_eq!(fingerprint(&root), before, "notes were touched by recovery");
    assert_eq!(svc.stats().note_count, 4, "index did not rebuild");
    assert!(!svc.search("orchestrator", None, None, 10).unwrap().hits.is_empty());
}

#[test]
fn editing_a_note_updates_what_search_finds() {
    let (_dir, root, db_path) = corpus();
    let svc = indexed(&root, &db_path);

    assert!(!svc.search("orchestrator", None, None, 10).unwrap().hits.is_empty());

    std::fs::write(
        root.join("work/autoqa.md"),
        "# AI AutoQA\n\nCompletely rewritten to talk about kittens instead.\n",
    )
    .unwrap();
    svc.reconcile(&root, &store::scan(&root).notes).unwrap();

    let stale = svc.search("orchestrator", None, None, 10).unwrap();
    assert!(
        !stale.hits.iter().any(|h| h.id == "work/autoqa.md"),
        "the old text is still searchable"
    );
    assert!(!svc.search("kittens", None, None, 10).unwrap().hits.is_empty());
}

/// Task 25: index and search at a realistic size.
///
/// Ignored by default because it writes 5,000 files; run with
/// `cargo test --test search_integration -- --ignored --nocapture`.
#[test]
#[ignore]
fn scales_to_five_thousand_notes() {
    use std::time::Instant;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("notes");
    let db_path = dir.path().join("mushroom.db");
    cache::bootstrap(&root).unwrap();

    for folder in 0..50 {
        let path = root.join(format!("folder-{folder:02}"));
        std::fs::create_dir_all(&path).unwrap();
        for note in 0..100 {
            let body = format!(
                "---\ntitle: Note {folder}-{note}\n---\n# Note {folder}-{note}\n\n\
                 {}\n\n## Details\n\nThe orchestrator stops the node pool early in \
                 case {folder}-{note}.\n",
                "Realistic paragraph text about capacity and scheduling. ".repeat(10)
            );
            std::fs::write(path.join(format!("note-{note:03}.md")), body).unwrap();
        }
    }

    let on_disk = store::scan(&root).notes;
    assert_eq!(on_disk.len(), 5000);

    let svc = SearchService::new();
    svc.open(&db_path).unwrap();

    let started = Instant::now();
    let progress = svc.rebuild(&root, &on_disk, |_| {}).unwrap();
    let build_time = started.elapsed();
    println!("full rebuild of {} notes in {:?}", progress.done, build_time);
    assert_eq!(progress.done, 5000);

    // A second reconcile has nothing to do, which is the startup case.
    let started = Instant::now();
    svc.reconcile(&root, &on_disk).unwrap();
    let reconcile_time = started.elapsed();
    println!("startup reconciliation in {reconcile_time:?}");

    for query in ["orchestrator", "node pool", "capacity scheduling"] {
        let started = Instant::now();
        let results = svc.search(query, None, None, 50).unwrap();
        let elapsed = started.elapsed();
        println!(
            "search {query:?}: {} hits in {elapsed:?}",
            results.hits.len()
        );
        assert!(!results.hits.is_empty());
        assert!(
            elapsed.as_millis() < 100,
            "search {query:?} took {elapsed:?}, budget is 100ms"
        );
    }

    let stats = svc.stats();
    println!(
        "index: {} notes, {} passages, {:.1} MB",
        stats.note_count,
        stats.passage_count,
        stats.database_bytes as f64 / 1_048_576.0
    );

    assert!(
        reconcile_time.as_secs_f64() < 2.0,
        "startup reconciliation took {reconcile_time:?}, budget is 2s"
    );
}
