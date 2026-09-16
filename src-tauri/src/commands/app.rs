use serde::Serialize;

use crate::error::AppErrorDto;
use crate::state::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub data_dir: String,
}

/// Proves the frontend-to-backend channel works end to end (R7.3).
/// The About dialog reads its version from here.
#[tauri::command]
pub fn ping(state: tauri::State<'_, AppState>) -> Result<AppInfo, AppErrorDto> {
    Ok(AppInfo {
        name: "Mushroom".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        data_dir: state.data_dir.display().to_string(),
    })
}
