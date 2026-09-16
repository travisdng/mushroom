//! Estimating how many tokens a piece of text will cost.
//!
//! There is no tokeniser here on purpose. Bundling one would mean shipping a
//! vocabulary per model family and keeping it current, to answer a question we
//! only need approximately: does this excerpt still fit in the budget?
//!
//! So: characters divided by a constant, rounded up, plus framing. The
//! constant is deliberately pessimistic — over-estimating costs a dropped
//! excerpt, under-estimating costs a 400 from the provider after the user has
//! already waited. Every response logs the real `prompt_tokens` next to this
//! estimate (R3.3), so the constant can be corrected from evidence.

/// Characters per token. English prose sits near 4.0; 3.6 leaves headroom for
/// code, punctuation and non-English text, which tokenise worse.
pub const CHARS_PER_TOKEN: f64 = 3.6;

/// Per-excerpt framing: the `=== EXCERPT n ===` header and the metadata lines.
pub const FRAMING_TOKENS: u32 = 8;

/// Estimated tokens for a piece of text, excluding framing.
pub fn estimate(text: &str) -> u32 {
    // Counting chars, not bytes: a multi-byte character is usually one token's
    // worth of information, not four.
    let chars = text.chars().count() as f64;
    (chars / CHARS_PER_TOKEN).ceil() as u32
}

/// Estimated tokens for one excerpt, including its framing.
pub fn estimate_excerpt(text: &str) -> u32 {
    estimate(text) + FRAMING_TOKENS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_costs_nothing_but_framing() {
        assert_eq!(estimate(""), 0);
        assert_eq!(estimate_excerpt(""), FRAMING_TOKENS);
    }

    #[test]
    fn the_estimate_rounds_up() {
        // 4 chars / 3.6 = 1.11 → 2. Never round down: a budget that is
        // slightly over is a request the provider rejects.
        assert_eq!(estimate("abcd"), 2);
        assert_eq!(estimate("a"), 1);
    }

    #[test]
    fn the_estimate_is_pessimistic_against_real_prose() {
        // A paragraph of ordinary English. Real tokenisers put this near
        // chars/4; we must come out higher, not lower.
        let prose = "The orchestrator can stop the node pool when it sees no \
                     active queue messages, even though transcript-processing \
                     work may still remain.";
        let chars = prose.chars().count() as f64;
        let realistic = (chars / 4.0).ceil() as u32;

        assert!(
            estimate(prose) >= realistic,
            "estimate {} should not undercut a chars/4 guess of {realistic}",
            estimate(prose)
        );
    }

    #[test]
    fn multi_byte_text_is_counted_by_character_not_byte() {
        // "café" is 5 bytes but 4 characters. Counting bytes would inflate
        // every accented or non-Latin note.
        let text = "café";
        assert_eq!(text.len(), 5, "precondition: this string is 5 bytes");
        assert_eq!(estimate(text), estimate("cafe"));
    }

    #[test]
    fn longer_text_never_estimates_lower() {
        let mut previous = 0;
        for n in [0, 1, 10, 100, 1000, 10_000] {
            let estimate = estimate(&"x".repeat(n));
            assert!(estimate >= previous, "not monotonic at {n}");
            previous = estimate;
        }
    }
}
