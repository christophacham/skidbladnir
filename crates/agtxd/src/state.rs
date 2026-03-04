use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use agtx_core::config::GlobalConfig;
use tokio::sync::RwLock;

use crate::session::SessionManager;
use crate::workflow::WorkflowService;

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
    /// Session manager for PTY process lifecycle
    pub session_manager: Arc<SessionManager>,
    /// Workflow engine for phase transitions
    pub workflow: Arc<WorkflowService>,
}

impl AppState {
    pub fn new(
        db_path: PathBuf,
        global_db_path: PathBuf,
        config: GlobalConfig,
        session_manager: Arc<SessionManager>,
    ) -> Self {
        let workflow = Arc::new(WorkflowService::new(
            session_manager.clone(),
            db_path.clone(),
            global_db_path.clone(),
        ));
        Self {
            db_path,
            global_db_path,
            start_time: Instant::now(),
            config: Arc::new(RwLock::new(config)),
            session_manager,
            workflow,
        }
    }
}
