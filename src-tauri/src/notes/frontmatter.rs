//! YAML frontmatter, handled as *text* rather than as parsed YAML.
//!
//! Mushroom owns `title`, `created`, `updated`, and `tags`. Every other key —
//! and every comment, blank line, quoting style, and ordering choice the user
//! made — is round-tripped byte for byte.
//!
//! That is why this is a line editor rather than a YAML library: serialising
//! through a YAML parser rewrites the whole block in the library's preferred
//! style, silently reformatting text the user wrote. Losing a comment or
//! reordering keys in someone's notes is exactly the kind of quiet damage
//! `.kiro/steering/data-integrity.md` forbids.

/// The parsed view of a note's frontmatter block.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Frontmatter {
    /// The block between the `---` fences, verbatim and without them.
    pub raw: Option<String>,
    pub title: Option<String>,
    pub created: Option<String>,
    pub updated: Option<String>,
    pub tags: Vec<String>,
    /// The user has marked this note as never to be sent to an AI endpoint,
    /// with `ai: false` or `private: true` (spec 08 R2.1).
    ///
    /// Read but never written: `render` does not own these keys, so they
    /// survive a save byte-for-byte like any other key Mushroom does not
    /// understand. Mushroom does not edit the user's text to protect them
    /// from it.
    pub ai_excluded: bool,
}

/// A note split into its frontmatter and its body.
#[derive(Debug, Clone)]
pub struct Split {
    pub frontmatter: Frontmatter,
    /// Everything after the closing fence, exactly as stored.
    pub body: String,
}

fn line_ending(text: &str) -> &'static str {
    // Match whatever the file already uses; never normalise it (R2.1).
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Split a note into frontmatter and body.
///
/// A file that does not open with a `---` fence has no frontmatter and its
/// content is the body untouched. A malformed block (no closing fence) is
/// treated the same way — the file is never rewritten to "fix" it (R2.4).
pub fn split(content: &str) -> Split {
    let trimmed = content.strip_prefix('\u{feff}').unwrap_or(content);

    let after_open = match trimmed.strip_prefix("---\r\n") {
        Some(rest) => rest,
        None => match trimmed.strip_prefix("---\n") {
            Some(rest) => rest,
            None => {
                return Split {
                    frontmatter: Frontmatter::default(),
                    body: content.to_string(),
                }
            }
        },
    };

    // The closing fence is a line that is exactly "---".
    let mut offset = 0usize;
    let mut close: Option<(usize, usize)> = None;
    for line in after_open.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\r', '\n']);
        if bare == "---" {
            close = Some((offset, offset + line.len()));
            break;
        }
        offset += line.len();
    }

    let Some((block_end, body_start)) = close else {
        // No closing fence: not frontmatter, just a file that starts with ---.
        return Split {
            frontmatter: Frontmatter::default(),
            body: content.to_string(),
        };
    };

    let raw = &after_open[..block_end];
    let body = &after_open[body_start..];

    Split {
        frontmatter: parse_block(raw),
        body: body.to_string(),
    }
}

/// Read the keys Mushroom owns out of a raw block. Everything else is ignored
/// here and preserved by [`render`].
fn parse_block(raw: &str) -> Frontmatter {
    let mut fm = Frontmatter {
        raw: Some(raw.to_string()),
        ..Default::default()
    };

    for line in raw.lines() {
        // Only top-level keys count: an indented line belongs to a nested
        // structure we do not own and must not interpret.
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();

        match key.trim() {
            "title" => fm.title = Some(unquote(value)).filter(|v| !v.is_empty()),
            "created" => fm.created = Some(unquote(value)).filter(|v| !v.is_empty()),
            "updated" => fm.updated = Some(unquote(value)).filter(|v| !v.is_empty()),
            "tags" => fm.tags = parse_inline_list(value),
            // `ai: false` and `private: true` both mean "never send this".
            // Two spellings because both are the obvious one to somebody, and
            // a privacy control that only works if you guessed the right word
            // is not a privacy control.
            "ai" => {
                if is_false(value) {
                    fm.ai_excluded = true;
                }
            }
            "private" => {
                if is_true(value) {
                    fm.ai_excluded = true;
                }
            }
            _ => {}
        }
    }

    fm
}

/// YAML's falsey spellings. `ai: false`, `ai: no`, `ai: off`, `ai: 0`.
fn is_false(value: &str) -> bool {
    matches!(
        unquote(value).trim().to_ascii_lowercase().as_str(),
        "false" | "no" | "off" | "0"
    )
}

/// YAML's truthy spellings, for `private:`.
fn is_true(value: &str) -> bool {
    matches!(
        unquote(value).trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "on" | "1"
    )
}

fn unquote(value: &str) -> String {
    let v = value.trim();
    if v.len() >= 2
        && ((v.starts_with('"') && v.ends_with('"')) || (v.starts_with('\'') && v.ends_with('\'')))
    {
        v[1..v.len() - 1].to_string()
    } else {
        v.to_string()
    }
}

/// Only the inline `[a, b]` form is understood. A block list is left alone:
/// unread, unmodified, and still in the file.
fn parse_inline_list(value: &str) -> Vec<String> {
    let v = value.trim();
    if !(v.starts_with('[') && v.ends_with(']')) {
        return Vec::new();
    }
    v[1..v.len() - 1]
        .split(',')
        .map(|part| unquote(part.trim()))
        .filter(|part| !part.is_empty())
        .collect()
}

fn quote_if_needed(value: &str) -> String {
    // YAML only needs a scalar quoted when a colon is *followed by a space*,
    // or a hash is preceded by one. Quoting on any bare colon would wrap every
    // ISO timestamp — `2026-01-01T00:00:00Z` is a perfectly valid plain scalar.
    let needs = value.is_empty()
        || value.starts_with(' ')
        || value.ends_with(' ')
        || value.contains(": ")
        || value.ends_with(':')
        || value.contains(" #")
        || value.starts_with([
            '[', '{', '"', '\'', '&', '*', '!', '|', '>', '%', '@', '`', '#',
        ]);
    if needs {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

/// Rebuild a full note, updating only the keys Mushroom owns.
///
/// Lines for owned keys are replaced in place, so their position in the block
/// is kept. Owned keys that were absent are appended. Every other line survives
/// exactly as it was.
pub fn render(fm: &Frontmatter, body: &str) -> String {
    let eol = line_ending(fm.raw.as_deref().unwrap_or(body));

    let mut updates: Vec<(&str, Option<String>)> = vec![
        ("title", fm.title.as_ref().map(|v| quote_if_needed(v))),
        ("created", fm.created.as_ref().map(|v| quote_if_needed(v))),
        ("updated", fm.updated.as_ref().map(|v| quote_if_needed(v))),
    ];
    if !fm.tags.is_empty() {
        let list = fm
            .tags
            .iter()
            .map(|t| quote_if_needed(t))
            .collect::<Vec<_>>()
            .join(", ");
        updates.push(("tags", Some(format!("[{list}]"))));
    }

    let previous = fm.raw.clone().unwrap_or_default();
    let mut out_lines: Vec<String> = Vec::new();
    let mut seen: Vec<&str> = Vec::new();

    for line in previous.lines() {
        let owned_key = if line.starts_with(char::is_whitespace) {
            None
        } else {
            line.split_once(':')
                .map(|(k, _)| k.trim())
                .filter(|k| updates.iter().any(|(name, _)| name == k))
        };

        match owned_key {
            Some(key) => {
                seen.push(match key {
                    "title" => "title",
                    "created" => "created",
                    "updated" => "updated",
                    _ => "tags",
                });
                if let Some((_, Some(value))) = updates.iter().find(|(name, _)| *name == key) {
                    out_lines.push(format!("{key}: {value}"));
                }
                // A key we own that is now None is dropped deliberately.
            }
            None => out_lines.push(line.to_string()),
        }
    }

    for (key, value) in &updates {
        if let (false, Some(value)) = (seen.contains(key), value) {
            out_lines.push(format!("{key}: {value}"));
        }
    }

    if out_lines.is_empty() {
        // Nothing to write: emit the body alone rather than an empty block.
        return body.to_string();
    }

    let mut out = String::new();
    out.push_str("---");
    out.push_str(eol);
    for line in out_lines {
        out.push_str(&line);
        out.push_str(eol);
    }
    out.push_str("---");
    out.push_str(eol);
    out.push_str(body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_without_frontmatter_is_all_body() {
        let content = "# Just a note\n\nSome text.\n";
        let split = split(content);
        assert!(split.frontmatter.raw.is_none());
        assert_eq!(split.body, content);
    }

    #[test]
    fn reads_owned_keys() {
        let content = "---\ntitle: AI AutoQA\ncreated: 2026-09-15T14:22:31Z\ntags: [gpu, azure]\n---\nBody here.\n";
        let s = split(content);
        assert_eq!(s.frontmatter.title.as_deref(), Some("AI AutoQA"));
        assert_eq!(
            s.frontmatter.created.as_deref(),
            Some("2026-09-15T14:22:31Z")
        );
        assert_eq!(s.frontmatter.tags, vec!["gpu", "azure"]);
        assert_eq!(s.body, "Body here.\n");
    }

    #[test]
    fn preserves_unknown_keys_comments_and_order() {
        let content = "---\n# a comment the user wrote\nzzz_custom: keep me\ntitle: Old\nnested:\n  a: 1\n  b: 2\nanother: value\n---\nBody\n";
        let mut s = split(content);
        s.frontmatter.title = Some("New".into());

        let out = render(&s.frontmatter, &s.body);

        assert!(out.contains("# a comment the user wrote"));
        assert!(out.contains("zzz_custom: keep me"));
        assert!(out.contains("nested:"));
        assert!(out.contains("  a: 1"));
        assert!(out.contains("  b: 2"));
        assert!(out.contains("another: value"));
        assert!(out.contains("title: New"));
        assert!(!out.contains("title: Old"));

        // The custom key must still come before title, as the user had it.
        let custom_at = out.find("zzz_custom").unwrap();
        let title_at = out.find("title: New").unwrap();
        assert!(custom_at < title_at, "key order must be preserved");
    }

    #[test]
    fn round_trips_unchanged_when_nothing_is_edited() {
        let content = "---\ntitle: Same\ncustom: thing\n---\nBody text\n";
        let s = split(content);
        assert_eq!(render(&s.frontmatter, &s.body), content);
    }

    #[test]
    fn appends_owned_keys_that_were_absent() {
        let content = "---\ncustom: thing\n---\nBody\n";
        let mut s = split(content);
        s.frontmatter.updated = Some("2026-01-01T00:00:00Z".into());
        let out = render(&s.frontmatter, &s.body);
        assert!(out.contains("custom: thing"));
        assert!(out.contains("updated: 2026-01-01T00:00:00Z"));
    }

    #[test]
    fn a_stray_fence_in_the_body_is_not_a_closing_fence() {
        let content = "---\ntitle: T\n---\nBody\n\n---\n\nMore body after a horizontal rule.\n";
        let s = split(content);
        assert_eq!(s.frontmatter.title.as_deref(), Some("T"));
        assert!(s.body.contains("More body after a horizontal rule."));
        assert!(s.body.starts_with("Body"));
    }

    #[test]
    fn unclosed_block_is_treated_as_body_not_frontmatter() {
        let content = "---\ntitle: never closed\n\nstill going\n";
        let s = split(content);
        assert!(s.frontmatter.raw.is_none(), "must not claim frontmatter");
        assert_eq!(s.body, content, "the file must be left exactly as it was");
    }

    #[test]
    fn preserves_crlf_line_endings() {
        let content = "---\r\ntitle: Windows\r\ncustom: x\r\n---\r\nBody\r\n";
        let mut s = split(content);
        s.frontmatter.title = Some("Changed".into());
        let out = render(&s.frontmatter, &s.body);
        assert!(out.contains("\r\n"), "CRLF must survive");
        assert!(!out.contains("\n\n"), "must not mix endings");
        assert!(out.contains("title: Changed\r\n"));
        assert!(out.ends_with("Body\r\n"));
    }

    #[test]
    fn quotes_values_that_need_it() {
        let mut fm = Frontmatter {
            raw: Some(String::new()),
            ..Default::default()
        };
        fm.title = Some("AI AutoQA: Batch Incident".into());
        let out = render(&fm, "Body\n");
        assert!(
            out.contains(r#"title: "AI AutoQA: Batch Incident""#),
            "a colon in a title must be quoted, got: {out}"
        );
    }

    #[test]
    fn leaves_timestamps_unquoted() {
        let mut fm = Frontmatter {
            raw: Some(String::new()),
            ..Default::default()
        };
        fm.updated = Some("2026-01-01T00:00:00Z".into());
        let out = render(
            &fm, "Body
",
        );
        assert!(
            out.contains("updated: 2026-01-01T00:00:00Z"),
            "an ISO timestamp is a valid plain scalar and must not be quoted: {out}"
        );
    }

    #[test]
    fn handles_quoted_values_on_read() {
        let content = "---\ntitle: \"Quoted: title\"\ncreated: '2026-01-01'\n---\nB\n";
        let s = split(content);
        assert_eq!(s.frontmatter.title.as_deref(), Some("Quoted: title"));
        assert_eq!(s.frontmatter.created.as_deref(), Some("2026-01-01"));
    }

    #[test]
    fn block_style_tag_lists_are_left_alone() {
        // We only read the inline form; a block list must survive untouched
        // rather than being flattened or dropped.
        let content = "---\ntags:\n  - gpu\n  - azure\ntitle: T\n---\nBody\n";
        let s = split(content);
        assert!(s.frontmatter.tags.is_empty(), "block lists are not parsed");
        let out = render(&s.frontmatter, &s.body);
        assert!(out.contains("  - gpu"));
        assert!(out.contains("  - azure"));
    }

    #[test]
    fn ignores_a_byte_order_mark() {
        let content = "\u{feff}---\ntitle: BOM\n---\nBody\n";
        let s = split(content);
        assert_eq!(s.frontmatter.title.as_deref(), Some("BOM"));
    }
}

#[cfg(test)]
mod exclusion_tests {
    use super::*;

    fn excluded(front: &str) -> bool {
        split(&format!("---\n{front}\n---\nbody\n"))
            .frontmatter
            .ai_excluded
    }

    #[test]
    fn ai_false_excludes_the_note() {
        assert!(excluded("ai: false"));
        assert!(excluded("ai: no"));
        assert!(excluded("ai: off"));
        assert!(excluded("ai: 0"));
        assert!(excluded("ai: FALSE"));
        assert!(excluded("ai: \"false\""));
    }

    #[test]
    fn private_true_excludes_the_note() {
        assert!(excluded("private: true"));
        assert!(excluded("private: yes"));
        assert!(excluded("private: on"));
        assert!(excluded("private: 1"));
    }

    #[test]
    fn the_opposite_values_do_not_exclude() {
        // Getting this backwards would be the worst kind of bug here: a note
        // the user deliberately opted *in* would stop reaching the AI, and
        // they would conclude the feature is broken rather than that they are
        // protected.
        assert!(!excluded("ai: true"));
        assert!(!excluded("private: false"));
        assert!(!excluded("title: Ordinary note"));
        assert!(!excluded("tags: [work]"));
    }

    #[test]
    fn an_indented_key_is_not_a_top_level_one() {
        // `ai: false` nested inside someone else's structure means something
        // to them, not to us.
        let split = split("---\nplugin:\n  ai: false\n---\nbody\n");
        assert!(!split.frontmatter.ai_excluded);
    }

    #[test]
    fn a_note_with_no_frontmatter_is_not_excluded() {
        assert!(!split("# Just a heading\n").frontmatter.ai_excluded);
    }

    #[test]
    fn saving_an_excluded_note_leaves_its_frontmatter_alone() {
        // The whole mechanism depends on this. If `render` ever started
        // owning `ai:`, a save would rewrite or drop the user's own marker
        // and the note would silently start being sent again.
        let original =
            "---\ntitle: Vault\nai: false\nprivate: true\ncustom: kept\n---\n# Vault\n\nbody\n";
        let parsed = split(original);
        assert!(parsed.frontmatter.ai_excluded);

        let rendered = render(&parsed.frontmatter, &parsed.body);
        assert!(rendered.contains("ai: false"), "{rendered}");
        assert!(rendered.contains("private: true"), "{rendered}");
        assert!(rendered.contains("custom: kept"), "{rendered}");

        // And round-tripping keeps it excluded.
        assert!(split(&rendered).frontmatter.ai_excluded);
        assert!(!rendered.is_empty());
    }
}
