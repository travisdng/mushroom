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

/// 2 added the AI section; 3 added `AiConfig::configured`. `serde(default)`
/// means an older file still loads; the version is here so a real migration is
/// possible.
pub const CONFIG_VERSION: u32 = 3;

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
    /// Stream the answer as it is written. On by default; a provider that
    /// refuses `stream: true` is fallen back on automatically, so this only
    /// needs turning off to force the one-shot path.
    pub stream: bool,
    /// How hard the privacy gate tries before anything is sent.
    ///
    /// Not in the credential store: it is a preference, not a secret. The
    /// struct-level `#[serde(default)]` means a config file written by 1.0.x
    /// lands on `Redact` rather than on whatever is first in the enum — an
    /// upgrade must never quietly turn the gate off (spec 08 R4.5).
    pub privacy_mode: crate::ai::privacy::PrivacyMode,

    /// Folders and globs the user has excluded from every AI path.
    ///
    /// Frontmatter exclusion (`ai: false`) is per note and lives in the note;
    /// this is the settings half, for whole folders. Stored with any parse
    /// problem attached so Settings can show a broken rule as broken rather
    /// than silently doing nothing with it.
    #[serde(default)]
    pub ai_exclusions: Vec<crate::exclusion::ExclusionRule>,

    /// Whether the user has ever applied AI settings.
    ///
    /// Not derivable from the other fields: `base_url` and `model` always hold
    /// a plausible default, so a fresh install looks identical to one pointed
    /// deliberately at a local LiteLLM. Without this, a first run would greet
    /// the user with "could not connect to localhost:4000" instead of saying
    /// AI is not set up yet (R1.6, R7.1).
    pub configured: bool,
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
            stream: true,
            privacy_mode: crate::ai::privacy::PrivacyMode::Redact,
            ai_exclusions: Vec::new(),
            configured: false,
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
    /// The last 20 questions asked, most recent first (R8.1).
    pub ai_history: Vec<String>,
    /// The last 20 notes opened, most recent first. Quick Open ranks by it, so
    /// the note you just had is one keystroke away.
    pub recent_notes: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            ui: UiState::default(),
            notes_root: None,
            ai: AiConfig::default(),
            ai_history: Vec::new(),
            recent_notes: Vec::new(),
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

    // Notepad and PowerShell both write UTF-8 with a byte-order mark by
    // default, and `serde_json` rejects one. Without this, hand-editing the
    // settings file on Windows silently resets every setting to its default:
    // the file reads as "not valid JSON", the app starts fresh, and the only
    // trace is a log line nobody reads.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);

    match serde_json::from_str::<AppConfig>(text) {
        Ok(config) => (migrate(config), None),
        Err(source) => (
            AppConfig::default(),
            Some(AppError::ConfigParse { path, source }),
        ),
    }
}

/// Bring an older file up to the current schema.
///
/// v1 -> v2 added the AI section, which `serde(default)` has already filled in
/// by the time we get here. The only thing left is to record that it happened:
/// without this the file keeps claiming v1 forever, and a future migration
/// keyed on the version would run against data that has already been migrated.
fn migrate(mut config: AppConfig) -> AppConfig {
    if config.version < CONFIG_VERSION {
        tracing::info!(
            target: "app",
            from = config.version,
            to = CONFIG_VERSION,
            "settings migrated"
        );

        // A file that already has an AI section was written by someone who had
        // been through Settings, so they are configured. Only a genuinely new
        // install starts at the current version with the flag still false.
        if config.version >= 2 {
            config.ai.configured = true;
        }

        config.version = CONFIG_VERSION;
    }
    config
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
    fn a_settings_file_with_a_byte_order_mark_still_loads() {
        // What Notepad and `Set-Content -Encoding utf8` produce. Rejecting it
        // silently reset everything the user had configured.
        let dir = tempfile::tempdir().unwrap();
        let json = r#"{"version":3,"ui":{"sidebarWidth":321}}"#;
        std::fs::write(config_path(dir.path()), format!("\u{feff}{json}")).unwrap();

        let (config, err) = load(dir.path());
        assert!(
            err.is_none(),
            "a BOM must not look like a broken file: {err:?}"
        );
        assert_eq!(config.ui.sidebar_width, 321);
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
        let ai = AiConfig {
            base_url: "http://localhost:4000/v1/".into(),
            ..AiConfig::default()
        };
        assert_eq!(ai.chat_url(), "http://localhost:4000/v1/chat/completions");
        assert_eq!(ai.models_url(), "http://localhost:4000/v1/models");
    }

    #[test]
    fn validation_rejects_what_would_fail_later() {
        let bad = |f: fn(AiConfig) -> AiConfig| {
            let ai = f(AiConfig::default());
            assert!(ai.validate().is_err(), "should have been rejected");
        };

        bad(|a| AiConfig {
            base_url: String::new(),
            ..a
        });
        bad(|a| AiConfig {
            base_url: "localhost:4000".into(),
            ..a
        });
        bad(|a| AiConfig {
            base_url: "ftp://localhost".into(),
            ..a
        });
        bad(|a| AiConfig {
            model: "  ".into(),
            ..a
        });
        bad(|a| AiConfig {
            timeout_secs: 0,
            ..a
        });
        bad(|a| AiConfig {
            temperature: 5.0,
            ..a
        });
        bad(|a| AiConfig {
            max_context_tokens: 10,
            ..a
        });
    }

    #[test]
    fn an_older_file_is_marked_as_migrated() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(config_path(dir.path()), r#"{"version":1,"ui":{}}"#).unwrap();

        let (config, _) = load(dir.path());
        assert_eq!(
            config.version, CONFIG_VERSION,
            "a file left claiming v1 would be migrated again next time"
        );

        // And the bump survives a round trip, rather than being recomputed.
        save(dir.path(), &config).unwrap();
        let (again, _) = load(dir.path());
        assert_eq!(again.version, CONFIG_VERSION);
    }

    #[test]
    fn a_fresh_install_is_not_treated_as_configured() {
        // The whole point of the flag: defaults look plausible, so without it
        // a first run cannot be told apart from a deliberate local setup.
        let dir = tempfile::tempdir().unwrap();
        let (config, _) = load(dir.path());
        assert!(!config.ai.configured);
        assert!(config.ai.is_configured(), "the defaults are still usable");
    }

    #[test]
    fn an_existing_ai_section_counts_as_configured() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            config_path(dir.path()),
            r#"{"version":2,"ui":{},"ai":{"baseUrl":"http://localhost:4000"}}"#,
        )
        .unwrap();

        let (config, _) = load(dir.path());
        assert!(
            config.ai.configured,
            "someone who had been through Settings must not be told to set it up again"
        );
    }

    #[test]
    fn a_pre_ai_config_is_not_treated_as_configured() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(config_path(dir.path()), r#"{"version":1,"ui":{}}"#).unwrap();
        let (config, _) = load(dir.path());
        assert!(
            !config.ai.configured,
            "v1 never had an AI section to set up"
        );
    }

    #[test]
    fn a_newer_file_is_left_alone() {
        // A config from a future version must not be silently downgraded.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(config_path(dir.path()), r#"{"version":99,"ui":{}}"#).unwrap();
        let (config, _) = load(dir.path());
        assert_eq!(config.version, 99);
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
