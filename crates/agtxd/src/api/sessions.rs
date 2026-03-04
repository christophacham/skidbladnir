use std::path::PathBuf;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::session::{SessionInfo, SpawnRequest};
use crate::state::AppState;

/// Request body for creating a new session.
#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub working_dir: String,
    #[serde(default)]
    pub env: Option<Vec<(String, String)>>,
    #[serde(default = "default_cols")]
    pub cols: u16,
    #[serde(default = "default_rows")]
    pub rows: u16,
}

fn default_cols() -> u16 {
    80
}

fn default_rows() -> u16 {
    24
}

/// Request body for writing to a session.
#[derive(Debug, Deserialize)]
pub struct WriteRequest {
    pub input: String,
}

/// Request body for resizing a session.
#[derive(Debug, Deserialize)]
pub struct ResizeRequest {
    pub rows: u16,
    pub cols: u16,
}

/// Response body for session output (base64-encoded).
#[derive(Debug, Serialize)]
pub struct OutputResponse {
    pub data: String,
    pub total_bytes: u64,
}

/// Build the session API router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(spawn_session).get(list_sessions))
        .route("/{id}", get(get_session).delete(kill_session))
        .route("/{id}/write", post(write_to_session))
        .route("/{id}/resize", post(resize_session))
        .route("/{id}/interrupt", post(interrupt_session))
        .route("/{id}/kill", post(kill_session_process))
        .route("/{id}/output", get(get_session_output))
}

/// Parse a UUID from a path parameter, returning 400 on invalid UUID.
fn parse_uuid(id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(id).map_err(|_| AppError::BadRequest(format!("Invalid UUID: {}", id)))
}

/// Simple base64 encoding (standard alphabet with padding).
fn base64_encode(data: &[u8]) -> String {
    #[rustfmt::skip]
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let combined = (b0 << 16) | (b1 << 8) | b2;

        result.push(TABLE[((combined >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((combined >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(TABLE[((combined >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(TABLE[(combined & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// POST /api/v1/sessions - Spawn a new session
async fn spawn_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<(StatusCode, Json<SessionInfo>), AppError> {
    let spawn_req = SpawnRequest {
        command: req.command,
        args: req.args,
        working_dir: PathBuf::from(req.working_dir),
        env: req.env.unwrap_or_default(),
        cols: req.cols,
        rows: req.rows,
    };

    let id = state
        .session_manager
        .spawn(spawn_req)
        .await
        .map_err(AppError::from)?;

    let info = state
        .session_manager
        .get(id)
        .await
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Session spawned but not found")))?;

    Ok((StatusCode::CREATED, Json(info)))
}

/// GET /api/v1/sessions - List all sessions
async fn list_sessions(State(state): State<AppState>) -> Result<Json<Vec<SessionInfo>>, AppError> {
    let sessions = state.session_manager.list().await;
    Ok(Json(sessions))
}

/// GET /api/v1/sessions/{id} - Get session info
async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SessionInfo>, AppError> {
    let uuid = parse_uuid(&id)?;
    let info = state
        .session_manager
        .get(uuid)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", id)))?;
    Ok(Json(info))
}

/// DELETE /api/v1/sessions/{id} - Kill and remove a session
async fn kill_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let uuid = parse_uuid(&id)?;
    state
        .session_manager
        .kill(uuid)
        .await
        .map_err(|_| AppError::NotFound(format!("Session not found: {}", id)))?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/sessions/{id}/write - Write input to session
async fn write_to_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<WriteRequest>,
) -> Result<StatusCode, AppError> {
    let uuid = parse_uuid(&id)?;
    state
        .session_manager
        .write(uuid, &req.input)
        .await
        .map_err(|_| AppError::NotFound(format!("Session not found: {}", id)))?;
    Ok(StatusCode::OK)
}

/// POST /api/v1/sessions/{id}/resize - Resize session PTY
async fn resize_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<ResizeRequest>,
) -> Result<StatusCode, AppError> {
    let uuid = parse_uuid(&id)?;
    state
        .session_manager
        .resize(uuid, req.rows, req.cols)
        .await
        .map_err(|_| AppError::NotFound(format!("Session not found: {}", id)))?;
    Ok(StatusCode::OK)
}

/// POST /api/v1/sessions/{id}/interrupt - Send SIGINT to session
async fn interrupt_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let uuid = parse_uuid(&id)?;
    let info = state
        .session_manager
        .get(uuid)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", id)))?;

    // Send SIGINT to the process
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(info.pid as i32),
        nix::sys::signal::Signal::SIGINT,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to send SIGINT: {}", e)))?;

    Ok(StatusCode::OK)
}

/// POST /api/v1/sessions/{id}/kill - Send SIGKILL to session
async fn kill_session_process(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let uuid = parse_uuid(&id)?;
    let info = state
        .session_manager
        .get(uuid)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", id)))?;

    // Send SIGKILL to the process
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(info.pid as i32),
        nix::sys::signal::Signal::SIGKILL,
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Failed to send SIGKILL: {}", e)))?;

    Ok(StatusCode::OK)
}

/// GET /api/v1/sessions/{id}/output - Get session output (base64 encoded)
async fn get_session_output(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OutputResponse>, AppError> {
    let uuid = parse_uuid(&id)?;

    let output = state
        .session_manager
        .get_output(uuid)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", id)))?;

    let info = state
        .session_manager
        .get(uuid)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", id)))?;

    let encoded = base64_encode(&output);

    Ok(Json(OutputResponse {
        data: encoded,
        total_bytes: info.total_bytes,
    }))
}
