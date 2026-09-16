//! Markdown to searchable passages.
//!
//! A *passage* is a heading-delimited section with the line range it occupies
//! in the original file. Storing passages rather than whole notes is what lets
//! a search result open at the right line, and it is what milestone 5 will
//! cite when the AI answers from your notes.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Split a section longer than this at a paragraph boundary.
const MAX_PASSAGE_CHARS: usize = 1200;
/// A section shorter than this (a bare heading, say) merges into the next.
const MIN_PASSAGE_CHARS: usize = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passage {
    pub ordinal: usize,
    /// Breadcrumb of enclosing headings, e.g. "AI AutoQA > Root cause".
    pub heading_path: String,
    /// 1-based, into the ORIGINAL Markdown — what the editor scrolls to.
    pub line_start: u32,
    pub line_end: u32,
    pub text: String,
}

/// Plain text for the note as a whole, plus its passages.
#[derive(Debug, Clone, Default)]
pub struct Extracted {
    pub plain: String,
    pub passages: Vec<Passage>,
}

/// One chunk of emitted text and the line it came from.
struct Piece {
    text: String,
    line: u32,
}

/// Build a byte-offset → line-number lookup once, rather than counting
/// newlines for every event.
fn line_index(source: &str) -> Vec<usize> {
    let mut starts = vec![0usize];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn line_at(starts: &[usize], offset: usize) -> u32 {
    match starts.binary_search(&offset) {
        Ok(i) => (i + 1) as u32,
        Err(i) => i as u32, // i is the count of starts <= offset
    }
    .max(1)
}

/// Extract searchable text and passages from Markdown.
///
/// Emits paragraph, list, table, and quote text, and the *contents* of code
/// blocks — searching for a function name has to work. Drops fence markers,
/// link URLs, and image syntax, which are noise in a search index.
pub fn extract(source: &str) -> Extracted {
    let starts = line_index(source);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    // Sections accumulate as (heading path, pieces).
    let mut sections: Vec<(String, Vec<Piece>)> = vec![(String::new(), Vec::new())];
    let mut heading_stack: Vec<(HeadingLevel, String)> = Vec::new();

    let mut in_heading: Option<HeadingLevel> = None;
    let mut heading_text = String::new();
    let mut heading_line = 1u32;
    let mut in_image = false;

    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        let line = line_at(&starts, range.start);

        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = Some(level);
                heading_text.clear();
                heading_line = line;
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(level) = in_heading.take() {
                    // Pop headings at or below this level, then push this one.
                    while heading_stack.last().is_some_and(|(prev, _)| *prev >= level) {
                        heading_stack.pop();
                    }
                    heading_stack.push((level, heading_text.trim().to_string()));

                    let path = heading_stack
                        .iter()
                        .map(|(_, t)| t.as_str())
                        .filter(|t| !t.is_empty())
                        .collect::<Vec<_>>()
                        .join(" > ");

                    // The heading itself is searchable text of its section.
                    sections.push((
                        path,
                        vec![Piece {
                            text: heading_text.trim().to_string(),
                            line: heading_line,
                        }],
                    ));
                }
            }

            // Image alt text is useful; the URL is not.
            Event::Start(Tag::Image { .. }) => in_image = true,
            Event::End(TagEnd::Image) => in_image = false,

            Event::Text(text) | Event::Code(text) => {
                if in_heading.is_some() {
                    heading_text.push_str(&text);
                } else if let Some((_, pieces)) = sections.last_mut() {
                    pieces.push(Piece {
                        text: text.to_string(),
                        line,
                    });
                }
                let _ = in_image;
            }

            // Fenced code: keep the contents, drop the fence and language tag.
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_) | CodeBlockKind::Indented)) => {}
            Event::End(TagEnd::CodeBlock) => {}

            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, pieces)) = sections.last_mut() {
                    pieces.push(Piece {
                        text: "\n".to_string(),
                        line,
                    });
                }
            }

            Event::End(TagEnd::Paragraph | TagEnd::Item | TagEnd::TableCell) => {
                if let Some((_, pieces)) = sections.last_mut() {
                    pieces.push(Piece {
                        text: "\n".to_string(),
                        line,
                    });
                }
            }

            _ => {}
        }
    }

    let passages = build_passages(sections);
    let plain = passages
        .iter()
        .map(|p| p.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    Extracted { plain, passages }
}

fn build_passages(sections: Vec<(String, Vec<Piece>)>) -> Vec<Passage> {
    let mut out: Vec<Passage> = Vec::new();

    for (path, pieces) in sections {
        if pieces.is_empty() {
            continue;
        }

        let text = pieces
            .iter()
            .map(|p| p.text.as_str())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        if text.trim().is_empty() {
            continue;
        }

        let first_line = pieces.iter().map(|p| p.line).min().unwrap_or(1);
        let last_line = pieces.iter().map(|p| p.line).max().unwrap_or(first_line);

        // A heading with almost nothing under it is not worth its own row;
        // fold it into the previous passage so the index is not full of stubs.
        if text.chars().count() < MIN_PASSAGE_CHARS {
            if let Some(previous) = out.last_mut() {
                previous.text.push(' ');
                previous.text.push_str(&text);
                previous.line_end = previous.line_end.max(last_line);
                continue;
            }
        }

        for (text, start, end) in split_long(&text, first_line, last_line) {
            out.push(Passage {
                ordinal: out.len(),
                heading_path: path.clone(),
                line_start: start,
                line_end: end,
                text,
            });
        }
    }

    for (i, passage) in out.iter_mut().enumerate() {
        passage.ordinal = i;
    }
    out
}

/// Break an over-long section at sentence boundaries.
///
/// Line numbers become approximate once a section is split — the whole section
/// range is attributed to each part rather than guessed at, because landing the
/// reader slightly high is better than landing them in the wrong place.
fn split_long(text: &str, start: u32, end: u32) -> Vec<(String, u32, u32)> {
    if text.chars().count() <= MAX_PASSAGE_CHARS {
        return vec![(text.to_string(), start, end)];
    }

    let mut parts = Vec::new();
    let mut current = String::new();

    for sentence in text.split_inclusive(['.', '!', '?']) {
        if current.chars().count() + sentence.chars().count() > MAX_PASSAGE_CHARS
            && !current.is_empty()
        {
            parts.push((current.trim().to_string(), start, end));
            current = String::new();
        }
        current.push_str(sentence);
    }
    if !current.trim().is_empty() {
        parts.push((current.trim().to_string(), start, end));
    }

    if parts.is_empty() {
        parts.push((text.to_string(), start, end));
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_drops_markup_but_keeps_words() {
        let source =
            "# Title\n\nSome **bold** and *italic* text with a [link](http://example.com).\n";
        let out = extract(source);

        assert!(out.plain.contains("bold"));
        assert!(out.plain.contains("italic"));
        assert!(out.plain.contains("link"), "link text is searchable");
        assert!(
            !out.plain.contains("example.com"),
            "link URLs are noise in an index: {}",
            out.plain
        );
        assert!(!out.plain.contains("**"));
    }

    #[test]
    fn code_block_contents_stay_searchable() {
        let source = "# Code\n\n```rust\nfn spawn_blocking() { todo!() }\n```\n";
        let out = extract(source);

        assert!(
            out.plain.contains("spawn_blocking"),
            "searching for a function name must work: {}",
            out.plain
        );
        assert!(!out.plain.contains("```"));
        assert!(
            !out.plain.contains("rust\n"),
            "the language tag is not text"
        );
    }

    #[test]
    fn headings_build_a_breadcrumb_path() {
        let source = "\
# AI AutoQA

Intro paragraph that is long enough to stand on its own as a passage here.

## Root cause

The orchestrator can stop the node pool while transcript work still remains.

### Detail

A deeper section with enough words in it to survive the minimum length rule.
";
        let out = extract(source);
        let paths: Vec<_> = out
            .passages
            .iter()
            .map(|p| p.heading_path.as_str())
            .collect();

        assert!(paths.contains(&"AI AutoQA"), "{paths:?}");
        assert!(paths.contains(&"AI AutoQA > Root cause"), "{paths:?}");
        assert!(
            paths
                .iter()
                .any(|p| *p == "AI AutoQA > Root cause > Detail"),
            "{paths:?}"
        );
    }

    #[test]
    fn a_sibling_heading_pops_the_stack() {
        let source = "\
## First

Enough words here to make this section a passage of its very own, truly.

## Second

Also enough words here to make this section a passage of its very own.
";
        let out = extract(source);
        let paths: Vec<_> = out
            .passages
            .iter()
            .map(|p| p.heading_path.as_str())
            .collect();

        assert!(paths.contains(&"First"));
        assert!(paths.contains(&"Second"));
        assert!(
            !paths.iter().any(|p| p.contains("First > Second")),
            "a sibling must not nest under its predecessor: {paths:?}"
        );
    }

    #[test]
    fn line_numbers_point_at_the_original_markdown() {
        let source = "# One\n\nfirst section text that is long enough to be kept as a passage.\n\n# Two\n\nsecond section text that is also long enough to be kept here.\n";
        let out = extract(source);

        let two = out
            .passages
            .iter()
            .find(|p| p.heading_path == "Two")
            .expect("second heading");

        // "# Two" is on line 5 of the source.
        assert_eq!(two.line_start, 5, "got {:?}", two);
    }

    #[test]
    fn a_note_without_headings_still_produces_a_passage() {
        let source = "Just some text with no headings at all, but enough of it to count.\n";
        let out = extract(source);

        assert_eq!(out.passages.len(), 1);
        assert_eq!(out.passages[0].heading_path, "");
        assert!(out.passages[0].text.contains("no headings"));
    }

    #[test]
    fn a_note_of_only_headings_does_not_explode() {
        let source = "# A\n## B\n### C\n";
        let out = extract(source);
        // Stubs merge rather than becoming three near-empty rows.
        assert!(out.passages.len() <= 1, "{:?}", out.passages);
    }

    #[test]
    fn long_sections_are_split_at_sentence_boundaries() {
        let sentence = "This is a sentence about GPU orchestration and node pools. ";
        let source = format!("# Long\n\n{}\n", sentence.repeat(60));
        let out = extract(&source);

        assert!(out.passages.len() > 1, "a long section must be split");
        for passage in &out.passages {
            assert!(
                passage.text.chars().count() <= MAX_PASSAGE_CHARS + 120,
                "passage of {} chars is too long",
                passage.text.chars().count()
            );
        }
    }

    #[test]
    fn tables_and_lists_contribute_their_text() {
        let source = "\
# Data

| Node | Status |
|------|--------|
| gpu-01 | draining |

- first bullet item
- second bullet item
";
        let out = extract(source);
        for word in ["gpu-01", "draining", "first bullet", "second bullet"] {
            assert!(out.plain.contains(word), "missing {word}: {}", out.plain);
        }
        assert!(!out.plain.contains("|---"), "table rules are not text");
    }

    #[test]
    fn handles_crlf_and_unicode() {
        let source = "# Café\r\n\r\nSome text with 日本語 characters and an em dash — here.\r\n";
        let out = extract(source);
        assert!(out.plain.contains("日本語"));
        assert!(out.plain.contains("Café"));
    }

    #[test]
    fn empty_input_is_not_a_panic() {
        assert!(extract("").passages.is_empty());
        assert!(extract("\n\n\n").passages.is_empty());
    }
}
