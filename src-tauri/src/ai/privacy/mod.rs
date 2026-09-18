//! The privacy gate: the one place note content crosses to the network.
//!
//! Notes contain passwords, keys, connection strings and private-key blocks,
//! because that is what people write down. Every request Mushroom sends is
//! therefore built here, and [`AiClient`](crate::ai::client::AiClient) accepts
//! nothing else.
//!
//! The mechanism is a sealed type. [`SanitisedRequest`] holds a private field
//! of a private type, so no code outside this module can construct one — not
//! with a struct literal, not with `..Default::default()`, not by any trick
//! that does not involve editing this file. A new AI path that forgets to
//! sanitise is therefore a compile error rather than a leak.
//!
//! That is the entire point of the type, so: **do not add a public
//! constructor, a `From` impl, or a `#[cfg(test)]` escape hatch to make a test
//! easier.** A test that needs a `SanitisedRequest` calls [`sanitise`], which
//! is what the real code does.
//!
//! What this module can and cannot promise is covered in
//! `.kiro/steering/ai-integration.md`, and it matters: exclusion is complete,
//! detection is best-effort, and no screen may claim Mushroom removes secrets.

pub mod rules;

use serde::{Deserialize, Serialize};

pub use crate::exclusion::{ExclusionRule, Exclusions};
pub use rules::{RuleSet, Rules};

use crate::ai::provider::ChatRequest;

/// The seal.
///
/// Private type in a private field: naming it is impossible from outside this
/// module, and a struct cannot be built without naming every field. This is
/// the whole enforcement mechanism.
#[derive(Debug, Clone, Copy)]
struct Seal;

/// How hard the gate tries, chosen by the user (R4.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PrivacyMode {
    /// Replace what is recognised and send. Keeps answers useful: an excerpt
    /// reading "authenticates with [redacted: aws-access-key]" still answers
    /// the question that was asked.
    #[default]
    Redact,
    /// Withhold the whole excerpt, tool result or message containing a match.
    Block,
    /// Send as typed. Requires explicit confirmation, and never applies to
    /// exclusion — that is the user's instruction, not a guess.
    Off,
}

impl PrivacyMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            PrivacyMode::Redact => "redact",
            PrivacyMode::Block => "block",
            PrivacyMode::Off => "off",
        }
    }
}

/// One rule's tally for a request.
///
/// Rule name and count only. The matched text is dropped on the floor and
/// never stored, logged, emitted or returned (R3.5) — the only place a person
/// can see a redaction is the local `Last Request` view, which shows the
/// marker, not the value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Redaction {
    pub rule: String,
    pub count: usize,
}

/// Something withheld entirely in [`PrivacyMode::Block`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Withheld {
    /// What kind of thing was dropped — "excerpt", "message", "tool result".
    pub kind: String,
    /// The rule that caused it, for the notice shown to the user.
    pub rule: String,
}

/// What the gate did to one request, for the UI and the log.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyReport {
    pub redactions: Vec<Redaction>,
    pub withheld: Vec<Withheld>,
    /// Notes dropped because the user excluded them (R2.6).
    pub excluded_notes: usize,
    pub mode: PrivacyMode,
}

impl PrivacyReport {
    fn clean(mode: PrivacyMode) -> Self {
        Self {
            redactions: Vec::new(),
            withheld: Vec::new(),
            excluded_notes: 0,
            mode,
        }
    }

    /// True when the gate changed nothing, so the UI can stay silent.
    pub fn is_clean(&self) -> bool {
        self.redactions.is_empty() && self.withheld.is_empty() && self.excluded_notes == 0
    }

    /// Total redactions across all rules, for the one-line notice.
    pub fn redacted_count(&self) -> usize {
        self.redactions.iter().map(|r| r.count).sum()
    }
}

/// What the gate was asked to enforce.
#[derive(Debug, Clone, Default)]
pub struct Policy {
    pub mode: PrivacyMode,
    pub rules: Rules,
}

impl Policy {
    pub fn new(mode: PrivacyMode) -> Self {
        Self {
            mode,
            rules: Rules::builtin(),
        }
    }

    /// A policy with a specific rule set, for tests and for a future Settings
    /// screen that can disable individual rules (task 18).
    pub fn with_rules(mode: PrivacyMode, rules: Rules) -> Self {
        Self { mode, rules }
    }
}

/// The gate failed, so nothing is sent (R1.5).
#[derive(Debug, thiserror::Error)]
pub enum PrivacyError {
    /// The rule set could not be compiled. Sending unscanned text because the
    /// scanner is broken is the one outcome this design must never produce.
    #[error("the credential rules could not be loaded: {detail}")]
    Rules { detail: String },
}

/// A request that has been through the gate.
///
/// Constructible only by [`sanitise`]. See the module docs before considering
/// changing that.
#[derive(Debug, Clone)]
pub struct SanitisedRequest {
    inner: ChatRequest,
    report: PrivacyReport,
    _seal: Seal,
}

impl SanitisedRequest {
    /// The request as it will go on the wire.
    pub fn request(&self) -> &ChatRequest {
        &self.inner
    }

    pub fn report(&self) -> &PrivacyReport {
        &self.report
    }

    pub fn model(&self) -> &str {
        &self.inner.model
    }
}

/// Take a request to the edge of the network.
///
/// Applies the user's exclusions, then the detection rules, and returns both
/// the request as it will be sent and a report of what changed.
///
/// Today this is a pass-through: the chokepoint is built before the rules, so
/// that the rules have somewhere to land that cannot be bypassed. Behaviour
/// arrives with the rule set; the guarantee that every path comes through here
/// arrives now.
pub fn sanitise(request: ChatRequest, policy: &Policy) -> Result<SanitisedRequest, PrivacyError> {
    // Before anything else, and deliberately even in `PrivacyMode::Off`: a
    // rule set that failed to load means Mushroom does not know what it is
    // about to send. `Off` is consent to skip *scanning*, not consent to send
    // blind because the scanner is broken.
    let _rules = policy.rules.ready()?;

    Ok(SanitisedRequest {
        inner: request,
        report: PrivacyReport::clean(policy.mode),
        _seal: Seal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::provider::Message;

    fn request() -> ChatRequest {
        ChatRequest {
            model: "test-model".into(),
            messages: vec![Message::user("what did I write about the GPU nodes?")],
            temperature: None,
            max_tokens: None,
        }
    }

    #[test]
    fn a_sanitised_request_carries_the_request_through() {
        let sanitised = sanitise(request(), &Policy::default()).unwrap();
        assert_eq!(sanitised.model(), "test-model");
        assert_eq!(sanitised.request().messages.len(), 1);
    }

    #[test]
    fn a_clean_request_reports_nothing_to_tell_the_user() {
        let sanitised = sanitise(request(), &Policy::default()).unwrap();
        assert!(sanitised.report().is_clean());
        assert_eq!(sanitised.report().redacted_count(), 0);
    }

    #[test]
    fn the_report_records_the_mode_that_was_in_force() {
        // The answer's notice has to say which posture produced it, or
        // "nothing was redacted" is ambiguous between "nothing matched" and
        // "the scanner was off".
        for mode in [PrivacyMode::Redact, PrivacyMode::Block, PrivacyMode::Off] {
            let sanitised = sanitise(request(), &Policy::new(mode)).unwrap();
            assert_eq!(sanitised.report().mode, mode);
        }
    }

    #[test]
    fn the_default_mode_is_redact_never_off() {
        // A default that sends unscanned text is the one default that cannot
        // be allowed to arrive by accident (R4.5).
        assert_eq!(PrivacyMode::default(), PrivacyMode::Redact);
        assert_eq!(Policy::default().mode, PrivacyMode::Redact);
    }

    #[test]
    fn modes_round_trip_through_json() {
        for mode in [PrivacyMode::Redact, PrivacyMode::Block, PrivacyMode::Off] {
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(serde_json::from_str::<PrivacyMode>(&json).unwrap(), mode);
        }
    }

    #[test]
    fn a_report_with_findings_is_not_clean() {
        let report = PrivacyReport {
            redactions: vec![
                Redaction {
                    rule: "aws-access-key".into(),
                    count: 2,
                },
                Redaction {
                    rule: "assigned-secret".into(),
                    count: 1,
                },
            ],
            withheld: Vec::new(),
            excluded_notes: 0,
            mode: PrivacyMode::Redact,
        };
        assert!(!report.is_clean());
        assert_eq!(report.redacted_count(), 3);
    }

    #[test]
    fn a_broken_rule_set_stops_the_request_dead() {
        // The point of the gate: when it cannot tell what it is about to
        // send, it does not send it (R1.5, R8.2).
        let policy =
            Policy::with_rules(PrivacyMode::Redact, Rules::broken("the table did not load"));
        let err = sanitise(request(), &policy).unwrap_err();
        assert!(matches!(err, PrivacyError::Rules { .. }));
    }

    #[test]
    fn a_broken_rule_set_stops_the_request_even_with_privacy_off() {
        // `Off` is consent to skip scanning, not consent to send blind
        // because the scanner is broken. Those are different things, and
        // conflating them would make the one mode people pick under pressure
        // the one mode with no floor.
        let policy = Policy::with_rules(PrivacyMode::Off, Rules::broken("the table did not load"));
        assert!(sanitise(request(), &policy).is_err());
    }

    #[test]
    fn the_failure_says_what_went_wrong_without_leaking_the_request() {
        let policy = Policy::with_rules(
            PrivacyMode::Redact,
            Rules::broken("rule aws-access-key is not a valid pattern"),
        );
        let message = sanitise(request(), &policy).unwrap_err().to_string();
        assert!(message.contains("aws-access-key"), "{message}");
        assert!(
            !message.contains("GPU nodes"),
            "the request must not appear in the error: {message}"
        );
    }

    #[test]
    fn excluded_notes_alone_are_worth_telling_the_user_about() {
        // Nothing was redacted, but the answer was built from less than the
        // user might expect, and silence there looks like a bad answer.
        let report = PrivacyReport {
            excluded_notes: 2,
            ..PrivacyReport::clean(PrivacyMode::Redact)
        };
        assert!(!report.is_clean());
    }
}
