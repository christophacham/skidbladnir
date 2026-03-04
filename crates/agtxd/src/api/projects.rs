use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::Value;

use agtx_core::db::Database;

use crate::error::AppError;
use crate::state::AppState;

/// Build the project routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_projects))
        .route("/{id}", get(get_project))
}

/// GET /api/v1/projects - List all projects
async fn list_projects(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let global_db_path = state.global_db_path.clone();

    let projects = tokio::task::spawn_blocking(move || {
        let db = Database::open_global_at(&global_db_path)?;
        db.get_all_projects()
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Task join error: {}", e)))?
    .map_err(AppError::from)?;

    Ok(Json(serde_json::to_value(projects).unwrap()))
}

/// GET /api/v1/projects/{id} - Get a project by ID
async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let global_db_path = state.global_db_path.clone();

    let project = tokio::task::spawn_blocking(move || {
        let db = Database::open_global_at(&global_db_path)?;
        let projects = db.get_all_projects()?;
        Ok::<_, anyhow::Error>(projects.into_iter().find(|p| p.id == id))
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Task join error: {}", e)))?
    .map_err(AppError::from)?;

    match project {
        Some(p) => Ok(Json(serde_json::to_value(p).unwrap())),
        None => Err(AppError::NotFound("Project not found".into())),
    }
}
