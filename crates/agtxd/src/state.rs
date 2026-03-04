use std::path::PathBuf;
use std::time::Instant;

/// Shared application state for all request handlers
#[derive(Clone)]
pub struct AppState {
    /// Path to the project database (task operations)
    pub db_path: PathBuf,
    /// Path to the global index database (project operations)
    pub global_db_path: PathBuf,
    /// Daemon start time (for uptime calculation)
    pub start_time: Instant,
}

impl AppState {
    pub fn new(db_path: PathBuf, global_db_path: PathBuf) -> Self {
        Self {
            db_path,
            global_db_path,
            start_time: Instant::now(),
        }
    }
}
