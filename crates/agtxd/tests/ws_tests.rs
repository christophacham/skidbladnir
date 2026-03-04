use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
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

/// Start an axum server on a random port and return the base URL.
async fn start_server(app: axum::Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://127.0.0.1:{}", addr.port());
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (base_url, handle)
}

/// Connect a WebSocket client to the given URL.
async fn ws_connect(
    url: &str,
) -> (
    futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) {
    let (stream, _response) = tokio_tungstenite::connect_async(url)
        .await
        .expect("WebSocket connect failed");
    stream.split()
}

/// Read messages until we find one matching a predicate, with timeout.
async fn read_until<F>(
    reader: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    deadline: Duration,
    pred: F,
) -> Option<Value>
where
    F: Fn(&Value) -> bool,
{
    let result = timeout(deadline, async {
        while let Some(Ok(msg)) = reader.next().await {
            if let Message::Text(text) = msg {
                if let Ok(val) = serde_json::from_str::<Value>(&text.to_string()) {
                    if pred(&val) {
                        return Some(val);
                    }
                }
            }
        }
        None
    })
    .await;
    result.unwrap_or(None)
}

#[tokio::test]
async fn ws_upgrade_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let app = build_test_app(tmp.path());
    let (base_url, _server) = start_server(app.clone()).await;

    let (id, _pid) = spawn_session(&app, "sleep 5").await;

    let ws_url = format!("{}/api/v1/sessions/{}/ws", base_url.replace("http", "ws"), id);
    let (_sink, mut stream) = ws_connect(&ws_url).await;

    // Should receive a "connected" message
    let msg = read_until(&mut stream, Duration::from_secs(5), |v| {
        v.get("type").and_then(|t| t.as_str()) == Some("connected")
    })
    .await;

    assert!(msg.is_some(), "Should receive connected message");
    let msg = msg.unwrap();
    assert_eq!(msg["session_id"].as_str().unwrap(), id);
    assert!(msg.get("total_bytes").is_some());
}

#[tokio::test]
async fn ws_upgrade_404_missing_session() {
    let tmp = tempfile::tempdir().unwrap();
    let app = build_test_app(tmp.path());
    let (base_url, _server) = start_server(app).await;

    let fake_id = uuid::Uuid::new_v4();
    let ws_url = format!(
        "{}/api/v1/sessions/{}/ws",
        base_url.replace("http", "ws"),
        fake_id
    );

    // connect_async should fail because the server returns 404 before upgrade
    let result = tokio_tungstenite::connect_async(&ws_url).await;
    assert!(
        result.is_err(),
        "Should fail to connect to non-existent session"
    );
}

#[tokio::test]
async fn ws_receives_live_output() {
    let tmp = tempfile::tempdir().unwrap();
    let app = build_test_app(tmp.path());
    let (base_url, _server) = start_server(app.clone()).await;

    let (id, _pid) = spawn_session(&app, "echo test_output_xyz").await;

    let ws_url = format!("{}/api/v1/sessions/{}/ws", base_url.replace("http", "ws"), id);
    let (_sink, mut stream) = ws_connect(&ws_url).await;

    // Read messages until we find output containing "test_output_xyz"
    let msg = read_until(&mut stream, Duration::from_secs(5), |v| {
        if v.get("type").and_then(|t| t.as_str()) == Some("output") {
            if let Some(data_b64) = v.get("data").and_then(|d| d.as_str()) {
                use base64::Engine;
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(data_b64) {
                    let text = String::from_utf8_lossy(&decoded);
                    return text.contains("test_output_xyz");
                }
            }
        }
        false
    })
    .await;

    assert!(
        msg.is_some(),
        "Should receive output containing test_output_xyz"
    );
}

#[tokio::test]
async fn ws_multiple_clients() {
    let tmp = tempfile::tempdir().unwrap();
    let app = build_test_app(tmp.path());
    let (base_url, _server) = start_server(app.clone()).await;

    // Spawn a session that waits then produces output
    let (id, _pid) = spawn_session(&app, "sleep 0.3 && echo multi_test_marker").await;

    let ws_url = format!("{}/api/v1/sessions/{}/ws", base_url.replace("http", "ws"), id);

    // Connect two clients
    let (_sink1, mut stream1) = ws_connect(&ws_url).await;
    let (_sink2, mut stream2) = ws_connect(&ws_url).await;

    let check_output = |v: &Value| -> bool {
        if v.get("type").and_then(|t| t.as_str()) == Some("output") {
            if let Some(data_b64) = v.get("data").and_then(|d| d.as_str()) {
                use base64::Engine;
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(data_b64) {
                    let text = String::from_utf8_lossy(&decoded);
                    return text.contains("multi_test_marker");
                }
            }
        }
        false
    };

    // Both clients should receive the output
    let (r1, r2) = tokio::join!(
        read_until(&mut stream1, Duration::from_secs(5), check_output),
        read_until(&mut stream2, Duration::from_secs(5), check_output),
    );

    assert!(r1.is_some(), "Client 1 should receive multi_test_marker");
    assert!(r2.is_some(), "Client 2 should receive multi_test_marker");
}

#[tokio::test]
async fn ws_write_input() {
    let tmp = tempfile::tempdir().unwrap();
    let app = build_test_app(tmp.path());
    let (base_url, _server) = start_server(app.clone()).await;

    // cat will echo back whatever we write
    let (id, _pid) = spawn_session(&app, "cat").await;

    let ws_url = format!("{}/api/v1/sessions/{}/ws", base_url.replace("http", "ws"), id);
    let (mut sink, mut stream) = ws_connect(&ws_url).await;

    // Wait for connected message first
    let _ = read_until(&mut stream, Duration::from_secs(3), |v| {
        v.get("type").and_then(|t| t.as_str()) == Some("connected")
    })
    .await;

    // Send a write message
    let write_msg = serde_json::json!({"type": "write", "input": "hello_ws_input\n"});
    sink.send(Message::Text(write_msg.to_string().into()))
        .await
        .unwrap();

    // cat echoes input, so we should see it in output
    let msg = read_until(&mut stream, Duration::from_secs(5), |v| {
        if v.get("type").and_then(|t| t.as_str()) == Some("output") {
            if let Some(data_b64) = v.get("data").and_then(|d| d.as_str()) {
                use base64::Engine;
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(data_b64) {
                    let text = String::from_utf8_lossy(&decoded);
                    return text.contains("hello_ws_input");
                }
            }
        }
        false
    })
    .await;

    assert!(
        msg.is_some(),
        "Should receive echoed input hello_ws_input"
    );
}

#[tokio::test]
async fn ws_cursor_reconnection() {
    let tmp = tempfile::tempdir().unwrap();
    let app = build_test_app(tmp.path());
    let (base_url, _server) = start_server(app.clone()).await;

    // Produce output in two phases
    let (id, _pid) =
        spawn_session(&app, "echo first_cursor_data && sleep 1 && echo second_cursor_data").await;

    let ws_url = format!("{}/api/v1/sessions/{}/ws", base_url.replace("http", "ws"), id);

    // First connection: read until we see "first_cursor_data"
    let (_sink1, mut stream1) = ws_connect(&ws_url).await;

    // Get the connected message to know total_bytes
    let connected = read_until(&mut stream1, Duration::from_secs(3), |v| {
        v.get("type").and_then(|t| t.as_str()) == Some("connected")
    })
    .await
    .expect("Should get connected message");

    let initial_total = connected["total_bytes"].as_u64().unwrap();

    // Read output until we see first_cursor_data
    let _ = read_until(&mut stream1, Duration::from_secs(3), |v| {
        if v.get("type").and_then(|t| t.as_str()) == Some("output") {
            if let Some(data_b64) = v.get("data").and_then(|d| d.as_str()) {
                use base64::Engine;
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(data_b64) {
                    return String::from_utf8_lossy(&decoded).contains("first_cursor_data");
                }
            }
        }
        false
    })
    .await;

    // Find the latest offset from the output messages -- we need total bytes after first output
    // Use the REST API to get current total_bytes
    tokio::time::sleep(Duration::from_millis(200)).await;
    let req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/sessions/{}", id))
        .body(Body::empty())
        .unwrap();
    let (_status, info) = json_response(&app, req).await;
    let cursor_offset = info["total_bytes"].as_u64().unwrap_or(initial_total);

    // Drop first connection
    drop(stream1);
    drop(_sink1);

    // Wait for second output to be produced
    tokio::time::sleep(Duration::from_millis(1200)).await;

    // Reconnect with cursor
    let ws_url_cursor = format!(
        "{}/api/v1/sessions/{}/ws?cursor={}",
        base_url.replace("http", "ws"),
        id,
        cursor_offset
    );
    let (_sink2, mut stream2) = ws_connect(&ws_url_cursor).await;

    // Collect all output messages (initial replay + connected)
    let mut found_second = false;
    let deadline = Duration::from_secs(5);
    let _ = timeout(deadline, async {
        while let Some(Ok(msg)) = stream2.next().await {
            if let Message::Text(text) = msg {
                if let Ok(val) = serde_json::from_str::<Value>(&text.to_string()) {
                    if val.get("type").and_then(|t| t.as_str()) == Some("output") {
                        if let Some(data_b64) = val.get("data").and_then(|d| d.as_str()) {
                            use base64::Engine;
                            if let Ok(decoded) =
                                base64::engine::general_purpose::STANDARD.decode(data_b64)
                            {
                                let text = String::from_utf8_lossy(&decoded);
                                if text.contains("second_cursor_data") {
                                    found_second = true;
                                    break;
                                }
                            }
                        }
                    }
                    if val.get("type").and_then(|t| t.as_str()) == Some("connected") {
                        // Keep going, look for output after connected
                    }
                }
            }
        }
    })
    .await;

    assert!(
        found_second,
        "Reconnected client with cursor should receive second_cursor_data"
    );
}

#[tokio::test]
async fn ws_state_change_on_exit() {
    let tmp = tempfile::tempdir().unwrap();
    let app = build_test_app(tmp.path());
    let (base_url, _server) = start_server(app.clone()).await;

    let (id, _pid) = spawn_session(&app, "echo done && exit 0").await;

    let ws_url = format!("{}/api/v1/sessions/{}/ws", base_url.replace("http", "ws"), id);
    let (_sink, mut stream) = ws_connect(&ws_url).await;

    // Should receive a "state" message with session_state containing "exited"
    let msg = read_until(&mut stream, Duration::from_secs(5), |v| {
        v.get("type").and_then(|t| t.as_str()) == Some("state")
    })
    .await;

    assert!(msg.is_some(), "Should receive state change message on exit");
    let msg = msg.unwrap();
    let state_str = msg["session_state"].as_str().unwrap_or("");
    assert!(
        state_str.contains("exited"),
        "session_state should indicate exited, got: {}",
        state_str
    );
}
