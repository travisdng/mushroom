//! Mushroom — local-first personal knowledge and notes.
//!
//! Keep this file thin: it wires plugins, managed state, and the command
//! handler list, and nothing else. See `.kiro/steering/structure.md`.

mod commands;
mod config;
pub mod database;
mod error;
pub mod logging;
pub mod notes;
pub mod search;
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
            let configured_root = config.notes_root.clone();
            app.manage(AppState::new(data_dir, config));

            if let Some(main) = app.get_webview_window("main") {
                window::restore(&main, saved_rect.as_ref(), saved_maximized);
            }

            // Show the window first, then scan. A large notes folder must not
            // delay the window appearing (R3.8).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(err) = tauri::async_runtime::spawn_blocking(move || {
                    start_notes(&handle, configured_root)
                })
                .await
                {
                    tracing::error!(target: "files", error = %err, "notes startup task failed");
                }
            });

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
            commands::config::set_notes_root,
            commands::notes::list_notes,
            commands::notes::get_folder_tree,
            commands::notes::read_note,
            commands::notes::save_note,
            commands::notes::create_note,
            commands::notes::rename_note,
            commands::notes::move_note,
            commands::notes::delete_note,
            commands::notes::create_folder,
            commands::notes::delete_folder,
            commands::notes::import_notes,
            commands::notes::export_notes,
            commands::notes::get_notes_status,
            commands::notes::refresh_notes,
            commands::search::search_notes,
            commands::search::get_index_stats,
            commands::search::rebuild_index,
            commands::search::cancel_rebuild,
        ])
        .run(tauri::generate_context!())
        .expect("Mushroom failed to start");
}

/// Resolve the notes folder, create it on first run, clean up any temp files a
/// crash left behind, and populate the cache.
///
/// Every step here is best-effort: a missing or unreadable notes folder must
/// leave the application running so the user can choose another one (R1.3).
fn start_notes(app: &tauri::AppHandle, configured: Option<std::path::PathBuf>) {
    use tauri::Emitter;

    let Some(root) = notes::cache::resolve_root(configured.as_ref()) else {
        tracing::warn!(target: "files", "no notes folder could be determined");
        return;
    };

    if let Err(err) = notes::cache::bootstrap(&root) {
        tracing::warn!(
            target: "files",
            path = %root.display(),
            error = %err,
            "notes folder could not be created"
        );
        let _ = app.emit("notes-unavailable", root.to_string_lossy().to_string());
        return;
    }

    let swept = notes::store::sweep_temp_files(&root);
    if swept > 0 {
        tracing::info!(target: "files", swept, "removed leftover temp files");
    }

    let state = app.state::<AppState>();
    if let Err(err) = state.notes.set_root(root.clone()) {
        tracing::error!(target: "files", error = %err, "notes root could not be set");
        return;
    }

    // Persist the resolved default so the next launch does not re-derive it.
    if configured.is_none() {
        if let Ok(mut config) = state.config.lock() {
            config.notes_root = Some(root.clone());
            let snapshot = config.clone();
            drop(config);
            let _ = config::save(&state.data_dir, &snapshot);
        }
    }

    match state.notes.rescan() {
        Ok(count) => {
            let _ = app.emit("notes-ready", count);
        }
        Err(err) => {
            tracing::error!(target: "files", error = %err, "notes scan failed");
            let _ = app.emit("notes-unavailable", root.to_string_lossy().to_string());
            return;
        }
    }

    start_search(app, &root);
}

/// Open the search index and bring it in line with what was just scanned.
///
/// Every failure here is survivable: without an index you cannot search, but
/// notes, editing, and navigation are untouched (R5.3).
fn start_search(app: &tauri::AppHandle, root: &std::path::Path) {
    use tauri::Emitter;

    let state = app.state::<AppState>();
    let index_path = state.data_dir.join("mushroom.db");

    if let Err(err) = state.search.open(&index_path) {
        tracing::error!(target: "db", error = %err, "search index could not be opened");
        state.search.mark_stale();
        let _ = app.emit("index-unavailable", ());
        return;
    }

    let on_disk = match state.notes.list(None) {
        Ok(notes) => notes,
        Err(err) => {
            tracing::error!(target: "db", error = %err, "note list unavailable for indexing");
            return;
        }
    };

    match state.search.reconcile(root, &on_disk) {
        Ok(progress) => {
            tracing::info!(
                target: "db",
                reindexed = progress.done,
                skipped = progress.skipped,
                "index ready"
            );
            let _ = app.emit("index-ready", progress);
        }
        Err(err) => {
            tracing::error!(target: "db", error = %err, "index reconciliation failed");
            state.search.mark_stale();
            let _ = app.emit("index-unavailable", ());
        }
    }
}
