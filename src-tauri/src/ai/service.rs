//! The configured AI client, and the only thing above `ai/` that talks to it.
//!
//! Holds one client built from the current settings plus the stored key, and
//! rebuilds it whenever either changes. Callers never construct an `AiClient`
//! themselves, so the key is read in exactly one place.

use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::ai::client::{AiClient, StreamSink};
use crate::ai::error::AiError;
use crate::ai::provider::{ChatRequest, ChatResponse, ConnectionInfo};
use crate::config::{secrets, AiConfig};

/// The result of the last Test Connection, for the Diagnostics panel (R5.4).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LastConnection {
    pub ok: bool,
    pub endpoint: String,
    pub model: String,
    pub latency_ms: Option<u64>,
    /// Already user-facing text: the same line shown beside the button.
    pub message: String,
    pub at: String,
}

pub struct AiService {
    config: Mutex<AiConfig>,
    /// `None` when the settings cannot produce a usable client.
    client: RwLock<Option<Arc<AiClient>>>,
    last_connection: Mutex<Option<LastConnection>>,
}

impl AiService {
    pub fn new(config: AiConfig) -> Self {
        let service = Self {
            config: Mutex::new(config),
            client: RwLock::new(None),
            last_connection: Mutex::new(None),
        };
        service.rebuild();
        service
    }

    /// Rebuild the client from the current config and the stored key.
    ///
    /// Called on startup and after any change to either. Reading the key here
    /// rather than per request means a key set in Settings takes effect
    /// immediately, and a cleared one stops being sent immediately.
    pub fn rebuild(&self) {
        let config = match self.config.lock() {
            Ok(config) => config.clone(),
            Err(_) => return,
        };

        let built = if config.is_configured() {
            let key = secrets::get(config.provider);
            match AiClient::new(config, key) {
                Ok(client) => Some(Arc::new(client)),
                Err(err) => {
                    tracing::error!(target: "ai", error = %err, "AI client could not be built");
                    None
                }
            }
        } else {
            tracing::info!(target: "ai", "no AI endpoint configured");
            None
        };

        if let Ok(mut slot) = self.client.write() {
            *slot = built;
        }
    }

    pub fn set_config(&self, config: AiConfig) {
        if let Ok(mut current) = self.config.lock() {
            *current = config;
        }
        self.rebuild();
    }

    pub fn config(&self) -> AiConfig {
        self.config
            .lock()
            .map(|c| c.clone())
            .unwrap_or_default()
    }

    fn client(&self) -> Result<Arc<AiClient>, AiError> {
        self.client
            .read()
            .ok()
            .and_then(|slot| slot.clone())
            .ok_or(AiError::Unconfigured)
    }

    pub fn last_connection(&self) -> Option<LastConnection> {
        self.last_connection.lock().ok().and_then(|c| c.clone())
    }

    /// A one-shot completion, with the usage line written for it.
    pub async fn chat(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<ChatResponse, AiError> {
        let client = self.client()?;
        self.log_request(&request);

        let outcome = client.chat(request, cancel).await;
        self.log_outcome(&outcome);
        outcome
    }

    /// A streamed completion. Same logging, same errors.
    pub async fn stream_chat(
        &self,
        request: ChatRequest,
        sink: StreamSink,
        cancel: CancellationToken,
    ) -> Result<ChatResponse, AiError> {
        let client = self.client()?;
        self.log_request(&request);

        let outcome = client.stream_chat(request, sink, cancel).await;
        self.log_outcome(&outcome);
        outcome
    }

    pub async fn test_connection(&self) -> Result<ConnectionInfo, AiError> {
        let client = self.client()?;
        let config = self.config();
        let outcome = client.test_connection().await;

        let record = match &outcome {
            Ok(info) => {
                tracing::info!(
                    target: "ai",
                    host = %host_of(&info.endpoint),
                    model = %info.model,
                    latency_ms = info.latency_ms,
                    "connection test succeeded"
                );
                LastConnection {
                    ok: true,
                    endpoint: info.endpoint.clone(),
                    model: info.model.clone(),
                    latency_ms: Some(info.latency_ms),
                    message: format!(
                        "Connected to {} as {} in {} ms.",
                        host_of(&info.endpoint),
                        info.provider_reported_model.as_deref().unwrap_or(&info.model),
                        info.latency_ms
                    ),
                    at: now(),
                }
            }
            Err(err) => {
                tracing::warn!(
                    target: "ai",
                    host = %host_of(&config.base_url),
                    model = %config.model,
                    error = %err,
                    "connection test failed"
                );
                let dto: crate::error::AppErrorDto = err.into();
                LastConnection {
                    ok: false,
                    endpoint: config.base_url.clone(),
                    model: config.model.clone(),
                    latency_ms: None,
                    message: dto.message,
                    at: now(),
                }
            }
        };

        if let Ok(mut slot) = self.last_connection.lock() {
            *slot = Some(record);
        }

        outcome
    }

    pub async fn list_models(&self) -> Result<Vec<String>, AiError> {
        self.client()?.list_models().await
    }

    /// Log what was asked for — never the prompt itself unless the user has
    /// turned that on, because a prompt is note content (R6.9, R2.2).
    fn log_request(&self, request: &ChatRequest) {
        let config = self.config();
        tracing::info!(
            target: "ai",
            host = %host_of(&config.base_url),
            model = %request.model,
            messages = request.messages.len(),
            "sending request"
        );

        if config.log_prompts {
            for message in &request.messages {
                tracing::debug!(
                    target: "ai",
                    role = ?message.role,
                    content = %message.content,
                    "prompt (log_prompts is on)"
                );
            }
        }
    }

    fn log_outcome(&self, outcome: &Result<ChatResponse, AiError>) {
        let config = self.config();
        match outcome {
            Ok(response) => {
                let usage = response.usage.unwrap_or_default();
                tracing::info!(
                    target: "ai",
                    host = %host_of(&config.base_url),
                    model = %response.model,
                    latency_ms = response.latency_ms,
                    prompt_tokens = usage.prompt_tokens,
                    completion_tokens = usage.completion_tokens,
                    total_tokens = usage.total_tokens,
                    chars = response.content.len(),
                    outcome = "ok",
                    "request complete"
                );

                if config.log_prompts {
                    tracing::debug!(
                        target: "ai",
                        content = %response.content,
                        "completion (log_prompts is on)"
                    );
                }
            }
            Err(err) => {
                tracing::warn!(
                    target: "ai",
                    host = %host_of(&config.base_url),
                    model = %config.model,
                    outcome = "error",
                    error = %err,
                    "request failed"
                );
            }
        }
    }
}

/// Just the host, so a log line says `api.openai.com` and not a full URL that
/// might carry a query string.
fn host_of(endpoint: &str) -> String {
    endpoint
        .split("://")
        .nth(1)
        .unwrap_or(endpoint)
        .split('/')
        .next()
        .unwrap_or(endpoint)
        .to_string()
}

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unconfigured_service_refuses_rather_than_guessing() {
        let config = AiConfig {
            base_url: String::new(),
            ..AiConfig::default()
        };
        let service = AiService::new(config);
        assert!(matches!(service.client(), Err(AiError::Unconfigured)));
        assert!(service.last_connection().is_none());
    }

    #[test]
    fn changing_the_config_rebuilds_the_client() {
        let service = AiService::new(AiConfig {
            base_url: String::new(),
            ..AiConfig::default()
        });
        assert!(service.client().is_err());

        service.set_config(AiConfig::default());
        assert!(service.client().is_ok(), "a valid config should give a client");
        assert_eq!(service.config().base_url, AiConfig::default().base_url);
    }

    #[test]
    fn only_the_host_is_logged_not_the_whole_url() {
        assert_eq!(host_of("https://api.openai.com/v1"), "api.openai.com");
        assert_eq!(host_of("http://localhost:4000/v1"), "localhost:4000");
    }

    #[test]
    fn a_timestamp_is_produced_in_a_sortable_format() {
        let at = now();
        assert!(at.contains('T'), "{at}");
        assert!(at.len() >= 20, "{at}");
    }
}
