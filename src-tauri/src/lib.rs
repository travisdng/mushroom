//! Mushroom — local-first personal knowledge and notes.
//!
//! Keep this file thin: it wires plugins, managed state, and the command
//! handler list, and nothing else. See `.kiro/steering/structure.md`.

mod commands;
mod config;
mod error;
pub mod logging;
pub mod notes;
mod state;
mod window;

use tauri::Manager;

use crate::error::AppError;
use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Deliberately NOT app_data_dir(): that resolves to the bundle
            // identifier (com.mushroom.desktop). The spec puts our data under the
            // product name, where a person would actually look for it.
            let data_dir = app
                .path()
                .data_dir()
                .map_err(|_| AppError::NoDataDir)?
                .join("Mushroom");
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

            let saved_rect = config.ui.window.clone();
            let saved_maximized = config.ui.maximized;
            app.manage(AppState::new(data_dir, config));

            if let Some(main) = app.get_webview_window("main") {
                window::restore(&main, saved_rect.as_ref(), saved_maximized);
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                if let Some(main) = window.app_handle().get_webview_window("main") {
                    window::persist(&main);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::ping,
            commands::config::get_config,
            commands::config::set_ui_state,
        ])
        .run(tauri::generate_context!())
        .expect("Mushroom failed to start");
}
