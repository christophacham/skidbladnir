//! REST API endpoints for workflow operations.
//!
//! Provides advance, diff, PR creation, PR description generation,
//! plugin listing, and PR status endpoints.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use agtx_core::db::{Database, Task};
use agtx_core::git::{GitOperations, GitProviderOperations, RealGitHubOps, RealGitOps};
use agtx_core::skills;

use crate::state::AppState;

/// Build the workflow sub-router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/tasks/{id}/advance", post(advance_task))
        .route("/tasks/{id}/diff", get(get_task_diff))
        .route("/tasks/{id}/pr", post(create_pr))
        .route("/tasks/{id}/pr/generate", get(generate_pr_description))
        .route("/tasks/{id}/pr/status", get(get_pr_status))
        .route("/plugins", get(list_plugins))
}

// === Request/Response types ===

#[derive(Deserialize)]
struct AdvanceRequest {
    #[serde(default = "default_next")]
    direction: String,
}

fn default_next() -> String {
    "next".to_string()
}

#[derive(Serialize)]
struct AdvanceResponse {
    task: Task,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

#[derive(Serialize)]
struct DiffResponse {
    diff: String,
}

#[derive(Deserialize)]
struct CreatePrRequest {
    title: String,
    body: String,
    #[serde(default)]
    #[allow(dead_code)]
    base: Option<String>,
}

#[derive(Serialize)]
struct PrResponse {
    pr_number: i32,
    pr_url: String,
}

#[derive(Serialize)]
struct GeneratePrResponse {
    title: String,
    body: String,
}

#[derive(Serialize)]
struct PrStatusResponse {
    state: String,
}

#[derive(Serialize)]
struct PluginInfo {
    name: String,
    description: String,
    source: String,
}

// === Error helper ===

struct ApiError(StatusCode, String);

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = serde_json::json!({ "error": self.1 });
        (self.0, Json(body)).into_response()
    }
}

// === Handlers ===

/// POST /api/v1/workflow/tasks/{id}/advance
///
/// Advance a task to the next phase. Optionally pass `direction: "cycle"` for cyclic transitions.
async fn advance_task(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<AdvanceRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .workflow
        .advance_task(&id, &req.direction)
        .await
        .map_err(|e| {
            let msg = format!("{:#}", e);
            if msg.contains("not found") {
                ApiError(StatusCode::NOT_FOUND, msg)
            } else {
                ApiError(StatusCode::BAD_REQUEST, msg)
            }
        })?;

    Ok(Json(AdvanceResponse {
        task: result.task,
        warning: result.warning,
    }))
}

/// GET /api/v1/workflow/tasks/{id}/diff
///
/// Returns the raw unified diff for the task's worktree against main.
async fn get_task_diff(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let db_path = state.db_path.clone();

    let task = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<Task>> {
        let db = Database::open_at(&db_path)?;
        db.get_task(&id)
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    let worktree = task
        .worktree_path
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Task has no worktree".to_string()))?;

    let wt_path = std::path::PathBuf::from(&worktree);

    let diff = tokio::task::spawn_blocking(move || {
        let git = RealGitOps;

        // Main diff against base branch
        let main_diff = std::process::Command::new("git")
            .current_dir(&wt_path)
            .args(["diff", "main", "--unified=3"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        // Untracked file diffs
        let untracked = git.list_untracked_files(&wt_path);
        let mut untracked_diffs = String::new();
        for file in untracked.lines() {
            let file = file.trim();
            if !file.is_empty() {
                let d = git.diff_untracked_file(&wt_path, file);
                untracked_diffs.push_str(&d);
            }
        }

        format!("{}{}", main_diff, untracked_diffs)
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(DiffResponse { diff }))
}

/// POST /api/v1/workflow/tasks/{id}/pr
///
/// Create a GitHub PR for the task. Stages all changes, commits, pushes, and creates PR.
async fn create_pr(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<CreatePrRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let db_path = state.db_path.clone();
    let global_db_path = state.global_db_path.clone();
    let tid = id.clone();

    let (task, project_path) = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let db = Database::open_at(&db_path)?;
        let task = db
            .get_task(&tid)
            .ok()
            .flatten()
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        let global_db = Database::open_global_at(&global_db_path)?;
        let projects = global_db.get_all_projects()?;
        let project = projects.iter().find(|p| p.id == task.project_id).cloned();
        let project_path = project.map(|p| std::path::PathBuf::from(&p.path));

        Ok((task, project_path))
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;

    let worktree = task
        .worktree_path
        .as_ref()
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Task has no worktree".to_string()))?
        .clone();

    let branch = task
        .branch_name
        .as_ref()
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Task has no branch".to_string()))?
        .clone();

    let pp = project_path
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Project not found".to_string()))?;

    let title = req.title.clone();
    let body = req.body.clone();
    let wt = worktree.clone();
    let br = branch.clone();
    let task_title = task.title.clone();

    // Stage, commit, push, create PR
    let (pr_number, pr_url) =
        tokio::task::spawn_blocking(move || -> anyhow::Result<(i32, String)> {
            let git = RealGitOps;
            let wt_path = std::path::Path::new(&wt);

            // Stage all changes
            git.add_all(wt_path)?;

            // Commit if there are changes
            if git.has_changes(wt_path) {
                git.commit(wt_path, &task_title)?;
            }

            // Push branch
            git.push(wt_path, &br, true)?;

            // Create PR
            RealGitHubOps.create_pr(&pp, &title, &body, &br)
        })
        .await
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, format!("{:#}", e)))?;

    // Update task with PR info
    let db_path = state.db_path.clone();
    let tid = id.clone();
    let url = pr_url.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let db = Database::open_at(&db_path)?;
        if let Some(mut t) = db.get_task(&tid)? {
            t.pr_number = Some(pr_number);
            t.pr_url = Some(url);
            t.updated_at = chrono::Utc::now();
            db.update_task(&t)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PrResponse { pr_number, pr_url }))
}

/// GET /api/v1/workflow/tasks/{id}/pr/generate
///
/// Generate a PR title and body from the task's diff using the agent.
async fn generate_pr_description(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let db_path = state.db_path.clone();

    let task = tokio::task::spawn_blocking(move || -> anyhow::Result<Option<Task>> {
        let db = Database::open_at(&db_path)?;
        db.get_task(&id)
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .ok_or_else(|| ApiError(StatusCode::NOT_FOUND, "Task not found".to_string()))?;

    let worktree = task
        .worktree_path
        .as_ref()
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Task has no worktree".to_string()))?
        .clone();

    let agent_name = task.agent.clone();
    let wt = worktree.clone();

    // Get diff and generate description
    let (title, body) = tokio::task::spawn_blocking(move || -> (String, String) {
        let wt_path = std::path::Path::new(&wt);

        // Get diff from main
        let diff = std::process::Command::new("git")
            .current_dir(wt_path)
            .args(["diff", "main"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        if diff.is_empty() {
            return (String::new(), String::new());
        }

        // Try to generate using agent
        let agent = agtx_core::agent::get_agent(&agent_name);
        if let Some(agent) = agent {
            let prompt = format!(
                "Generate a concise PR title and description for these changes:\n\n{}",
                diff
            );
            // Use CodingAgent wrapper for generate_text capability
            let coding_agent = agtx_core::agent::CodingAgent::new(agent);
            use agtx_core::agent::AgentOperations;
            match coding_agent.generate_text(wt_path, &prompt) {
                Ok(text) => {
                    let mut lines = text.lines();
                    let title = lines.next().unwrap_or("").to_string();
                    let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
                    (title, body)
                }
                Err(_) => (String::new(), String::new()),
            }
        } else {
            (String::new(), String::new())
        }
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(GeneratePrResponse { title, body }))
}

/// GET /api/v1/workflow/tasks/{id}/pr/status
///
/// Get the current state of the task's PR.
async fn get_pr_status(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let db_path = state.db_path.clone();
    let global_db_path = state.global_db_path.clone();

    let (task, project_path) = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
        let db = Database::open_at(&db_path)?;
        let task = db
            .get_task(&id)
            .ok()
            .flatten()
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        let global_db = Database::open_global_at(&global_db_path)?;
        let projects = global_db.get_all_projects()?;
        let project = projects.iter().find(|p| p.id == task.project_id).cloned();
        let project_path = project.map(|p| std::path::PathBuf::from(&p.path));

        Ok((task, project_path))
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
    .map_err(|e| ApiError(StatusCode::NOT_FOUND, e.to_string()))?;

    let pr_number = task
        .pr_number
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Task has no PR".to_string()))?;

    let pp = project_path
        .ok_or_else(|| ApiError(StatusCode::BAD_REQUEST, "Project not found".to_string()))?;

    let state_str = tokio::task::spawn_blocking(move || -> String {
        match RealGitHubOps.get_pr_state(&pp, pr_number) {
            Ok(s) => match s {
                agtx_core::git::PullRequestState::Open => "open".to_string(),
                agtx_core::git::PullRequestState::Merged => "merged".to_string(),
                agtx_core::git::PullRequestState::Closed => "closed".to_string(),
                agtx_core::git::PullRequestState::Unknown => "unknown".to_string(),
            },
            Err(_) => "unknown".to_string(),
        }
    })
    .await
    .map_err(|e| ApiError(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PrStatusResponse { state: state_str }))
}

/// GET /api/v1/workflow/plugins
///
/// List all available workflow plugins (bundled + global + project-local).
async fn list_plugins() -> impl IntoResponse {
    let plugins = tokio::task::spawn_blocking(|| {
        let mut result: Vec<PluginInfo> = Vec::new();

        // Bundled plugins
        for (name, description, _content) in skills::BUNDLED_PLUGINS {
            result.push(PluginInfo {
                name: name.to_string(),
                description: description.to_string(),
                source: "bundled".to_string(),
            });
        }

        // Global plugins
        if let Ok(home) = std::env::var("HOME") {
            let global_dir = std::path::PathBuf::from(home)
                .join(".config")
                .join("agtx")
                .join("plugins");
            if let Ok(entries) = std::fs::read_dir(&global_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.join("plugin.toml").exists() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Skip if already in bundled list
                        if result.iter().any(|p| p.name == name) {
                            continue;
                        }
                        let description = agtx_core::config::WorkflowPlugin::load(&name, None)
                            .ok()
                            .and_then(|p| p.description.clone())
                            .unwrap_or_default();
                        result.push(PluginInfo {
                            name,
                            description,
                            source: "global".to_string(),
                        });
                    }
                }
            }
        }

        result
    })
    .await
    .unwrap_or_default();

    Json(plugins)
}
