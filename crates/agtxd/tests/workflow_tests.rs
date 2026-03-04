//! Workflow engine integration tests (Wave 0 stubs)
//! These tests verify daemon-side workflow behavior for FLOW-01, FLOW-05, FLOW-06.

/// Helper to build the app with temp databases (same pattern as api_tests.rs)
#[allow(dead_code)]
fn build_test_app(tmp_dir: &std::path::Path) -> axum::Router {
    let db_path = tmp_dir.join("test_project.db");
    let global_db_path = tmp_dir.join("test_global.db");
    agtx_core::db::Database::open_at(&db_path).expect("project db");
    agtx_core::db::Database::open_global_at(&global_db_path).expect("global db");
    let sessions_dir = tmp_dir.join("sessions");
    let session_manager = std::sync::Arc::new(agtxd::session::SessionManager::new(sessions_dir));
    let state = agtxd::state::AppState::new(
        db_path,
        global_db_path,
        agtx_core::config::GlobalConfig::default(),
        session_manager,
    );
    agtxd::api::api_router().with_state(state)
}

// FLOW-01: Phase transitions trigger correct side effects
#[tokio::test]
async fn advance_task_backlog_to_planning_returns_200() {
    // Stub: POST /api/v1/workflow/tasks/{id}/advance with direction "next"
    // from Backlog should return 200 with updated task in Planning status.
    // Will be implemented after Task 1 creates WorkflowService.
}

#[tokio::test]
async fn advance_task_invalid_transition_returns_400() {
    // Stub: Advancing a Done task should return 400 (no valid next state).
}

#[tokio::test]
async fn advance_task_nonexistent_returns_404() {
    // Stub: Advancing a task ID that doesn't exist returns 404.
}

// FLOW-05: Artifact detection
#[tokio::test]
async fn artifact_polling_detects_completion_file() {
    // Stub: When a plugin artifact file exists in the worktree,
    // the artifact poller should detect it and mark the task as artifact-ready.
}

// FLOW-06: Cyclic phases
#[tokio::test]
async fn cyclic_advance_increments_cycle_counter() {
    // Stub: Advancing from Review with direction "cycle" when plugin.cyclic is true
    // should increment task.cycle and transition back to Planning.
}

#[tokio::test]
async fn cyclic_advance_rejected_when_not_cyclic() {
    // Stub: Advancing with direction "cycle" when plugin.cyclic is false
    // should return an error.
}
