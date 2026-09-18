//! The credential rule set.
//!
//! Compiled once and held, never per request (R7.2). The table itself arrives
//! in task 13, vendored from gitleaks; what exists now is the shape it lands
//! in, and the failure state that makes the gate safe when it is wrong.

use regex::RegexSet;

use super::PrivacyError;

/// One credential pattern.
#[derive(Debug, Clone, Copy)]
pub struct Rule {
    /// Appears in the marker the model sees: `[redacted: aws-access-key]`.
    pub name: &'static str,
    pub pattern: &'static str,
}

/// The compiled rule set.
#[derive(Debug)]
pub struct RuleSet {
    names: Vec<&'static str>,
    set: RegexSet,
}

impl RuleSet {
    /// Compile a table, or say which rule broke.
    ///
    /// The name is in the error because "one of 170 patterns is invalid" is
    /// not something anyone can act on.
    pub fn compile(rules: &[Rule]) -> Result<Self, PrivacyError> {
        // Compiled individually first: RegexSet reports a syntax error without
        // saying which pattern produced it, which is the one fact needed.
        for rule in rules {
            regex::Regex::new(rule.pattern).map_err(|err| PrivacyError::Rules {
                detail: format!("rule {} is not a valid pattern: {err}", rule.name),
            })?;
        }

        let set =
            RegexSet::new(rules.iter().map(|r| r.pattern)).map_err(|err| PrivacyError::Rules {
                detail: format!("the rule set could not be compiled: {err}"),
            })?;

        Ok(Self {
            names: rules.iter().map(|r| r.name).collect(),
            set,
        })
    }

    /// The rules Mushroom ships with.
    ///
    /// Empty until task 13 vendors the gitleaks table. An empty set is honest:
    /// the gate runs and finds nothing, rather than pretending to protect.
    pub fn builtin() -> Result<Self, PrivacyError> {
        Self::compile(&[])
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Which rules match anywhere in `text`, by name.
    ///
    /// One pass over the text for the whole set; the caller runs the
    /// individual patterns only for these, to get spans.
    pub fn matching(&self, text: &str) -> Vec<&'static str> {
        self.set
            .matches(text)
            .into_iter()
            .map(|i| self.names[i])
            .collect()
    }
}

/// The rule set as the gate sees it: ready, or broken and why.
///
/// A broken set is carried rather than swallowed, so [`super::sanitise`] can
/// refuse. Sending unscanned note content *because the scanner is broken* is
/// the one outcome this design must never produce (R8.2).
#[derive(Debug, Clone)]
pub enum Rules {
    Ready(std::sync::Arc<RuleSet>),
    Broken(String),
}

impl Rules {
    /// Compile the shipped table, keeping the failure rather than panicking.
    ///
    /// A bad built-in pattern is a bug, but a bug that takes out the whole
    /// application is worse than one that turns off AI and says so.
    pub fn builtin() -> Self {
        match RuleSet::builtin() {
            Ok(set) => Rules::Ready(std::sync::Arc::new(set)),
            Err(err) => {
                tracing::error!(target: "ai", error = %err, "the credential rules could not be compiled");
                Rules::Broken(err.to_string())
            }
        }
    }

    /// A deliberately broken set.
    ///
    /// Safe to expose: it can only make the gate *refuse*. There is no
    /// constructor anywhere that makes the gate more permissive, which is the
    /// property that matters.
    pub fn broken(detail: impl Into<String>) -> Self {
        Rules::Broken(detail.into())
    }

    pub fn ready(&self) -> Result<&RuleSet, PrivacyError> {
        match self {
            Rules::Ready(set) => Ok(set),
            Rules::Broken(detail) => Err(PrivacyError::Rules {
                detail: detail.clone(),
            }),
        }
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self::builtin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &[Rule] = &[
        Rule {
            name: "aws-access-key",
            pattern: r"(AKIA|ASIA)[0-9A-Z]{16}",
        },
        Rule {
            name: "github-token",
            pattern: r"gh[pousr]_[A-Za-z0-9]{36,}",
        },
    ];

    #[test]
    fn a_valid_table_compiles() {
        let set = RuleSet::compile(GOOD).unwrap();
        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());
    }

    #[test]
    fn matching_names_the_rules_that_fired() {
        let set = RuleSet::compile(GOOD).unwrap();
        let hits = set.matching("deploy key AK1AIOSFODNN7EXAMPLE in the runbook");
        assert_eq!(hits, vec!["aws-access-key"]);
        assert!(set.matching("nothing interesting here").is_empty());
    }

    #[test]
    fn a_broken_pattern_names_the_rule_that_broke() {
        // "one of 170 patterns is invalid" is not actionable.
        let err = RuleSet::compile(&[Rule {
            name: "nonsense",
            pattern: "(unclosed",
        }])
        .unwrap_err();
        assert!(err.to_string().contains("nonsense"), "{err}");
    }

    #[test]
    fn the_builtin_set_compiles() {
        // Empty today, but this is the test that catches a bad vendored
        // pattern the moment task 13 lands one.
        let rules = Rules::builtin();
        assert!(rules.ready().is_ok(), "the shipped rules must compile");
    }

    #[test]
    fn a_broken_set_refuses_rather_than_pretending_to_be_empty() {
        // An empty set and a broken set look the same from the outside — both
        // find nothing — which is exactly why they must not behave the same.
        let rules = Rules::broken("the table did not load");
        assert!(rules.ready().is_err());
    }
}
