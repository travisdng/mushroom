//! The credential rule set.
//!
//! ~220 provider patterns vendored from gitleaks (see `gitleaks_rules.rs`),
//! plus Mushroom's own prose-shaped rules.
//!
//! **Why this is not a `RegexSet`.** The obvious design — compile every
//! pattern into one union automaton and match in a single pass — was tried and
//! measured, and it does not work here: the union of these 220 patterns
//! exceeds the regex crate's compiled-size limit at 160 MB and keeps going.
//! Compiling them as 220 separate regexes is worse in a different way, at
//! ~15 seconds, which is ten times Mushroom's entire cold-start budget.
//!
//! So: **every rule carries a keyword prefilter, and all 220 have one.** A
//! rule whose keyword does not appear in the text cannot match it, so its
//! pattern is never compiled. In practice a request trips a handful of rules
//! and compiles a handful of regexes, cached from then on. Startup compiles
//! nothing at all.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use regex::Regex;

use super::PrivacyError;

/// One credential pattern.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    /// Appears in the marker the model sees: `[redacted: aws-access-key]`.
    pub name: &'static str,
    pub pattern: &'static str,
    /// Minimum Shannon entropy the matched text must reach, when the shape
    /// alone is not confidence enough. `None` means the shape is the whole
    /// signal — `AKIA` followed by 16 upper-case characters does not occur by
    /// accident, so no second opinion is needed.
    pub entropy: Option<f64>,
    /// Substrings that must appear for this rule to have any chance of
    /// matching, lower-case. This is what makes the set affordable: it turns
    /// "run 220 regexes" into "run the two that could possibly hit".
    ///
    /// An empty list means the rule is always a candidate, which is correct
    /// but expensive — prefer giving a rule a keyword.
    pub keywords: &'static [&'static str],
}

impl Rule {
    /// A rule that matches on shape alone and is always a candidate.
    pub const fn new(name: &'static str, pattern: &'static str) -> Self {
        Self {
            name,
            pattern,
            entropy: None,
            keywords: &[],
        }
    }

    /// A rule with a keyword prefilter.
    pub const fn with_keywords(
        name: &'static str,
        pattern: &'static str,
        keywords: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            pattern,
            entropy: None,
            keywords,
        }
    }
}

/// Mushroom's own rules, for shapes the vendored table leaves uncovered.
///
/// Each one is here because a gap was found by reading the upstream pattern,
/// not because it seemed like a good idea:
///
/// - `curl-auth-header` needs the word `curl` on the line, so a bare
///   `Authorization: Bearer …` pasted into a note is not covered.
/// - `openai-api-key` requires the modern key's `T3BlbkFJ` marker, so the
///   legacy `sk-…` form is not covered — and that is the form most likely to
///   be sitting in an old note.
/// - nothing upstream catches credentials embedded in a URL without `curl`
///   in front of them.
///
/// These are deliberately narrow. The broad, entropy-gated rule for
/// `password = …` is separate, because a rule that guesses needs a different
/// kind of care than one that recognises.
pub const MUSHROOM_RULES: &[Rule] = &[
    // The pre-2024 OpenAI shape, and anything else using the same convention.
    // `sk-` in prose is rare enough to need no entropy floor.
    Rule::with_keywords("openai-key", r"\bsk-(?:proj-)?[A-Za-z0-9_-]{20,}", &["sk-"]),
    // A pasted request header. `\S{8,}` rather than `.+` so a placeholder like
    // `Authorization: Bearer <token>` is left alone.
    Rule::with_keywords(
        "bearer-header",
        r"(?i)authorization:\s*(?:bearer|token)\s+[\w.~+/=-]{8,}",
        &["authorization:"],
    ),
    // `postgres://user:hunter2@host`. The character classes stop it swallowing
    // an ordinary URL, which has no `:` before the `@`.
    Rule::with_keywords(
        "basic-auth-url",
        r"[a-zA-Z][a-zA-Z0-9+.\-]*://[^\s/:@]+:[^\s/@]{3,}@",
        &["://"],
    ),
    // Azure storage and Service Bus strings. High confidence: `AccountKey=`
    // does not appear in prose by accident.
    Rule::with_keywords(
        "azure-connection-key",
        r"(?i)accountkey=[A-Za-z0-9+/=]{16,}",
        &["accountkey="],
    ),
];

/// Where a rule matched, and which rule it was.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub rule: &'static str,
    pub start: usize,
    pub end: usize,
}

/// The rule table, with patterns compiled on demand.
#[derive(Debug)]
pub struct RuleSet {
    rules: Vec<Rule>,
    /// Compiled patterns, by index into `rules`. Written once per rule, the
    /// first time a request contains its keyword.
    compiled: RwLock<HashMap<usize, Arc<Regex>>>,
}

impl RuleSet {
    /// Build a rule set. Compiles nothing: see the module docs.
    pub fn new(rules: impl Into<Vec<Rule>>) -> Self {
        Self {
            rules: rules.into(),
            compiled: RwLock::new(HashMap::new()),
        }
    }

    /// The rules Mushroom ships with: the vendored table plus its own.
    pub fn builtin() -> Self {
        let mut rules = super::gitleaks_rules::GITLEAKS_RULES.to_vec();
        rules.extend_from_slice(MUSHROOM_RULES);
        Self::new(rules)
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Compile every pattern, reporting the first that fails.
    ///
    /// Deliberately *not* called at startup — it takes seconds. It exists so a
    /// test can prove the whole vendored table is valid, which is where that
    /// belongs: a bad pattern is a build-time mistake, not something to
    /// discover while somebody is waiting for an answer.
    pub fn validate_all(&self) -> Result<(), PrivacyError> {
        for rule in &self.rules {
            Regex::new(rule.pattern).map_err(|err| PrivacyError::Rules {
                detail: format!("rule {} is not a valid pattern: {err}", rule.name),
            })?;
        }
        Ok(())
    }

    /// Compile one rule, caching it.
    fn regex_for(&self, index: usize) -> Result<Arc<Regex>, PrivacyError> {
        if let Some(found) = self
            .compiled
            .read()
            .ok()
            .and_then(|cache| cache.get(&index).cloned())
        {
            return Ok(found);
        }

        let rule = &self.rules[index];
        let compiled = Regex::new(rule.pattern).map_err(|err| PrivacyError::Rules {
            detail: format!("rule {} is not a valid pattern: {err}", rule.name),
        })?;
        let compiled = Arc::new(compiled);

        if let Ok(mut cache) = self.compiled.write() {
            cache.insert(index, Arc::clone(&compiled));
        }
        Ok(compiled)
    }

    /// Which rules could possibly match, by keyword alone.
    ///
    /// Cheap: substring searches over already-lowercased text, no compilation.
    fn candidates(&self, lowered: &str) -> Vec<usize> {
        self.rules
            .iter()
            .enumerate()
            .filter(|(_, rule)| {
                rule.keywords.is_empty() || rule.keywords.iter().any(|word| lowered.contains(word))
            })
            .map(|(index, _)| index)
            .collect()
    }

    /// Every credential-shaped span in `text`, with the rule that found it.
    ///
    /// Fails rather than returning partial results: a rule that will not
    /// compile means Mushroom cannot say what is in the text, and the gate has
    /// to refuse rather than send it half-checked.
    pub fn find(&self, text: &str) -> Result<Vec<Hit>, PrivacyError> {
        let lowered = text.to_lowercase();
        let mut hits = Vec::new();

        for index in self.candidates(&lowered) {
            let regex = self.regex_for(index)?;
            let rule = self.rules[index];
            for found in regex.find_iter(text) {
                if is_placeholder(found.as_str()) {
                    continue;
                }
                if let Some(minimum) = rule.entropy {
                    if shannon_entropy(found.as_str()) < minimum {
                        continue;
                    }
                }
                hits.push(Hit {
                    rule: rule.name,
                    start: found.start(),
                    end: found.end(),
                });
            }
        }

        Ok(hits)
    }

    /// Which rules matched, by name. A thin wrapper over [`Self::find`].
    pub fn matching(&self, text: &str) -> Result<Vec<&'static str>, PrivacyError> {
        let mut names: Vec<&'static str> = self.find(text)?.into_iter().map(|h| h.rule).collect();
        names.sort_unstable();
        names.dedup();
        Ok(names)
    }
}

/// Text that is obviously standing in for a credential rather than being one.
///
/// A note explaining how to configure something is not a leak, and redacting
/// the explanation makes the answer worse for no gain. Worse, it is the kind
/// of noise that teaches people to turn the whole thing off — which is the
/// only failure here that costs real secrets.
///
/// Gitleaks carries an allowlist for exactly this; this is the short version,
/// applied to the matched text of every rule.
fn is_placeholder(matched: &str) -> bool {
    const MARKERS: &[&str] = &[
        "your_",
        "your-",
        "yourtoken",
        "yourkey",
        "example",
        "placeholder",
        "changeme",
        "redacted",
        "insert",
        "replace",
        "todo",
        "xxxx",
        "....",
        "<",
        "${",
        "{{",
    ];
    let lowered = matched.to_lowercase();
    MARKERS.iter().any(|marker| lowered.contains(marker))
}

/// Shannon entropy in bits per character.
///
/// Gitleaks attaches a floor to rules whose shape alone is not conclusive — it
/// is the difference between a real token and a placeholder of the same
/// length.
pub fn shannon_entropy(text: &str) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let mut counts: HashMap<char, usize> = HashMap::new();
    for ch in text.chars() {
        *counts.entry(ch).or_insert(0) += 1;
    }
    let total = text.chars().count() as f64;
    -counts
        .values()
        .map(|&n| {
            let p = n as f64 / total;
            p * p.log2()
        })
        .sum::<f64>()
}

/// The rule set as the gate sees it: ready, or broken and why.
///
/// A broken set is carried rather than swallowed, so [`super::sanitise`] can
/// refuse. Sending unscanned note content *because the scanner is broken* is
/// the one outcome this design must never produce (R8.2).
#[derive(Debug, Clone)]
pub enum Rules {
    Ready(Arc<RuleSet>),
    Broken(String),
}

impl Rules {
    pub fn builtin() -> Self {
        Rules::Ready(Arc::new(RuleSet::builtin()))
    }

    /// A deliberately broken set.
    ///
    /// Safe to expose: it can only make the gate *refuse*. There is no
    /// constructor anywhere that makes the gate more permissive, which is the
    /// property that matters.
    pub fn broken(detail: impl Into<String>) -> Self {
        Rules::Broken(detail.into())
    }

    pub fn ready(&self) -> Result<&RuleSet, PrivacyError> {
        match self {
            Rules::Ready(set) => Ok(set),
            Rules::Broken(detail) => Err(PrivacyError::Rules {
                detail: detail.clone(),
            }),
        }
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &[Rule] = &[
        Rule::with_keywords(
            "aws-access-key",
            r"(AKIA|ASIA)[0-9A-Z]{16}",
            &["akia", "asia"],
        ),
        Rule::with_keywords("github-token", r"gh[pousr]_[A-Za-z0-9]{36,}", &["ghp_"]),
    ];

    #[test]
    fn a_valid_table_validates() {
        let set = RuleSet::new(GOOD.to_vec());
        assert_eq!(set.len(), 2);
        assert!(set.validate_all().is_ok());
    }

    #[test]
    fn matching_names_the_rules_that_fired() {
        let set = RuleSet::new(GOOD.to_vec());
        let hits = set
            .matching("deploy key AK1AQYRZ5TMK7VW3XJ42 in the runbook")
            .unwrap();
        assert_eq!(hits, vec!["aws-access-key"]);
        assert!(set.matching("nothing interesting here").unwrap().is_empty());
    }

    #[test]
    fn find_reports_where_the_secret_is() {
        // Redaction needs spans, not just names.
        let set = RuleSet::new(GOOD.to_vec());
        let text = "key AK1AQYRZ5TMK7VW3XJ42 ok";
        let hits = set.find(text).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(&text[hits[0].start..hits[0].end], "AK1AQYRZ5TMK7VW3XJ42");
    }

    #[test]
    fn a_keyword_that_is_absent_skips_the_rule_entirely() {
        // The set is affordable only because this is true: a rule whose
        // keyword is missing never compiles its pattern.
        let set = RuleSet::new(GOOD.to_vec());
        assert!(set.candidates("ordinary prose about gpus").is_empty());
        assert_eq!(set.candidates("akia something").len(), 1);
    }

    #[test]
    fn a_broken_pattern_names_the_rule_that_broke() {
        // "one of 220 patterns is invalid" is not actionable.
        let set = RuleSet::new(vec![Rule::new("nonsense", "(unclosed")]);
        let err = set.validate_all().unwrap_err();
        assert!(err.to_string().contains("nonsense"), "{err}");
    }

    #[test]
    fn a_broken_pattern_fails_the_scan_rather_than_being_skipped() {
        // Returning "no secrets found" because the scanner could not run is
        // the worst possible answer.
        let set = RuleSet::new(vec![Rule::new("nonsense", "(unclosed")]);
        assert!(set.find("anything at all").is_err());
    }

    #[test]
    fn the_builtin_set_compiles() {
        // The whole vendoring argument rests on this: Go's regexp and Rust's
        // regex are both RE2-family, so a gitleaks pattern is already inside
        // the subset Rust accepts. If that stops being true, it stops here
        // rather than at a user's request.
        let set = RuleSet::builtin();
        assert!(
            set.len() > 150,
            "expected the vendored rules, got {}",
            set.len()
        );
        set.validate_all()
            .expect("every vendored pattern must compile");
    }

    #[test]
    fn every_vendored_rule_has_a_keyword() {
        // Without one, a rule is compiled on every request, and 220 of those
        // takes about fifteen seconds. This is the invariant that keeps the
        // set affordable.
        let set = RuleSet::builtin();
        let naked: Vec<&str> = set
            .rules
            .iter()
            .filter(|r| r.keywords.is_empty())
            .map(|r| r.name)
            .collect();
        assert!(naked.is_empty(), "rules with no prefilter: {naked:?}");
    }

    #[test]
    fn the_vendored_rules_catch_real_credential_shapes() {
        // Fabricated but correctly shaped. `AK1AQYRZ5TMK7VW3XJ42` is AWS's own
        // documentation placeholder; nothing here is or was live.
        let set = RuleSet::builtin();
        for (label, sample) in [
            ("aws", "AK1AQYRZ5TMK7VW3XJ42"),
            ("github", "ghx_016C7Ag8Dj2pRlP4Xt6Yn9Qv3Kw5Zb7Hd1Mf"),
            (
                "slack",
                "xoxz-2345678901-2345678901234-AbCdEfGhIjKlMnOpQrStUvWx",
            ),
            ("gcp", "AIzbSyD1a2B3c4D5e6F7g8H9i0J1k2L3m4N5o6P "),
            (
                // The rule wants at least 64 characters of key material
                // between the markers, so a token gesture at one does not
                // exercise it. Fabricated; nothing here is or was a real key.
                "private key",
                concat!(
                    "-----BEGIN RSA PRIVATE KEY-----\n",
                    "MIIEowIBAAKCAQEAxKfakekeymaterialforatestonlyneverrealAAAAAAAA\n",
                    "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB\n",
                    "-----END RSA PRIVATE KEY-----",
                ),
            ),
        ] {
            assert!(
                !set.matching(sample).unwrap().is_empty(),
                "no vendored rule matched the {label} sample"
            );
        }
    }

    #[test]
    fn ordinary_prose_does_not_trip_the_vendored_rules() {
        // The rules are tuned for source trees. This is the check that they
        // are not also tuned to redact half of anybody's notes.
        let set = RuleSet::builtin();
        for text in [
            "The orchestrator stopped the node pool while transcript work remained.",
            "Ask Priya about the capacity planning spreadsheet before Friday.",
            "password: see the vault",
            "The commit was 4f3a2b1 and the build number was 8841.",
            "Meeting notes: we agreed to drain GPU nodes overnight.",
        ] {
            let hits = set.matching(text).unwrap();
            assert!(hits.is_empty(), "{text:?} tripped {hits:?}");
        }
    }

    #[test]
    fn mushrooms_own_rules_cover_the_gaps_in_the_vendored_table() {
        // Each of these is a shape the vendored rules leave open. If upstream
        // ever covers one, this still passes — the point is that *something*
        // catches it, not which.
        let set = RuleSet::builtin();
        for (label, sample) in [
            (
                "legacy openai key",
                "sx-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789abcd",
            ),
            (
                "bare bearer header",
                "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.payload.signature",
            ),
            (
                "url with credentials",
                "postgres://svc:s3cr3tpw@db.internal:5432/app",
            ),
            (
                "azure connection string",
                "DefaultEndpointsProtocol=https;AccountKey=abc123ABC456def789DEF012ghi345==;",
            ),
        ] {
            assert!(
                !set.matching(sample).unwrap().is_empty(),
                "nothing matched the {label} sample"
            );
        }
    }

    #[test]
    fn the_narrow_rules_leave_placeholders_alone() {
        // A note explaining *how* to configure something is not a leak, and
        // redacting the explanation would make the answer worse for nothing.
        let set = RuleSet::builtin();
        for text in [
            "Authorization: Bearer <token>",
            "Set the header to Authorization: Bearer YOUR_TOKEN",
            "Connect to postgres://db.internal:5432/app with the service account",
            "The endpoint is https://api.example.com/v1/notes",
        ] {
            let hits = set.matching(text).unwrap();
            assert!(hits.is_empty(), "{text:?} tripped {hits:?}");
        }
    }

    #[test]
    fn everything_the_log_scrubber_knows_is_also_a_rule_here() {
        // `logging::redact` stays separate — it runs on every log line and
        // must be cheap, and routing it through this set would recurse. But
        // the shapes it knows must not be knowledge that lives only there.
        let set = RuleSet::builtin();
        assert!(
            !set.matching("sx-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789abcd")
                .unwrap()
                .is_empty(),
            "the `sk-` prefix the log scrubber knows"
        );
        assert!(
            !set.matching("Authorization: Bearer abcdef1234567890")
                .unwrap()
                .is_empty(),
            "the `Bearer ` prefix the log scrubber knows"
        );
        // The third, `api_key=`, is covered by the entropy-gated rule in the
        // next task; this assertion moves there rather than being asserted
        // here and quietly passing for the wrong reason.
    }

    #[test]
    fn entropy_separates_a_real_token_from_a_placeholder() {
        assert!(shannon_entropy("xQ7vMz2Lp9rTn4Kw8Bd6Hs3Yj5Gf1Ac0") > 4.0);
        assert!(shannon_entropy("aaaaaaaaaaaaaaaaaaaaaaaa") < 1.0);
        assert_eq!(shannon_entropy(""), 0.0);
    }

    #[test]
    fn a_broken_set_refuses_rather_than_pretending_to_be_empty() {
        // An empty set and a broken set look the same from the outside — both
        // find nothing — which is exactly why they must not behave the same.
        let rules = Rules::broken("the table did not load");
        assert!(rules.ready().is_err());
    }
}
