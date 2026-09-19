//! Performance measurements against a real corpus (R5).
//!
//! Ignored by default: these are measurements, not assertions about the
//! machine they happen to run on. A slow CI box failing the suite would teach
//! us to loosen the thresholds rather than to fix the app.
//!
//!   node scripts/make-corpus.mjs C:\Users\you\Mushroom\bench-notes
//!   $env:MUSHROOM_BENCH_NOTES = "C:\Users\you\Mushroom\bench-notes"
//!   cargo test --release --test bench -- --ignored --nocapture
//!
//! Always `--release`: a debug build measures rustc's inlining decisions, not
//! the product. The remaining two numbers in R5 — time to an interactive
//! window and idle memory — come from `scripts/bench.ps1`, because they are
//! properties of the running application rather than of the library.

use std::path::PathBuf;
use std::time::Instant;

use mushroom_lib::notes::{model::NoteId, store};
use mushroom_lib::search::service::SearchService;

/// Percentile of a sorted sample, nearest-rank.
fn percentile(sorted: &[u128], p: f64) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

/// Microseconds in, milliseconds out — a millisecond clock rounds a 1.6 ms
/// operation to "1 ms" or "2 ms" depending on nothing at all.
fn report(label: &str, mut samples: Vec<u128>) {
    samples.sort_unstable();
    let total: u128 = samples.iter().sum();
    let ms = |micros: u128| micros as f64 / 1000.0;
    println!(
        "{label}: n={} median={:.1} ms p95={:.1} ms max={:.1} ms mean={:.1} ms",
        samples.len(),
        ms(percentile(&samples, 50.0)),
        ms(percentile(&samples, 95.0)),
        ms(samples.last().copied().unwrap_or(0)),
        ms(total / samples.len() as u128),
    );
}

/// The corpus to measure against, or `None` when the env var is unset.
fn corpus() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var("MUSHROOM_BENCH_NOTES").ok()?);
    assert!(
        root.is_dir(),
        "MUSHROOM_BENCH_NOTES is set to {root:?}, which is not a folder. \
         Generate one with `node scripts/make-corpus.mjs <folder>`."
    );
    Some(root)
}

macro_rules! bench_root {
    () => {
        match corpus() {
            Some(root) => root,
            None => {
                println!("skipped: set MUSHROOM_BENCH_NOTES to a corpus folder");
                return;
            }
        }
    };
}

/// R5.3 — keyword search over 5,000 notes under 100 ms.
///
/// Queries deliberately span the range that matters: a single common term
/// (the widest result set), a two-word AND (the intersection), and a term
/// that matches nothing (which still has to scan).
#[test]
#[ignore = "measurement; needs MUSHROOM_BENCH_NOTES"]
fn search_latency() {
    let root = bench_root!();
    let dir = tempfile::tempdir().unwrap();
    let svc = SearchService::new();
    svc.open(&dir.path().join("mushroom.db")).unwrap();

    let scan = store::scan(&root);
    println!(
        "corpus: {} notes under {}",
        scan.notes.len(),
        root.display()
    );

    let started = Instant::now();
    svc.reconcile(&root, &scan.notes).unwrap();
    println!("full index build: {} ms", started.elapsed().as_millis());

    // The topic terms are in roughly 7% of the corpus, which is what a
    // meaningful search term looks like in a real notebook. `mireshi` is the
    // commonest word the generator produced — it is in every note, so ranking
    // has to score the whole corpus. That is the worst case FTS5 can be
    // handed, and it is the number worth holding the app to.
    const QUERIES: [&str; 7] = [
        "capacity",
        "drain window",
        "orchestrator batch",
        "GPU node pool drain",
        "compliance recording scorecard",
        "mireshi",
        "zzzznothingmatchesthis",
    ];

    // Once round the loop untimed: the first query pays for page cache and
    // SQLite's own lazy setup, which no real user ever waits for twice.
    for q in QUERIES {
        svc.search(q, None, None, 50).unwrap();
    }

    // Per query, not pooled: an aggregate hides which shape of query is slow,
    // and "which one" is the only part that tells you what to fix.
    let mut all = Vec::new();
    for q in QUERIES {
        let mut samples = Vec::new();
        let mut matched = 0;
        for _ in 0..20 {
            let started = Instant::now();
            let results = svc.search(q, None, None, 50).unwrap();
            samples.push(started.elapsed().as_micros());
            matched = results.hits.len();
        }
        all.extend(samples.iter().copied());
        report(&format!("search {q:?} ({matched} hits)"), samples);
    }
    report("search (all queries)", all);
}

/// R5.2 — opening a note under 200 KB in under 100 ms.
///
/// Measured through `store::read`, which is what the command calls: parsing
/// front matter and decoding is the part that could be slow, not the syscall.
#[test]
#[ignore = "measurement; needs MUSHROOM_BENCH_NOTES"]
fn note_open_latency() {
    let root = bench_root!();
    let scan = store::scan(&root);
    assert!(!scan.notes.is_empty(), "corpus is empty");

    // Spread across the corpus rather than the first N, which share a folder
    // and would all be in one directory's cache.
    let step = (scan.notes.len() / 200).max(1);
    let ids: Vec<NoteId> = scan
        .notes
        .iter()
        .step_by(step)
        .map(|n| n.id.clone())
        .collect();

    let mut samples = Vec::new();
    let mut largest = 0usize;
    for id in &ids {
        let started = Instant::now();
        let content = store::read(&root, id).unwrap();
        samples.push(started.elapsed().as_micros());
        largest = largest.max(content.body.len());
    }
    println!("largest note read: {largest} bytes");
    report("note open", samples);
}

/// Reconcile on a warm index — the startup path once the notes are indexed.
///
/// This is the work that stands between launch and a usable window on every
/// run but the first, so it belongs with the startup number in R5.1.
#[test]
#[ignore = "measurement; needs MUSHROOM_BENCH_NOTES"]
fn warm_reconcile() {
    let root = bench_root!();
    let dir = tempfile::tempdir().unwrap();
    let svc = SearchService::new();
    svc.open(&dir.path().join("mushroom.db")).unwrap();

    let scan = store::scan(&root);
    svc.reconcile(&root, &scan.notes).unwrap();

    let mut samples = Vec::new();
    for _ in 0..5 {
        let started = Instant::now();
        let progress = svc.reconcile(&root, &scan.notes).unwrap();
        samples.push(started.elapsed().as_micros());
        assert_eq!(progress.done, 0, "nothing changed, so nothing to reindex");
    }
    report("warm reconcile", samples);

    let mut samples = Vec::new();
    for _ in 0..5 {
        let started = Instant::now();
        let scan = store::scan(&root);
        samples.push(started.elapsed().as_micros());
        assert!(!scan.notes.is_empty());
    }
    report("cold scan of the notes folder", samples);
}

/// Spec 08 R7.3 — the exclusion filter must not move the retrieval budget.
///
/// Retrieval for a question has 200 ms to work with (spec 05 R2.7). This
/// milestone adds two filters to that path: a SQL predicate on an indexed
/// column, and a glob match per candidate row in Rust. Both are cheap in
/// theory. "Cheap in theory" is not a measurement, so this measures the same
/// retrieval three ways and prints all three.
#[test]
#[ignore = "measurement; needs MUSHROOM_BENCH_NOTES"]
fn retrieval_exclusion_cost() {
    use mushroom_lib::exclusion::{ExclusionRule, Exclusions};

    let root = bench_root!();
    let dir = tempfile::tempdir().unwrap();
    let svc = SearchService::new();
    svc.open(&dir.path().join("mushroom.db")).unwrap();

    let scan = store::scan(&root);
    println!("corpus: {} notes", scan.notes.len());
    svc.reconcile(&root, &scan.notes).unwrap();

    const QUERIES: [&str; 4] = ["capacity", "drain window", "GPU node pool drain", "mireshi"];

    // Ten rules, which is more than anyone will actually configure, because a
    // measurement taken at a comfortable size proves nothing about the size
    // that hurts.
    let rules: Vec<ExclusionRule> = [
        "personal/**",
        "personal/finance/**",
        "**/secrets.md",
        "work/cred*",
        "archive/2019/**",
        "archive/2020/**",
        "scratch",
        "**/*-private.md",
        "vault",
        "notes/keys/**",
    ]
    .iter()
    .map(|p| ExclusionRule::parse(p))
    .collect();

    let cases: [(&str, Option<Exclusions>); 3] = [
        ("no exclusion (keyword search)", None),
        ("frontmatter only (no rules)", Some(Exclusions::default())),
        ("frontmatter + 10 glob rules", Some(Exclusions::new(&rules))),
    ];

    for (label, excluded) in cases {
        // Warm the cache before timing, as `search_latency` does.
        for q in QUERIES {
            let _ = svc.retrieve_passages(q, None, 12, 3, excluded.clone().unwrap_or_default());
        }

        let mut samples = Vec::new();
        for _ in 0..20 {
            for q in QUERIES {
                let started = Instant::now();
                let found = svc
                    .retrieve_passages(q, None, 12, 3, excluded.clone().unwrap_or_default())
                    .unwrap();
                samples.push(started.elapsed().as_micros());
                std::hint::black_box(found.passages.len());
            }
        }
        report(&format!("retrieve_passages \u{2014} {label}"), samples);
    }
}

/// Spec 08 R7.1 — sanitising a request must take under 5 ms.
///
/// This is on the path of every question, so it is the number that decides
/// whether the gate is felt. Needs no corpus: the text is built here so the
/// measurement is the same on any machine.
#[test]
#[ignore = "measurement"]
fn sanitise_cost() {
    use mushroom_lib::ai::privacy::{sanitise, Policy, PrivacyMode, RuleSet};
    use mushroom_lib::ai::provider::{ChatRequest, Message};

    // ~24 KB of plausible note prose, which is what a full context looks like.
    let paragraph = "The orchestrator stopped the node pool while transcript work remained. \
         Capacity planning assumes no GPU failure during the drain window, and the \
         runbook says to quiet the queue first. Ask Priya before changing the schedule. ";
    let mut body = String::new();
    while body.len() < 24 * 1024 {
        body.push_str(paragraph);
    }
    println!("context: {} bytes", body.len());

    // Compiling the shipped table, which startup deliberately does not do.
    let started = Instant::now();
    let set = RuleSet::builtin();
    println!(
        "building the rule set: {} rules in {} \u{b5}s",
        set.len(),
        started.elapsed().as_micros()
    );

    let request = || ChatRequest {
        model: "test-model".into(),
        messages: vec![
            Message::system(body.clone()),
            Message::user("what happened?"),
        ],
        temperature: None,
        max_tokens: None,
        sources: Vec::new(),
    };

    for (label, mode) in [
        ("clean prose", PrivacyMode::Redact),
        ("privacy off", PrivacyMode::Off),
    ] {
        let policy = Policy::new(mode);
        // Warm: the first call compiles whichever rules the text trips.
        let _ = sanitise(request(), &policy).unwrap();

        let mut samples = Vec::new();
        for _ in 0..20 {
            let started = Instant::now();
            let out = sanitise(request(), &policy).unwrap();
            samples.push(started.elapsed().as_micros());
            std::hint::black_box(out.report().redacted_count());
        }
        report(&format!("sanitise \u{2014} {label}"), samples);
    }

    // And with something to find, since the work is in the matching.
    let mut with_secrets = body.clone();
    with_secrets.push_str(concat!(
        "\nThe runner used AKIA",
        "QYRZ5TMK7VW3XJ42 and ghp",
        "_016C7Ag8Dj2pRlP4Xt6Yn9Qv3Kw5Zb7Hd1Mf.\n"
    ));
    let policy = Policy::new(PrivacyMode::Redact);
    let with = || ChatRequest {
        model: "test-model".into(),
        messages: vec![Message::system(with_secrets.clone())],
        temperature: None,
        max_tokens: None,
        sources: Vec::new(),
    };
    let _ = sanitise(with(), &policy).unwrap();

    let mut samples = Vec::new();
    for _ in 0..20 {
        let started = Instant::now();
        let out = sanitise(with(), &policy).unwrap();
        samples.push(started.elapsed().as_micros());
        assert_eq!(out.report().redacted_count(), 2);
    }
    report("sanitise \u{2014} two credentials found", samples);
}
