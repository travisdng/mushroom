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
    /// When the process started doing our work, for the startup timing in the
    /// log. Taken as early as the state exists, which is the closest thing to
    /// "launch" that is observable from inside the process.
    pub started: std::time::Instant,
    /// Set once the window reports its first paint, so a second report cannot
    /// overwrite the first with a larger number.
    reported_ready: AtomicBool,
    /// The welcome note, when this launch created one. Taken by the window
    /// rather than pushed to it: the startup scan can finish before the
    /// WebView exists, and an event emitted then reaches nobody.
    first_run_note: Mutex<Option<String>>,
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
            started: std::time::Instant::now(),
            reported_ready: AtomicBool::new(false),
            first_run_note: Mutex::new(None),
        }
    }

    pub fn set_first_run_note(&self, id: String) {
        if let Ok(mut slot) = self.first_run_note.lock() {
            *slot = Some(id);
        }
    }

    /// Take the welcome note, if there is one. Clearing it as it goes is what
    /// makes this safe to call from more than one place, which it has to be:
    /// the window asks on mount and again when the scan reports in, because
    /// either can happen first.
    pub fn take_first_run_note(&self) -> Option<String> {
        self.first_run_note.lock().ok()?.take()
    }

    /// Claim the one-shot startup report. `true` the first time only.
    pub fn claim_ready_report(&self) -> bool {
        !self.reported_ready.swap(true, Ordering::Relaxed)
    }

    pub fn set_watching(&self, watching: bool) {
        self.watching.store(watching, Ordering::Relaxed);
    }

    pub fn is_watching(&self) -> bool {
        self.watching.load(Ordering::Relaxed)
    }
}
