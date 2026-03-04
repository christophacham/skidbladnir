use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use agtx_core::config::GlobalConfig;
use tokio::sync::RwLock;

/// Shared application state for all request handlers
#[derive(Clone)]
pub struct AppState {
    /// Path to the project database (task operations)
    pub db_path: PathBuf,
    /// Path to the global index database (project operations)
    pub global_db_path: PathBuf,
    /// Daemon start time (for uptime calculation)
    pub start_time: Instant,
    /// Live configuration (updated by config watcher)
    pub config: Arc<RwLock<GlobalConfig>>,
}

impl AppState {
    pub fn new(db_path: PathBuf, global_db_path: PathBuf, config: GlobalConfig) -> Self {
        Self {
            db_path,
            global_db_path,
            start_time: Instant::now(),
            config: Arc::new(RwLock::new(config)),
        }
    }
}
