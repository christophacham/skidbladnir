use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use pty_process::OwnedWritePty;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio::task::JoinHandle;
use uuid::Uuid;

use super::metrics::MetricsSnapshot;
use super::output::SessionOutput;

/// Event broadcast to WebSocket subscribers.
#[derive(Debug, Clone)]
pub enum OutputEvent {
    /// New PTY output bytes with their offset in the session log.
    Data { bytes: Vec<u8>, offset: u64 },
    /// Session state changed (e.g., process exited).
    StateChange(SessionState),
}

/// State of a PTY session in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "exit_code")]
pub enum SessionState {
    /// Session is being set up (PTY allocated, process spawning).
    Spawning,
    /// Child process is running and attached to PTY.
    Running,
    /// Child process has exited with the given exit code.
    Exited(i32),
}

impl fmt::Display for SessionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionState::Spawning => write!(f, "Spawning"),
            SessionState::Running => write!(f, "Running"),
            SessionState::Exited(code) => write!(f, "Exited({})", code),
        }
    }
}

/// Request to spawn a new PTY session.
#[derive(Debug, Deserialize)]
pub struct SpawnRequest {
    /// Command to execute (e.g., "claude", "sh").
    pub command: String,
    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Vec<String>,
    /// Working directory for the spawned process.
    pub working_dir: PathBuf,
    /// Additional environment variables.
    #[serde(default)]
    pub env: Vec<(String, String)>,
    /// Terminal width in columns.
    #[serde(default = "default_cols")]
    pub cols: u16,
    /// Terminal height in rows.
    #[serde(default = "default_rows")]
    pub rows: u16,
}

fn default_cols() -> u16 {
    80
}

fn default_rows() -> u16 {
    24
}

/// API-facing session information (serializable).
#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    /// Unique session identifier.
    pub id: Uuid,
    /// PID of the child process.
    pub pid: u32,
    /// Current session state.
    pub state: SessionState,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// Total bytes of output captured.
    pub total_bytes: u64,
    /// Latest resource metrics (None if not yet collected).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<MetricsSnapshot>,
}

/// Internal session handle (not serializable).
/// Holds all resources for a running PTY session.
pub struct SessionHandle {
    /// Unique session identifier.
    pub id: Uuid,
    /// PID of the child process.
    pub pid: u32,
    /// Current session state.
    pub state: SessionState,
    /// The child process handle.
    pub child: tokio::process::Child,
    /// Write half of the PTY (behind Mutex for concurrent access with read lock on sessions map).
    pub write_pty: Mutex<OwnedWritePty>,
    /// Session output (ring buffer + append-only file).
    pub output: Arc<RwLock<SessionOutput>>,
    /// Handle to the reader task (reads PTY output).
    pub reader_handle: JoinHandle<()>,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// Latest cached metrics snapshot (updated by polling task).
    pub metrics: Arc<RwLock<Option<MetricsSnapshot>>>,
    /// Handle to the metrics polling task (aborted on session kill).
    pub metrics_handle: Option<JoinHandle<()>>,
    /// Broadcast sender for fan-out of output events to WebSocket subscribers.
    pub output_tx: broadcast::Sender<OutputEvent>,
    /// Path to the append-only log file (needed for reconnection/history reads).
    pub output_path: PathBuf,
}
