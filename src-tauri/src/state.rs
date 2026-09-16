//! Managed application state.
//!
//! The only shared mutable state in the process. Later milestones add the
//! database connection, the notes cache, and the AI client here.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::notes::service::NotesService;
use crate::search::service::SearchService;

pub struct AppState {
    pub data_dir: PathBuf,
    pub config: Mutex<AppConfig>,
    /// Arc so a command can move a handle into a blocking task without
    /// borrowing from Tauri's managed state across an await point.
    pub notes: Arc<NotesService>,
    pub search: Arc<SearchService>,
}

impl AppState {
    pub fn new(data_dir: PathBuf, config: AppConfig) -> Self {
        Self {
            data_dir,
            config: Mutex::new(config),
            notes: Arc::new(NotesService::new()),
            search: Arc::new(SearchService::new()),
        }
    }
}
