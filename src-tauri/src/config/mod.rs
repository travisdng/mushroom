//! Persisted settings, stored as pretty JSON in the app data directory.
//!
//! Milestone 1 stores only window and panel state. Notes root, AI endpoint,
//! and model are added by their own milestones — `version` exists from the
//! first release so those additions can migrate.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ai::provider::Provider;
use crate::error::AppError;

pub mod secrets;

/// Bumped to 2 when the AI section arrived. `serde(default)` means an older
/// file still loads; the version is here so a real migration is possible.
pub const CONFIG_VERSION: u32 = 2;

/// Settings for the AI service. The API key is deliberately absent — it lives
/// in the OS credential store, never in this file (R2.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AiConfig {
    pub provider: Provider,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
    pub max_context_tokens: u32,
    pub temperature: f32,
    /// Debug aid, off by default: logs whole prompts, which are note content.
    pub log_prompts: bool,
}

impl Default for AiConfig {
    fn default() -> Self {
        let provider = Provider::default();
        Self {
            base_url: provider.default_base_url().to_string(),
            model: provider.default_model().to_string(),
            provider,
            timeout_secs: 120,
            max_context_tokens: 6000,
            temperature: 0.2,
            log_prompts: false,
        }
    }
}

impl AiConfig {
    /// Whether there is enough here to attempt a request at all.
    pub fn is_configured(&self) -> bool {
        !self.base_url.trim().is_empty() && !self.model.trim().is_empty()
    }

    /// Validate what the user typed, before it is saved (R3.5).
    pub fn validate(&self) -> Result<(), String> {
        let url = self.base_url.trim();
        if url.is_empty() {
            return Err("Enter an endpoint URL.".into());
        }
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err("The endpoint must begin with http:// or https://".into());
        }
        if self.model.trim().is_empty() {
            return Err("Enter a model name.".into());
        }
        if self.timeout_secs == 0 || self.timeout_secs > 3600 {
            return Err("Timeout must be between 1 and 3600 seconds.".into());
        }
        if !(0.0..=2.0).contains(&self.temperature) {
            return Err("Temperature must be between 0 and 2.".into());
        }
        if self.max_context_tokens < 500 {
            return Err("Context budget must be at least 500 tokens.".into());
        }
        Ok(())
    }

    /// The URL to POST a chat completion to, tolerating a trailing slash.
    pub fn chat_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }

    pub fn models_url(&self) -> String {
        format!("{}/models", self.base_url.trim_end_matches('/'))
    }
}

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
    /// Reopen maximised. `window` keeps the restored-state rect so that
    /// un-maximising returns to a sensible size rather than filling the screen.
    pub maximized: bool,
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
            maximized: false,
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
    /// Where the Markdown lives. `None` until first run resolves the default.
    pub notes_root: Option<PathBuf>,
    pub ai: AiConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            ui: UiState::default(),
            notes_root: None,
            ai: AiConfig::default(),
        }
    }
}

/// `%USERPROFILE%\Mushroom\notes`, the default home for a new install.
pub fn default_notes_root() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .or_else(dirs_home)
        .map(|home| home.join("Mushroom").join("notes"))
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
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
    fn ai_defaults_are_sane_and_unconfigured_is_not_an_error() {
        let ai = AiConfig::default();
        assert!(ai.validate().is_ok());
        assert!(ai.is_configured());
        assert_eq!(ai.chat_url(), "http://localhost:4000/chat/completions");
        assert!(!ai.log_prompts, "prompt logging must be off by default");
    }

    #[test]
    fn chat_url_tolerates_a_trailing_slash() {
        let mut ai = AiConfig::default();
        ai.base_url = "http://localhost:4000/v1/".into();
        assert_eq!(ai.chat_url(), "http://localhost:4000/v1/chat/completions");
        assert_eq!(ai.models_url(), "http://localhost:4000/v1/models");
    }

    #[test]
    fn validation_rejects_what_would_fail_later() {
        let bad = |f: fn(&mut AiConfig)| {
            let mut ai = AiConfig::default();
            f(&mut ai);
            assert!(ai.validate().is_err(), "should have been rejected");
        };

        bad(|a| a.base_url = String::new());
        bad(|a| a.base_url = "localhost:4000".into());
        bad(|a| a.base_url = "ftp://localhost".into());
        bad(|a| a.model = "  ".into());
        bad(|a| a.timeout_secs = 0);
        bad(|a| a.temperature = 5.0);
        bad(|a| a.max_context_tokens = 10);
    }

    #[test]
    fn a_config_written_before_the_ai_section_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            config_path(dir.path()),
            r#"{"version":1,"ui":{"sidebarWidth":200}}"#,
        )
        .unwrap();

        let (config, err) = load(dir.path());
        assert!(err.is_none(), "an older config must not be fatal");
        assert_eq!(config.ui.sidebar_width, 200);
        assert!(config.ai.is_configured(), "AI section filled from defaults");
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
