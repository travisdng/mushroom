//! Making a diagnostics report safe to paste anywhere.
//!
//! The report exists so a person can send it to someone else without reading
//! it first. That only works if it cannot contain anything private: no API
//! key, no note title, no note content, and no absolute path that names the
//! person or their machine.
//!
//! Redaction happens on the way out rather than at every call site that builds
//! a line, for the same reason log scrubbing does: a rule applied in one place
//! cannot be forgotten in another.

use std::path::Path;

/// Rewrite a report so it carries no personal information.
///
/// `notes_root` is rewritten to `<notes>` and the user profile to `<user>`, so
/// paths stay legible as structure without naming anyone.
pub fn redact_report(report: &str, notes_root: Option<&Path>, home: Option<&Path>) -> String {
    // Credentials first: a key inside a path must not survive because the path
    // rule happened to run first.
    let mut out = crate::logging::redact(report);

    // Longest first, so `C:\Users\alice\Mushroom\notes` becomes `<notes>`
    // rather than `<user>\Mushroom\notes`.
    let mut replacements: Vec<(String, &str)> = Vec::new();
    if let Some(root) = notes_root {
        replacements.push((root.to_string_lossy().to_string(), "<notes>"));
    }
    if let Some(home) = home {
        replacements.push((home.to_string_lossy().to_string(), "<user>"));
    }
    replacements.sort_by_key(|(from, _)| std::cmp::Reverse(from.len()));

    for (from, to) in replacements {
        if from.is_empty() {
            continue;
        }
        out = replace_ignoring_case_and_separators(&out, &from, to);
    }

    out
}

/// Replace `from` with `to`, treating `/` and `\` as the same and ignoring
/// case — which is how Windows paths actually turn up in logs.
fn replace_ignoring_case_and_separators(haystack: &str, from: &str, to: &str) -> String {
    let normalise = |s: &str| s.replace('/', "\\").to_lowercase();
    let flat_hay = normalise(haystack);
    let flat_from = normalise(from);

    if flat_from.is_empty() || !flat_hay.contains(&flat_from) {
        return haystack.to_string();
    }

    let mut out = String::with_capacity(haystack.len());
    let mut at = 0;

    while let Some(found) = flat_hay[at..].find(&flat_from) {
        let start = at + found;
        // Byte offsets line up because normalising only changes `/` to `\`
        // and ASCII case, neither of which changes a character's length.
        out.push_str(&haystack[at..start]);
        out.push_str(to);
        at = start + flat_from.len();
    }
    out.push_str(&haystack[at..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notes() -> &'static Path {
        Path::new(r"C:\Users\alice\Mushroom\notes")
    }

    fn home() -> &'static Path {
        Path::new(r"C:\Users\alice")
    }

    #[test]
    fn an_api_key_never_survives() {
        let report = "endpoint=https://api.openai.com key=sk-live-SECRETVALUE123 ok";
        let out = redact_report(report, Some(notes()), Some(home()));
        assert!(!out.contains("SECRETVALUE123"), "{out}");
    }

    #[test]
    fn a_bearer_header_never_survives() {
        let out = redact_report(
            "Authorization: Bearer sk-abc-NEVERSHOWTHIS",
            Some(notes()),
            Some(home()),
        );
        assert!(!out.contains("NEVERSHOWTHIS"), "{out}");
    }

    #[test]
    fn the_notes_root_becomes_a_placeholder() {
        let out = redact_report(
            r"reading C:\Users\alice\Mushroom\notes\work\gpu.md",
            Some(notes()),
            Some(home()),
        );
        assert_eq!(out, r"reading <notes>\work\gpu.md");
        assert!(!out.contains("alice"));
    }

    #[test]
    fn the_user_profile_becomes_a_placeholder() {
        let out = redact_report(
            r"log at C:\Users\alice\AppData\Roaming\Mushroom\logs",
            Some(notes()),
            Some(home()),
        );
        assert_eq!(out, r"log at <user>\AppData\Roaming\Mushroom\logs");
    }

    #[test]
    fn the_notes_root_wins_over_the_profile_it_sits_inside() {
        // Replacing the profile first would leave `<user>\Mushroom\notes`,
        // which still says more than it needs to.
        let out = redact_report(
            r"C:\Users\alice\Mushroom\notes\a.md",
            Some(notes()),
            Some(home()),
        );
        assert!(out.starts_with("<notes>"), "{out}");
    }

    #[test]
    fn a_path_written_with_forward_slashes_is_still_rewritten() {
        // Rust's Display and the log both produce these.
        let out = redact_report(
            "reading C:/Users/alice/Mushroom/notes/work/gpu.md",
            Some(notes()),
            Some(home()),
        );
        assert!(!out.contains("alice"), "{out}");
        assert!(out.contains("<notes>"), "{out}");
    }

    #[test]
    fn case_differences_are_still_rewritten() {
        let out = redact_report(
            r"reading c:\users\ALICE\mushroom\NOTES\work\gpu.md",
            Some(notes()),
            Some(home()),
        );
        assert!(!out.to_lowercase().contains("alice"), "{out}");
    }

    #[test]
    fn several_occurrences_are_all_rewritten() {
        let out = redact_report(
            r"a C:\Users\alice\Mushroom\notes\x.md b C:\Users\alice\Mushroom\notes\y.md",
            Some(notes()),
            Some(home()),
        );
        assert_eq!(out.matches("<notes>").count(), 2, "{out}");
        assert!(!out.contains("alice"));
    }

    #[test]
    fn ordinary_text_is_left_alone() {
        let report = "version 1.0.0\nnotes indexed: 412\nsearch: 67 ms";
        assert_eq!(redact_report(report, Some(notes()), Some(home())), report);
    }

    #[test]
    fn missing_paths_are_not_an_error() {
        let out = redact_report("nothing to rewrite", None, None);
        assert_eq!(out, "nothing to rewrite");
    }

    #[test]
    fn a_key_inside_a_path_is_still_removed() {
        // Rewriting paths first could otherwise hide a key from the scrubber.
        let out = redact_report(
            r"C:\Users\alice\Mushroom\notes\sk-live-HIDDENKEY.md",
            Some(notes()),
            Some(home()),
        );
        assert!(!out.contains("HIDDENKEY"), "{out}");
    }

    #[test]
    fn non_ascii_content_does_not_corrupt_the_output() {
        // The replacement walks byte offsets; a multi-byte character must not
        // shift them.
        let report = r"café — reading C:\Users\alice\Mushroom\notes\café.md";
        let out = redact_report(report, Some(notes()), Some(home()));
        assert!(out.starts_with("café — reading <notes>"), "{out}");
    }
}
