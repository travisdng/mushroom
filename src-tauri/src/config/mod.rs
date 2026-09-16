//! Persisted settings, stored as pretty JSON in the app data directory.
//!
//! Milestone 1 stores only window and panel state. Notes root, AI endpoint,
//! and model are added by their own milestones — `version` exists from the
//! first release so those additions can migrate.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiState {
    pub window: Option<WindowRect>,
    pub sidebar_width: u32,
    pub notebook_height: u32,
    pub ai_width: u32,
    pub show_status_bar: bool,
    pub show_notes_panel: bool,
    pub show_search_panel: bool,
    pub show_ai_panel: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            window: None,
            sidebar_width: 240,
            notebook_height: 220,
            ai_width: 320,
            show_status_bar: true,
            show_notes_panel: true,
            show_search_panel: false,
            show_ai_panel: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub version: u32,
    pub ui: UiState,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            ui: UiState::default(),
        }
    }
}

pub fn config_path(data_dir: &Path) -> PathBuf {
    data_dir.join("config.json")
}

/// Load settings, falling back to defaults.
///
/// A missing file is normal (first run). An unreadable or invalid file is not
/// fatal either: Mushroom starts with defaults and reports the problem, rather
/// than refusing to open. Returns the config plus any error worth surfacing.
pub fn load(data_dir: &Path) -> (AppConfig, Option<AppError>) {
    let path = config_path(data_dir);

    if !path.exists() {
        return (AppConfig::default(), None);
    }

    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) => {
            return (
                AppConfig::default(),
                Some(AppError::ConfigRead { path, source }),
            )
        }
    };

    match serde_json::from_str::<AppConfig>(&text) {
        Ok(config) => (config, None),
        Err(source) => (
            AppConfig::default(),
            Some(AppError::ConfigParse { path, source }),
        ),
    }
}

pub fn save(data_dir: &Path, config: &AppConfig) -> Result<(), AppError> {
    let path = config_path(data_dir);

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| AppError::ConfigWrite {
            path: path.clone(),
            source,
        })?;
    }

    let text = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::Internal(format!("settings could not be serialised: {e}")))?;

    std::fs::write(&path, text).map_err(|source| AppError::ConfigWrite { path, source })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults_without_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let (config, err) = load(dir.path());
        assert!(err.is_none());
        assert_eq!(config.version, CONFIG_VERSION);
        assert!(config.ui.show_status_bar);
    }

    #[test]
    fn round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = AppConfig::default();
        config.ui.sidebar_width = 333;
        config.ui.show_ai_panel = true;

        save(dir.path(), &config).unwrap();
        let (loaded, err) = load(dir.path());

        assert!(err.is_none());
        assert_eq!(loaded.ui.sidebar_width, 333);
        assert!(loaded.ui.show_ai_panel);
    }

    #[test]
    fn invalid_json_falls_back_to_defaults_and_reports() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(config_path(dir.path()), "{ not json").unwrap();

        let (config, err) = load(dir.path());
        assert_eq!(config.ui.sidebar_width, UiState::default().sidebar_width);
        assert!(matches!(err, Some(AppError::ConfigParse { .. })));
    }

    #[test]
    fn unknown_fields_and_partial_files_still_load() {
        let dir = tempfile::tempdir().unwrap();
        // A config written by a future version, and one missing most fields.
        std::fs::write(
            config_path(dir.path()),
            r#"{"version":1,"ui":{"sidebarWidth":200},"somethingNew":true}"#,
        )
        .unwrap();

        let (config, err) = load(dir.path());
        assert!(err.is_none(), "unknown keys must not be fatal");
        assert_eq!(config.ui.sidebar_width, 200);
        // Absent fields fall back to their defaults rather than zeroing.
        assert!(config.ui.show_status_bar);
    }
}
