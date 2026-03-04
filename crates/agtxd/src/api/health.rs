use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::state::AppState;

/// GET /health - Returns daemon health status
pub async fn handler(State(state): State<AppState>) -> Json<Value> {
    let uptime = state.start_time.elapsed().as_secs();

    Json(json!({
        "status": "healthy",
        "uptime_secs": uptime,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
