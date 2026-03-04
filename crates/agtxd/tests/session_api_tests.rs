use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

/// Helper to build the app with temp databases and session manager
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

/// Helper to make a request and get the response body as JSON.
/// The Router is cloned so the original can be reused.
async fn json_response(app: &axum::Router, req: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(req).await.expect("request failed");
    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    (status, json)
}

/// Helper to make a request and get just the status code.
async fn status_response(app: &axum::Router, req: Request<Body>) -> StatusCode {
    let response = app.clone().oneshot(req).await.expect("request failed");
    response.status()
}

/// Helper to spawn a session via the API, returning the session id.
async fn spawn_session(app: &axum::Router, cmd: &str) -> (String, u32) {
    let body = serde_json::json!({
        "command": "sh",
        "args": ["-c", cmd],
        "working_dir": "/tmp"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sessions")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let (status, json) = json_response(app, req).await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "Spawn should return 201, got {}: {:?}",
        status,
        json
    );

    let id = json["id"].as_str().expect("should have id").to_string();
    let pid = json["pid"].as_u64().expect("should have pid") as u32;
    (id, pid)
}

/// Helper to kill a session (cleanup)
async fn kill_session(app: &axum::Router, session_id: &str) {
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/sessions/{}", session_id))
        .body(Body::empty())
        .unwrap();
    let _ = app.clone().oneshot(req).await;
}

// === Test 1: POST /api/v1/sessions with valid SpawnRequest returns 201 ===

#[tokio::test]
async fn test_session_api_spawn_returns_201_with_session_info() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let body = serde_json::json!({
        "command": "sh",
        "args": ["-c", "echo hello && sleep 5"],
        "working_dir": "/tmp"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/sessions")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let (status, json) = json_response(&app, req).await;

    assert_eq!(status, StatusCode::CREATED);
    assert!(json["id"].is_string(), "should have session id");
    assert!(json["pid"].is_number(), "should have pid");
    assert_eq!(json["state"]["type"], "Running");

    // Cleanup
    kill_session(&app, json["id"].as_str().unwrap()).await;
}

// === Test 2: GET /api/v1/sessions returns array ===

#[tokio::test]
async fn test_session_api_list_returns_array() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    // Empty list first
    let req = Request::builder()
        .uri("/api/v1/sessions")
        .body(Body::empty())
        .unwrap();

    let (status, json) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.is_array(), "should return array");
    assert_eq!(
        json.as_array().unwrap().len(),
        0,
        "should be empty initially"
    );

    // Spawn one, then check list
    let (session_id, _) = spawn_session(&app, "sleep 10").await;

    let req = Request::builder()
        .uri("/api/v1/sessions")
        .body(Body::empty())
        .unwrap();

    let (status, json) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 1, "should have one session");

    // Cleanup
    kill_session(&app, &session_id).await;
}

// === Test 3: GET /api/v1/sessions/{id} returns 200 or 404 ===

#[tokio::test]
async fn test_session_api_get_returns_200_for_existing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let (session_id, _) = spawn_session(&app, "sleep 10").await;

    let req = Request::builder()
        .uri(format!("/api/v1/sessions/{}", session_id))
        .body(Body::empty())
        .unwrap();

    let (status, json) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["id"].as_str().unwrap(), session_id);

    // Cleanup
    kill_session(&app, &session_id).await;
}

#[tokio::test]
async fn test_session_api_get_returns_404_for_missing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let fake_id = uuid::Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/v1/sessions/{}", fake_id))
        .body(Body::empty())
        .unwrap();

    let (status, _) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// === Test 4: POST /api/v1/sessions/{id}/write ===

#[tokio::test]
async fn test_session_api_write_returns_200() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let (session_id, _) = spawn_session(&app, "cat && sleep 5").await;

    // Give a moment for process to start
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let body = serde_json::json!({ "input": "hello" });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/sessions/{}/write", session_id))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let status = status_response(&app, req).await;
    assert_eq!(status, StatusCode::OK);

    // 404 for missing session
    let fake_id = uuid::Uuid::new_v4();
    let body = serde_json::json!({ "input": "hello" });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/sessions/{}/write", fake_id))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let (status, _) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Cleanup
    kill_session(&app, &session_id).await;
}

// === Test 5: POST /api/v1/sessions/{id}/resize ===

#[tokio::test]
async fn test_session_api_resize_returns_200() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let (session_id, _) = spawn_session(&app, "sleep 10").await;

    let body = serde_json::json!({ "rows": 40, "cols": 120 });
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/sessions/{}/resize", session_id))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let status = status_response(&app, req).await;
    assert_eq!(status, StatusCode::OK);

    // Cleanup
    kill_session(&app, &session_id).await;
}

// === Test 6: DELETE /api/v1/sessions/{id} kills session ===

#[tokio::test]
async fn test_session_api_delete_kills_session_returns_204() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let (session_id, _) = spawn_session(&app, "sleep 10").await;

    // First DELETE should return 204
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/sessions/{}", session_id))
        .body(Body::empty())
        .unwrap();

    let status = status_response(&app, req).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Second DELETE should return 404
    let req = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/sessions/{}", session_id))
        .body(Body::empty())
        .unwrap();

    let (status, _) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

// === Test 7: POST /api/v1/sessions/{id}/interrupt sends SIGINT ===

#[tokio::test]
async fn test_session_api_interrupt_returns_200() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let (session_id, _) = spawn_session(&app, "sleep 30").await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/sessions/{}/interrupt", session_id))
        .body(Body::empty())
        .unwrap();

    let status = status_response(&app, req).await;
    assert_eq!(status, StatusCode::OK);

    // 404 for missing session
    let fake_id = uuid::Uuid::new_v4();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/sessions/{}/interrupt", fake_id))
        .body(Body::empty())
        .unwrap();

    let (status, _) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Cleanup
    kill_session(&app, &session_id).await;
}

// === Test 8: POST /api/v1/sessions/{id}/kill sends SIGKILL ===

#[tokio::test]
async fn test_session_api_kill_process_returns_200() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let (session_id, _) = spawn_session(&app, "sleep 30").await;

    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/sessions/{}/kill", session_id))
        .body(Body::empty())
        .unwrap();

    let status = status_response(&app, req).await;
    assert_eq!(status, StatusCode::OK);

    // 404 for missing session
    let fake_id = uuid::Uuid::new_v4();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/sessions/{}/kill", fake_id))
        .body(Body::empty())
        .unwrap();

    let (status, _) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Cleanup -- session may already be dead from SIGKILL, but try anyway
    kill_session(&app, &session_id).await;
}

// === Test 9: GET /api/v1/sessions/{id}/output returns output data ===

#[tokio::test]
async fn test_session_api_output_returns_200_with_data() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let app = build_test_app(tmp.path());

    let (session_id, _) = spawn_session(&app, "echo hello && sleep 5").await;

    // Wait for output to be captured
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let req = Request::builder()
        .uri(format!("/api/v1/sessions/{}/output", session_id))
        .body(Body::empty())
        .unwrap();

    let (status, json) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(json["data"].is_string(), "should have data field");
    assert!(
        json["total_bytes"].is_number(),
        "should have total_bytes field"
    );

    // Decode base64 data and check it contains "hello"
    let data_b64 = json["data"].as_str().unwrap();
    let decoded_bytes = base64_decode(data_b64);
    let decoded = String::from_utf8_lossy(&decoded_bytes);
    assert!(
        decoded.contains("hello"),
        "Output should contain 'hello', got: {:?}",
        decoded
    );

    // 404 for missing session
    let fake_id = uuid::Uuid::new_v4();
    let req = Request::builder()
        .uri(format!("/api/v1/sessions/{}/output", fake_id))
        .body(Body::empty())
        .unwrap();

    let (status, _) = json_response(&app, req).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Cleanup
    kill_session(&app, &session_id).await;
}

/// Simple base64 decode helper (standard alphabet)
fn base64_decode(input: &str) -> Vec<u8> {
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let input = input.trim_end_matches('=');
    let mut output = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in input.as_bytes() {
        let val = match table.iter().position(|&b| b == byte) {
            Some(v) => v as u32,
            None => continue,
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    output
}
