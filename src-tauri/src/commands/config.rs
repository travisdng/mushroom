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
