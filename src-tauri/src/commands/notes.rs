//! Note commands.
//!
//! Thin by design: validate, call one service method on a blocking thread,
//! return a DTO. Filesystem work must never run on Tauri's async threads.

use std::path::PathBuf;

use crate::error::{AppError, AppErrorDto};
use crate::notes::model::{FolderNode, NoteContent, NoteId, NoteMeta};
use crate::notes::service::{ImportReport, TrashEntry};
use crate::search::service::SearchService;
use crate::state::AppState;

/// Run filesystem work off the async runtime, mapping a join failure to a
/// plain internal error rather than a panic.
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
pub async fn list_notes(
    state: tauri::State<'_, AppState>,
    folder: Option<String>,
) -> Result<Vec<NoteMeta>, AppErrorDto> {
    let notes = state.notes.clone();
    blocking(move || notes.list(folder.as_deref())).await
}

#[tauri::command]
pub async fn get_folder_tree(state: tauri::State<'_, AppState>) -> Result<FolderNode, AppErrorDto> {
    let notes = state.notes.clone();
    blocking(move || notes.folder_tree()).await
}

#[tauri::command]
pub async fn read_note(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<NoteContent, AppErrorDto> {
    let notes = state.notes.clone();
    blocking(move || notes.read(&NoteId::new(id))).await
}

#[tauri::command]
pub async fn save_note(
    state: tauri::State<'_, AppState>,
    id: String,
    body: String,
    expected_modified: Option<i64>,
) -> Result<NoteMeta, AppErrorDto> {
    let notes = state.notes.clone();
    let search = state.search.clone();
    blocking(move || {
        let meta = notes.save(&NoteId::new(id), &body, expected_modified)?;
        // Index only after the Markdown write succeeded, and never let an
        // index failure turn a successful save into an error (R2.7).
        reindex(&search, &notes, &meta.id);
        Ok(meta)
    })
    .await
}

/// Update the index for one note, best effort.
fn reindex(search: &SearchService, notes: &crate::notes::service::NotesService, id: &NoteId) {
    if let Ok(root) = notes.root() {
        search.index_note_best_effort(&root, id);
    }
}

#[tauri::command]
pub async fn create_note(
    state: tauri::State<'_, AppState>,
    folder: String,
    title: String,
) -> Result<NoteMeta, AppErrorDto> {
    let notes = state.notes.clone();
    let search = state.search.clone();
    blocking(move || {
        let meta = notes.create(&folder, &title)?;
        reindex(&search, &notes, &meta.id);
        Ok(meta)
    })
    .await
}

#[tauri::command]
pub async fn rename_note(
    state: tauri::State<'_, AppState>,
    id: String,
    new_title: String,
) -> Result<NoteMeta, AppErrorDto> {
    let notes = state.notes.clone();
    let search = state.search.clone();
    blocking(move || {
        let from = NoteId::new(id);
        let meta = notes.rename(&from, &new_title)?;
        if let Ok(root) = notes.root() {
            search.rename_note_best_effort(&root, &from, &meta.id);
        }
        Ok(meta)
    })
    .await
}

#[tauri::command]
pub async fn move_note(
    state: tauri::State<'_, AppState>,
    id: String,
    new_folder: String,
) -> Result<NoteMeta, AppErrorDto> {
    let notes = state.notes.clone();
    let search = state.search.clone();
    blocking(move || {
        let from = NoteId::new(id);
        let meta = notes.move_note(&from, &new_folder)?;
        if let Ok(root) = notes.root() {
            search.rename_note_best_effort(&root, &from, &meta.id);
        }
        Ok(meta)
    })
    .await
}

#[tauri::command]
pub async fn delete_note(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<TrashEntry, AppErrorDto> {
    let notes = state.notes.clone();
    let search = state.search.clone();
    blocking(move || {
        let id = NoteId::new(id);
        let entry = notes.delete(&id)?;
        search.remove_note_best_effort(&id);
        Ok(entry)
    })
    .await
}

#[tauri::command]
pub async fn create_folder(
    state: tauri::State<'_, AppState>,
    parent: String,
    name: String,
) -> Result<FolderNode, AppErrorDto> {
    let notes = state.notes.clone();
    blocking(move || notes.create_folder(&parent, &name)).await
}

#[tauri::command]
pub async fn delete_folder(
    state: tauri::State<'_, AppState>,
    folder: String,
) -> Result<usize, AppErrorDto> {
    let notes = state.notes.clone();
    let search = state.search.clone();
    blocking(move || {
        let affected = notes.delete_folder(&folder)?;
        // Whole-folder changes are easier to reconcile than to track
        // note-by-note.
        if let Ok(root) = notes.root() {
            let remaining = notes.list(None)?;
            let _ = search.reconcile(&root, &remaining);
        }
        Ok(affected)
    })
    .await
}

#[tauri::command]
pub async fn import_notes(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
    target_folder: String,
) -> Result<ImportReport, AppErrorDto> {
    let notes = state.notes.clone();
    let search = state.search.clone();
    let sources: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    blocking(move || {
        let report = notes.import(&sources, &target_folder)?;
        if let Ok(root) = notes.root() {
            let on_disk = notes.list(None)?;
            let _ = search.reconcile(&root, &on_disk);
        }
        Ok(report)
    })
    .await
}

#[tauri::command]
pub async fn export_notes(
    state: tauri::State<'_, AppState>,
    ids: Vec<String>,
    target_dir: String,
) -> Result<usize, AppErrorDto> {
    let notes = state.notes.clone();
    let ids: Vec<NoteId> = ids.into_iter().map(NoteId::new).collect();
    let target = PathBuf::from(target_dir);
    blocking(move || notes.export(&ids, &target)).await
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotesStatus {
    pub root: Option<String>,
    pub count: usize,
    pub skipped: usize,
    pub available: bool,
}

#[tauri::command]
pub fn get_notes_status(state: tauri::State<'_, AppState>) -> NotesStatus {
    let root = state.notes.root().ok();
    NotesStatus {
        available: root.as_ref().map(|r| r.is_dir()).unwrap_or(false),
        root: root.map(|r| r.to_string_lossy().to_string()),
        count: state.notes.count(),
        skipped: state.notes.skipped(),
    }
}

/// Re-walk the notes folder. Used after an import, or by an explicit refresh.
#[tauri::command]
pub async fn refresh_notes(state: tauri::State<'_, AppState>) -> Result<usize, AppErrorDto> {
    let notes = state.notes.clone();
    let search = state.search.clone();
    blocking(move || {
        let count = notes.rescan()?;
        if let Ok(root) = notes.root() {
            let on_disk = notes.list(None)?;
            let _ = search.reconcile(&root, &on_disk);
        }
        Ok(count)
    })
    .await
}
