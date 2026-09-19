//! Turning retrieved passages into the excerpt block the model sees.
//!
//! Two rules do most of the work here.
//!
//! **Excerpts are dropped whole, never truncated.** Half a sentence is exactly
//! the kind of context that produces a confidently wrong answer, and the user
//! has no way to tell it happened.
//!
//! **The citation map is built here, not parsed back out of the answer.** The
//! number the model writes as `[2]` only means something because this module
//! decided what excerpt 2 was.

use crate::ai::tokens;

/// One retrieved passage, ready to be numbered.
///
/// Its own type rather than `RetrievedPassage` so this module stays a pure
/// function over data — no database, no retriever — and so the modified date,
/// which retrieval does not carry, can be attached by the caller.
#[derive(Debug, Clone)]
pub struct Excerpt {
    pub note_id: String,
    pub note_title: String,
    pub folder: String,
    pub heading_path: String,
    pub line_start: u32,
    pub line_end: u32,
    pub text: String,
    /// Unix seconds. `None` when unknown — the line is then omitted rather
    /// than showing a wrong or epoch date.
    pub modified: Option<i64>,
}

/// What `[n]` in the answer resolves to (R5.1, R5.2).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Citation {
    pub number: usize,
    pub note_id: String,
    pub note_title: String,
    pub folder: String,
    pub heading_path: String,
    pub line_start: u32,
    pub line_end: u32,
    /// The excerpt as retrieved from the note, **before** the privacy gate.
    ///
    /// Not what went on the wire: since spec 08 credentials are replaced with
    /// markers, and in `block` mode the excerpt may be withheld entirely. This
    /// is shown locally so the user can check the model against their own
    /// note; `AI → Last Request…` is the record of what was sent.
    ///
    /// Shown beside each source so the user
    /// can check the model against what it was given (R5.3).
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct BuiltContext {
    /// The excerpt block for the prompt.
    pub block: String,
    pub citations: Vec<Citation>,
    /// Excerpts left out to fit the budget. Shown to the user (R3.7).
    pub dropped: usize,
    pub estimated_tokens: u32,
    /// A single excerpt exceeded the whole budget and was sent anyway.
    /// Truncating it is not allowed, so the caller is told instead.
    pub oversized: bool,
}

/// Build the excerpt block within a token budget.
///
/// `excerpts` must be ranked best-first; the budget is enforced by dropping
/// from the end. `per_note_cap` limits how many excerpts any one note may
/// contribute — separate from the retriever's own cap, because it is
/// reasonable to retrieve more passages for the "found these notes" list than
/// are worth spending context on.
pub fn build(excerpts: &[Excerpt], budget_tokens: u32, per_note_cap: usize) -> BuiltContext {
    let capped = apply_per_note_cap(excerpts, per_note_cap);
    let considered = capped.len();

    let mut block = String::new();
    let mut citations = Vec::new();
    let mut used = 0u32;
    let mut oversized = false;

    for excerpt in capped {
        let rendered = render(citations.len() + 1, &excerpt);
        let cost = tokens::estimate_excerpt(&rendered);

        if used + cost > budget_tokens {
            if citations.is_empty() {
                // The best excerpt alone does not fit. Sending nothing would
                // guarantee a useless answer, and cutting it would invent a
                // half-sentence, so send it and say so.
                oversized = true;
            } else {
                // Everything after this is lower-ranked, so stop.
                break;
            }
        }

        let number = citations.len() + 1;
        block.push_str(&rendered);
        used += cost;
        citations.push(Citation {
            number,
            note_id: excerpt.note_id,
            note_title: excerpt.note_title,
            folder: excerpt.folder,
            heading_path: excerpt.heading_path,
            line_start: excerpt.line_start,
            line_end: excerpt.line_end,
            text: excerpt.text,
        });

        if oversized {
            break;
        }
    }

    BuiltContext {
        dropped: considered.saturating_sub(citations.len()),
        estimated_tokens: used,
        block,
        citations,
        oversized,
    }
}

/// Keep at most `cap` excerpts per note, preserving rank order.
fn apply_per_note_cap(excerpts: &[Excerpt], cap: usize) -> Vec<Excerpt> {
    if cap == 0 {
        return excerpts.to_vec();
    }

    let mut counts: Vec<(String, usize)> = Vec::new();
    let mut kept = Vec::new();

    for excerpt in excerpts {
        let seen = match counts.iter_mut().find(|(id, _)| *id == excerpt.note_id) {
            Some((_, n)) => n,
            None => {
                counts.push((excerpt.note_id.clone(), 0));
                // `last_mut` is always Some here — just pushed.
                &mut counts.last_mut().expect("just pushed").1
            }
        };

        if *seen < cap {
            *seen += 1;
            kept.push(excerpt.clone());
        }
    }

    kept
}

fn render(number: usize, excerpt: &Excerpt) -> String {
    let mut out = format!("=== EXCERPT {number} ===\n");
    out.push_str(&format!("Note: {}\n", excerpt.note_title));

    if !excerpt.folder.is_empty() {
        out.push_str(&format!("Folder: {}\n", excerpt.folder));
    }
    if !excerpt.heading_path.is_empty() {
        out.push_str(&format!("Section: {}\n", excerpt.heading_path));
    }
    if let Some(date) = excerpt.modified.and_then(format_date) {
        out.push_str(&format!("Modified: {date}\n"));
    }

    out.push('\n');
    out.push_str(excerpt.text.trim());
    out.push_str("\n\n");
    out
}

fn format_date(unix_seconds: i64) -> Option<String> {
    let date = time::OffsetDateTime::from_unix_timestamp(unix_seconds).ok()?;
    let format = time::macros::format_description!("[year]-[month]-[day]");
    date.format(&format).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn excerpt(id: &str, text: &str) -> Excerpt {
        Excerpt {
            note_id: id.to_string(),
            note_title: format!("Note {id}"),
            folder: "work".into(),
            heading_path: "Heading".into(),
            line_start: 1,
            line_end: 5,
            text: text.to_string(),
            modified: Some(1_757_894_400), // 2025-09-15
        }
    }

    /// Roughly the budget one excerpt of this size needs.
    fn cost_of(e: &Excerpt) -> u32 {
        tokens::estimate_excerpt(&render(1, e))
    }

    #[test]
    fn excerpts_are_numbered_from_one_and_mapped() {
        let items = vec![excerpt("a", "first"), excerpt("b", "second")];
        let built = build(&items, 10_000, 3);

        assert!(built.block.contains("=== EXCERPT 1 ==="));
        assert!(built.block.contains("=== EXCERPT 2 ==="));
        assert_eq!(built.citations.len(), 2);
        assert_eq!(built.citations[0].number, 1);
        assert_eq!(built.citations[0].note_id, "a");
        assert_eq!(built.citations[1].number, 2);
        assert_eq!(built.citations[1].note_id, "b");
        assert_eq!(built.dropped, 0);
    }

    #[test]
    fn metadata_the_model_needs_is_attached() {
        let items = vec![excerpt("a", "body text here")];
        let built = build(&items, 10_000, 3);

        assert!(built.block.contains("Note: Note a"), "{}", built.block);
        assert!(built.block.contains("Folder: work"), "{}", built.block);
        assert!(built.block.contains("Section: Heading"), "{}", built.block);
        assert!(
            built.block.contains("Modified: 2025-09-15"),
            "{}",
            built.block
        );
        assert!(built.block.contains("body text here"), "{}", built.block);
    }

    #[test]
    fn an_unknown_date_omits_the_line_rather_than_inventing_one() {
        let mut item = excerpt("a", "text");
        item.modified = None;
        let built = build(&[item], 10_000, 3);
        assert!(!built.block.contains("Modified:"), "{}", built.block);
    }

    #[test]
    fn a_budget_exactly_met_drops_nothing() {
        let items = vec![excerpt("a", "one"), excerpt("b", "two")];
        let exact = cost_of(&items[0]) + cost_of(&items[1]);

        let built = build(&items, exact, 3);
        assert_eq!(built.dropped, 0, "an exact fit must not drop anything");
        assert_eq!(built.citations.len(), 2);
        assert!(built.estimated_tokens <= exact);
    }

    #[test]
    fn a_budget_exceeded_by_one_excerpt_drops_the_lowest_ranked() {
        let items = vec![
            excerpt("a", "one"),
            excerpt("b", "two"),
            excerpt("c", "three"),
        ];
        // Room for the first two only.
        let budget = cost_of(&items[0]) + cost_of(&items[1]);

        let built = build(&items, budget, 3);
        assert_eq!(built.citations.len(), 2);
        assert_eq!(built.dropped, 1);
        // The one dropped is the last, not an arbitrary one.
        assert_eq!(built.citations[0].note_id, "a");
        assert_eq!(built.citations[1].note_id, "b");
        assert!(!built.block.contains("three"), "{}", built.block);
    }

    #[test]
    fn a_single_excerpt_larger_than_the_budget_is_kept_and_flagged() {
        // Truncating would hand the model half a sentence; sending nothing
        // guarantees a useless answer. Send it, and tell the caller.
        let big = excerpt("a", &"word ".repeat(5_000));
        let built = build(std::slice::from_ref(&big), 10, 3);

        assert!(built.oversized, "the caller must be told");
        assert_eq!(built.citations.len(), 1);
        assert!(
            built.block.contains(&"word word".to_string()),
            "the text must be intact, not cut"
        );
        assert!(built.block.trim_end().ends_with("word"), "no mid-text cut");
    }

    #[test]
    fn no_excerpt_is_ever_cut_mid_text() {
        let items: Vec<Excerpt> = (0..10)
            .map(|i| {
                excerpt(
                    &format!("n{i}"),
                    &format!("body of excerpt {i} ").repeat(20),
                )
            })
            .collect();

        // A budget that lands partway through the set.
        let built = build(&items, cost_of(&items[0]) * 3 + 5, 3);

        for citation in &built.citations {
            assert!(
                built.block.contains(citation.text.trim()),
                "excerpt {} was altered on its way into the block",
                citation.number
            );
        }
    }

    #[test]
    fn per_note_capping_stops_one_note_crowding_out_the_rest() {
        let items = vec![
            excerpt("same", "first from same"),
            excerpt("same", "second from same"),
            excerpt("same", "third from same"),
            excerpt("other", "from other"),
        ];

        let built = build(&items, 10_000, 2);
        assert_eq!(
            built.citations.len(),
            3,
            "two from `same`, one from `other`"
        );
        assert_eq!(
            built
                .citations
                .iter()
                .filter(|c| c.note_id == "same")
                .count(),
            2
        );
        assert!(built.citations.iter().any(|c| c.note_id == "other"));
        // Not counted as dropped: `dropped` is shown to the user as "left out
        // to fit the context limit", and this one was not — it lost to the
        // per-note cap, with the whole budget still free.
        assert_eq!(built.dropped, 0);
    }

    #[test]
    fn dropped_counts_only_what_the_budget_removed() {
        // Both mechanisms at once, so the two cannot be confused.
        let items = vec![
            excerpt("same", "first from same"),
            excerpt("same", "second from same"),
            excerpt("other", "from other"),
        ];
        // Cap keeps 1 from `same`, leaving 2 excerpts; budget fits only 1.
        let budget = cost_of(&items[0]);
        let built = build(&items, budget, 1);

        assert_eq!(built.citations.len(), 1);
        assert_eq!(
            built.dropped, 1,
            "one excerpt survived the cap and then lost to the budget"
        );
    }

    #[test]
    fn a_cap_of_zero_means_no_cap() {
        let items = vec![excerpt("same", "a"), excerpt("same", "b")];
        let built = build(&items, 10_000, 0);
        assert_eq!(built.citations.len(), 2);
    }

    #[test]
    fn no_excerpts_produces_an_empty_context_not_a_panic() {
        let built = build(&[], 1000, 3);
        assert!(built.block.is_empty());
        assert!(built.citations.is_empty());
        assert_eq!(built.dropped, 0);
        assert!(!built.oversized);
    }

    #[test]
    fn citation_numbers_are_contiguous_after_dropping() {
        // A gap in the numbering would make the model cite an excerpt that
        // does not exist in the block it can see.
        let items: Vec<Excerpt> = (0..6)
            .map(|i| excerpt(&format!("n{i}"), &format!("text {i}")))
            .collect();
        let built = build(&items, cost_of(&items[0]) * 3, 3);

        for (index, citation) in built.citations.iter().enumerate() {
            assert_eq!(citation.number, index + 1);
            assert!(built
                .block
                .contains(&format!("=== EXCERPT {} ===", index + 1)));
        }
    }
}
