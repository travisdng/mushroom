//! Note commands.
//!
//! Thin by design: validate, call one service method on a blocking thread,
//! return a DTO. Filesystem work must never run on Tauri's async threads.

use std::path::PathBuf;

use crate::error::{AppError, AppErrorDto};
use crate::notes::model::{FolderNode, NoteContent, NoteId, NoteMeta};
use crate::notes::service::{ImportReport, TrashEntry};
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
    blocking(move || notes.save(&NoteId::new(id), &body, expected_modified)).await
}

#[tauri::command]
pub async fn create_note(
    state: tauri::State<'_, AppState>,
    folder: String,
    title: String,
) -> Result<NoteMeta, AppErrorDto> {
    let notes = state.notes.clone();
    blocking(move || notes.create(&folder, &title)).await
}

#[tauri::command]
pub async fn rename_note(
    state: tauri::State<'_, AppState>,
    id: String,
    new_title: String,
) -> Result<NoteMeta, AppErrorDto> {
    let notes = state.notes.clone();
    blocking(move || notes.rename(&NoteId::new(id), &new_title)).await
}

#[tauri::command]
pub async fn move_note(
    state: tauri::State<'_, AppState>,
    id: String,
    new_folder: String,
) -> Result<NoteMeta, AppErrorDto> {
    let notes = state.notes.clone();
    blocking(move || notes.move_note(&NoteId::new(id), &new_folder)).await
}

#[tauri::command]
pub async fn delete_note(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<TrashEntry, AppErrorDto> {
    let notes = state.notes.clone();
    blocking(move || notes.delete(&NoteId::new(id))).await
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
    blocking(move || notes.delete_folder(&folder)).await
}

#[tauri::command]
pub async fn import_notes(
    state: tauri::State<'_, AppState>,
    paths: Vec<String>,
    target_folder: String,
) -> Result<ImportReport, AppErrorDto> {
    let notes = state.notes.clone();
    let sources: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    blocking(move || notes.import(&sources, &target_folder)).await
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
    blocking(move || notes.rescan()).await
}
