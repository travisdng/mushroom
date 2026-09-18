//! Mushroom — local-first personal knowledge and notes.
//!
//! Keep this file thin: it wires plugins, managed state, and the command
//! handler list, and nothing else. See `.kiro/steering/structure.md`.

pub mod ai;
mod commands;
pub mod config;
pub mod database;
pub mod diagnostics;
mod error;
pub mod exclusion;
pub mod logging;
pub mod notes;
pub mod panics;
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
        .plugin(tauri_plugin_dialog::init())
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
            let removed = diagnostics::logs::prune(&log_dir);
            let guard = logging::init(&log_dir);
            if removed > 0 {
                tracing::info!(target: "app", removed, "old logs removed");
            }
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
            commands::app::report_window_ready,
            commands::app::take_first_run_note,
            commands::ai::get_ai_settings,
            commands::ai::get_provider_defaults,
            commands::ai::set_ai_config,
            commands::ai::set_ai_key,
            commands::ai::clear_ai_key,
            commands::ai::get_key_status,
            commands::ai::test_ai_connection,
            commands::ai::list_ai_models,
            commands::config::get_config,
            commands::config::set_ui_state,
            commands::config::set_notes_root,
            commands::config::record_recent_note,
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
            commands::ai_search::ai_search,
            commands::ai_search::ai_search_cancel,
            commands::ai_search::get_ai_history,
            commands::ai_search::clear_ai_history,
            commands::ai_search::get_ai_usage_stats,
            commands::diagnostics::get_diagnostics,
            commands::diagnostics::copy_diagnostics,
            commands::diagnostics::get_log_tail,
            commands::diagnostics::open_log_folder,
            commands::diagnostics::delete_logs,
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

    let welcome = match notes::cache::bootstrap_with_welcome(&root) {
        Ok(welcome) => welcome,
        Err(err) => {
            tracing::warn!(
                target: "files",
                path = %root.display(),
                error = %err,
                "notes folder could not be created"
            );
            let _ = app.emit("notes-unavailable", root.to_string_lossy().to_string());
            return;
        }
    };

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
            // Set before the event, so a window that asks the moment it hears
            // `notes-ready` cannot arrive before the answer is there.
            if let Some(id) = welcome {
                tracing::info!(target: "files", "first run: created the welcome note");
                state.set_first_run_note(id.as_str().to_string());
            }
            let _ = app.emit("notes-ready", count);
        }
        Err(err) => {
            tracing::error!(target: "files", error = %err, "notes scan failed");
            let _ = app.emit("notes-unavailable", root.to_string_lossy().to_string());
            return;
        }
    }

    start_search(app, &root);
    start_watching(app, &root);
}

/// Watch the notes folder so edits made elsewhere show up without an F5.
///
/// Failure is survivable and deliberately quiet: a network share or a locked
/// folder means no watching, not no application. The flag is surfaced in
/// Diagnostics and manual refresh keeps working (R2.7).
fn start_watching(app: &tauri::AppHandle, root: &std::path::Path) {
    use tauri::Emitter;

    let handle = app.clone();

    let outcome = notes::watcher::watch(
        root,
        // The very registry `store::atomic_write` records into. A fresh one
        // here would claim nothing, and every save would reload itself.
        notes::selfwrites::shared(),
        move |changes| {
            on_notes_changed(&handle, changes);
        },
    );

    let state = app.state::<AppState>();
    match outcome {
        Ok(watcher) => {
            if let Ok(mut slot) = state.watcher.lock() {
                *slot = Some(watcher);
            }
            state.set_watching(true);
            tracing::info!(target: "files", path = %root.display(), "watching the notes folder");
        }
        Err(err) => {
            state.set_watching(false);
            tracing::warn!(
                target: "files",
                error = %err,
                "the notes folder could not be watched; use F5 to refresh"
            );
            let _ = app.emit("watch-unavailable", ());
        }
    }
}

/// Apply changes the watcher reported and tell the window which notes moved.
fn on_notes_changed(app: &tauri::AppHandle, changes: Vec<notes::watcher::NoteChange>) {
    use tauri::Emitter;

    use crate::notes::model::NoteId;
    use crate::notes::watcher::NoteChange;

    let state = app.state::<AppState>();
    let Ok(root) = state.notes.root() else {
        return;
    };

    let mut touched: Vec<String> = Vec::new();

    for change in changes {
        let path = match &change {
            NoteChange::Written(path) | NoteChange::Removed(path) => path.clone(),
        };
        let Ok(relative) = path.strip_prefix(&root) else {
            continue;
        };
        let id = NoteId::new(relative.to_string_lossy().to_string());

        match change {
            NoteChange::Written(_) => state.search.index_note_best_effort(&root, &id),
            NoteChange::Removed(_) => state.search.remove_note_best_effort(&id),
        }
        touched.push(id.as_str().to_string());
    }

    if touched.is_empty() {
        return;
    }

    // Bring the cached list back in line before telling the window, so that
    // whatever it does next sees the new state.
    if let Err(err) = state.notes.rescan() {
        tracing::warn!(target: "files", error = %err, "rescan after an external change failed");
    }

    tracing::info!(target: "files", count = touched.len(), "notes changed on disk");
    let _ = app.emit("notes-changed", touched);
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
