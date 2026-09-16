//! Structured logging to a daily-rolling file, plus stdout in debug builds.
//!
//! Categories are used as tracing targets: `app`, `files`, `db`, `search`,
//! `ai`, `net`. Never log API keys, note bodies, or full prompts — see
//! `.kiro/steering/error-handling.md`.
//!
//! Redaction is applied at the writer, not at the call site, so it cannot be
//! forgotten: every line any part of the app logs goes through it, including
//! lines from dependencies that know nothing about our rules.

use std::io::{self, Write};
use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Anything matching this is redacted before a line is written. A second line
/// of defence — nothing should be passing a key to `tracing` in the first place.
const KEY_PREFIXES: [&str; 3] = ["sk-", "Bearer ", "api_key="];

/// Replace anything that looks like a credential with `***`.
///
/// Deliberately blunt: it would rather mangle a harmless string than let a key
/// reach the disk.
pub fn redact(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    loop {
        // The earliest match across all prefixes, not the first prefix that
        // happens to occur somewhere. Scanning prefix by prefix would copy
        // everything before a later match out verbatim — including an earlier
        // secret that a different prefix would have caught.
        let Some((at, prefix)) = KEY_PREFIXES
            .iter()
            .filter_map(|p| rest.find(p).map(|at| (at, *p)))
            .min_by_key(|(at, _)| *at)
        else {
            out.push_str(rest);
            break;
        };

        out.push_str(&rest[..at + prefix.len()]);
        out.push_str("***");

        // Skip the secret itself: run to the next separator.
        let after = &rest[at + prefix.len()..];
        let end = after
            .find(|c: char| c.is_whitespace() || c == '"' || c == ',' || c == '}')
            .unwrap_or(after.len());
        rest = &after[end..];
    }

    out
}

/// A writer that redacts whole lines on their way to the one underneath.
///
/// Buffers until a newline because a formatted event can arrive in several
/// `write` calls, and redacting half a key does not redact it.
pub struct Scrubbed<W: Write> {
    inner: W,
    buffer: Vec<u8>,
}

impl<W: Write> Scrubbed<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            buffer: Vec::with_capacity(256),
        }
    }

    fn drain_lines(&mut self) -> io::Result<()> {
        while let Some(at) = self.buffer.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.buffer.drain(..=at).collect();
            let text = String::from_utf8_lossy(&line);
            self.inner.write_all(redact(&text).as_bytes())?;
        }
        Ok(())
    }
}

impl<W: Write> Write for Scrubbed<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        self.drain_lines()?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.drain_lines()?;
        if !self.buffer.is_empty() {
            // A final line with no newline still deserves scrubbing.
            let line = std::mem::take(&mut self.buffer);
            let text = String::from_utf8_lossy(&line);
            self.inner.write_all(redact(&text).as_bytes())?;
        }
        self.inner.flush()
    }
}

impl<W: Write> Drop for Scrubbed<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

/// Wraps any `MakeWriter` so everything written through it is scrubbed.
#[derive(Clone)]
pub struct ScrubbingWriter<M>(pub M);

impl<'a, M> MakeWriter<'a> for ScrubbingWriter<M>
where
    M: MakeWriter<'a>,
{
    type Writer = Scrubbed<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        Scrubbed::new(self.0.make_writer())
    }

    fn make_writer_for(&'a self, meta: &tracing::Metadata<'_>) -> Self::Writer {
        Scrubbed::new(self.0.make_writer_for(meta))
    }
}

/// Initialise logging. The returned guard must be held for the life of the
/// process — dropping it stops the background writer flushing.
pub fn init(log_dir: &Path) -> WorkerGuard {
    let appender = tracing_appender::rolling::daily(log_dir, "mushroom.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let filter = EnvFilter::try_from_env("MUSHROOM_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(ScrubbingWriter(writer))
        .with_ansi(false)
        .with_target(true);

    let registry = tracing_subscriber::registry().with(filter).with(file_layer);

    #[cfg(debug_assertions)]
    let registry = registry.with(
        tracing_subscriber::fmt::layer()
            // Scrubbed on the console too: a screenshot of a terminal is the
            // easiest way for a key to end up in a bug report.
            .with_writer(ScrubbingWriter(std::io::stdout))
            .with_target(true),
    );

    registry.init();
    guard
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_openai_style_keys() {
        let out = redact("calling with sk-abc123DEF456ghi789 now");
        assert!(!out.contains("abc123DEF456ghi789"), "{out}");
        assert!(out.contains("sk-***"));
        assert!(out.contains("now"), "surrounding text should survive: {out}");
    }

    #[test]
    fn redacts_bearer_headers_and_json_fields() {
        assert!(!redact("Authorization: Bearer sk-live-xyz").contains("xyz"));
        assert!(!redact(r#"{"api_key=secret123","model":"x"}"#).contains("secret123"));
    }

    #[test]
    fn an_earlier_secret_is_not_leaked_on_the_way_to_a_later_one() {
        // Two different patterns in one line, the later one listed first in
        // KEY_PREFIXES. Scanning by prefix order copied everything before the
        // "sk-" out verbatim, including the api_key= value.
        let out = redact("api_key=firstsecret then sk-secondsecret");
        assert!(!out.contains("firstsecret"), "{out}");
        assert!(!out.contains("secondsecret"), "{out}");
    }

    #[test]
    fn leaves_ordinary_text_alone() {
        let text = "indexed 412 notes in 84ms";
        assert_eq!(redact(text), text);
    }

    #[test]
    fn a_key_split_across_two_writes_is_still_redacted() {
        // The case that makes line buffering necessary rather than nice.
        let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));

        struct Shared(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
        impl Write for Shared {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        {
            let mut w = Scrubbed::new(Shared(sink.clone()));
            w.write_all(b"using sk-liv").unwrap();
            w.write_all(b"e-SPLITSECRET now\n").unwrap();
        }

        let text = String::from_utf8(sink.lock().unwrap().clone()).unwrap();
        assert!(!text.contains("SPLITSECRET"), "{text}");
        assert!(text.contains("now"), "{text}");
    }

    /// The test task 3 actually asks for: log a key, read the file, find nothing.
    #[test]
    fn a_key_logged_by_mistake_never_reaches_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("scrubbed.log");
        let file = std::fs::File::create(&path).unwrap();

        let make = move || Scrubbed::new(file.try_clone().unwrap());
        let subscriber = tracing_subscriber::fmt()
            .with_writer(make)
            .with_ansi(false)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            // Every shape a key has ever leaked in.
            tracing::info!(target: "ai", key = "sk-live-NEVERWRITEME", "sending request");
            tracing::warn!(target: "net", "Authorization: Bearer sk-live-NEVERWRITEME");
            tracing::error!(target: "ai", "request failed: api_key=NEVERWRITEME");
        });

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(!text.is_empty(), "the test wrote nothing, so it proves nothing");
        assert!(
            !text.contains("NEVERWRITEME"),
            "a key reached the log file:\n{text}"
        );
        // The surrounding line must still be useful for debugging.
        assert!(text.contains("sending request"), "{text}");
    }
}
