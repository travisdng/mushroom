//! Managed application state.
//!
//! The only shared mutable state in the process.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::ai::service::AiService;
use crate::commands::ai_search::{InFlight, UsageLog};
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
    /// Questions currently running, so each can be cancelled by its id.
    pub in_flight: Arc<InFlight>,
    pub ai_usage: Arc<UsageLog>,
    /// Kept alive for the life of the process; dropping it stops watching.
    pub watcher: Mutex<Option<crate::notes::watcher::NotesWatcher>>,
    /// False when the watcher could not start. Diagnostics shows it, and the
    /// user falls back to F5 (R2.7).
    watching: AtomicBool,
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
            in_flight: Arc::new(InFlight::default()),
            ai_usage: Arc::new(UsageLog::default()),
            watcher: Mutex::new(None),
            watching: AtomicBool::new(false),
        }
    }

    pub fn set_watching(&self, watching: bool) {
        self.watching.store(watching, Ordering::Relaxed);
    }

    pub fn is_watching(&self) -> bool {
        self.watching.load(Ordering::Relaxed)
    }
}
