//! What can go wrong talking to an AI service, and what to tell the user.
//!
//! Each variant maps to one specific, actionable message. "Something went
//! wrong" is not an acceptable answer when the fix is usually a typo in a URL.

use crate::error::AppErrorDto;

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("no AI service is configured")]
    Unconfigured,

    #[error("could not connect to {endpoint}")]
    Connect { endpoint: String },

    #[error("could not resolve {host}")]
    Dns { host: String },

    #[error("TLS handshake with {endpoint} failed")]
    Tls { endpoint: String },

    #[error("no response after {seconds} seconds")]
    Timeout { seconds: u64 },

    #[error("the API key was rejected")]
    Unauthorized,

    #[error("access denied for model {model}")]
    Forbidden { model: String },

    #[error("{endpoint} returned 404 for model {model}")]
    NotFound { endpoint: String, model: String },

    #[error("rate limited")]
    RateLimited { retry_after: Option<u64> },

    #[error("the service returned {status}")]
    ServerError { status: u16 },

    #[error("the response could not be understood")]
    BadResponse { detail: String },

    #[error("cancelled")]
    Cancelled,
}

impl AiError {
    /// True when the failure is worth one automatic retry (R6.5).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AiError::Connect { .. } | AiError::ServerError { .. } | AiError::Timeout { .. }
        )
    }

    /// Classify a reqwest failure into something a person can act on.
    pub fn from_reqwest(err: &reqwest::Error, endpoint: &str, timeout_secs: u64) -> Self {
        if err.is_timeout() {
            return AiError::Timeout {
                seconds: timeout_secs,
            };
        }
        if err.is_connect() {
            // reqwest folds DNS and TLS into connect errors, so read the
            // source chain to tell the user which one it actually was.
            let chain = {
                let mut text = err.to_string();
                let mut source: Option<&dyn std::error::Error> = std::error::Error::source(err);
                while let Some(cause) = source {
                    text.push_str(&format!(": {cause}"));
                    source = cause.source();
                }
                text.to_lowercase()
            };

            if chain.contains("dns") || chain.contains("resolve") || chain.contains("name") {
                let host = url_host(endpoint);
                return AiError::Dns { host };
            }
            if chain.contains("tls") || chain.contains("certificate") || chain.contains("handshake")
            {
                return AiError::Tls {
                    endpoint: endpoint.to_string(),
                };
            }
            return AiError::Connect {
                endpoint: endpoint.to_string(),
            };
        }

        AiError::BadResponse {
            detail: err.to_string(),
        }
    }

    /// Classify an HTTP status into the right variant.
    pub fn from_status(status: u16, endpoint: &str, model: &str, retry_after: Option<u64>) -> Self {
        match status {
            401 => AiError::Unauthorized,
            403 => AiError::Forbidden {
                model: model.to_string(),
            },
            404 => AiError::NotFound {
                endpoint: endpoint.to_string(),
                model: model.to_string(),
            },
            429 => AiError::RateLimited { retry_after },
            500..=599 => AiError::ServerError { status },
            other => AiError::ServerError { status: other },
        }
    }
}

fn url_host(endpoint: &str) -> String {
    endpoint
        .split("://")
        .nth(1)
        .unwrap_or(endpoint)
        .split('/')
        .next()
        .unwrap_or(endpoint)
        .to_string()
}

impl From<AiError> for AppErrorDto {
    fn from(err: AiError) -> Self {
        let detail = Some(err.to_string());

        let (code, title, message, hint): (&str, &str, String, Option<String>) = match &err {
            AiError::Unconfigured => (
                "AI_UNCONFIGURED",
                "AI is not configured",
                "Mushroom does not have an AI endpoint yet.".to_string(),
                Some("Open Tools \u{2192} Settings to add one.".to_string()),
            ),

            AiError::Connect { endpoint } => (
                "AI_CONNECT",
                "Could not reach the AI service",
                format!("Mushroom could not connect to {endpoint}."),
                Some("Check that the service is running and the endpoint is correct.".to_string()),
            ),

            AiError::Dns { host } => (
                "AI_DNS",
                "Could not find the AI service",
                format!("The address {host} could not be resolved."),
                Some("Check the endpoint spelling and your network connection.".to_string()),
            ),

            AiError::Tls { endpoint } => (
                "AI_TLS",
                "Secure connection failed",
                format!("Mushroom could not establish a secure connection to {endpoint}."),
                Some(
                    "Check the URL uses the right scheme and the certificate is valid.".to_string(),
                ),
            ),

            AiError::Timeout { seconds } => (
                "AI_TIMEOUT",
                "The AI service did not respond",
                format!("No response after {seconds} seconds."),
                Some(
                    "The model may be slow or overloaded. Try again, or raise the timeout in Settings."
                        .to_string(),
                ),
            ),

            AiError::Unauthorized => (
                "AI_UNAUTHORIZED",
                "The API key was rejected",
                "The AI service refused the key (401).".to_string(),
                Some("Check the key in Tools \u{2192} Settings.".to_string()),
            ),

            AiError::Forbidden { model } => (
                "AI_FORBIDDEN",
                "Access denied",
                format!(
                    "The service accepted the key but refused the request (403)."
                ),
                Some(format!("Check that this key is allowed to use {model}.")),
            ),

            AiError::NotFound { endpoint, model } => {
                // By far the most common real-world misconfiguration, so say
                // so rather than leaving the user to guess.
                let hint = if !endpoint.trim_end_matches('/').ends_with("/v1") {
                    format!(
                        "Check the model name, and note that LiteLLM and OpenAI \
                         usually need the endpoint to end in /v1 \u{2014} try \
                         {}/v1.",
                        endpoint.trim_end_matches('/')
                    )
                } else {
                    format!("Check that the model {model} exists on this service.")
                };
                (
                    "AI_NOT_FOUND",
                    "Model or endpoint not found",
                    format!("{endpoint} returned 404."),
                    Some(hint),
                )
            }

            AiError::RateLimited { retry_after } => (
                "AI_RATE_LIMITED",
                "Rate limited",
                "The AI service is rate limiting requests (429).".to_string(),
                Some(match retry_after {
                    Some(s) => format!("Wait about {s} seconds and try again."),
                    None => "Wait a moment and try again.".to_string(),
                }),
            ),

            AiError::ServerError { status } => (
                "AI_SERVER_ERROR",
                "The AI service had a problem",
                format!("The service returned {status}."),
                Some(
                    "This is a problem at the service, not in your notes. Try again shortly."
                        .to_string(),
                ),
            ),

            AiError::BadResponse { .. } => (
                "AI_BAD_RESPONSE",
                "Unexpected response",
                "Mushroom could not understand the service's reply.".to_string(),
                Some(
                    "See Details, and check the endpoint is OpenAI-compatible.".to_string(),
                ),
            ),

            AiError::Cancelled => (
                "AI_CANCELLED",
                "Stopped",
                "The request was stopped.".to_string(),
                None,
            ),
        };

        AppErrorDto {
            code: code.to_string(),
            title: title.to_string(),
            message,
            detail,
            hint,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto(err: AiError) -> AppErrorDto {
        err.into()
    }

    #[test]
    fn every_variant_says_something_specific() {
        let cases = [
            (AiError::Unconfigured, "AI_UNCONFIGURED"),
            (
                AiError::Connect {
                    endpoint: "http://localhost:4000".into(),
                },
                "AI_CONNECT",
            ),
            (
                AiError::Dns {
                    host: "nope.example".into(),
                },
                "AI_DNS",
            ),
            (
                AiError::Tls {
                    endpoint: "https://x".into(),
                },
                "AI_TLS",
            ),
            (AiError::Timeout { seconds: 120 }, "AI_TIMEOUT"),
            (AiError::Unauthorized, "AI_UNAUTHORIZED"),
            (
                AiError::Forbidden {
                    model: "gpt-4.1-mini".into(),
                },
                "AI_FORBIDDEN",
            ),
            (
                AiError::RateLimited {
                    retry_after: Some(30),
                },
                "AI_RATE_LIMITED",
            ),
            (AiError::ServerError { status: 503 }, "AI_SERVER_ERROR"),
        ];

        for (err, expected_code) in cases {
            let d = dto(err);
            assert_eq!(d.code, expected_code);
            assert!(!d.message.trim().is_empty());
            // No Rust or crate jargon in anything the user reads.
            for jargon in ["reqwest", "Error {", "std::io", "hyper"] {
                assert!(
                    !d.message.contains(jargon),
                    "{expected_code}: {}",
                    d.message
                );
                assert!(!d.title.contains(jargon));
            }
        }
    }

    #[test]
    fn a_404_without_v1_suggests_adding_it() {
        // The single most common misconfiguration with LiteLLM.
        let d = dto(AiError::NotFound {
            endpoint: "http://localhost:4000".into(),
            model: "gpt-4.1-mini".into(),
        });
        let hint = d.hint.unwrap();
        assert!(hint.contains("/v1"), "{hint}");
        assert!(hint.contains("http://localhost:4000/v1"), "{hint}");
    }

    #[test]
    fn a_404_with_v1_already_talks_about_the_model_instead() {
        let d = dto(AiError::NotFound {
            endpoint: "https://api.openai.com/v1".into(),
            model: "no-such-model".into(),
        });
        let hint = d.hint.unwrap();
        assert!(hint.contains("no-such-model"), "{hint}");
    }

    #[test]
    fn only_transient_failures_are_retryable() {
        assert!(AiError::Connect {
            endpoint: "x".into()
        }
        .is_retryable());
        assert!(AiError::ServerError { status: 502 }.is_retryable());
        assert!(AiError::Timeout { seconds: 10 }.is_retryable());

        // Retrying a 4xx just annoys the service and the user.
        assert!(!AiError::Unauthorized.is_retryable());
        assert!(!AiError::Forbidden { model: "m".into() }.is_retryable());
        assert!(!AiError::NotFound {
            endpoint: "e".into(),
            model: "m".into()
        }
        .is_retryable());
        assert!(!AiError::RateLimited { retry_after: None }.is_retryable());
        assert!(!AiError::Cancelled.is_retryable());
    }

    #[test]
    fn status_codes_map_to_the_right_variant() {
        let e = |s| AiError::from_status(s, "http://x/v1", "m", None);
        assert!(matches!(e(401), AiError::Unauthorized));
        assert!(matches!(e(403), AiError::Forbidden { .. }));
        assert!(matches!(e(404), AiError::NotFound { .. }));
        assert!(matches!(e(429), AiError::RateLimited { .. }));
        assert!(matches!(e(500), AiError::ServerError { status: 500 }));
        assert!(matches!(e(503), AiError::ServerError { status: 503 }));
    }

    #[test]
    fn host_is_extracted_for_dns_errors() {
        assert_eq!(url_host("https://api.openai.com/v1"), "api.openai.com");
        assert_eq!(url_host("http://localhost:4000"), "localhost:4000");
    }
}
