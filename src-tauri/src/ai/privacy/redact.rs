//! Replacing credentials with markers, keeping the prose around them.
//!
//! The marker names the kind of thing that was removed — `[redacted:
//! aws-access-key]` — rather than blanking the text. That is deliberate: the
//! model still needs to understand that a credential exists there, or an
//! answer about "how does the batch runner authenticate" becomes nonsense.
//! Redaction that destroys the context is redaction nobody leaves switched on.

use super::rules::{Hit, RuleSet};
use super::{PrivacyError, Redaction};

/// What redaction did to one piece of text.
#[derive(Debug, Clone, Default)]
pub struct Redacted {
    pub text: String,
    /// Rule name and count. Never the matched value (R3.5).
    pub redactions: Vec<Redaction>,
}

impl Redacted {
    pub fn is_clean(&self) -> bool {
        self.redactions.is_empty()
    }
}

/// Replace every credential-shaped span with a marker naming its rule.
pub fn redact(text: &str, rules: &RuleSet) -> Result<Redacted, PrivacyError> {
    let hits = merge_overlapping(rules.find(text)?);
    if hits.is_empty() {
        return Ok(Redacted {
            text: text.to_string(),
            redactions: Vec::new(),
        });
    }

    let mut out = String::with_capacity(text.len());
    let mut counts: Vec<Redaction> = Vec::new();
    let mut at = 0usize;

    for hit in &hits {
        // Defensive: a span outside the text, or one that starts before the
        // cursor, would panic on slicing. Neither should happen after
        // merging, and neither is worth crashing a request over.
        if hit.start < at || hit.end > text.len() {
            continue;
        }
        out.push_str(&text[at..hit.start]);
        out.push_str(&format!("[redacted: {}]", hit.rule));
        at = hit.end;

        match counts.iter_mut().find(|r| r.rule == hit.rule) {
            Some(existing) => existing.count += 1,
            None => counts.push(Redaction {
                rule: hit.rule.to_string(),
                count: 1,
            }),
        }
    }
    out.push_str(&text[at..]);

    counts.sort_by(|a, b| a.rule.cmp(&b.rule));
    Ok(Redacted {
        text: out,
        redactions: counts,
    })
}

/// Does this text contain anything the rules recognise?
///
/// For `Block` mode, which withholds the whole containing item rather than
/// editing it.
pub fn contains_secret(text: &str, rules: &RuleSet) -> Result<Option<String>, PrivacyError> {
    Ok(rules.find(text)?.first().map(|hit| hit.rule.to_string()))
}

/// Resolve overlapping matches so the output is not corrupted.
///
/// Several rules can match the same text — a key inside a connection string
/// inside a URL. Replacing them independently would splice a marker into the
/// middle of another marker. Longest match wins at each position, which keeps
/// the most specific rule and never leaves a fragment of a secret behind.
fn merge_overlapping(mut hits: Vec<Hit>) -> Vec<Hit> {
    // Earliest first; at the same start, longest first.
    hits.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

    let mut kept: Vec<Hit> = Vec::with_capacity(hits.len());
    for hit in hits {
        match kept.last_mut() {
            // Overlaps what we already kept. Extend if this one reaches
            // further, so no tail of the secret survives.
            Some(previous) if hit.start < previous.end => {
                if hit.end > previous.end {
                    previous.end = hit.end;
                }
            }
            _ => kept.push(hit),
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::privacy::rules::Rule;

    fn rules() -> RuleSet {
        RuleSet::new(vec![
            Rule::with_keywords(
                "aws-access-key",
                r"(AKIA|ASIA)[A-Z2-7]{16}",
                &["akia", "asia"],
            ),
            Rule::with_keywords("github-token", r"ghp_[A-Za-z0-9]{36,}", &["ghp_"]),
        ])
    }

    #[test]
    fn a_secret_becomes_a_marker_and_the_prose_survives() {
        // The whole point: the model must still understand that the batch
        // runner authenticates with *something*.
        let out = redact(
            concat!(
                "The batch runner authenticated with AKIA",
                "QYRZ5TMK7VW3XJ42 last week."
            ),
            &rules(),
        )
        .unwrap();

        assert_eq!(
            out.text,
            "The batch runner authenticated with [redacted: aws-access-key] last week."
        );
        assert!(!out.text.contains(concat!("AKIA", "QYRZ5TMK7VW3XJ42")));
        assert_eq!(out.redactions.len(), 1);
        assert_eq!(out.redactions[0].count, 1);
    }

    #[test]
    fn clean_text_comes_back_untouched() {
        let text = "The orchestrator stopped the node pool overnight.";
        let out = redact(text, &rules()).unwrap();
        assert_eq!(out.text, text);
        assert!(out.is_clean());
    }

    #[test]
    fn several_secrets_of_the_same_kind_are_counted_once_with_a_tally() {
        let out = redact(
            concat!(
                "keys AKIA",
                "QYRZ5TMK7VW3XJ42 and AKIA",
                "2345TMK7VW3XJ42Q rotated"
            ),
            &rules(),
        )
        .unwrap();
        assert_eq!(out.redactions.len(), 1, "one rule");
        assert_eq!(out.redactions[0].count, 2, "twice");
        assert_eq!(out.text.matches("[redacted: aws-access-key]").count(), 2);
    }

    #[test]
    fn different_kinds_are_reported_separately() {
        let out = redact(
            concat!(
                "AKIA",
                "QYRZ5TMK7VW3XJ42 and ghp",
                "_016C7Ag8Dj2pRlP4Xt6Yn9Qv3Kw5Zb7Hd1Mf"
            ),
            &rules(),
        )
        .unwrap();
        let names: Vec<&str> = out.redactions.iter().map(|r| r.rule.as_str()).collect();
        assert_eq!(names, vec!["aws-access-key", "github-token"]);
    }

    #[test]
    fn overlapping_matches_do_not_corrupt_the_output() {
        // Two rules matching the same region must not splice a marker into
        // the middle of another marker, and no tail of the secret may survive.
        let overlapping = RuleSet::new(vec![
            Rule::with_keywords("outer", r"AKIA[A-Z2-7]{16}", &["akia"]),
            Rule::with_keywords("inner", r"[A-Z2-7]{10}", &["akia"]),
        ]);
        let out = redact(concat!("key AKIA", "QYRZ5TMK7VW3XJ42 end"), &overlapping).unwrap();

        assert!(!out.text.contains("QYRZ5TMK7V"), "{}", out.text);
        assert!(out.text.starts_with("key [redacted:"), "{}", out.text);
        assert!(out.text.ends_with("end"), "{}", out.text);
        assert_eq!(out.text.matches("[redacted:").count(), 1, "{}", out.text);
    }

    #[test]
    fn a_secret_at_the_very_start_or_end_is_handled() {
        let out = redact(concat!("AKIA", "QYRZ5TMK7VW3XJ42"), &rules()).unwrap();
        assert_eq!(out.text, "[redacted: aws-access-key]");
    }

    #[test]
    fn multi_byte_text_is_not_sliced_through_a_character() {
        // Byte offsets from the regex must line up with the string, or this
        // panics rather than redacting.
        let out = redact(
            concat!(
                "café — the runner used AKIA",
                "QYRZ5TMK7VW3XJ42 — see Ø notes"
            ),
            &rules(),
        )
        .unwrap();
        assert!(out.text.contains("café"));
        assert!(out.text.contains('Ø'));
        assert!(!out.text.contains(concat!("AKIA", "QYRZ5TMK7VW3XJ42")));
    }

    #[test]
    fn block_mode_asks_only_whether_there_is_anything_here() {
        assert_eq!(
            contains_secret(concat!("key AKIA", "QYRZ5TMK7VW3XJ42"), &rules()).unwrap(),
            Some("aws-access-key".to_string())
        );
        assert_eq!(contains_secret("nothing here", &rules()).unwrap(), None);
    }

    #[test]
    fn a_broken_rule_set_fails_rather_than_returning_the_text_unchanged() {
        // Returning the original text on failure would be the worst possible
        // outcome: it looks exactly like "nothing to redact".
        let broken = RuleSet::new(vec![Rule::new("nonsense", "(unclosed")]);
        assert!(redact("anything", &broken).is_err());
    }
}
