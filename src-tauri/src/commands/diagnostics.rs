//! What Mushroom can tell you about itself.

use std::path::PathBuf;

use crate::diagnostics::{self, logs, AiInfo, AppInfo, Diagnostics, IndexInfo, LogLine};
use crate::error::{AppError, AppErrorDto};
use crate::state::AppState;

fn log_dir(state: &AppState) -> PathBuf {
    state.data_dir.join("logs")
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

#[tauri::command]
pub fn get_diagnostics(state: tauri::State<'_, AppState>) -> Diagnostics {
    let (notes_root, config_version, ai_config) = match state.config.lock() {
        Ok(config) => (config.notes_root.clone(), config.version, config.ai.clone()),
        Err(_) => (None, 0, crate::config::AiConfig::default()),
    };

    let stats = state.search.stats();
    let usage = state.ai_usage.snapshot();

    Diagnostics {
        app: AppInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            platform: std::env::consts::OS.to_string(),
            data_dir: state.data_dir.to_string_lossy().to_string(),
            log_dir: log_dir(&state).to_string_lossy().to_string(),
            notes_root: notes_root.map(|p| p.to_string_lossy().to_string()),
            notes_count: state.notes.count(),
            notes_skipped: state.notes.skipped(),
            watching: state.is_watching(),
            config_version,
        },
        index: IndexInfo {
            database_path: state
                .search
                .database_path()
                .map(|p| p.to_string_lossy().to_string()),
            note_count: stats.note_count,
            passage_count: stats.passage_count,
            stale: stats.stale,
            indexing: stats.indexing,
            skipped: stats.skipped,
            last_full_rebuild: stats.last_full_rebuild,
        },
        ai: AiInfo {
            provider: format!("{:?}", ai_config.provider),
            endpoint: ai_config.base_url.clone(),
            model: ai_config.model.clone(),
            key_status: format!("{:?}", crate::config::secrets::status(ai_config.provider)),
            configured: ai_config.configured,
            streaming: ai_config.stream,
            last_connection: state.ai.last_connection().map(|c| c.message),
            requests: usage.requests,
            total_tokens: usage.total_tokens,
            failures: usage.failures,
            privacy_mode: ai_config.privacy_mode.as_str().to_string(),
            exclusion_rules: ai_config.ai_exclusions.len(),
            disabled_rules: ai_config.ai_disabled_rules.clone(),
        },
    }
}

/// The report, already safe to paste anywhere (R3.4).
#[tauri::command]
pub fn copy_diagnostics(state: tauri::State<'_, AppState>) -> String {
    let report = diagnostics::render(&get_diagnostics(state.clone()));

    let notes_root = state
        .config
        .lock()
        .ok()
        .and_then(|config| config.notes_root.clone());

    diagnostics::redact::redact_report(&report, notes_root.as_deref(), home().as_deref())
}

/// The tail of the current log, newest last.
#[tauri::command]
pub fn get_log_tail(
    state: tauri::State<'_, AppState>,
    category: Option<String>,
    level: Option<String>,
) -> Vec<LogLine> {
    let Some(path) = logs::current_log(&log_dir(&state)) else {
        return Vec::new();
    };

    // Filter before truncating, so asking for `ai` lines gives 200 of them
    // rather than whatever happens to be in the last 200 of everything.
    let all = logs::tail(&path, usize::MAX);

    let wanted: Vec<LogLine> = all
        .into_iter()
        .filter(|line| {
            category
                .as_deref()
                .is_none_or(|c| c.is_empty() || line.category == c)
        })
        .filter(|line| {
            level
                .as_deref()
                .is_none_or(|l| l.is_empty() || line.level == l)
        })
        .collect();

    let start = wanted.len().saturating_sub(logs::TAIL_LINES);
    wanted[start..].to_vec()
}

#[tauri::command]
pub fn open_log_folder(state: tauri::State<'_, AppState>) -> Result<(), AppErrorDto> {
    let dir = log_dir(&state);
    std::fs::create_dir_all(&dir).ok();

    // The opener plugin handles the platform's own file manager.
    tauri_plugin_opener::open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::Internal(format!("the log folder could not be opened: {e}")))?;
    Ok(())
}

/// Delete every log but the one being written.
#[tauri::command]
pub fn delete_logs(state: tauri::State<'_, AppState>) -> usize {
    let dir = log_dir(&state);
    let files = logs::daily_logs(&dir);
    if files.is_empty() {
        return 0;
    }

    let mut removed = 0;
    // Never the last one: it is open, and on Windows deleting it would fail
    // anyway — but the intent matters more than the platform.
    for path in &files[..files.len().saturating_sub(1)] {
        if std::fs::remove_file(path).is_ok() {
            removed += 1;
        }
    }

    tracing::info!(target: "app", removed, "logs deleted by the user");
    removed
}
