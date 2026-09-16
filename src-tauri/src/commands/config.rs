use crate::config::{save, AppConfig, UiState};
use crate::error::AppErrorDto;
use crate::state::AppState;

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Result<AppConfig, AppErrorDto> {
    let config = state
        .config
        .lock()
        .map_err(|_| crate::error::AppError::Internal("settings lock poisoned".into()))?;
    Ok(config.clone())
}

/// Persist window and panel geometry. Called on change, debounced by the UI.
#[tauri::command]
pub fn set_ui_state(state: tauri::State<'_, AppState>, ui: UiState) -> Result<(), AppErrorDto> {
    let snapshot = {
        let mut config = state
            .config
            .lock()
            .map_err(|_| crate::error::AppError::Internal("settings lock poisoned".into()))?;
        config.ui = ui;
        config.clone()
    };

    save(&state.data_dir, &snapshot)?;
    tracing::debug!(target: "app", "ui state saved");
    Ok(())
}

/// Change the notes folder, then re-scan it.
///
/// Validates before accepting: pointing Mushroom at a folder it cannot write
/// would fail later, at save time, when it matters most (R3.4).
#[tauri::command]
pub async fn set_notes_root(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<usize, AppErrorDto> {
    let root = std::path::PathBuf::from(path);
    let notes = state.notes.clone();
    let data_dir = state.data_dir.clone();

    let snapshot = {
        let mut config = state
            .config
            .lock()
            .map_err(|_| crate::error::AppError::Internal("settings lock poisoned".into()))?;
        config.notes_root = Some(root.clone());
        config.clone()
    };

    let count = tauri::async_runtime::spawn_blocking(move || {
        crate::notes::cache::bootstrap(&root).map_err(|source| {
            crate::error::AppError::NotesRootUnavailable {
                path: root.clone(),
                source,
            }
        })?;
        notes.set_root(root)?;
        notes.rescan()
    })
    .await
    .map_err(|e| crate::error::AppError::Internal(format!("background task failed: {e}")))??;

    save(&data_dir, &snapshot)?;
    Ok(count)
}
