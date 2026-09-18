//! Which notes the user has told Mushroom never to send.
//!
//! This is the half of the privacy gate that can actually be trusted. The
//! rule set in `rules.rs` guesses at what a credential looks like and will
//! miss things; a note matched here is never sent, full stop, in every privacy
//! mode including `Off`.
//!
//! Patterns are matched against the note id — its path relative to the notes
//! root, `/` separated — which is the identity the rest of the application
//! already uses.
//!
//! The matcher is written here rather than pulled from a glob crate because it
//! is thirty lines, because path matching is the most security-sensitive code
//! in this application (see `notes/paths.rs`), and because a second
//! path-matching implementation with its own opinions about edge cases is a
//! poor trade for those thirty lines.

use serde::{Deserialize, Serialize};

/// The trash is never sent, without anybody configuring it (R2.9).
///
/// A deleted note is the one a person is least likely to be thinking about
/// when they consider what their AI can read.
const ALWAYS_EXCLUDED: &[&str] = &[".trash/**"];

/// One user-supplied rule, and whether it made sense.
///
/// A rule that cannot be understood is kept rather than dropped, so Settings
/// can show it as broken. Both silent directions are wrong: silently excluding
/// makes the AI look broken, and silently including makes the gate a lie
/// (R2.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExclusionRule {
    pub pattern: String,
    /// `None` when the rule is usable; otherwise why it is not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

impl ExclusionRule {
    pub fn parse(pattern: &str) -> Self {
        let trimmed = pattern.trim();
        Self {
            pattern: trimmed.to_string(),
            problem: problem_with(trimmed),
        }
    }

    pub fn is_usable(&self) -> bool {
        self.problem.is_none()
    }
}

/// Why a pattern cannot be used, in words a person can act on.
fn problem_with(pattern: &str) -> Option<String> {
    if pattern.is_empty() {
        return Some("The rule is empty.".into());
    }
    if pattern.contains("..") {
        return Some("A rule cannot contain `..`. Write the path from the notes folder.".into());
    }
    if pattern.starts_with('/') || pattern.starts_with('\\') {
        return Some(
            "A rule is relative to the notes folder, so it cannot start with a slash.".into(),
        );
    }
    // `C:\secrets` — an absolute Windows path, which this can never match.
    if pattern.len() >= 2 && pattern.as_bytes()[1] == b':' {
        return Some("A rule is relative to the notes folder, not a full drive path.".into());
    }
    // `**` means "any number of folders" and only makes sense as a whole
    // segment. `a**b` reads like it should mean something and does not.
    //
    // Checked against the slash-normalised form: `Personal\Finance\**` is one
    // segment to `split('/')`, so validating the raw string rejected a
    // perfectly good Windows-style rule as malformed.
    if pattern
        .replace('\\', "/")
        .split('/')
        .any(|segment| segment.contains("**") && segment != "**")
    {
        return Some("`**` has to be a whole folder step, as in `personal/**`.".into());
    }
    None
}

/// The user's exclusions, compiled once.
#[derive(Debug, Clone, Default)]
pub struct Exclusions {
    patterns: Vec<String>,
}

impl Exclusions {
    /// Build from user rules, keeping only the usable ones.
    ///
    /// A broken rule excludes nothing — and Settings shows it as broken, so it
    /// is not silently doing nothing either.
    pub fn new(rules: &[ExclusionRule]) -> Self {
        let mut patterns: Vec<String> = ALWAYS_EXCLUDED.iter().map(|p| p.to_string()).collect();
        patterns.extend(
            rules
                .iter()
                .filter(|rule| rule.is_usable())
                .map(|rule| normalise(&rule.pattern)),
        );
        Self { patterns }
    }

    /// Does the user's configuration exclude this note?
    ///
    /// Frontmatter exclusion is checked separately, where the note is read.
    pub fn excludes(&self, note_id: &str) -> bool {
        let path = normalise(note_id);
        self.patterns.iter().any(|pattern| matches(pattern, &path))
    }

    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
}

/// Lower-case with forward slashes: how paths actually turn up on Windows.
fn normalise(value: &str) -> String {
    value.replace('\\', "/").to_lowercase()
}

/// Glob match over path segments, plus the folder-prefix shorthand.
fn matches(pattern: &str, path: &str) -> bool {
    let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if match_segments(&pattern_segments, &path_segments) {
        return true;
    }

    // Folder shorthand: `personal` excludes everything under `personal/`.
    // People type the folder, not `personal/**`, and a privacy control that
    // requires knowing glob syntax will be got wrong in the unsafe direction.
    // Only for wildcard-free patterns — a pattern with a wildcard said what it
    // meant.
    if pattern_segments.iter().any(|s| is_wildcard(s)) {
        return false;
    }
    path_segments.len() > pattern_segments.len()
        && pattern_segments
            .iter()
            .zip(&path_segments)
            .all(|(p, s)| p == s)
}

fn is_wildcard(segment: &str) -> bool {
    segment.contains('*') || segment.contains('?')
}

/// Recursive segment match, with `**` spanning any number of segments.
fn match_segments(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.first() {
        None => path.is_empty(),
        Some(&"**") => {
            // Zero segments, or one and try again.
            if match_segments(&pattern[1..], path) {
                return true;
            }
            (0..path.len()).any(|skip| match_segments(&pattern[1..], &path[skip + 1..]))
        }
        Some(head) => match path.first() {
            Some(segment) if match_one(head, segment) => match_segments(&pattern[1..], &path[1..]),
            _ => false,
        },
    }
}

/// `*` matches any run of characters within a segment; `?` matches one.
fn match_one(pattern: &str, segment: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = segment.chars().collect();
    let (mut pi, mut si) = (0usize, 0usize);
    // Where to resume if a `*` guess turns out to be wrong.
    let (mut star, mut resume) = (None, 0usize);

    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            resume = si;
            pi += 1;
        } else if let Some(at) = star {
            // Backtrack: let the star swallow one more character.
            pi = at + 1;
            resume += 1;
            si = resume;
        } else {
            return false;
        }
    }

    p[pi..].iter().all(|c| *c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(patterns: &[&str]) -> Exclusions {
        let parsed: Vec<ExclusionRule> = patterns.iter().map(|p| ExclusionRule::parse(p)).collect();
        Exclusions::new(&parsed)
    }

    #[test]
    fn a_double_star_covers_a_whole_folder_tree() {
        let ex = rules(&["personal/**"]);
        assert!(ex.excludes("personal/finances.md"));
        assert!(ex.excludes("personal/bank/statements.md"));
        assert!(!ex.excludes("work/gpu.md"));
        // Segment boundary: `personal/**` is not a prefix match on the name.
        assert!(!ex.excludes("personal-notes.md"));
    }

    #[test]
    fn a_leading_double_star_matches_at_any_depth_including_the_root() {
        let ex = rules(&["**/secrets.md"]);
        assert!(ex.excludes("secrets.md"));
        assert!(ex.excludes("work/secrets.md"));
        assert!(ex.excludes("work/deep/nested/secrets.md"));
        assert!(!ex.excludes("work/secrets-plan.md"));
    }

    #[test]
    fn a_star_matches_within_one_segment_only() {
        let ex = rules(&["work/cred*"]);
        assert!(ex.excludes("work/credentials.md"));
        assert!(ex.excludes("work/creds.md"));
        assert!(!ex.excludes("work/gpu.md"));
        // A star does not cross a folder boundary.
        assert!(!ex.excludes("work/creds/aws.md"));
    }

    #[test]
    fn a_question_mark_matches_exactly_one_character() {
        let ex = rules(&["work/key?.md"]);
        assert!(ex.excludes("work/key1.md"));
        assert!(!ex.excludes("work/key.md"));
        assert!(!ex.excludes("work/key12.md"));
    }

    #[test]
    fn a_bare_folder_name_excludes_what_is_under_it() {
        // People type the folder, not `personal/**`. A privacy control that
        // needs glob syntax to work will be got wrong in the unsafe direction.
        let ex = rules(&["personal"]);
        assert!(ex.excludes("personal/finances.md"));
        assert!(ex.excludes("personal/bank/statements.md"));
        assert!(!ex.excludes("personal.md"));
        assert!(!ex.excludes("work/personal-thoughts.md"));
    }

    #[test]
    fn matching_ignores_case_and_slash_direction() {
        let ex = rules(&["Personal\\Finance\\**"]);
        assert!(ex.excludes("personal/finance/isa.md"));
        assert!(ex.excludes("PERSONAL/FINANCE/isa.md"));
    }

    #[test]
    fn the_trash_is_excluded_without_anybody_configuring_it() {
        let ex = rules(&[]);
        assert!(ex.excludes(".trash/work/gpu.md.2026-01-01.md"));
        assert!(!ex.excludes("work/gpu.md"));
    }

    #[test]
    fn a_rule_that_matches_nothing_is_still_a_valid_rule() {
        let ex = rules(&["archive/2019/**"]);
        assert!(ex.excludes("archive/2019/old.md"));
        assert!(!ex.excludes("archive/2020/new.md"));
    }

    #[test]
    fn malformed_rules_are_reported_rather_than_guessed_at() {
        for (pattern, expect) in [
            ("", "empty"),
            ("../../etc/passwd", "`..`"),
            ("/personal", "slash"),
            ("C:\\secrets", "drive"),
            ("work/a**b", "whole folder step"),
        ] {
            let rule = ExclusionRule::parse(pattern);
            assert!(!rule.is_usable(), "{pattern:?} should be rejected");
            let problem = rule.problem.unwrap();
            assert!(
                problem.to_lowercase().contains(expect),
                "{pattern:?} said {problem:?}, expected it to mention {expect:?}"
            );
        }
    }

    #[test]
    fn a_broken_rule_excludes_nothing_and_does_not_break_the_others() {
        // The dangerous failure would be a broken rule silently swallowing
        // the good ones, leaving the user believing they are protected.
        let ex = rules(&["../nope", "personal/**"]);
        assert!(ex.excludes("personal/finances.md"));
        assert!(!ex.excludes("work/gpu.md"));
    }

    #[test]
    fn whitespace_around_a_rule_is_forgiven() {
        let rule = ExclusionRule::parse("  personal/**  ");
        assert!(rule.is_usable());
        assert_eq!(rule.pattern, "personal/**");
    }

    #[test]
    fn a_star_backtracks_rather_than_giving_up_on_the_first_guess() {
        // `*.md` against `a.md.md` is the classic case a greedy matcher that
        // cannot backtrack gets wrong.
        assert!(match_one("*.md", "a.md.md"));
        assert!(match_one("*secret*", "the-secret-file"));
        assert!(!match_one("*.md", "notes.txt"));
    }
}
