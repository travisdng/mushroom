//! Managed application state.
//!
//! The only shared mutable state in the process.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::ai::service::AiService;
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
    pub ai: Arc<AiService>,
}

impl AppState {
    pub fn new(data_dir: PathBuf, config: AppConfig) -> Self {
        let ai = Arc::new(AiService::new(config.ai.clone()));
        Self {
            data_dir,
            config: Mutex::new(config),
            notes: Arc::new(NotesService::new()),
            search: Arc::new(SearchService::new()),
            ai,
        }
    }
}
