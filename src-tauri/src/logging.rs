//! Structured logging to a daily-rolling file, plus stdout in debug builds.
//!
//! Categories are used as tracing targets: `app`, `files`, `db`, `search`,
//! `ai`, `net`. Never log API keys, note bodies, or full prompts — see
//! `.kiro/steering/error-handling.md`.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
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

    'outer: while !rest.is_empty() {
        for prefix in KEY_PREFIXES {
            if let Some(at) = rest.find(prefix) {
                out.push_str(&rest[..at + prefix.len()]);
                out.push_str("***");
                let after = &rest[at + prefix.len()..];
                // Skip the secret itself: run to the next separator.
                let end = after
                    .find(|c: char| c.is_whitespace() || c == '"' || c == ',' || c == '}')
                    .unwrap_or(after.len());
                rest = &after[end..];
                continue 'outer;
            }
        }
        out.push_str(rest);
        break;
    }

    out
}

/// Initialise logging. The returned guard must be held for the life of the
/// process — dropping it stops the background writer flushing.
pub fn init(log_dir: &Path) -> WorkerGuard {
    let appender = tracing_appender::rolling::daily(log_dir, "mushroom.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let filter = EnvFilter::try_from_env("MUSHROOM_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true);

    let registry = tracing_subscriber::registry().with(filter).with(file_layer);

    #[cfg(debug_assertions)]
    let registry = registry.with(
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_target(true),
    );

    registry.init();
    guard
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn redacts_openai_style_keys() {
        let out = redact("calling with sk-abc123DEF456ghi789 now");
        assert!(!out.contains("abc123DEF456ghi789"), "{out}");
        assert!(out.contains("sk-***"));
        assert!(
            out.contains("now"),
            "surrounding text should survive: {out}"
        );
    }

    #[test]
    fn redacts_bearer_headers_and_json_fields() {
        assert!(!redact("Authorization: Bearer sk-live-xyz").contains("xyz"));
        assert!(!redact(r#"{"api_key=secret123","model":"x"}"#).contains("secret123"));
    }

    #[test]
    fn leaves_ordinary_text_alone() {
        let text = "indexed 412 notes in 84ms";
        assert_eq!(redact(text), text);
    }
}
