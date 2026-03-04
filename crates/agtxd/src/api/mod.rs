pub mod health;
pub mod projects;
pub mod tasks;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

/// Build the full API router
pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::handler))
        .nest("/api/v1/tasks", tasks::router())
        .nest("/api/v1/projects", projects::router())
}
