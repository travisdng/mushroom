//! Turning what a person typed into an FTS5 expression.
//!
//! User input is never concatenated into SQL, and never into FTS5 syntax
//! either: a stray quote or operator would otherwise turn a search into a
//! parse error. Anything that cannot be understood degrades to a literal
//! phrase search rather than an error dialog (R3.8).

/// Longest query we will build an expression from. Beyond this the extra words
/// add nothing and the FTS5 parser starts to cost real time.
const MAX_TERMS: usize = 32;

/// An FTS5 MATCH expression, always safe to bind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtsQuery {
    pub expression: String,
    /// The bare words, for highlighting and for explaining what was searched.
    pub terms: Vec<String>,
}

impl FtsQuery {
    pub fn is_empty(&self) -> bool {
        self.expression.trim().is_empty()
    }
}

/// Escape a bare word into an FTS5 string literal.
fn quote(term: &str) -> String {
    format!("\"{}\"", term.replace('"', "\"\""))
}

/// Split a query into quoted phrases and bare words, keeping order.
fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut current = String::new();

    let flush = |current: &mut String, tokens: &mut Vec<Token>| {
        if !current.is_empty() {
            tokens.push(Token::Word(std::mem::take(current)));
        }
    };

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                flush(&mut current, &mut tokens);
                let mut phrase = String::new();
                // An unterminated quote simply runs to the end of the input,
                // rather than failing.
                for c in chars.by_ref() {
                    if c == '"' {
                        break;
                    }
                    phrase.push(c);
                }
                if !phrase.trim().is_empty() {
                    tokens.push(Token::Phrase(phrase.trim().to_string()));
                }
            }
            c if c.is_whitespace() => flush(&mut current, &mut tokens),
            c => current.push(c),
        }
    }
    flush(&mut current, &mut tokens);
    tokens
}

#[derive(Debug)]
enum Token {
    Word(String),
    Phrase(String),
}

/// How positive terms are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    /// Every term must appear. What someone typing two keywords into the
    /// search box means.
    All,
    /// Any term may appear, ranked by how well it matches.
    ///
    /// For questions. A question expands to five or six terms, and requiring
    /// all of them finds nothing: "gpu nodes shutting down too early" would
    /// need a note containing the words "shutting" and "too". Recall is what
    /// matters when the result is context for a model — bm25 puts the notes
    /// matching more terms first, and an answer with nothing to cite is caught
    /// by the citation check rather than by the query.
    Any,
}

/// Build an FTS5 expression from a person's query, requiring every term.
///
/// Supports quoted phrases, a leading `-` for exclusion, and a trailing `*`
/// for prefix matching. Every other character FTS5 would treat as syntax is
/// escaped into a literal.
pub fn parse(input: &str) -> FtsQuery {
    parse_with(input, Match::All)
}

/// Build an expression that matches any of the terms. See [`Match::Any`].
pub fn parse_any(input: &str) -> FtsQuery {
    parse_with(input, Match::Any)
}

pub fn parse_with(input: &str, mode: Match) -> FtsQuery {
    let mut clauses: Vec<String> = Vec::new();
    let mut terms: Vec<String> = Vec::new();

    for token in tokenize(input).into_iter().take(MAX_TERMS) {
        match token {
            Token::Phrase(phrase) => {
                clauses.push(quote(&phrase));
                terms.extend(phrase.split_whitespace().map(str::to_string));
            }
            Token::Word(word) => {
                let negated = word.starts_with('-') && word.len() > 1;
                let body = if negated { &word[1..] } else { &word[..] };

                let prefix = body.ends_with('*') && body.len() > 1;
                let bare: String = body
                    .trim_end_matches('*')
                    // Strip anything FTS5 treats as an operator. Keeping it
                    // would be a parse error; escaping it is what the user meant.
                    .chars()
                    .filter(|c| !matches!(c, '(' | ')' | ':' | '^' | '"' | '*'))
                    .collect();

                // A "term" of pure punctuation ("--", "...") is not something
                // anyone searched for; dropping it beats emitting NOT "-".
                if !bare.chars().any(|c| c.is_alphanumeric()) {
                    continue;
                }

                let mut clause = quote(&bare);
                if prefix {
                    // FTS5 prefix syntax lives outside the quotes.
                    clause = format!("{clause} *");
                    clause = clause.replace("\" *", "\"*");
                }
                if negated {
                    clause = format!("NOT {clause}");
                } else {
                    terms.push(bare.clone());
                }
                clauses.push(clause);
            }
        }
    }

    // FTS5 has no implicit AND across a mix of NOT clauses, so join the
    // positive ones with AND and append the negatives.
    let (positive, negative): (Vec<_>, Vec<_>) =
        clauses.into_iter().partition(|c| !c.starts_with("NOT "));

    let joiner = match mode {
        Match::All => " AND ",
        Match::Any => " OR ",
    };
    let mut expression = positive.join(joiner);
    for clause in negative {
        if expression.is_empty() {
            // A query of only exclusions matches nothing useful; drop them.
            continue;
        }
        expression.push(' ');
        expression.push_str(&clause);
    }

    FtsQuery { expression, terms }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_mode_joins_with_or_for_questions() {
        // Requiring all of a question's terms finds nothing: this is the
        // difference between the brief's example question working and not.
        let q = parse_any("gpu nodes shutting down too early");
        assert!(q.expression.contains(" OR "), "{}", q.expression);
        assert!(!q.expression.contains(" AND "), "{}", q.expression);
        assert_eq!(q.terms.len(), 6);
    }

    #[test]
    fn any_mode_still_excludes_negated_terms() {
        let q = parse_any("gpu -failure");
        assert!(q.expression.contains("NOT"), "{}", q.expression);
        assert!(!q.terms.contains(&"failure".to_string()));
    }

    #[test]
    fn plain_words_become_an_and_query() {
        let q = parse("gpu failure");
        assert_eq!(q.expression, "\"gpu\" AND \"failure\"");
        assert_eq!(q.terms, vec!["gpu", "failure"]);
    }

    #[test]
    fn quoted_phrases_stay_together() {
        let q = parse("\"node pool\" gpu");
        assert!(q.expression.contains("\"node pool\""), "{}", q.expression);
        assert!(q.expression.contains("\"gpu\""));
    }

    #[test]
    fn exclusion_uses_not() {
        let q = parse("gpu -azure");
        assert!(q.expression.starts_with("\"gpu\""), "{}", q.expression);
        assert!(q.expression.contains("NOT \"azure\""), "{}", q.expression);
        assert_eq!(q.terms, vec!["gpu"], "an excluded word is not highlighted");
    }

    #[test]
    fn prefix_matching_is_supported() {
        let q = parse("orch*");
        assert_eq!(q.expression, "\"orch\"*");
    }

    #[test]
    fn unbalanced_quotes_do_not_error() {
        let q = parse("\"unterminated phrase");
        assert!(!q.is_empty());
        assert!(q.expression.contains("unterminated phrase"));
    }

    #[test]
    fn fts5_operators_in_plain_text_are_escaped() {
        // These would all be parse errors if passed through.
        for evil in [
            "NEAR(a b)",
            "column:value",
            "^anchored",
            "a AND (b OR c)",
            "\"\"\"",
            "*",
            "()",
        ] {
            let q = parse(evil);
            // The point is only that it does not blow up and produces
            // something bindable.
            assert!(
                q.expression.is_empty() || q.expression.contains('"'),
                "{evil:?} produced {:?}",
                q.expression
            );
        }
    }

    #[test]
    fn sql_injection_shaped_input_is_just_quoted_text() {
        let q = parse("'; DROP TABLE notes; --");

        // The words survive as *quoted literals*. They are also bound as a
        // parameter rather than concatenated, so this is belt and braces.
        assert!(q.expression.contains("\"DROP\""), "{}", q.expression);
        assert!(q.expression.contains("\"TABLE\""), "{}", q.expression);

        // Nothing FTS5 would read as an operator escaped the quoting: every
        // run of characters outside quotes is only AND / NOT / whitespace.
        let outside: String = q
            .expression
            .split('"')
            .step_by(2)
            .collect::<Vec<_>>()
            .join(" ");
        for word in outside.split_whitespace() {
            assert!(
                word == "AND" || word == "NOT",
                "unquoted {word:?} in {}",
                q.expression
            );
        }
    }

    #[test]
    fn punctuation_only_terms_are_dropped() {
        let q = parse("gpu -- ... !!");
        assert_eq!(q.expression, "\"gpu\"", "got {}", q.expression);
    }

    #[test]
    fn empty_and_whitespace_queries_are_empty() {
        assert!(parse("").is_empty());
        assert!(parse("   \t  ").is_empty());
        assert!(parse("* ( ) :").is_empty());
    }

    #[test]
    fn emoji_and_unicode_survive() {
        let q = parse("café 日本語 🍄");
        assert!(q.expression.contains("café"));
        assert!(q.expression.contains("日本語"));
    }

    #[test]
    fn a_very_long_query_is_bounded() {
        let long = (0..500)
            .map(|i| format!("word{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let q = parse(&long);
        assert!(q.terms.len() <= MAX_TERMS, "got {} terms", q.terms.len());
    }

    #[test]
    fn a_query_of_only_exclusions_is_dropped() {
        // "NOT x" alone matches every row, which is not what anyone means.
        let q = parse("-azure");
        assert!(q.is_empty(), "got {:?}", q.expression);
    }
}
