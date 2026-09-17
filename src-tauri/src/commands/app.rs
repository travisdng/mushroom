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

/// Called by the window once it has painted its first frame.
///
/// The only honest way to measure "time to an interactive window" from
/// outside is to ask the window. A process handle plus `Responding` says the
/// message loop is alive, which on a WebView2 app happens a couple of hundred
/// milliseconds before there is anything on screen — measuring that and
/// calling it startup would flatter the number by an order of magnitude.
///
/// Logged at INFO because it is also the first thing worth knowing when
/// somebody says Mushroom is slow to start.
#[tauri::command]
pub fn report_window_ready(state: tauri::State<'_, AppState>) {
    // Only the first paint counts. React strict mode mounts twice in
    // development, and a later remount is not a launch.
    if !state.claim_ready_report() {
        return;
    }
    tracing::info!(
        target: "app",
        ms = state.started.elapsed().as_millis() as u64,
        "window painted"
    );
}

/// The welcome note, if this launch just created one — once.
///
/// Pulled by the window rather than pushed to it. The startup scan can finish
/// before the WebView has booted (on a fresh profile it finishes in 30 ms and
/// the window paints 300 ms later), and an event emitted into that gap is
/// simply lost, which is how the first version of this failed.
#[tauri::command]
pub fn take_first_run_note(state: tauri::State<'_, AppState>) -> Option<String> {
    state.take_first_run_note()
}
