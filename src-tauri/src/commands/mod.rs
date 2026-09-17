//! Tauri command wrappers.
//!
//! Commands stay thin: validate, call one service function, map the error,
//! return a DTO. No business logic here — see `.kiro/steering/structure.md`.

pub mod ai;
pub mod ai_search;
pub mod app;
pub mod config;
pub mod diagnostics;
pub mod notes;
pub mod search;
