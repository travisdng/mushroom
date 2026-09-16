//! The API key.
//!
//! Stored in the Windows Credential Manager, never in `config.json`, never in
//! a log, and never returned across the Tauri boundary. The UI only ever
//! learns whether a key is set.

use std::sync::Mutex;

use serde::Serialize;

use crate::ai::provider::Provider;

const SERVICE: &str = "Mushroom";

fn account(provider: Provider) -> String {
    // Keyed per provider so switching between LiteLLM and OpenAI does not
    // destroy the other one's key.
    match provider {
        Provider::LiteLlm => "ai-key:litellm".to_string(),
        Provider::OpenAi => "ai-key:openai".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyStatus {
    Set,
    NotSet,
    /// The credential store was unavailable, so the key lives in memory for
    /// this session only and is forgotten on exit (R2.5).
    SessionOnly,
}

/// Fallback when the OS credential store cannot be used.
static SESSION_KEYS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn session_get(account: &str) -> Option<String> {
    SESSION_KEYS
        .lock()
        .ok()?
        .iter()
        .find(|(a, _)| a == account)
        .map(|(_, k)| k.clone())
}

fn session_set(account: &str, key: &str) {
    if let Ok(mut keys) = SESSION_KEYS.lock() {
        keys.retain(|(a, _)| a != account);
        keys.push((account.to_string(), key.to_string()));
    }
}

fn session_clear(account: &str) {
    if let Ok(mut keys) = SESSION_KEYS.lock() {
        keys.retain(|(a, _)| a != account);
    }
}

/// Store a key. Falls back to memory if the credential store refuses.
pub fn set(provider: Provider, key: &str) -> KeyStatus {
    let account = account(provider);

    match keyring::Entry::new(SERVICE, &account).and_then(|e| e.set_password(key)) {
        Ok(()) => {
            session_clear(&account);
            tracing::info!(target: "ai", "API key stored in the credential manager");
            KeyStatus::Set
        }
        Err(err) => {
            // Deliberately not an error: refusing to work because a keychain
            // is unavailable would be worse than a session-scoped key, as
            // long as the user is told (R2.5).
            tracing::warn!(
                target: "ai",
                error = %err,
                "credential store unavailable; keeping the key for this session only"
            );
            session_set(&account, key);
            KeyStatus::SessionOnly
        }
    }
}

/// Read the key. Only `ai/` ever calls this, and it never crosses the
/// Tauri boundary.
pub fn get(provider: Provider) -> Option<String> {
    let account = account(provider);

    if let Some(key) = session_get(&account) {
        return Some(key);
    }

    keyring::Entry::new(SERVICE, &account)
        .and_then(|e| e.get_password())
        .ok()
        .filter(|k| !k.trim().is_empty())
}

pub fn clear(provider: Provider) {
    let account = account(provider);
    session_clear(&account);

    if let Ok(entry) = keyring::Entry::new(SERVICE, &account) {
        match entry.delete_credential() {
            Ok(()) => tracing::info!(target: "ai", "API key removed"),
            Err(keyring::Error::NoEntry) => {}
            Err(err) => tracing::warn!(target: "ai", error = %err, "key could not be removed"),
        }
    }
}

/// What the UI is allowed to know: whether a key exists, not what it is.
pub fn status(provider: Provider) -> KeyStatus {
    let account = account(provider);

    if session_get(&account).is_some() {
        return KeyStatus::SessionOnly;
    }
    match get(provider) {
        Some(_) => KeyStatus::Set,
        None => KeyStatus::NotSet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// These tests share genuinely global state — the Windows Credential
    /// Manager and the session fallback — so they must not run concurrently
    /// with each other. Rust runs tests in parallel by default.
    static SERIALISE: Mutex<()> = Mutex::new(());

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        SERIALISE.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn a_missing_key_reports_not_set() {
        let _lock = guard();
        // Use a provider slot the other tests do not write to, and make sure
        // it starts clean.
        clear(Provider::OpenAi);
        assert_eq!(status(Provider::OpenAi), KeyStatus::NotSet);
        assert!(get(Provider::OpenAi).is_none());
    }

    #[test]
    fn a_key_round_trips_and_can_be_cleared() {
        let _lock = guard();
        let secret = "sk-test-do-not-use-1234567890";
        let outcome = set(Provider::LiteLlm, secret);
        assert!(matches!(outcome, KeyStatus::Set | KeyStatus::SessionOnly));

        assert_eq!(get(Provider::LiteLlm).as_deref(), Some(secret));
        assert_ne!(status(Provider::LiteLlm), KeyStatus::NotSet);

        clear(Provider::LiteLlm);
        assert!(get(Provider::LiteLlm).is_none());
        assert_eq!(status(Provider::LiteLlm), KeyStatus::NotSet);
    }

    #[test]
    fn the_two_providers_keep_separate_keys() {
        let _lock = guard();
        set(Provider::LiteLlm, "litellm-key");
        set(Provider::OpenAi, "openai-key");

        assert_eq!(get(Provider::LiteLlm).as_deref(), Some("litellm-key"));
        assert_eq!(get(Provider::OpenAi).as_deref(), Some("openai-key"));

        // Clearing one must not touch the other.
        clear(Provider::LiteLlm);
        assert!(get(Provider::LiteLlm).is_none());
        assert_eq!(get(Provider::OpenAi).as_deref(), Some("openai-key"));

        clear(Provider::OpenAi);
    }

    #[test]
    fn accounts_are_distinct_per_provider() {
        assert_ne!(account(Provider::LiteLlm), account(Provider::OpenAi));
    }
}
