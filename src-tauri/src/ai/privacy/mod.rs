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

pub mod gitleaks_rules;
pub mod redact;
pub mod rules;

use serde::{Deserialize, Serialize};

pub use crate::exclusion::{ExclusionRule, Exclusions};
pub use rules::{RuleSet, Rules};

use crate::ai::provider::{ChatRequest, Message, Role};

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
    /// The same exclusions retrieval was given. Checked again here, on purpose
    /// — see [`sanitise`].
    pub exclusions: Exclusions,
}

impl Policy {
    pub fn new(mode: PrivacyMode) -> Self {
        Self {
            mode,
            rules: Rules::builtin(),
            exclusions: Exclusions::default(),
        }
    }

    /// A policy with a specific rule set, for tests and for a future Settings
    /// screen that can disable individual rules (task 18).
    pub fn with_rules(mode: PrivacyMode, rules: Rules) -> Self {
        Self {
            mode,
            rules,
            exclusions: Exclusions::default(),
        }
    }

    pub fn with_exclusions(mut self, exclusions: Exclusions) -> Self {
        self.exclusions = exclusions;
        self
    }
}

/// The gate failed, so nothing is sent (R1.5).
#[derive(Debug, thiserror::Error)]
pub enum PrivacyError {
    /// The rule set could not be compiled. Sending unscanned text because the
    /// scanner is broken is the one outcome this design must never produce.
    #[error("the credential rules could not be loaded: {detail}")]
    Rules { detail: String },

    /// An excluded note reached the gate, which means something upstream is
    /// wrong. Refusing is the point: see [`sanitise`].
    #[error("{note_id} is excluded from AI but reached the request")]
    ExcludedNote { note_id: String },
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
    let rules = policy.rules.ready()?;

    // Retrieval already filtered these out, so reaching here means a bug
    // upstream — a new retrieval path that forgot, most likely. Refuse the
    // whole request rather than trying to edit the excluded text back out of
    // an assembled prompt: this is a check that should never fire, and a check
    // that should never fire must be loud when it does. Quietly patching it up
    // would hide the bug and leave the next path to leak for real.
    //
    // Applies in every mode, `Off` included. `Off` turns off the guessing
    // layer; it is not consent to send a note the user named.
    for note_id in &request.sources {
        if policy.exclusions.excludes(note_id) {
            tracing::error!(
                target: "ai",
                note = %note_id,
                "an excluded note reached the privacy gate; refusing the request"
            );
            return Err(PrivacyError::ExcludedNote {
                note_id: note_id.clone(),
            });
        }
    }

    // `Off` is consent to skip the guessing layer. Exclusion above still
    // applied, and still applies, because that is the user's own instruction
    // rather than something Mushroom inferred.
    if policy.mode == PrivacyMode::Off {
        return Ok(SanitisedRequest {
            inner: request,
            report: PrivacyReport::clean(policy.mode),
            _seal: Seal,
        });
    }

    let mut report = PrivacyReport::clean(policy.mode);
    let mut messages = Vec::with_capacity(request.messages.len());

    for message in request.messages {
        match policy.mode {
            PrivacyMode::Off => unreachable!("handled above"),
            PrivacyMode::Redact => {
                let done = redact::redact(&message.content, rules)?;
                for found in done.redactions {
                    match report.redactions.iter_mut().find(|r| r.rule == found.rule) {
                        Some(existing) => existing.count += found.count,
                        None => report.redactions.push(found),
                    }
                }
                messages.push(Message {
                    role: message.role,
                    content: done.text,
                });
            }
            PrivacyMode::Block => {
                // Withhold the whole message rather than editing it. The user
                // chose to lose the context rather than trust the marker.
                match redact::contains_secret(&message.content, rules)? {
                    Some(rule) => report.withheld.push(Withheld {
                        kind: describe(&message.role),
                        rule,
                    }),
                    None => messages.push(message),
                }
            }
        }
    }

    report.redactions.sort_by(|a, b| a.rule.cmp(&b.rule));

    Ok(SanitisedRequest {
        inner: ChatRequest {
            messages,
            ..request
        },
        report,
        _seal: Seal,
    })
}

/// What kind of thing was withheld, for the notice the user reads.
fn describe(role: &Role) -> String {
    match role {
        // The excerpts live in the system prompt, and "system prompt" is not
        // a phrase anybody outside this codebase should have to read.
        Role::System => "note excerpt".to_string(),
        Role::User => "your message".to_string(),
        Role::Assistant => "earlier reply".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::provider::Message;
    use crate::exclusion::ExclusionRule;

    fn request() -> ChatRequest {
        ChatRequest {
            model: "test-model".into(),
            messages: vec![Message::user("what did I write about the GPU nodes?")],
            temperature: None,
            max_tokens: None,
            sources: Vec::new(),
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
    fn an_excluded_note_reaching_the_gate_stops_the_request() {
        // Retrieval already filters these out, so this can only fire on a
        // bug — a new AI path that forgot. Refusing loudly is the point: the
        // alternative is editing the note back out of an assembled prompt and
        // hiding the bug until the next path leaks for real.
        let rules = [ExclusionRule::parse("personal/**")];
        let policy = Policy::new(PrivacyMode::Redact).with_exclusions(Exclusions::new(&rules));

        let leaked = request().from_notes(["personal/vault.md".to_string()]);
        let err = sanitise(leaked, &policy).unwrap_err();

        match err {
            PrivacyError::ExcludedNote { note_id } => {
                assert_eq!(note_id, "personal/vault.md")
            }
            other => panic!("expected an exclusion refusal, got {other:?}"),
        }
    }

    #[test]
    fn the_gate_refuses_an_excluded_note_even_with_privacy_off() {
        // `Off` turns off the guessing layer. It is not consent to send a note
        // the user named.
        let rules = [ExclusionRule::parse("personal/**")];
        let policy = Policy::new(PrivacyMode::Off).with_exclusions(Exclusions::new(&rules));
        let leaked = request().from_notes(["personal/vault.md".to_string()]);
        assert!(sanitise(leaked, &policy).is_err());
    }

    #[test]
    fn an_ordinary_note_passes_the_gate() {
        let rules = [ExclusionRule::parse("personal/**")];
        let policy = Policy::new(PrivacyMode::Redact).with_exclusions(Exclusions::new(&rules));
        let fine = request().from_notes(["work/gpu-incident.md".to_string()]);
        assert!(sanitise(fine, &policy).is_ok());
    }

    #[test]
    fn declaring_sources_de_duplicates_them() {
        // Several excerpts commonly come from one note.
        let req = request().from_notes([
            "work/a.md".to_string(),
            "work/a.md".to_string(),
            "work/b.md".to_string(),
        ]);
        assert_eq!(req.sources, vec!["work/a.md", "work/b.md"]);
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

#[cfg(test)]
mod value_escape_tests {
    use super::*;
    use crate::ai::provider::Message;

    /// R3.5 — the matched value never leaves this module.
    ///
    /// The thing being protected must not turn up in a report, an event, a log
    /// line or an error on the way to protecting it. The report carries a rule
    /// name and a count and nothing else, by construction; this is the test
    /// that says so out loud, and it checks the serialised form because that
    /// is what actually crosses to the window.
    #[test]
    fn the_matched_value_never_reaches_the_report() {
        const SECRET: &str = "AK1AQYRZ5TMK7VW3XJ42";

        let request = ChatRequest {
            model: "test-model".into(),
            messages: vec![Message::user(format!(
                "the runner used {SECRET} until it was rotated"
            ))],
            temperature: None,
            max_tokens: None,
            sources: Vec::new(),
        };

        let sanitised = sanitise(request, &Policy::new(PrivacyMode::Redact)).unwrap();

        let report = serde_json::to_string(sanitised.report()).unwrap();
        assert!(
            !report.contains(SECRET),
            "the value reached the report: {report}"
        );
        // Gitleaks calls it `aws-access-token`; the marker uses the rule's
        // own name so a reader can look it up.
        assert!(
            report.contains("aws-access-token"),
            "the rule name should reach the report: {report}"
        );

        // And the request itself carries the marker rather than the value.
        let sent = &sanitised.request().messages[0].content;
        assert!(!sent.contains(SECRET), "{sent}");
        assert!(sent.contains("[redacted: aws-access-token]"), "{sent}");
        assert!(
            sent.contains("until it was rotated"),
            "prose survives: {sent}"
        );
    }

    #[test]
    fn a_privacy_error_never_carries_the_request() {
        let policy = Policy::with_rules(PrivacyMode::Redact, Rules::broken("table did not load"));
        let request = ChatRequest {
            model: "m".into(),
            messages: vec![Message::user("the password is AK1AQYRZ5TMK7VW3XJ42")],
            temperature: None,
            max_tokens: None,
            sources: Vec::new(),
        };
        let message = sanitise(request, &policy).unwrap_err().to_string();
        assert!(!message.contains("AK1AQYRZ5TMK7VW3XJ42"), "{message}");
    }
}

#[cfg(test)]
mod mode_tests {
    use super::*;
    use crate::ai::provider::Message;

    const SECRET: &str = "AK1AQYRZ5TMK7VW3XJ42";

    fn request_with_secret() -> ChatRequest {
        ChatRequest {
            model: "test-model".into(),
            messages: vec![
                Message::system(format!("=== EXCERPT 1 ===\nthe runner used {SECRET}")),
                Message::user("which key did the runner use?"),
            ],
            temperature: None,
            max_tokens: None,
            sources: Vec::new(),
        }
    }

    #[test]
    fn redact_replaces_and_keeps_the_message() {
        let out = sanitise(request_with_secret(), &Policy::new(PrivacyMode::Redact)).unwrap();
        assert_eq!(out.request().messages.len(), 2, "nothing withheld");
        assert!(!out.request().messages[0].content.contains(SECRET));
        assert!(out.request().messages[0].content.contains("[redacted:"));
        assert_eq!(out.report().redacted_count(), 1);
        assert!(out.report().withheld.is_empty());
    }

    #[test]
    fn block_withholds_the_whole_message_and_names_what_it_dropped() {
        // The user chose to lose the context rather than trust the marker.
        let out = sanitise(request_with_secret(), &Policy::new(PrivacyMode::Block)).unwrap();

        assert_eq!(out.request().messages.len(), 1, "the excerpt is gone");
        assert!(!out.request().messages[0].content.contains(SECRET));
        assert_eq!(out.report().withheld.len(), 1);
        assert_eq!(out.report().withheld[0].kind, "note excerpt");
        assert_eq!(out.report().withheld[0].rule, "aws-access-token");
        // Nothing was edited, so nothing is reported as redacted.
        assert!(out.report().redactions.is_empty());
    }

    #[test]
    fn block_leaves_clean_messages_alone() {
        let clean = ChatRequest {
            model: "m".into(),
            messages: vec![Message::user("what happened to the node pool?")],
            temperature: None,
            max_tokens: None,
            sources: Vec::new(),
        };
        let out = sanitise(clean, &Policy::new(PrivacyMode::Block)).unwrap();
        assert_eq!(out.request().messages.len(), 1);
        assert!(out.report().is_clean());
    }

    #[test]
    fn off_sends_what_it_was_given() {
        // Consent to skip the guessing layer. Recorded in the report so the
        // notice can say which posture produced the answer.
        let out = sanitise(request_with_secret(), &Policy::new(PrivacyMode::Off)).unwrap();
        assert!(out.request().messages[0].content.contains(SECRET));
        assert_eq!(out.report().mode, PrivacyMode::Off);
        assert!(out.report().is_clean());
    }

    #[test]
    fn a_disabled_rule_stops_firing() {
        let policy = Policy::with_rules(
            PrivacyMode::Redact,
            Rules::builtin_without(&["aws-access-token".to_string()]),
        );
        let out = sanitise(request_with_secret(), &policy).unwrap();
        assert!(
            out.request().messages[0].content.contains(SECRET),
            "the rule was switched off, so it should not fire"
        );
        assert!(out.report().is_clean());
    }

    #[test]
    fn disabling_a_rule_that_does_not_exist_is_ignored() {
        // The vendored table changes between versions. A rule that has since
        // been renamed must not stop the application starting.
        let rules = Rules::builtin_without(&["no-such-rule-was-ever-shipped".to_string()]);
        let set = rules.ready().unwrap();
        assert!(set.len() > 150);
    }
}
