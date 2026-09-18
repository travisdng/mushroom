//! AI settings and connectivity commands.
//!
//! The API key crosses this boundary in one direction only. Nothing here
//! returns it, and `get_ai_settings` deliberately reports a status instead.

use serde::{Deserialize, Serialize};

use crate::ai::provider::Provider;
use crate::ai::service::LastConnection;
use crate::config::secrets::{self, KeyStatus};
use crate::config::{save, AiConfig};
use crate::error::{AppError, AppErrorDto};
use crate::state::AppState;

/// Everything the Settings dialog needs, and nothing it does not.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    pub config: AiConfig,
    /// Whether a key exists — never the key itself (R2.3).
    pub key_status: KeyStatus,
    pub key_required: bool,
    /// So the dialog can show "(default: …)" without hard-coding it.
    pub default_base_url: String,
    pub default_model: String,
    pub last_connection: Option<LastConnection>,
}

fn settings_for(state: &AppState, config: AiConfig) -> AiSettings {
    AiSettings {
        key_status: secrets::status(config.provider),
        key_required: config.provider.key_required(),
        default_base_url: config.provider.default_base_url().to_string(),
        default_model: config.provider.default_model().to_string(),
        last_connection: state.ai.last_connection(),
        config,
    }
}

#[tauri::command]
pub fn get_ai_settings(state: tauri::State<'_, AppState>) -> Result<AiSettings, AppErrorDto> {
    let config = state
        .config
        .lock()
        .map_err(|_| AppError::Internal("settings lock poisoned".into()))?
        .ai
        .clone();

    Ok(settings_for(&state, config))
}

/// The defaults for a provider, so switching the drop-down can offer them
/// without the front end keeping its own copy of the table.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDefaults {
    pub base_url: String,
    pub model: String,
    pub key_required: bool,
}

#[tauri::command]
pub fn get_provider_defaults(provider: Provider) -> ProviderDefaults {
    ProviderDefaults {
        base_url: provider.default_base_url().to_string(),
        model: provider.default_model().to_string(),
        key_required: provider.key_required(),
    }
}

/// Save the AI settings and rebuild the client.
///
/// Validated before anything is written: a bad URL saved now is a confusing
/// failure later, at the moment the user is trying to do something else.
#[tauri::command]
pub fn set_ai_config(
    state: tauri::State<'_, AppState>,
    mut config: AiConfig,
) -> Result<AiSettings, AppErrorDto> {
    config
        .validate()
        .map_err(|message| AppError::InvalidSetting { message })?;

    // Applying settings is what "configured" means, whatever the values are.
    config.configured = true;

    let snapshot = {
        let mut current = state
            .config
            .lock()
            .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
        current.ai = config.clone();
        current.clone()
    };

    save(&state.data_dir, &snapshot)?;
    state.ai.set_config(config.clone());

    tracing::info!(
        target: "ai",
        provider = ?config.provider,
        model = %config.model,
        "AI settings saved"
    );

    Ok(settings_for(&state, config))
}

#[derive(Debug, Deserialize)]
pub struct KeyInput {
    /// Named explicitly rather than read from the saved config. The Settings
    /// dialog can have a different provider selected than the one last
    /// applied, and storing an OpenAI key under LiteLLM because the user had
    /// not pressed Apply yet is the kind of bug nobody would think to look for.
    pub provider: Provider,
    pub key: String,
}

/// Store the key for a provider.
#[tauri::command]
pub fn set_ai_key(
    state: tauri::State<'_, AppState>,
    input: KeyInput,
) -> Result<KeyStatus, AppErrorDto> {
    if input.key.trim().is_empty() {
        return Err(AppError::InvalidSetting {
            message: "Enter a key, or use Clear to remove the stored one.".into(),
        }
        .into());
    }

    let status = secrets::set(input.provider, input.key.trim());
    // The client holds the key, so it has to be rebuilt for the new one to be
    // used — otherwise the next request still sends the old key.
    state.ai.rebuild();
    Ok(status)
}

#[tauri::command]
pub fn clear_ai_key(
    state: tauri::State<'_, AppState>,
    provider: Provider,
) -> Result<KeyStatus, AppErrorDto> {
    secrets::clear(provider);
    state.ai.rebuild();
    Ok(secrets::status(provider))
}

/// The key status for a provider the user is considering but has not applied.
#[tauri::command]
pub fn get_key_status(provider: Provider) -> KeyStatus {
    secrets::status(provider)
}

/// Test the settings the user is looking at, which may not be the saved ones.
#[tauri::command]
pub async fn test_ai_connection(
    state: tauri::State<'_, AppState>,
    config: Option<AiConfig>,
) -> Result<LastConnection, AppErrorDto> {
    let ai = state.ai.clone();
    match ai.test_connection(config).await {
        // Both outcomes are reported the same way: Test Connection failing is
        // an answer, not an exception, and the dialog shows it in place.
        Ok(_) | Err(_) => ai
            .last_connection()
            .ok_or_else(|| AppError::Internal("connection test produced no result".into()).into()),
    }
}

/// What the model drop-down shows.
///
/// An endpoint with no `/models` route is ordinary, so this reports
/// unavailability rather than failing (R4.2).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelList {
    pub models: Vec<String>,
    pub available: bool,
    pub message: Option<String>,
}

#[tauri::command]
pub async fn list_ai_models(
    state: tauri::State<'_, AppState>,
    config: Option<AiConfig>,
) -> Result<ModelList, AppErrorDto> {
    let ai = state.ai.clone();

    Ok(match ai.list_models(config).await {
        Ok(models) => ModelList {
            available: !models.is_empty(),
            message: models
                .is_empty()
                .then(|| "This service listed no models. Type a model name.".to_string()),
            models,
        },
        Err(err) => {
            tracing::info!(target: "ai", error = %err, "model list unavailable");
            let dto: AppErrorDto = (&err).into();
            ModelList {
                models: Vec::new(),
                available: false,
                message: Some(format!("{} Type a model name instead.", dto.message)),
            }
        }
    })
}

/// The request that actually went to the endpoint, markers and all (R5.1).
///
/// Local only, in memory only. This is the screen that lets somebody check
/// rather than trust, which is worth more than any assurance in a dialog.
#[tauri::command]
pub fn get_last_ai_request(
    state: tauri::State<'_, AppState>,
) -> Option<crate::ai::privacy::LastRequest> {
    state.ai.last_request()
}

/// Every detection rule Mushroom ships, for the Settings list.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyRules {
    /// Rule names, sorted. The list is long; Settings filters it.
    pub available: Vec<String>,
    pub disabled: Vec<String>,
}

#[tauri::command]
pub fn get_privacy_rules(state: tauri::State<'_, AppState>) -> PrivacyRules {
    let disabled = state
        .config
        .lock()
        .map(|c| c.ai.ai_disabled_rules.clone())
        .unwrap_or_default();

    // Every shipped rule, including the ones currently switched off — a rule
    // you have disabled must still be visible, or you cannot switch it back.
    let all = crate::ai::privacy::RuleSet::builtin();
    PrivacyRules {
        available: all.names().into_iter().map(str::to_string).collect(),
        disabled,
    }
}

/// Set the exclusion rules, parsing each so Settings can show a broken one.
#[tauri::command]
pub fn set_ai_exclusions(
    state: tauri::State<'_, AppState>,
    patterns: Vec<String>,
) -> Result<Vec<crate::exclusion::ExclusionRule>, AppErrorDto> {
    let parsed: Vec<crate::exclusion::ExclusionRule> = patterns
        .iter()
        .map(|p| crate::exclusion::ExclusionRule::parse(p))
        .collect();

    let snapshot = {
        let mut config = state
            .config
            .lock()
            .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
        config.ai.ai_exclusions = parsed.clone();
        config.clone()
    };
    save(&state.data_dir, &snapshot)?;
    state.ai.set_config(snapshot.ai);
    Ok(parsed)
}

/// Switch detection rules on or off by name.
#[tauri::command]
pub fn set_disabled_privacy_rules(
    state: tauri::State<'_, AppState>,
    disabled: Vec<String>,
) -> Result<(), AppErrorDto> {
    let snapshot = {
        let mut config = state
            .config
            .lock()
            .map_err(|_| AppError::Internal("settings lock poisoned".into()))?;
        config.ai.ai_disabled_rules = disabled;
        config.clone()
    };
    save(&state.data_dir, &snapshot)?;
    // Takes effect on the next question, not the next launch.
    state.ai.set_config(snapshot.ai);
    Ok(())
}
