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

    #[error("that path is outside the notes folder")]
    PathOutsideRoot { path: String },

    #[error("that name cannot be used for a file")]
    InvalidNoteName { name: String },

    #[error("the notes folder is not available")]
    NotesRootUnavailable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("the note could not be read")]
    NoteRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("the note could not be written")]
    NoteWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("the file is not text Mushroom can edit")]
    NoteNotText { path: PathBuf },

    #[error("the note changed on disk since it was opened")]
    NoteChangedOnDisk { path: PathBuf, disk_modified: i64 },

    #[error("a note already exists there")]
    NoteExists { path: PathBuf },

    #[error("no notes folder has been chosen")]
    NotesRootMissing,

    #[error("the search index could not be used")]
    Database {
        what: String,
        #[source]
        source: rusqlite::Error,
    },

    #[error("this build of SQLite has no full-text search")]
    Fts5Unavailable {
        #[source]
        source: rusqlite::Error,
    },

    #[error("the search index file could not be replaced")]
    IndexUnavailable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("the search index was written by a newer version of Mushroom")]
    IndexTooNew { found: u32, supported: u32 },

    #[error("the search index is not open")]
    IndexNotOpen,

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

            AppError::PathOutsideRoot { path } => AppErrorDto::new(
                "PATH_OUTSIDE_ROOT",
                "That note is outside your notes folder",
                format!(
                    "Mushroom only opens notes stored inside your notes folder, \
                     and {path} is not."
                ),
                detail,
                Some("Use File \u{2192} Import to copy an outside file into your notes."),
            ),

            AppError::InvalidNoteName { name } => AppErrorDto::new(
                "INVALID_NOTE_NAME",
                "That name cannot be used",
                format!(
                    "Windows does not allow a file called {name}. Avoid the \
                     characters < > : \" / \\ | ? * , names ending in a dot or \
                     space, and reserved names such as CON or NUL."
                ),
                detail,
                Some("Try a different title."),
            ),

            AppError::NotesRootUnavailable { path, .. } => AppErrorDto::new(
                "NOTES_ROOT_UNAVAILABLE",
                "Mushroom cannot reach your notes folder",
                format!("The folder at {} could not be opened.", path.display()),
                detail,
                Some("Check the folder still exists, then choose it again in Settings."),
            ),

            AppError::NoteRead { path, .. } => AppErrorDto::new(
                "NOTE_READ",
                "Mushroom could not open that note",
                format!("The file at {} could not be read.", path.display()),
                detail,
                Some("Check the file still exists and is not open in another program."),
            ),

            AppError::NoteWrite { path, .. } => AppErrorDto::new(
                "NOTE_WRITE",
                "Mushroom could not save that note",
                format!(
                    "The file at {} could not be written. Your changes are still \
                     in the editor.",
                    path.display()
                ),
                detail,
                Some("Check the disk is not full and the file is not read-only."),
            ),

            AppError::NoteNotText { path } => AppErrorDto::new(
                "NOTE_NOT_TEXT",
                "That file is not editable text",
                format!(
                    "{} is not valid UTF-8 text, so Mushroom will not open it for \
                     editing — saving would corrupt it.",
                    path.display()
                ),
                detail,
                Some("Open it in a program that understands its format."),
            ),

            AppError::NoteChangedOnDisk { path, .. } => AppErrorDto::new(
                "NOTE_CHANGED_ON_DISK",
                "That note changed outside Mushroom",
                format!(
                    "{} was modified by another program after you opened it. \
                     Mushroom has not saved, so neither version is lost.",
                    path.display()
                ),
                detail,
                Some("Choose whether to keep your version, load theirs, or save a copy."),
            ),

            AppError::NoteExists { path } => AppErrorDto::new(
                "NOTE_EXISTS",
                "There is already a note with that name",
                format!(
                    "{} already exists, and Mushroom will not overwrite it.",
                    path.display()
                ),
                detail,
                Some("Pick a different name."),
            ),

            AppError::NotesRootMissing => AppErrorDto::new(
                "NOTES_ROOT_MISSING",
                "Mushroom does not have a notes folder yet",
                "Choose where your notes should live before creating one.".to_string(),
                detail,
                Some("Open Tools \u{2192} Settings to choose a folder."),
            ),

            AppError::Database { what, .. } => AppErrorDto::new(
                "DATABASE",
                "Search is not working",
                format!(
                    "Mushroom had a problem with its search index while {what}. \
                     Your notes are not affected — they are ordinary files, and \
                     the index is only a cache that can be rebuilt."
                ),
                detail,
                Some("Try Tools \u{2192} Rebuild Index."),
            ),

            AppError::Fts5Unavailable { .. } => AppErrorDto::new(
                "FTS5_UNAVAILABLE",
                "Search is unavailable in this build",
                "This copy of Mushroom was built without full-text search \
                 support, so searching cannot work. Notes and editing are \
                 unaffected."
                    .to_string(),
                detail,
                Some("Reinstall Mushroom from an official build."),
            ),

            AppError::IndexUnavailable { path, .. } => AppErrorDto::new(
                "INDEX_UNAVAILABLE",
                "The search index could not be replaced",
                format!(
                    "Mushroom could not move the damaged index at {}.",
                    path.display()
                ),
                detail,
                Some(
                    "Close any program using that file, or delete it by hand — it is only a cache.",
                ),
            ),

            AppError::IndexTooNew { found, supported } => AppErrorDto::new(
                "INDEX_TOO_NEW",
                "This index came from a newer Mushroom",
                format!(
                    "The search index is version {found} and this build understands \
                     version {supported}. Mushroom will not downgrade it, because \
                     that could lose information."
                ),
                detail,
                Some("Use the newer version, or delete the index file to rebuild it."),
            ),

            AppError::IndexNotOpen => AppErrorDto::new(
                "INDEX_NOT_OPEN",
                "Search is not ready yet",
                "Mushroom has not finished opening its search index. Your notes \
                 are available; searching will work shortly."
                    .to_string(),
                detail,
                Some("Wait a moment, or use Tools \u{2192} Rebuild Index."),
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
