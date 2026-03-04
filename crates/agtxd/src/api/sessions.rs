use std::path::PathBuf;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use base64::Engine;

use crate::error::AppError;
use crate::session::{MetricsSnapshot, SessionInfo, SessionOutput, SpawnRequest};
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

/// Query parameters for the output endpoint.
#[derive(Debug, Deserialize)]
pub struct OutputQuery {
    /// Byte offset to start reading from (default: None = ring buffer mode).
    #[serde(default)]
    pub offset: Option<u64>,
    /// Maximum bytes to return (default: 65536).
    #[serde(default = "default_output_limit")]
    pub limit: Option<u64>,
}

fn default_output_limit() -> Option<u64> {
    Some(65_536)
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
        .route("/{id}/metrics", get(get_session_metrics))
}

/// Parse a UUID from a path parameter, returning 400 on invalid UUID.
fn parse_uuid(id: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(id).map_err(|_| AppError::BadRequest(format!("Invalid UUID: {}", id)))
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
///
/// Supports optional query parameters:
/// - `offset`: byte offset to start reading from (>0 reads from log file)
/// - `limit`: maximum bytes to return (default: 65536)
///
/// When offset is None or 0, returns the ring buffer tail (default behavior).
/// When offset > 0, reads from the append-only log file at that position.
async fn get_session_output(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<OutputQuery>,
) -> Result<Json<OutputResponse>, AppError> {
    let uuid = parse_uuid(&id)?;

    let info = state
        .session_manager
        .get(uuid)
        .await
        .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", id)))?;

    let limit = query.limit.unwrap_or(65_536) as usize;

    let output = match query.offset {
        Some(offset) if offset > 0 => {
            // Read from append-only log file at the given offset
            let path = state
                .session_manager
                .get_output_path(uuid)
                .await
                .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", id)))?;
            SessionOutput::read_range(&path, offset, limit)
                .await
                .map_err(AppError::Internal)?
        }
        _ => {
            // Default: return ring buffer tail
            state
                .session_manager
                .get_output(uuid)
                .await
                .ok_or_else(|| AppError::NotFound(format!("Session not found: {}", id)))?
        }
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(&output);

    Ok(Json(OutputResponse {
        data: encoded,
        total_bytes: info.total_bytes,
    }))
}

/// GET /api/v1/sessions/{id}/metrics - Get session resource metrics
async fn get_session_metrics(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MetricsSnapshot>, AppError> {
    let uuid = parse_uuid(&id)?;
    match state.session_manager.get_metrics(uuid).await {
        Some(metrics) => Ok(Json(metrics)),
        None => Err(AppError::NotFound(
            "Session metrics not available".to_string(),
        )),
    }
}
