//! Server-sent events from a chat completion.
//!
//! Built before anything consumes it, because retrofitting streaming into a
//! request path designed for one-shot responses means rewriting the path.
//!
//! The parser has to survive a network, not a well-formed fixture: frames
//! split mid-JSON across chunks, several frames in one chunk, keep-alive
//! comments, and the occasional unparseable frame.

use serde::Deserialize;

/// What one SSE frame turned out to mean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Frame {
    /// A piece of the answer.
    Delta(String),
    /// The stream finished normally.
    Done,
    /// A frame we could not read. Logged and skipped, never fatal.
    Unreadable,
}

#[derive(Debug, Deserialize)]
struct ChunkEnvelope {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    choices: Vec<ChunkChoice>,
    #[serde(default)]
    usage: Option<super::provider::Usage>,
}

#[derive(Debug, Deserialize)]
struct ChunkChoice {
    #[serde(default)]
    delta: Option<ChunkDelta>,
}

#[derive(Debug, Deserialize, Default)]
struct ChunkDelta {
    #[serde(default)]
    content: Option<String>,
}

/// Accumulates bytes and yields whole SSE frames.
#[derive(Debug, Default)]
pub struct SseParser {
    buffer: String,
    pub usage: Option<super::provider::Usage>,
    /// The model the service says it used, which a proxy can change. Kept as
    /// parser state rather than a frame because chunks carry it alongside
    /// content, and a frame can only be one thing.
    pub model: Option<String>,
}

impl SseParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed a chunk of bytes; get back whatever complete frames it completed.
    ///
    /// Anything partial stays in the buffer for the next chunk — this is the
    /// whole point, and the thing a naive line-splitting parser gets wrong.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Frame> {
        self.buffer.push_str(&String::from_utf8_lossy(bytes));
        let mut frames = Vec::new();

        // SSE separates events with a blank line.
        while let Some(end) = find_separator(&self.buffer) {
            let (event, rest) = self.buffer.split_at(end.0);
            let event = event.to_string();
            self.buffer = rest[end.1..].to_string();

            if let Some(frame) = self.parse_event(&event) {
                frames.push(frame);
            }
        }

        frames
    }

    /// Flush whatever is left when the connection closes.
    ///
    /// A server that ends without a trailing blank line still deserves to have
    /// its last frame read.
    pub fn finish(&mut self) -> Vec<Frame> {
        if self.buffer.trim().is_empty() {
            return Vec::new();
        }
        let event = std::mem::take(&mut self.buffer);
        self.parse_event(&event).into_iter().collect()
    }

    fn parse_event(&mut self, event: &str) -> Option<Frame> {
        let mut data = String::new();

        for line in event.lines() {
            let line = line.trim_end_matches('\r');

            // Comments and keep-alives start with ':' and mean nothing.
            if line.is_empty() || line.starts_with(':') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("data:") {
                // The space after "data:" is optional in the spec.
                if !data.is_empty() {
                    data.push('\n');
                }
                data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
            }
        }

        let data = data.trim();
        if data.is_empty() {
            return None;
        }
        if data == "[DONE]" {
            return Some(Frame::Done);
        }

        match serde_json::from_str::<ChunkEnvelope>(data) {
            Ok(chunk) => {
                if let Some(usage) = chunk.usage {
                    self.usage = Some(usage);
                }
                if let Some(model) = chunk.model {
                    self.model = Some(model);
                }
                let text: String = chunk
                    .choices
                    .iter()
                    .filter_map(|c| c.delta.as_ref().and_then(|d| d.content.clone()))
                    .collect();

                (!text.is_empty()).then_some(Frame::Delta(text))
            }
            Err(err) => {
                // One bad frame must not abort an answer that is otherwise
                // arriving fine.
                tracing::debug!(target: "ai", error = %err, "skipping an unreadable stream frame");
                Some(Frame::Unreadable)
            }
        }
    }
}

/// Find the end of the first event and the length of its separator.
fn find_separator(buffer: &str) -> Option<(usize, usize)> {
    let lf = buffer.find("\n\n").map(|i| (i, 2));
    let crlf = buffer.find("\r\n\r\n").map(|i| (i, 4));
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(frames: &[Frame]) -> String {
        frames
            .iter()
            .filter_map(|f| match f {
                Frame::Delta(t) => Some(t.as_str()),
                _ => None,
            })
            .collect()
    }

    fn chunk(content: &str) -> String {
        format!("data: {{\"choices\":[{{\"delta\":{{\"content\":\"{content}\"}}}}]}}\n\n")
    }

    #[test]
    fn reads_a_simple_stream() {
        let mut p = SseParser::new();
        let mut frames = Vec::new();
        frames.extend(p.push(chunk("Hello").as_bytes()));
        frames.extend(p.push(chunk(" world").as_bytes()));
        frames.extend(p.push(b"data: [DONE]\n\n"));

        assert_eq!(text_of(&frames), "Hello world");
        assert!(frames.contains(&Frame::Done));
    }

    #[test]
    fn survives_a_frame_split_across_chunks() {
        // The case a line-splitting parser gets wrong: the JSON is cut in half
        // by the network.
        let whole = chunk("split");
        let (a, b) = whole.split_at(whole.len() / 2);

        let mut p = SseParser::new();
        let first = p.push(a.as_bytes());
        assert!(first.is_empty(), "half a frame is not a frame");

        let second = p.push(b.as_bytes());
        assert_eq!(text_of(&second), "split");
    }

    #[test]
    fn survives_a_split_at_every_possible_offset() {
        let whole = format!("{}{}{}", chunk("one"), chunk("two"), "data: [DONE]\n\n");

        for cut in 1..whole.len() {
            let mut p = SseParser::new();
            let mut frames = Vec::new();
            frames.extend(p.push(&whole.as_bytes()[..cut]));
            frames.extend(p.push(&whole.as_bytes()[cut..]));
            frames.extend(p.finish());

            assert_eq!(text_of(&frames), "onetwo", "cut at byte {cut} lost content");
            assert!(frames.contains(&Frame::Done), "cut at {cut} lost [DONE]");
        }
    }

    #[test]
    fn handles_several_frames_in_one_chunk() {
        let mut p = SseParser::new();
        let together = format!("{}{}{}", chunk("a"), chunk("b"), chunk("c"));
        let frames = p.push(together.as_bytes());
        assert_eq!(text_of(&frames), "abc");
    }

    #[test]
    fn ignores_comments_and_keep_alives() {
        let mut p = SseParser::new();
        let frames = p.push(b": keep-alive\n\n");
        assert!(frames.is_empty());

        let frames = p.push(format!(": ping\n{}", chunk("real")).as_bytes());
        assert_eq!(text_of(&frames), "real");
    }

    #[test]
    fn accepts_data_with_and_without_a_space() {
        let mut p = SseParser::new();
        let frames = p.push(b"data:{\"choices\":[{\"delta\":{\"content\":\"tight\"}}]}\n\n");
        assert_eq!(text_of(&frames), "tight");
    }

    #[test]
    fn a_bad_frame_is_skipped_not_fatal() {
        let mut p = SseParser::new();
        let mut frames = Vec::new();
        frames.extend(p.push(chunk("before").as_bytes()));
        frames.extend(p.push(b"data: {not json at all\n\n"));
        frames.extend(p.push(chunk("after").as_bytes()));

        assert!(frames.contains(&Frame::Unreadable));
        assert_eq!(
            text_of(&frames),
            "beforeafter",
            "a bad frame must not lose the good ones around it"
        );
    }

    #[test]
    fn a_stream_cut_mid_answer_keeps_what_arrived() {
        let mut p = SseParser::new();
        let mut frames = Vec::new();
        frames.extend(p.push(chunk("partial answer").as_bytes()));
        // Connection drops here: no [DONE], and a half-written frame.
        frames.extend(p.push(b"data: {\"choices\":[{\"delta\":{\"cont"));
        frames.extend(p.finish());

        assert!(text_of(&frames).starts_with("partial answer"));
        assert!(
            !frames.contains(&Frame::Done),
            "an interrupted stream must not look finished"
        );
    }

    #[test]
    fn handles_crlf_separators() {
        let mut p = SseParser::new();
        let frames = p.push(b"data: {\"choices\":[{\"delta\":{\"content\":\"crlf\"}}]}\r\n\r\n");
        assert_eq!(text_of(&frames), "crlf");
    }

    #[test]
    fn picks_up_usage_in_the_snake_case_the_wire_actually_uses() {
        let mut p = SseParser::new();
        p.push(
            b"data: {\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n\n",
        );
        let usage = p.usage.expect("usage should have been read");
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[test]
    fn records_the_model_even_when_the_chunk_also_carries_content() {
        // A chunk is both at once, so treating the model as an alternative to
        // content silently loses whichever one is not returned.
        let mut p = SseParser::new();
        let frames = p.push(
            b"data: {\"model\":\"routed-elsewhere\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n",
        );
        assert_eq!(text_of(&frames), "hi");
        assert_eq!(p.model.as_deref(), Some("routed-elsewhere"));
    }

    #[test]
    fn empty_input_yields_nothing() {
        let mut p = SseParser::new();
        assert!(p.push(b"").is_empty());
        assert!(p.finish().is_empty());
    }
}
