//! Checking the model actually cited the notes it was given.
//!
//! This is a cheap, mechanical check for the exact failure the brief warns
//! about: an answer that reads perfectly and is not grounded in anything. It
//! cannot catch a model that writes `[1]` and then invents its content — for
//! that the excerpt text is shown beside each source so the user can look.
//! What it does catch is an answer citing nothing at all, or citing an
//! excerpt number that was never sent.

use crate::ai::context::Citation;

/// How well an answer is grounded in the excerpts it was given (R4.2, R4.3).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Grounding {
    /// Citations that resolved, in the order first used.
    pub used: Vec<Citation>,
    /// Numbers the model cited that were never sent. Almost always a
    /// hallucinated reference.
    pub unmatched: Vec<usize>,
    /// The model cited nothing. The answer renders as unverified.
    pub uncited: bool,
    /// Sent but never cited — the "Also searched" list (R5.5).
    pub unused: Vec<Citation>,
}

/// Find every `[n]` marker in the answer, in order of first appearance.
///
/// Markers inside fenced or inline code are ignored: `[0]` in a code sample
/// is an array index, not a citation.
pub fn extract_markers(answer: &str) -> Vec<usize> {
    let mut found = Vec::new();

    for chunk in outside_code(answer) {
        let bytes = chunk.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            if bytes[i] != b'[' {
                i += 1;
                continue;
            }
            let Some(close) = chunk[i..].find(']') else {
                break;
            };
            let inner = &chunk[i + 1..i + close];

            // `[1, 2]` and `[1,2]` are one marker naming two excerpts.
            let numbers: Vec<usize> = inner
                .split(',')
                .map(|part| part.trim())
                .filter(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
                .filter_map(|part| part.parse().ok())
                .collect();

            let parsed_all = !inner.trim().is_empty()
                && inner.split(',').filter(|p| !p.trim().is_empty()).count() == numbers.len();

            if parsed_all && !numbers.is_empty() {
                for n in numbers {
                    if !found.contains(&n) {
                        found.push(n);
                    }
                }
            }

            i += close + 1;
        }
    }

    found
}

/// Split the answer into the parts that are not code.
fn outside_code(answer: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = answer;

    // Fenced blocks first, then inline spans within what is left.
    loop {
        let Some(open) = rest.find("```") else {
            parts.push(rest);
            break;
        };
        parts.push(&rest[..open]);

        let after = &rest[open + 3..];
        match after.find("```") {
            Some(close) => rest = &after[close + 3..],
            // An unterminated fence means the rest is code.
            None => break,
        }
    }

    parts
        .into_iter()
        .flat_map(|part| {
            let mut spans = Vec::new();
            let mut rest = part;
            loop {
                let Some(open) = rest.find('`') else {
                    spans.push(rest);
                    break;
                };
                spans.push(&rest[..open]);
                let after = &rest[open + 1..];
                match after.find('`') {
                    Some(close) => rest = &after[close + 1..],
                    None => break,
                }
            }
            spans
        })
        .collect()
}

/// Resolve an answer's citations against the map built for it.
pub fn check(answer: &str, map: &[Citation]) -> Grounding {
    let markers = extract_markers(answer);

    let mut used = Vec::new();
    let mut unmatched = Vec::new();

    for number in &markers {
        match map.iter().find(|c| c.number == *number) {
            Some(citation) => used.push(citation.clone()),
            None => unmatched.push(*number),
        }
    }

    let unused = map
        .iter()
        .filter(|c| !markers.contains(&c.number))
        .cloned()
        .collect();

    Grounding {
        uncited: used.is_empty(),
        used,
        unmatched,
        unused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(count: usize) -> Vec<Citation> {
        (1..=count)
            .map(|n| Citation {
                number: n,
                note_id: format!("note-{n}.md"),
                note_title: format!("Note {n}"),
                folder: "work".into(),
                heading_path: String::new(),
                line_start: 1,
                line_end: 2,
                text: format!("text {n}"),
            })
            .collect()
    }

    #[test]
    fn a_single_marker() {
        assert_eq!(extract_markers("The pool drains early [1]."), vec![1]);
    }

    #[test]
    fn adjacent_markers() {
        assert_eq!(extract_markers("Both agree [1][2]."), vec![1, 2]);
    }

    #[test]
    fn a_comma_separated_marker_names_several() {
        assert_eq!(extract_markers("Both agree [1, 2]."), vec![1, 2]);
        assert_eq!(extract_markers("Both agree [1,2]."), vec![1, 2]);
    }

    #[test]
    fn a_two_digit_marker_is_one_number_not_two() {
        // `[12]` with 3 excerpts is an unmatched citation 12, not 1 and 2.
        let grounding = check("As noted [12].", &map(3));
        assert_eq!(grounding.unmatched, vec![12]);
        assert!(grounding.used.is_empty());
    }

    #[test]
    fn markers_inside_a_fenced_code_block_are_ignored() {
        let answer =
            "Use the first element [1].\n\n```rust\nlet x = items[0];\nlet y = items[2];\n```\n";
        assert_eq!(extract_markers(answer), vec![1]);
    }

    #[test]
    fn markers_inside_inline_code_are_ignored() {
        let answer = "The config key is `settings[0]` and it is documented [2].";
        assert_eq!(extract_markers(answer), vec![2]);
    }

    #[test]
    fn an_unterminated_fence_does_not_swallow_earlier_citations() {
        let answer = "Grounded here [1].\n\n```\nunterminated [9]\n";
        assert_eq!(extract_markers(answer), vec![1]);
    }

    #[test]
    fn non_numeric_brackets_are_not_citations() {
        // Markdown links and ordinary brackets must not be read as markers.
        let answer = "See [the docs](https://example.com) and [TODO] and [].";
        assert!(extract_markers(answer).is_empty());
    }

    #[test]
    fn repeated_markers_are_reported_once_in_order() {
        assert_eq!(extract_markers("[2] then [1] then [2] again"), vec![2, 1]);
    }

    #[test]
    fn a_grounded_answer_resolves_every_marker() {
        let grounding = check("Drains early [1], and the queue empties [3].", &map(3));

        assert_eq!(grounding.used.len(), 2);
        assert_eq!(grounding.used[0].note_id, "note-1.md");
        assert_eq!(grounding.used[1].note_id, "note-3.md");
        assert!(grounding.unmatched.is_empty());
        assert!(!grounding.uncited);
        // Excerpt 2 was sent but not cited.
        assert_eq!(grounding.unused.len(), 1);
        assert_eq!(grounding.unused[0].number, 2);
    }

    #[test]
    fn an_answer_citing_nothing_is_flagged_as_unverified() {
        let grounding = check("The node pool drains on a schedule.", &map(2));
        assert!(
            grounding.uncited,
            "this is the answer the brief warns about"
        );
        assert!(grounding.used.is_empty());
        assert_eq!(grounding.unused.len(), 2, "both were sent, neither used");
    }

    #[test]
    fn a_citation_that_was_never_sent_is_reported() {
        let grounding = check("As established [5].", &map(2));
        assert_eq!(grounding.unmatched, vec![5]);
        assert!(
            grounding.uncited,
            "nothing resolved, so nothing is verified"
        );
    }

    #[test]
    fn a_mix_of_matched_and_unmatched_keeps_both() {
        let grounding = check("Real [1] and invented [9].", &map(2));
        assert_eq!(grounding.used.len(), 1);
        assert_eq!(grounding.unmatched, vec![9]);
        assert!(!grounding.uncited, "one citation did resolve");
    }

    #[test]
    fn an_empty_answer_is_uncited_not_a_panic() {
        let grounding = check("", &map(2));
        assert!(grounding.uncited);
        assert!(grounding.unmatched.is_empty());
    }

    #[test]
    fn an_answer_with_no_excerpts_sent_has_nothing_to_resolve_against() {
        let grounding = check("Something [1].", &[]);
        assert_eq!(grounding.unmatched, vec![1]);
        assert!(grounding.unused.is_empty());
    }
}
