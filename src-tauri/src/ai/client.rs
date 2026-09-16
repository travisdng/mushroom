//! The HTTP client for any OpenAI-compatible endpoint.
//!
//! One implementation. LiteLLM and OpenAI differ in base URL, key requirement
//! and default model, all of which are configuration — writing two clients
//! would double the streaming parser, the error mapping and the test surface
//! for nothing, and would make Azure-via-LiteLLM or Ollama a third client.

use std::time::Duration;

use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::ai::error::AiError;
use crate::ai::provider::{ChatRequest, ChatResponse, ConnectionInfo, Message, Role, Usage};
use crate::ai::stream::{Frame, SseParser};
use crate::config::AiConfig;

/// Connecting should be quick even when generation is slow, so this is fixed
/// while the overall timeout is configurable.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// `Test Connection` must answer promptly rather than hanging for two minutes.
const TEST_TIMEOUT: Duration = Duration::from_secs(15);
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// Where streamed deltas go. Boxed so tests can collect into a Vec and the
/// app can push into a Tauri channel.
pub type StreamSink = Box<dyn FnMut(&str) + Send>;

pub struct AiClient {
    http: reqwest::Client,
    config: AiConfig,
    api_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompletionEnvelope {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    choices: Vec<CompletionChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct CompletionChoice {
    #[serde(default)]
    message: Option<CompletionMessage>,
}

#[derive(Debug, Deserialize)]
struct CompletionMessage {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ModelsEnvelope {
    #[serde(default)]
    data: Vec<ModelEntry>,
}

#[derive(Debug, Deserialize)]
struct ModelEntry {
    #[serde(default)]
    id: Option<String>,
}

#[derive(Debug, Serialize)]
struct WireMessage<'a> {
    role: &'a str,
    content: &'a str,
}

fn role_name(role: &Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

impl AiClient {
    pub fn new(config: AiConfig, api_key: Option<String>) -> Result<Self, AiError> {
        let http = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(Duration::from_secs(config.timeout_secs))
            // One pooled client, reused across requests.
            .pool_idle_timeout(Duration::from_secs(90))
            .build()
            .map_err(|e| AiError::BadResponse {
                detail: format!("HTTP client could not be created: {e}"),
            })?;

        Ok(Self {
            http,
            config,
            api_key: api_key.filter(|k| !k.trim().is_empty()),
        })
    }

    pub fn config(&self) -> &AiConfig {
        &self.config
    }

    fn body(&self, request: &ChatRequest, stream: bool) -> serde_json::Value {
        let messages: Vec<WireMessage> = request
            .messages
            .iter()
            .map(|m| WireMessage {
                role: role_name(&m.role),
                content: &m.content,
            })
            .collect();

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "stream": stream,
        });

        if let Some(t) = request.temperature {
            // Rounded on the way out. Widening an f32 to the f64 that JSON
            // uses turns 0.2 into 0.20000000298023224, which is the same
            // number but looks like a bug in anyone's request log.
            body["temperature"] = json!(((t as f64) * 1000.0).round() / 1000.0);
        }
        if let Some(m) = request.max_tokens {
            body["max_tokens"] = json!(m);
        }
        body
    }

    fn post(&self, url: &str, body: &serde_json::Value) -> reqwest::RequestBuilder {
        let mut req = self.http.post(url).json(body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        req
    }

    /// Read `Retry-After`, which a well-behaved service sends with a 429.
    fn retry_after(response: &reqwest::Response) -> Option<u64> {
        response
            .headers()
            .get("retry-after")?
            .to_str()
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    async fn send_once(
        &self,
        url: &str,
        body: &serde_json::Value,
        model: &str,
    ) -> Result<reqwest::Response, AiError> {
        let response = self.post(url, body).send().await.map_err(|e| {
            AiError::from_reqwest(&e, &self.config.base_url, self.config.timeout_secs)
        })?;

        let status = response.status().as_u16();
        if status >= 400 {
            let retry_after = Self::retry_after(&response);
            return Err(AiError::from_status(
                status,
                &self.config.base_url,
                model,
                retry_after,
            ));
        }
        Ok(response)
    }

    /// Send, with at most one retry on a transient failure (R6.5).
    ///
    /// Never retries a 4xx: the request itself is wrong, and repeating it just
    /// annoys the service and delays the error the user needs to see.
    async fn send_with_retry(
        &self,
        url: &str,
        body: &serde_json::Value,
        model: &str,
        cancel: &CancellationToken,
    ) -> Result<reqwest::Response, AiError> {
        match self.send_once(url, body, model).await {
            Ok(response) => Ok(response),
            Err(err) if err.is_retryable() => {
                tracing::debug!(target: "net", error = %err, "retrying once");

                tokio::select! {
                    _ = cancel.cancelled() => return Err(AiError::Cancelled),
                    _ = tokio::time::sleep(RETRY_DELAY) => {}
                }

                self.send_once(url, body, model).await
            }
            Err(err) => Err(err),
        }
    }

    /// A whole answer in one response.
    pub async fn chat(
        &self,
        request: ChatRequest,
        cancel: CancellationToken,
    ) -> Result<ChatResponse, AiError> {
        // Checked before the select, not inside it: `select!` polls its
        // branches in a random order, so an already-cancelled request could
        // otherwise still be put on the wire before the cancel branch wins.
        if cancel.is_cancelled() {
            return Err(AiError::Cancelled);
        }

        let started = std::time::Instant::now();
        let body = self.body(&request, false);
        let url = self.config.chat_url();

        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(AiError::Cancelled),
            result = self.send_with_retry(&url, &body, &request.model, &cancel) => result?,
        };

        let envelope: CompletionEnvelope =
            response.json().await.map_err(|e| AiError::BadResponse {
                detail: format!("the reply was not the expected JSON: {e}"),
            })?;

        let content = envelope
            .choices
            .iter()
            .filter_map(|c| c.message.as_ref().and_then(|m| m.content.clone()))
            .collect::<String>();

        Ok(ChatResponse {
            content,
            model: envelope.model.unwrap_or_else(|| request.model.clone()),
            usage: envelope.usage,
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }

    /// The same answer, delivered as it is written.
    ///
    /// Returns the assembled response too, so a caller gets usage and the full
    /// text without having to re-accumulate the deltas itself.
    pub async fn stream_chat(
        &self,
        request: ChatRequest,
        mut sink: StreamSink,
        cancel: CancellationToken,
    ) -> Result<ChatResponse, AiError> {
        if cancel.is_cancelled() {
            return Err(AiError::Cancelled);
        }

        let started = std::time::Instant::now();
        let body = self.body(&request, true);
        let url = self.config.chat_url();

        let response = tokio::select! {
            _ = cancel.cancelled() => return Err(AiError::Cancelled),
            result = self.send_with_retry(&url, &body, &request.model, &cancel) => result?,
        };

        let mut parser = SseParser::new();
        let mut assembled = String::new();
        let mut stream = response.bytes_stream();
        let mut unreadable = 0usize;

        loop {
            let next = tokio::select! {
                _ = cancel.cancelled() => {
                    // Dropping the stream aborts the HTTP request (R6.6).
                    return Err(AiError::Cancelled);
                }
                chunk = stream.next() => chunk,
            };

            let Some(chunk) = next else { break };

            let bytes = chunk.map_err(|e| {
                AiError::from_reqwest(&e, &self.config.base_url, self.config.timeout_secs)
            })?;

            for frame in parser.push(&bytes) {
                match frame {
                    Frame::Delta(text) => {
                        assembled.push_str(&text);
                        sink(&text);
                    }
                    Frame::Unreadable => unreadable += 1,
                    Frame::Done => {}
                }
            }
        }

        for frame in parser.finish() {
            if let Frame::Delta(text) = frame {
                assembled.push_str(&text);
                sink(&text);
            }
        }

        if unreadable > 0 {
            tracing::warn!(target: "ai", frames = unreadable, "some stream frames were unreadable");
        }

        Ok(ChatResponse {
            content: assembled,
            model: parser.model.unwrap_or(request.model),
            usage: parser.usage,
            latency_ms: started.elapsed().as_millis() as u64,
        })
    }

    /// A minimal real completion, to prove the settings work (R5.1).
    pub async fn test_connection(&self) -> Result<ConnectionInfo, AiError> {
        if !self.config.is_configured() {
            return Err(AiError::Unconfigured);
        }

        let started = std::time::Instant::now();
        let body = json!({
            "model": self.config.model,
            "messages": [{ "role": "user", "content": "ping" }],
            // Keep it tiny: this is a connectivity check, not a conversation.
            "max_tokens": 1,
            "stream": false,
        });

        let response = self
            .post(&self.config.chat_url(), &body)
            .timeout(TEST_TIMEOUT)
            .send()
            .await
            .map_err(|e| {
                AiError::from_reqwest(&e, &self.config.base_url, TEST_TIMEOUT.as_secs())
            })?;

        let status = response.status().as_u16();
        if status >= 400 {
            let retry_after = Self::retry_after(&response);
            return Err(AiError::from_status(
                status,
                &self.config.base_url,
                &self.config.model,
                retry_after,
            ));
        }

        let envelope: CompletionEnvelope =
            response.json().await.map_err(|e| AiError::BadResponse {
                detail: format!("the reply was not the expected JSON: {e}"),
            })?;

        Ok(ConnectionInfo {
            endpoint: self.config.base_url.clone(),
            model: self.config.model.clone(),
            latency_ms: started.elapsed().as_millis() as u64,
            provider_reported_model: envelope.model,
        })
    }

    /// Models the endpoint offers.
    ///
    /// An endpoint that does not implement `/models` is normal, not an error —
    /// the caller falls back to a free-text field (R4.2).
    pub async fn list_models(&self) -> Result<Vec<String>, AiError> {
        let mut req = self
            .http
            .get(self.config.models_url())
            .timeout(TEST_TIMEOUT);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let response = req.send().await.map_err(|e| {
            AiError::from_reqwest(&e, &self.config.base_url, TEST_TIMEOUT.as_secs())
        })?;

        let status = response.status().as_u16();
        if status >= 400 {
            return Err(AiError::from_status(
                status,
                &self.config.base_url,
                &self.config.model,
                None,
            ));
        }

        let envelope: ModelsEnvelope = response.json().await.map_err(|e| AiError::BadResponse {
            detail: format!("the model list was not the expected JSON: {e}"),
        })?;

        let mut models: Vec<String> = envelope.data.into_iter().filter_map(|m| m.id).collect();
        models.sort();
        models.dedup();
        Ok(models)
    }
}

/// Build a request from config plus messages, so callers do not repeat the
/// temperature and model plumbing.
pub fn request_from(
    config: &AiConfig,
    messages: Vec<Message>,
    max_tokens: Option<u32>,
) -> ChatRequest {
    ChatRequest {
        model: config.model.clone(),
        messages,
        temperature: Some(config.temperature),
        max_tokens,
    }
}
