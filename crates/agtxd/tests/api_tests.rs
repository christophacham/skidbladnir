use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// Helper to build the app with temp databases
fn build_test_app(tmp_dir: &std::path::Path) -> axum::Router {
    let db_path = tmp_dir.join("test_project.db");
    let global_db_path = tmp_dir.join("test_global.db");

    // Initialize databases
    agtx_core::db::Database::open_at(&db_path).expect("project db");
    agtx_core::db::Database::open_global_at(&global_db_path).expect("global db");

    let state = agtxd::state::AppState::new(
        db_path,
        global_db_path,
        agtx_core::config::GlobalConfig::default(),
    );
    agtxd::api::api_router().with_state(state)
}

/// Helper to make a request and get the response body as JSON
async fn json_response(app: axum::Router, req: Request<Body>) -> (StatusCode, Value) {
    let response = app.oneshot(req).await.expect("request failed");
    let status = response.status();
    let body = response.into_body().collect().await.expect("body").to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

// === Health Endpoint ===

#[tokio::test]
async fn test_health_returns_200_with_status_uptime_version() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let req = Request::builder()
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let (status, json) = json_response(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["status"], "healthy");
    assert!(json["uptime_secs"].is_number(), "uptime_secs should be a number");
    assert!(json["version"].is_string(), "version should be a string");
    assert!(!json["version"].as_str().unwrap().is_empty(), "version should not be empty");
}

// === Task Endpoints ===

#[tokio::test]
async fn test_list_tasks_returns_empty_array() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let req = Request::builder()
        .uri("/api/v1/tasks")
        .body(Body::empty())
        .unwrap();

    let (status, json) = json_response(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json.is_array(), "should return array");
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_create_task_returns_201() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let body = serde_json::json!({
        "title": "test task",
        "agent": "claude",
        "project_id": "proj1"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let (status, json) = json_response(app, req).await;

    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(json["title"], "test task");
    assert_eq!(json["agent"], "claude");
    assert!(json["id"].is_string(), "should have an id");
    assert!(!json["id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_get_task_returns_200_for_existing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    // Create a task first
    let body = serde_json::json!({
        "title": "find me",
        "agent": "claude",
        "project_id": "proj1"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let (_, created) = json_response(app, req).await;
    let task_id = created["id"].as_str().unwrap();

    // Now fetch it
    let app2 = build_test_app(tmp.path());
    let req = Request::builder()
        .uri(format!("/api/v1/tasks/{}", task_id))
        .body(Body::empty())
        .unwrap();

    let (status, json) = json_response(app2, req).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["id"], task_id);
    assert_eq!(json["title"], "find me");
}

#[tokio::test]
async fn test_get_task_returns_404_for_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let req = Request::builder()
        .uri("/api/v1/tasks/nonexistent-id")
        .body(Body::empty())
        .unwrap();

    let (status, _json) = json_response(app, req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_task_returns_204_for_existing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    // Create a task
    let body = serde_json::json!({
        "title": "delete me",
        "agent": "claude",
        "project_id": "proj1"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/tasks")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let (_, created) = json_response(app, req).await;
    let task_id = created["id"].as_str().unwrap();

    // Delete it
    let app2 = build_test_app(tmp.path());
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/tasks/{}", task_id))
        .body(Body::empty())
        .unwrap();

    let response = app2.oneshot(req).await.expect("request failed");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_delete_task_returns_404_for_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let req = Request::builder()
        .method("DELETE")
        .uri("/api/v1/tasks/nonexistent-id")
        .body(Body::empty())
        .unwrap();

    let (status, _) = json_response(app, req).await;

    assert_eq!(status, StatusCode::NOT_FOUND);
}

// === Project Endpoints ===

#[tokio::test]
async fn test_list_projects_returns_empty_array() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let req = Request::builder()
        .uri("/api/v1/projects")
        .body(Body::empty())
        .unwrap();

    let (status, json) = json_response(app, req).await;

    assert_eq!(status, StatusCode::OK);
    assert!(json.is_array(), "should return array");
    assert_eq!(json.as_array().unwrap().len(), 0);
}

// === 404 for unknown routes ===

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let req = Request::builder()
        .uri("/nonexistent")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.expect("request failed");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
