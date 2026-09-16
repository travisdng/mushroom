//! Asking a question of your notes.
//!
//! Each request gets its own typed channel and its own cancellation token,
//! keyed by a request id the front end generates. Two panels, or a stale
//! request the user has already replaced, therefore cannot write into each
//! other's answer.

use std::collections::HashMap;
use std::sync::Mutex;

use serde::Serialize;
use tauri::ipc::Channel;
use tokio_util::sync::CancellationToken;

use crate::ai::search::{AiDelta, AiSearchService};
use crate::config::save;
use crate::error::{AppError, AppErrorDto};
use crate::state::AppState;

/// Questions kept for the history drop-down (R8.1).
const HISTORY_LIMIT: usize = 20;

/// In-flight requests, so a later `ai_search_cancel` can find its token.
#[derive(Default)]
pub struct InFlight(Mutex<HashMap<String, CancellationToken>>);

impl InFlight {
    pub fn register(&self, id: &str) -> CancellationToken {
        let token = CancellationToken::new();
        if let Ok(mut map) = self.0.lock() {
            // A repeated id means the caller reused it; cancel the old one
            // rather than leaking it.
            if let Some(previous) = map.insert(id.to_string(), token.clone()) {
                previous.cancel();
            }
        }
        token
    }

    pub fn cancel(&self, id: &str) -> bool {
        match self.0.lock() {
            Ok(mut map) => match map.remove(id) {
                Some(token) => {
                    token.cancel();
                    true
                }
                None => false,
            },
            Err(_) => false,
        }
    }

    pub fn finish(&self, id: &str) {
        if let Ok(mut map) = self.0.lock() {
            map.remove(id);
        }
    }
}

/// One record per request, for Diagnostics (R8.2).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRecord {
    pub at: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub latency_ms: u64,
    pub outcome: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub requests: u32,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
    pub failures: u32,
    pub recent: Vec<UsageRecord>,
}

/// Usage totals for the session, and the last few requests.
#[derive(Default)]
pub struct UsageLog(Mutex<UsageStats>);

impl UsageLog {
    /// Keep only the most recent few: this is a diagnostic aid, not an audit
    /// trail, and it lives in memory.
    const RECENT_LIMIT: usize = 20;

    pub fn record(&self, record: UsageRecord) {
        if let Ok(mut stats) = self.0.lock() {
            stats.requests += 1;
            stats.prompt_tokens += u64::from(record.prompt_tokens);
            stats.completion_tokens += u64::from(record.completion_tokens);
            stats.total_tokens += u64::from(record.total_tokens);
            if record.outcome != "ok" {
                stats.failures += 1;
            }
            stats.recent.insert(0, record);
            stats.recent.truncate(Self::RECENT_LIMIT);
        }
    }

    pub fn snapshot(&self) -> UsageStats {
        self.0.lock().map(|s| s.clone()).unwrap_or_default()
    }
}

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

/// Ask a question of the notes, streaming progress into `channel`.
#[tauri::command]
pub async fn ai_search(
    state: tauri::State<'_, AppState>,
    question: String,
    folder: Option<String>,
    request_id: String,
    channel: Channel<AiDelta>,
) -> Result<(), AppErrorDto> {
    if question.trim().is_empty() {
        return Err(AppError::InvalidSetting {
            message: "Type a question first.".into(),
        }
        .into());
    }

    remember_question(&state, &question);

    let service = AiSearchService::new(state.search.clone(), state.ai.clone());
    let cancel = state.in_flight.register(&request_id);
    let usage = state.ai_usage.clone();
    let in_flight = state.in_flight.clone();
    let id = request_id.clone();

    // Every delta goes to the front end as it happens. A send failure means
    // the window is gone, which is not worth failing the request over.
    let sink = Box::new(move |delta: AiDelta| {
        let _ = channel.send(delta);
    });

    let outcome = service.answer(&question, folder, sink, cancel).await;
    in_flight.finish(&id);

    match outcome {
        Ok(answer) => {
            let used = answer.usage.unwrap_or_default();
            usage.record(UsageRecord {
                at: now(),
                model: answer.model,
                prompt_tokens: used.prompt_tokens,
                completion_tokens: used.completion_tokens,
                total_tokens: used.total_tokens,
                latency_ms: answer.latency_ms,
                outcome: if answer.no_results {
                    "no-results"
                } else {
                    "ok"
                }
                .into(),
            });
            Ok(())
        }
        Err(err) => {
            usage.record(UsageRecord {
                at: now(),
                model: state.ai.config().model,
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                latency_ms: 0,
                outcome: "error".into(),
            });
            // The Failed delta has already gone to the panel; this is what the
            // awaiting caller sees.
            Err((&err).into())
        }
    }
}

/// Stop an in-flight question (R1.5).
#[tauri::command]
pub fn ai_search_cancel(state: tauri::State<'_, AppState>, request_id: String) -> bool {
    let cancelled = state.in_flight.cancel(&request_id);
    if cancelled {
        tracing::info!(target: "ai", "question cancelled by the user");
    }
    cancelled
}

#[tauri::command]
pub fn get_ai_history(state: tauri::State<'_, AppState>) -> Vec<String> {
    state
        .config
        .lock()
        .map(|c| c.ai_history.clone())
        .unwrap_or_default()
}

#[tauri::command]
pub fn clear_ai_history(state: tauri::State<'_, AppState>) -> Result<(), AppErrorDto> {
    let snapshot = {
        let mut config = state
            .config
            .lock()
            .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
        config.ai_history.clear();
        config.clone()
    };
    save(&state.data_dir, &snapshot)?;
    Ok(())
}

#[tauri::command]
pub fn get_ai_usage_stats(state: tauri::State<'_, AppState>) -> UsageStats {
    state.ai_usage.snapshot()
}

/// Add a question to the history, most recent first, without duplicates.
fn remember_question(state: &AppState, question: &str) {
    let question = question.trim().to_string();

    let snapshot = {
        let Ok(mut config) = state.config.lock() else {
            return;
        };
        config.ai_history.retain(|q| q != &question);
        config.ai_history.insert(0, question);
        config.ai_history.truncate(HISTORY_LIMIT);
        config.clone()
    };

    // Best effort: failing to persist the history must not fail the question.
    if let Err(err) = save(&state.data_dir, &snapshot) {
        tracing::warn!(target: "app", error = %err, "question history could not be saved");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_an_unknown_request_is_not_an_error() {
        let in_flight = InFlight::default();
        assert!(!in_flight.cancel("never-registered"));
    }

    #[test]
    fn a_registered_request_can_be_cancelled_once() {
        let in_flight = InFlight::default();
        let token = in_flight.register("r1");
        assert!(!token.is_cancelled());

        assert!(in_flight.cancel("r1"));
        assert!(token.is_cancelled());

        // Already removed, so a second cancel finds nothing.
        assert!(!in_flight.cancel("r1"));
    }

    #[test]
    fn reusing_a_request_id_cancels_the_one_it_replaces() {
        // Otherwise a superseded request keeps streaming into the panel.
        let in_flight = InFlight::default();
        let first = in_flight.register("same");
        let second = in_flight.register("same");

        assert!(first.is_cancelled(), "the replaced request must be stopped");
        assert!(!second.is_cancelled());
    }

    #[test]
    fn finishing_a_request_stops_it_being_cancellable() {
        let in_flight = InFlight::default();
        in_flight.register("r1");
        in_flight.finish("r1");
        assert!(!in_flight.cancel("r1"));
    }

    #[test]
    fn usage_totals_accumulate_and_count_failures() {
        let log = UsageLog::default();

        log.record(UsageRecord {
            at: now(),
            model: "m".into(),
            prompt_tokens: 100,
            completion_tokens: 20,
            total_tokens: 120,
            latency_ms: 900,
            outcome: "ok".into(),
        });
        log.record(UsageRecord {
            at: now(),
            model: "m".into(),
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
            latency_ms: 0,
            outcome: "error".into(),
        });

        let stats = log.snapshot();
        assert_eq!(stats.requests, 2);
        assert_eq!(stats.prompt_tokens, 100);
        assert_eq!(stats.total_tokens, 120);
        assert_eq!(stats.failures, 1);
        assert_eq!(stats.recent.len(), 2);
        assert_eq!(stats.recent[0].outcome, "error", "most recent first");
    }

    #[test]
    fn the_recent_list_is_bounded() {
        let log = UsageLog::default();
        for _ in 0..50 {
            log.record(UsageRecord {
                at: now(),
                model: "m".into(),
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
                latency_ms: 1,
                outcome: "ok".into(),
            });
        }
        let stats = log.snapshot();
        assert_eq!(stats.requests, 50, "totals count everything");
        assert_eq!(
            stats.recent.len(),
            UsageLog::RECENT_LIMIT,
            "the list does not"
        );
    }
}
