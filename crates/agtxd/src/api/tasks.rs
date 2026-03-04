use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;

use agtx_core::db::{Database, Task, TaskStatus};

use crate::error::AppError;
use crate::state::AppState;

/// Build the task routes
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/{id}", get(get_task).put(update_task).delete(delete_task))
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub agent: String,
    pub project_id: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub agent: Option<String>,
}

/// GET /api/v1/tasks - List all tasks
async fn list_tasks(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let db_path = state.db_path.clone();

    let tasks = tokio::task::spawn_blocking(move || {
        let db = Database::open_at(&db_path)?;
        db.get_all_tasks()
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Task join error: {}", e)))?
    .map_err(AppError::from)?;

    Ok(Json(serde_json::to_value(tasks).unwrap()))
}

/// POST /api/v1/tasks - Create a new task
async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> Result<impl IntoResponse, AppError> {
    let db_path = state.db_path.clone();

    let task = tokio::task::spawn_blocking(move || {
        let db = Database::open_at(&db_path)?;
        let mut task = Task::new(req.title, req.agent, req.project_id);
        task.description = req.description;
        db.create_task(&task)?;
        Ok::<Task, anyhow::Error>(task)
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Task join error: {}", e)))?
    .map_err(AppError::from)?;

    let json = serde_json::to_value(&task).unwrap();
    Ok((StatusCode::CREATED, Json(json)))
}

/// GET /api/v1/tasks/{id} - Get a task by ID
async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let db_path = state.db_path.clone();

    let task = tokio::task::spawn_blocking(move || {
        let db = Database::open_at(&db_path)?;
        db.get_task(&id)
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Task join error: {}", e)))?
    .map_err(AppError::from)?;

    match task {
        Some(t) => Ok(Json(serde_json::to_value(t).unwrap())),
        None => Err(AppError::NotFound("Task not found".into())),
    }
}

/// PUT /api/v1/tasks/{id} - Update a task
async fn update_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> Result<Json<Value>, AppError> {
    let db_path = state.db_path.clone();

    let task = tokio::task::spawn_blocking(move || {
        let db = Database::open_at(&db_path)?;
        let mut task = db
            .get_task(&id)?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        if let Some(title) = req.title {
            task.title = title;
        }
        if let Some(description) = req.description {
            task.description = Some(description);
        }
        if let Some(status_str) = req.status {
            if let Some(status) = TaskStatus::from_str(&status_str) {
                task.status = status;
            }
        }
        if let Some(agent) = req.agent {
            task.agent = agent;
        }
        task.updated_at = chrono::Utc::now();

        db.update_task(&task)?;
        Ok::<Task, anyhow::Error>(task)
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Task join error: {}", e)))?
    .map_err(|e: anyhow::Error| {
        if e.to_string().contains("not found") {
            AppError::NotFound("Task not found".into())
        } else {
            AppError::Internal(e)
        }
    })?;

    Ok(Json(serde_json::to_value(task).unwrap()))
}

/// DELETE /api/v1/tasks/{id} - Delete a task
async fn delete_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let db_path = state.db_path.clone();

    let existed = tokio::task::spawn_blocking(move || {
        let db = Database::open_at(&db_path)?;
        // Check if task exists first
        let task = db.get_task(&id)?;
        if task.is_some() {
            db.delete_task(&id)?;
            Ok::<bool, anyhow::Error>(true)
        } else {
            Ok(false)
        }
    })
    .await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("Task join error: {}", e)))?
    .map_err(AppError::from)?;

    if existed {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("Task not found".into()))
    }
}
