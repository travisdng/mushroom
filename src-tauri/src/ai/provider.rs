//! Provider types.
//!
//! LiteLLM and OpenAI both speak the OpenAI chat-completions API. They differ
//! in three things: base URL, whether a key is required, and the default
//! model. That is configuration, not architecture — so there is one client and
//! a preset enum, and anything else OpenAI-compatible (Azure through LiteLLM,
//! Ollama, vLLM) works by typing a URL.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    #[default]
    LiteLlm,
    OpenAi,
}

impl Provider {
    pub fn default_base_url(&self) -> &'static str {
        match self {
            Provider::LiteLlm => "http://localhost:4000",
            Provider::OpenAi => "https://api.openai.com/v1",
        }
    }

    /// A local LiteLLM proxy often needs no key at all.
    pub fn key_required(&self) -> bool {
        matches!(self, Provider::OpenAi)
    }

    pub fn default_model(&self) -> &'static str {
        "gpt-4.1-mini"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    /// Ids of the notes whose content went into these messages.
    ///
    /// Provenance travels with the request so the privacy gate can check it
    /// without every caller growing an extra argument — and so that spec 10's
    /// tool results, which are also note content, have somewhere obvious to
    /// declare themselves. Empty means "no note content", which is true of a
    /// plain conversation and of `Test Connection`.
    pub sources: Vec<String>,
}

impl ChatRequest {
    /// Declare which notes contributed to this request.
    pub fn from_notes(mut self, ids: impl IntoIterator<Item = String>) -> Self {
        self.sources = ids.into_iter().collect();
        self.sources.sort();
        self.sources.dedup();
        self
    }
}

/// Token counts. Serialised camelCase for the UI, but the wire format is
/// snake_case, so both spellings are accepted on the way in.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    #[serde(alias = "prompt_tokens")]
    pub prompt_tokens: u32,
    #[serde(alias = "completion_tokens")]
    pub completion_tokens: u32,
    #[serde(alias = "total_tokens")]
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<Usage>,
    pub latency_ms: u64,
}

/// What `Test Connection` reports back (R5.2).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionInfo {
    pub endpoint: String,
    pub model: String,
    pub latency_ms: u64,
    /// The model name the service itself echoed, which can differ from what
    /// was asked for when a proxy routes elsewhere.
    pub provider_reported_model: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_differ_only_where_they_should() {
        assert_eq!(
            Provider::LiteLlm.default_base_url(),
            "http://localhost:4000"
        );
        assert_eq!(
            Provider::OpenAi.default_base_url(),
            "https://api.openai.com/v1"
        );

        // The key requirement is the real difference: a local proxy usually
        // needs none, OpenAI always does.
        assert!(!Provider::LiteLlm.key_required());
        assert!(Provider::OpenAi.key_required());
    }

    #[test]
    fn provider_round_trips_through_json() {
        for provider in [Provider::LiteLlm, Provider::OpenAi] {
            let json = serde_json::to_string(&provider).unwrap();
            assert_eq!(serde_json::from_str::<Provider>(&json).unwrap(), provider);
        }
    }
}
