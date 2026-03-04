pub mod health;
pub mod projects;
pub mod sessions;
pub mod tasks;
pub mod workflow;
pub mod ws;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

/// Build the full API router
pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::handler))
        .route("/api/v1/sessions/{id}/ws", get(ws::ws_handler))
        .nest("/api/v1/tasks", tasks::router())
        .nest("/api/v1/projects", projects::router())
        .nest("/api/v1/sessions", sessions::router())
        .nest("/api/v1/workflow", workflow::router())
}
