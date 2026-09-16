//! The error contract for the whole application.
//!
//! Every error that crosses the Tauri boundary becomes an [`AppErrorDto`]:
//! a plain-language `message` for the user, and a technical `detail` that only
//! the Diagnostics screen shows. Raw Rust error text never reaches the UI.
//!
//! See `.kiro/steering/error-handling.md`.

use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("configuration could not be read")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("configuration could not be written")]
    ConfigWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("configuration file was not valid JSON")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("the application data directory could not be determined")]
    NoDataDir,

    #[error("internal error")]
    Internal(String),
}

/// What the frontend receives. `detail` is for Diagnostics, never a dialog body.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppErrorDto {
    pub code: String,
    pub title: String,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
}

impl AppErrorDto {
    fn new(
        code: &str,
        title: &str,
        message: String,
        detail: Option<String>,
        hint: Option<&str>,
    ) -> Self {
        Self {
            code: code.to_string(),
            title: title.to_string(),
            message,
            detail,
            hint: hint.map(str::to_string),
        }
    }
}

/// Render an error and its full source chain, for `detail` and the log only.
fn chain(err: &dyn std::error::Error) -> String {
    let mut out = err.to_string();
    let mut source = err.source();
    while let Some(cause) = source {
        out.push_str(": ");
        out.push_str(&cause.to_string());
        source = cause.source();
    }
    out
}

impl From<AppError> for AppErrorDto {
    fn from(err: AppError) -> Self {
        let detail = Some(chain(&err));

        match &err {
            AppError::ConfigRead { path, .. } => AppErrorDto::new(
                "CONFIG_READ",
                "Mushroom could not read its settings",
                format!(
                    "The settings file at {} could not be opened. Mushroom has \
                     started with default settings.",
                    path.display()
                ),
                detail,
                Some("Check that the file exists and is not open in another program."),
            ),

            AppError::ConfigWrite { path, .. } => AppErrorDto::new(
                "CONFIG_WRITE",
                "Mushroom could not save its settings",
                format!(
                    "The settings file at {} could not be written.",
                    path.display()
                ),
                detail,
                Some("Check that the folder exists and the disk is not full or read-only."),
            ),

            AppError::ConfigParse { path, .. } => AppErrorDto::new(
                "CONFIG_PARSE",
                "Mushroom could not understand its settings",
                format!(
                    "The settings file at {} is not valid. Mushroom has started \
                     with default settings.",
                    path.display()
                ),
                detail,
                Some("Delete the file to start again with defaults."),
            ),

            AppError::NoDataDir => AppErrorDto::new(
                "NO_DATA_DIR",
                "Mushroom could not find a place to store its data",
                "Windows did not report an application data folder.".to_string(),
                detail,
                Some("This is unusual. Check that your user profile is available."),
            ),

            AppError::Internal(_) => AppErrorDto::new(
                "INTERNAL",
                "Something went wrong inside Mushroom",
                "An unexpected problem stopped that action. Your notes are not \
                 affected."
                    .to_string(),
                detail,
                Some("See Tools \u{2192} Diagnostics for the technical details."),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn io_err() -> std::io::Error {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Access is denied. (os error 5)",
        )
    }

    #[test]
    fn detail_carries_the_technical_text_and_message_does_not() {
        let dto: AppErrorDto = AppError::ConfigRead {
            path: PathBuf::from(r"C:\Users\x\AppData\Roaming\Mushroom\config.json"),
            source: io_err(),
        }
        .into();

        let detail = dto.detail.expect("detail should be present");
        assert!(
            detail.contains("os error 5"),
            "detail should keep the OS text: {detail}"
        );

        // The user-facing sentence stays free of Rust and OS jargon.
        for jargon in ["os error", "Error {", "std::io", "Kind("] {
            assert!(
                !dto.message.contains(jargon),
                "message leaked {jargon:?}: {}",
                dto.message
            );
        }
        assert_eq!(dto.code, "CONFIG_READ");
        assert!(dto.hint.is_some());
    }

    #[test]
    fn internal_errors_do_not_leak_their_payload_to_the_user() {
        let dto: AppErrorDto =
            AppError::Internal("thread panicked at src/notes/store.rs:42".into()).into();

        assert!(!dto.message.contains("store.rs"));
        assert!(dto.detail.unwrap().contains("internal error"));
    }

    #[test]
    fn chain_includes_the_source() {
        let err = AppError::ConfigWrite {
            path: PathBuf::from("config.json"),
            source: io_err(),
        };
        let text = chain(&err);
        assert!(text.starts_with("configuration could not be written"));
        assert!(text.contains("Access is denied"));
    }
}
