use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use base64::Engine;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::AppError;
use crate::session::{OutputEvent, SessionOutput, SessionState};
use crate::state::AppState;

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

/// Query parameters for WebSocket connections.
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    /// Byte offset cursor for reconnection. If provided, server sends delta from this offset.
    #[serde(default)]
    pub cursor: Option<u64>,
}

/// Axum handler: validate session and upgrade to WebSocket.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let uuid = Uuid::parse_str(&id)
        .map_err(|_| AppError::BadRequest(format!("Invalid UUID: {}", id)))?;

    // Validate session exists BEFORE upgrading
    let sub = state
        .session_manager
        .subscribe(uuid)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", id)))?;

    Ok(ws.on_upgrade(move |socket| handle_ws(socket, uuid, state, sub, query.cursor)))
}

/// Helper to send a ServerMessage as JSON over WebSocket.
async fn send_msg(socket: &mut WebSocket, msg: &ServerMessage) -> bool {
    match serde_json::to_string(msg) {
        Ok(json) => socket.send(Message::Text(json.into())).await.is_ok(),
        Err(_) => false,
    }
}

/// Core WebSocket connection handler.
///
/// Critical ordering per RESEARCH.md Pitfall 4:
/// Subscribe to broadcast BEFORE reading initial state to avoid missing output in the gap.
async fn handle_ws(
    mut socket: WebSocket,
    session_id: Uuid,
    state: AppState,
    sub: (
        tokio::sync::broadcast::Receiver<OutputEvent>,
        std::sync::Arc<tokio::sync::RwLock<SessionOutput>>,
        std::path::PathBuf,
    ),
    cursor: Option<u64>,
) {
    let (mut broadcast_rx, output_arc, output_path) = sub;

    // --- Initial state delivery ---
    let total_bytes = {
        let output = output_arc.read().await;
        output.total_bytes()
    };

    // Send initial data based on cursor
    match cursor {
        Some(offset) if offset > 0 => {
            // Cursor reconnection: send delta from offset
            if let Ok(data) = SessionOutput::read_range(&output_path, offset, 1_048_576).await {
                if !data.is_empty() {
                    let msg = ServerMessage::Output {
                        data: base64::engine::general_purpose::STANDARD.encode(&data),
                        offset,
                    };
                    if !send_msg(&mut socket, &msg).await {
                        return;
                    }
                }
            }
        }
        _ => {
            // No cursor or cursor=0: send ring buffer tail
            let tail = {
                let output = output_arc.read().await;
                output.tail()
            };
            if !tail.is_empty() {
                let offset = total_bytes.saturating_sub(tail.len() as u64);
                let msg = ServerMessage::Output {
                    data: base64::engine::general_purpose::STANDARD.encode(&tail),
                    offset,
                };
                if !send_msg(&mut socket, &msg).await {
                    return;
                }
            }
        }
    }

    // Send connected message with session_id and total_bytes
    let connected = ServerMessage::Connected {
        session_id: session_id.to_string(),
        total_bytes,
    };
    if !send_msg(&mut socket, &connected).await {
        return;
    }

    // Use an mpsc channel to funnel outbound messages from the broadcast task
    // to the socket writer, since WebSocket is not Clone/Split-friendly without futures-util.
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<String>(64);

    // --- Broadcast listener task: forward broadcast events to mpsc ---
    let bc_session_id = session_id;
    let broadcast_task = tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(event) => {
                    let msg = match event {
                        OutputEvent::Data { bytes, offset } => ServerMessage::Output {
                            data: base64::engine::general_purpose::STANDARD.encode(&bytes),
                            offset,
                        },
                        OutputEvent::StateChange(ss) => {
                            let (state_str, exit_code) = match ss {
                                SessionState::Exited(code) => {
                                    ("exited".to_string(), Some(code))
                                }
                                SessionState::Running => ("running".to_string(), None),
                                SessionState::Spawning => ("spawning".to_string(), None),
                            };
                            ServerMessage::State {
                                session_state: state_str,
                                exit_code,
                            }
                        }
                    };
                    if let Ok(json) = serde_json::to_string(&msg) {
                        if outbound_tx.send(json).await.is_err() {
                            break; // receiver dropped (client disconnected)
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        session_id = %bc_session_id,
                        missed = n,
                        "WebSocket client lagged behind broadcast"
                    );
                    let err_msg = ServerMessage::Error {
                        message: format!("Missed {} messages, may need to re-sync", n),
                    };
                    if let Ok(json) = serde_json::to_string(&err_msg) {
                        if outbound_tx.send(json).await.is_err() {
                            break;
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    });

    // --- Main loop: multiplex outbound messages and inbound client messages ---
    loop {
        tokio::select! {
            // Outbound: broadcast -> WebSocket
            Some(json) = outbound_rx.recv() => {
                if socket.send(Message::Text(json.into())).await.is_err() {
                    break; // client disconnected
                }
            }
            // Inbound: WebSocket -> PTY
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Write { input }) => {
                                if let Err(e) = state
                                    .session_manager
                                    .write_raw(session_id, input.as_bytes())
                                    .await
                                {
                                    tracing::warn!(
                                        session_id = %session_id,
                                        error = %e,
                                        "Failed to write to PTY"
                                    );
                                }
                            }
                            Ok(ClientMessage::Resize { rows, cols }) => {
                                if let Err(e) = state
                                    .session_manager
                                    .resize(session_id, rows, cols)
                                    .await
                                {
                                    tracing::warn!(
                                        session_id = %session_id,
                                        error = %e,
                                        "Failed to resize PTY"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::debug!(
                                    session_id = %session_id,
                                    error = %e,
                                    "Failed to parse client message"
                                );
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {} // Ping/Pong handled by axum automatically
                }
            }
        }
    }

    // Cleanup: abort the broadcast listener task
    broadcast_task.abort();
}
