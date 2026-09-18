//! The most recent request, as it went on the wire.
//!
//! So a person can check rather than trust. `AI → Last Request…` shows exactly
//! what left the machine, markers and all.
//!
//! **In memory, never on disk.** One request, replaced each send, gone at
//! exit. On disk it would be a second copy of note content in a file nobody
//! expects — which is the problem `log_prompts` already has, and not one worth
//! having twice.

use std::sync::Mutex;

use serde::Serialize;

use super::{PrivacyReport, SanitisedRequest};

/// What the last request looked like after the gate.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastRequest {
    /// Role and content per message, exactly as sent.
    pub messages: Vec<SentMessage>,
    pub model: String,
    pub report: PrivacyReport,
    pub at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SentMessage {
    pub role: String,
    pub content: String,
}

/// Holds one request. Replaced on each send.
#[derive(Default)]
pub struct LastRequestLog(Mutex<Option<LastRequest>>);

impl LastRequestLog {
    pub fn record(&self, request: &SanitisedRequest) {
        let snapshot = LastRequest {
            messages: request
                .request()
                .messages
                .iter()
                .map(|m| SentMessage {
                    role: format!("{:?}", m.role).to_lowercase(),
                    content: m.content.clone(),
                })
                .collect(),
            model: request.model().to_string(),
            report: request.report().clone(),
            at: now(),
        };
        if let Ok(mut slot) = self.0.lock() {
            *slot = Some(snapshot);
        }
    }

    pub fn snapshot(&self) -> Option<LastRequest> {
        self.0.lock().ok().and_then(|slot| slot.clone())
    }
}

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::privacy::{sanitise, Policy, PrivacyMode};
    use crate::ai::provider::{ChatRequest, Message};

    fn sanitised(text: &str) -> SanitisedRequest {
        sanitise(
            ChatRequest {
                model: "test-model".into(),
                messages: vec![Message::user(text)],
                temperature: None,
                max_tokens: None,
                sources: Vec::new(),
            },
            &Policy::new(PrivacyMode::Redact),
        )
        .unwrap()
    }

    #[test]
    fn nothing_is_recorded_until_something_is_sent() {
        assert!(LastRequestLog::default().snapshot().is_none());
    }

    #[test]
    fn the_recorded_text_is_what_was_sent_markers_and_all() {
        let log = LastRequestLog::default();
        log.record(&sanitised(
            "the key was AK1AQYRZ5TMK7VW3XJ42 before rotation",
        ));

        let last = log.snapshot().unwrap();
        let sent = &last.messages[0].content;
        assert!(!sent.contains("AK1AQYRZ5TMK7VW3XJ42"), "{sent}");
        assert!(sent.contains("[redacted:"), "{sent}");
        assert!(sent.contains("before rotation"), "{sent}");
        assert_eq!(last.report.redacted_count(), 1);
    }

    #[test]
    fn only_the_most_recent_request_is_kept() {
        // One request, not a history. A log of everything ever sent would be
        // the thing this feature exists to avoid.
        let log = LastRequestLog::default();
        log.record(&sanitised("first question"));
        log.record(&sanitised("second question"));

        let last = log.snapshot().unwrap();
        assert!(last.messages[0].content.contains("second"));
        assert_eq!(last.messages.len(), 1);
    }
}
