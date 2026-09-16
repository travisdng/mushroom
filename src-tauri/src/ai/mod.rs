//! The AI client.
//!
//! One HTTP client for any OpenAI-compatible endpoint. Nothing above this
//! module knows whether the bytes came from LiteLLM or OpenAI.

pub mod client;
pub mod context;
pub mod error;
pub mod provider;
pub mod question;
pub mod service;
pub mod stream;
pub mod tokens;
