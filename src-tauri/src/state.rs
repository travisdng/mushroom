//! Managed application state.
//!
//! The only shared mutable state in the process. Later milestones add the
//! database connection, the notes cache, and the AI client here.

use std::path::PathBuf;
use std::sync::Mutex;

use crate::config::AppConfig;

pub struct AppState {
    pub data_dir: PathBuf,
    pub config: Mutex<AppConfig>,
}

impl AppState {
    pub fn new(data_dir: PathBuf, config: AppConfig) -> Self {
        Self {
            data_dir,
            config: Mutex::new(config),
        }
    }
}
