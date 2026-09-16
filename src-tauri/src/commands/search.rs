//! Search commands.

use crate::error::{AppError, AppErrorDto};
use crate::search::service::{IndexProgress, IndexStats, SearchResults};
use crate::state::AppState;

async fn blocking<T, F>(work: F) -> Result<T, AppErrorDto>
where
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|e| AppError::Internal(format!("background task failed: {e}")))?
        .map_err(AppErrorDto::from)
}

#[tauri::command]
pub async fn search_notes(
    state: tauri::State<'_, AppState>,
    query: String,
    folder: Option<String>,
    modified_after: Option<i64>,
    limit: Option<usize>,
) -> Result<SearchResults, AppErrorDto> {
    let search = state.search.clone();
    blocking(move || search.search(&query, folder, modified_after, limit.unwrap_or(50))).await
}

#[tauri::command]
pub fn get_index_stats(state: tauri::State<'_, AppState>) -> IndexStats {
    state.search.stats()
}

/// Rebuild the index from the Markdown. Never modifies a note.
#[tauri::command]
pub async fn rebuild_index(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<IndexProgress, AppErrorDto> {
    use tauri::Emitter;

    let search = state.search.clone();
    let notes = state.notes.clone();
    let root = notes.root().map_err(AppErrorDto::from)?;

    blocking(move || {
        let on_disk = notes.list(None)?;
        search.rebuild(&root, &on_disk, |progress| {
            let _ = app.emit("index-progress", progress);
        })
    })
    .await
}

#[tauri::command]
pub fn cancel_rebuild(state: tauri::State<'_, AppState>) {
    state.search.cancel_rebuild();
}
