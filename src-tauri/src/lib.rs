//! Mushroom — local-first personal knowledge and notes.
//!
//! Keep this file thin: it wires plugins, managed state, and the command
//! handler list, and nothing else. See `.kiro/steering/structure.md`.

mod commands;
mod config;
mod error;
pub mod logging;
mod state;

use tauri::Manager;

use crate::error::AppError;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir().map_err(|_| AppError::NoDataDir)?;
            std::fs::create_dir_all(&data_dir).ok();

            let log_dir = data_dir.join("logs");
            std::fs::create_dir_all(&log_dir).ok();
            let guard = logging::init(&log_dir);
            // Hold the writer guard for the life of the process.
            app.manage(guard);

            let (config, load_error) = config::load(&data_dir);

            tracing::info!(
                target: "app",
                version = env!("CARGO_PKG_VERSION"),
                data_dir = %data_dir.display(),
                "Mushroom starting"
            );

            if let Some(err) = load_error {
                // Not fatal: the app runs on defaults and says so in the log.
                tracing::warn!(target: "app", error = %err, "settings could not be loaded");
            }

            app.manage(AppState::new(data_dir, config));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::ping,
            commands::config::get_config,
            commands::config::set_ui_state,
        ])
        .run(tauri::generate_context!())
        .expect("Mushroom failed to start");
}
