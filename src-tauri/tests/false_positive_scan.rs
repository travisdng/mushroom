//! What the rules would redact from a real notes folder.
//!
//! Spec 08 task 28b. Every hit that is not a credential is a false positive
//! worth fixing *before* shipping, because the alternative is discovering it
//! as a mangled answer — and a scanner people learn to ignore is the only
//! failure here that costs real secrets.
//!
//! Ignored by default: it needs a corpus.
//!
//!   $env:MUSHROOM_SCAN_NOTES = "C:\Users\you\Mushroom\notes"
//!   cargo test --test false_positive_scan -- --ignored --nocapture

use mushroom_lib::ai::privacy::rules::RuleSet;

#[test]
#[ignore = "inspection; needs MUSHROOM_SCAN_NOTES"]
fn report_what_would_be_redacted() {
    let Ok(root) = std::env::var("MUSHROOM_SCAN_NOTES") else {
        println!("skipped: set MUSHROOM_SCAN_NOTES to a notes folder");
        return;
    };

    let set = RuleSet::builtin();
    let mut files = 0usize;
    let mut hits = 0usize;

    for entry in walkdir::WalkDir::new(&root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
    {
        let Ok(text) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        files += 1;

        for hit in set.find(&text).expect("the rules must compile") {
            hits += 1;
            // Print the matched text. This runs against the developer's own
            // machine, on demand, and the whole point is to look at what would
            // be removed — a report of counts could not be judged.
            let matched = &text[hit.start..hit.end];
            let shown: String = matched.chars().take(60).collect();
            println!(
                "{}\n    rule: {}\n    text: {}",
                entry.path().display(),
                hit.rule,
                shown.replace('\n', " ")
            );
        }
    }

    println!("\nscanned {files} notes, {hits} would be redacted");
    if hits == 0 {
        println!("nothing would be redacted \u{2014} no false positives to judge");
    }
}
