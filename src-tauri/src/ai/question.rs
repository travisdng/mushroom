//! Turning a question into search terms.
//!
//! `What did I write about the GPU nodes shutting down too early?` sent
//! straight to FTS5 is dominated by `what`, `did`, `i`, `write`, `about` —
//! and with porter stemming those match nearly every note. The useful part is
//! `gpu nodes shutting down early`.
//!
//! Deliberately mechanical rather than a model call: rewriting every question
//! with an LLM would double the latency and the cost, and would mean Mushroom
//! reaches for the network before it has even looked at your own notes. If
//! recall turns out to be poor, the fix is semantic retrieval in v1.1, not a
//! rewrite step here.

/// Leading question frames, longest first so `what do my notes say about`
/// is stripped before `what do`. Order is the whole correctness argument.
const FRAMES: [&str; 26] = [
    "do i have anything written about",
    "do i have anything about",
    "what do my notes say about",
    "what did i say about",
    "what did i write about",
    "what have i written about",
    "what did i note about",
    "where did i write about",
    "where did i mention",
    "when did i write about",
    "when did i note",
    "can you tell me about",
    "can you find anything about",
    "tell me about",
    "find me anything about",
    "find anything about",
    "search my notes for",
    "look up",
    "what about",
    "what is",
    "what are",
    "what was",
    "where is",
    "where are",
    "who is",
    "how do i",
];

/// A fixed English stop list. Short and boring on purpose: an aggressive list
/// throws away words like `down` and `early` that carry the actual meaning.
const STOP_WORDS: [&str; 44] = [
    "a", "an", "the", "and", "or", "but", "if", "of", "to", "in", "on", "at", "by", "for", "with",
    "about", "into", "from", "as", "is", "are", "was", "were", "be", "been", "being", "do", "does",
    "did", "have", "has", "had", "i", "me", "my", "we", "our", "you", "your", "it", "its", "that",
    "this", "there",
];

/// What the question was turned into, and what to show the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchTerms {
    /// The query handed to the retriever.
    pub query: String,
    /// The individual terms, for the "searched for: …" line (R2.5).
    pub terms: Vec<String>,
    /// True when the question reduced to almost nothing and the original was
    /// used instead. Worth showing, because it explains odd results.
    pub fell_back: bool,
}

/// Extract search terms from a natural-language question.
pub fn to_search_terms(question: &str) -> SearchTerms {
    let lowered = question.trim().to_lowercase();
    // Strip terminal punctuation only — an apostrophe or hyphen inside a word
    // belongs to the word.
    let cleaned = lowered.trim_end_matches(['?', '!', '.', ',']).trim();

    let quoted = quoted_phrases(cleaned);
    let without_frame = strip_frame(cleaned);

    let kept: Vec<String> = tokenise(&without_frame)
        .into_iter()
        // A word inside a quoted phrase survives even if it is a stop word:
        // the user asked for it literally.
        .filter(|w| !is_stop_word(w) || quoted.iter().any(|p| phrase_contains(p, w)))
        .collect();

    // Only when *nothing* survives. The design said "fewer than two terms",
    // but one strong term is the best possible query: "what did i write about
    // onboarding" reduces to `onboarding`, and falling back to the whole
    // question would put `write` back in and match every note that has it.
    if kept.is_empty() {
        let fallback: Vec<String> = tokenise(cleaned);
        if !fallback.is_empty() {
            return SearchTerms {
                query: fallback.join(" "),
                terms: fallback,
                fell_back: true,
            };
        }
    }

    SearchTerms {
        query: kept.join(" "),
        terms: kept,
        fell_back: false,
    }
}

fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.contains(&word)
}

fn phrase_contains(phrase: &str, word: &str) -> bool {
    phrase.split_whitespace().any(|w| w == word)
}

/// Remove a leading question frame, if one matches.
fn strip_frame(text: &str) -> String {
    for frame in FRAMES {
        if let Some(rest) = text.strip_prefix(frame) {
            // Only a whole-word match: "what is" must not eat "what island".
            if rest.is_empty() || rest.starts_with(' ') {
                return rest.trim().to_string();
            }
        }
    }
    text.to_string()
}

/// The contents of every `"quoted phrase"`, lowercased.
fn quoted_phrases(text: &str) -> Vec<String> {
    let mut phrases = Vec::new();
    let mut rest = text;

    while let Some(open) = rest.find('"') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('"') else { break };
        let phrase = after[..close].trim();
        if !phrase.is_empty() {
            phrases.push(phrase.to_string());
        }
        rest = &after[close + 1..];
    }

    phrases
}

/// Split into words, dropping punctuation but keeping intra-word marks.
fn tokenise(text: &str) -> Vec<String> {
    text.split(|c: char| c.is_whitespace() || c == '"')
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '\'' && c != '.')
                .to_string()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(question: &str) -> String {
        to_search_terms(question).query
    }

    #[test]
    fn the_briefs_own_example() {
        // The question the product brief uses, and the reason this module
        // exists at all.
        assert_eq!(
            q("What did I write about the GPU nodes shutting down too early?"),
            "gpu nodes shutting down too early"
        );
    }

    #[test]
    fn a_table_of_real_shaped_questions() {
        let cases = [
            (
                "What did I write about the GPU nodes shutting down too early?",
                "gpu nodes shutting down too early",
            ),
            (
                "What do my notes say about capacity planning?",
                "capacity planning",
            ),
            (
                "Where did I mention the retention policy?",
                "retention policy",
            ),
            ("Do I have anything about onboarding?", "onboarding"),
            ("Tell me about the quarterly review", "quarterly review"),
            ("Search my notes for dial rate limits", "dial rate limits"),
            ("What did I say about agent scripting?", "agent scripting"),
            ("When did I write about the migration?", "migration"),
            ("How do I configure the dialler?", "configure dialler"),
            (
                "what about the outbound campaign settings",
                "outbound campaign settings",
            ),
            // No frame at all — just a statement of the topic.
            (
                "GPU nodes draining on a schedule",
                "gpu nodes draining schedule",
            ),
            ("call recording retention", "call recording retention"),
            // Frames in the middle must not be stripped; only a leading one.
            (
                "notes about what i write when the queue backs up",
                "notes what write when queue backs up",
            ),
            // Extra whitespace and mixed case.
            (
                "  WHAT DID I WRITE ABOUT   Disaster Recovery  ",
                "disaster recovery",
            ),
            // Trailing punctuation of several kinds.
            ("What did I write about failover!", "failover"),
            ("What did I write about failover.", "failover"),
        ];

        for (question, expected) in cases {
            assert_eq!(q(question), expected, "question: {question}");
        }
    }

    #[test]
    fn a_terse_question_is_left_alone() {
        // "GPU shutdown?" has nothing to strip and nothing to spare.
        let terms = to_search_terms("GPU shutdown?");
        assert_eq!(terms.query, "gpu shutdown");
        assert!(!terms.fell_back, "nothing was removed, so nothing was lost");
    }

    #[test]
    fn one_strong_term_is_kept_rather_than_falling_back() {
        // The case that made the original "fewer than two terms" rule wrong:
        // falling back here would restore `write`, which matches everything.
        let terms = to_search_terms("What did I write about onboarding?");
        assert_eq!(terms.query, "onboarding");
        assert!(!terms.fell_back, "{terms:?}");
    }

    #[test]
    fn a_question_that_is_all_frame_falls_back_to_the_original() {
        // Stripping leaves nothing useful, so searching the original beats
        // searching an empty string.
        let terms = to_search_terms("What did I write about it?");
        assert!(terms.fell_back, "{terms:?}");
        assert!(terms.query.contains("write"), "{terms:?}");
        assert!(!terms.query.is_empty());
    }

    #[test]
    fn a_question_of_only_stop_words_still_produces_something() {
        let terms = to_search_terms("what is it about?");
        assert!(!terms.query.trim().is_empty(), "{terms:?}");
        assert!(terms.fell_back);
    }

    #[test]
    fn quoted_words_survive_even_when_they_are_stop_words() {
        // The user asked for the literal phrase, so "the" is not noise here.
        let terms = to_search_terms(r#"What did I write about "the big one" failing?"#);
        assert!(terms.terms.contains(&"the".to_string()), "{terms:?}");
        assert!(terms.terms.contains(&"big".to_string()), "{terms:?}");
        assert!(terms.terms.contains(&"one".to_string()), "{terms:?}");
    }

    #[test]
    fn an_unquoted_stop_word_is_still_removed() {
        let terms = to_search_terms("What did I write about the big one failing?");
        assert!(!terms.terms.contains(&"the".to_string()), "{terms:?}");
    }

    #[test]
    fn a_frame_is_only_stripped_on_a_word_boundary() {
        // "what is" must not eat the start of "what island".
        let terms = to_search_terms("what island did i visit");
        assert!(terms.terms.contains(&"island".to_string()), "{terms:?}");
    }

    #[test]
    fn the_longest_matching_frame_wins() {
        // "what do my notes say about" must be tried before "what do".
        assert_eq!(q("What do my notes say about latency?"), "latency");
    }

    #[test]
    fn order_is_preserved() {
        // FTS5 phrase proximity depends on it, and it reads better in the
        // "searched for" line.
        let terms = to_search_terms("What did I write about queue depth and agent idle time?");
        assert_eq!(terms.terms, vec!["queue", "depth", "agent", "idle", "time"]);
    }

    #[test]
    fn hyphens_and_apostrophes_inside_words_are_kept() {
        let terms = to_search_terms("What did I write about real-time guidance?");
        assert!(terms.terms.contains(&"real-time".to_string()), "{terms:?}");
    }

    #[test]
    fn an_empty_question_produces_nothing_rather_than_panicking() {
        let terms = to_search_terms("   ");
        assert!(terms.terms.is_empty());
        assert!(terms.query.is_empty());
    }

    #[test]
    fn an_unterminated_quote_does_not_hang_or_panic() {
        let terms = to_search_terms(r#"What did I write about "unclosed quote"#);
        assert!(!terms.query.is_empty(), "{terms:?}");
    }
}
