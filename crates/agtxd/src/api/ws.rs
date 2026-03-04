use serde::{Deserialize, Serialize};

/// Server -> Client messages sent over WebSocket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// PTY output bytes (base64-encoded).
    #[serde(rename = "output")]
    Output {
        /// Base64-encoded PTY output bytes.
        data: String,
        /// Byte offset in the session log file where this chunk starts.
        offset: u64,
    },
    /// Session state change notification.
    #[serde(rename = "state")]
    State {
        /// Current state: "running", "exited", "spawning".
        session_state: String,
        /// Exit code if state is "exited".
        #[serde(skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
    },
    /// Resource metrics update.
    #[serde(rename = "metrics")]
    Metrics {
        cpu_percent: f32,
        rss_bytes: u64,
        uptime_secs: f64,
    },
    /// Error notification.
    #[serde(rename = "error")]
    Error { message: String },
    /// Initial connection acknowledgment (sent immediately on WS connect).
    #[serde(rename = "connected")]
    Connected {
        session_id: String,
        /// Total bytes in the session log (client's initial cursor position).
        total_bytes: u64,
    },
}

/// Client -> Server messages received over WebSocket.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Write raw text to PTY stdin (no newline appended).
    #[serde(rename = "write")]
    Write { input: String },
    /// Resize PTY dimensions.
    #[serde(rename = "resize")]
    Resize { rows: u16, cols: u16 },
}
