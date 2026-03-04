use std::path::PathBuf;

use agtxd::session::{SessionManager, SessionOutput, SessionState, SpawnRequest};

/// Test 1: SessionOutput::new creates file at specified path and initializes empty ring buffer
#[tokio::test]
async fn test_session_output_new_creates_file_and_empty_ring() {
    let tmp = tempfile::tempdir().unwrap();
    let log_path = tmp.path().join("session.log");

    let output = SessionOutput::new(&log_path).await.unwrap();

    // File should exist
    assert!(log_path.exists(), "Log file should be created");

    // Ring buffer should be empty
    assert!(
        output.tail().is_empty(),
        "Ring buffer should be empty initially"
    );

    // Total bytes should be zero
    assert_eq!(
        output.total_bytes(),
        0,
        "Total bytes should be zero initially"
    );
}

/// Test 2: SessionOutput::append writes bytes to both file and ring buffer; tail() returns the appended bytes
#[tokio::test]
async fn test_session_output_append_writes_to_file_and_ring() {
    let tmp = tempfile::tempdir().unwrap();
    let log_path = tmp.path().join("session.log");

    let mut output = SessionOutput::new(&log_path).await.unwrap();

    let data = b"hello world";
    output.append(data).await.unwrap();

    // Ring buffer should contain the data
    assert_eq!(
        output.tail(),
        data.to_vec(),
        "Ring buffer should contain appended data"
    );

    // File should contain the data
    let file_contents = tokio::fs::read(&log_path).await.unwrap();
    assert_eq!(
        file_contents,
        data.to_vec(),
        "File should contain appended data"
    );
}

/// Test 3: Ring buffer evicts oldest bytes when exceeding 64KB capacity; file retains all bytes
#[tokio::test]
async fn test_ring_buffer_evicts_oldest_when_exceeding_capacity() {
    let tmp = tempfile::tempdir().unwrap();
    let log_path = tmp.path().join("session.log");

    let mut output = SessionOutput::new(&log_path).await.unwrap();

    // Write 64KB of 'A' bytes
    let first_chunk = vec![b'A'; 65_536];
    output.append(&first_chunk).await.unwrap();

    // Write 1KB of 'B' bytes (should evict 1KB of 'A' bytes from the ring)
    let second_chunk = vec![b'B'; 1024];
    output.append(&second_chunk).await.unwrap();

    let tail = output.tail();

    // Ring buffer should be exactly 64KB
    assert_eq!(tail.len(), 65_536, "Ring buffer should be capped at 64KB");

    // First bytes in ring should be 'A' (the un-evicted ones)
    assert_eq!(tail[0], b'A', "First byte should be 'A' (oldest retained)");

    // Last 1024 bytes should be 'B'
    let last_1024 = &tail[tail.len() - 1024..];
    assert!(
        last_1024.iter().all(|&b| b == b'B'),
        "Last 1024 bytes should be 'B'"
    );

    // File should contain all bytes (64KB + 1KB)
    let file_contents = tokio::fs::read(&log_path).await.unwrap();
    assert_eq!(
        file_contents.len(),
        65_536 + 1024,
        "File should retain all bytes"
    );
}

/// Test 4: SessionOutput::total_bytes tracks cumulative byte count accurately
#[tokio::test]
async fn test_total_bytes_tracks_cumulative_count() {
    let tmp = tempfile::tempdir().unwrap();
    let log_path = tmp.path().join("session.log");

    let mut output = SessionOutput::new(&log_path).await.unwrap();

    output.append(b"hello").await.unwrap();
    assert_eq!(output.total_bytes(), 5);

    output.append(b" world").await.unwrap();
    assert_eq!(output.total_bytes(), 11);

    // Even when ring buffer wraps, total_bytes keeps counting
    let big_data = vec![0u8; 100_000];
    output.append(&big_data).await.unwrap();
    assert_eq!(output.total_bytes(), 100_011);
}

/// Test 5: SessionState enum has Spawning, Running, Exited variants with Display impl
#[test]
fn test_session_state_display() {
    assert_eq!(SessionState::Spawning.to_string(), "Spawning");
    assert_eq!(SessionState::Running.to_string(), "Running");
    assert_eq!(SessionState::Exited(0).to_string(), "Exited(0)");
    assert_eq!(SessionState::Exited(1).to_string(), "Exited(1)");
    assert_eq!(SessionState::Exited(-1).to_string(), "Exited(-1)");

    // Verify enum variants exist and are comparable
    assert_ne!(SessionState::Spawning, SessionState::Running);
    assert_ne!(SessionState::Running, SessionState::Exited(0));
    assert_eq!(SessionState::Exited(42), SessionState::Exited(42));
}

// ==========================================================================
// Task 2: SessionManager tests
// ==========================================================================

/// Helper to create a SpawnRequest for a shell command
fn shell_spawn_request(cmd: &str, tmp_dir: &std::path::Path) -> SpawnRequest {
    SpawnRequest {
        command: "sh".to_string(),
        args: vec!["-c".to_string(), cmd.to_string()],
        working_dir: tmp_dir.to_path_buf(),
        env: vec![],
        cols: 80,
        rows: 24,
    }
}

/// Test 1 (PTY-01): spawn() creates a session, returns UUID;
/// session appears in list(); get() returns SessionInfo with Running state and non-zero PID
#[tokio::test]
async fn test_spawn_creates_session_in_list_with_running_state() {
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join("sessions");
    let mgr = SessionManager::new(sessions_dir);

    let req = shell_spawn_request("sleep 10", tmp.path());
    let id = mgr.spawn(req).await.unwrap();

    // Session should appear in list
    let list = mgr.list().await;
    assert_eq!(list.len(), 1, "Should have exactly one session");
    assert_eq!(list[0].id, id, "Listed session ID should match");

    // get() should return Running state
    let info = mgr.get(id).await.expect("Session should exist");
    assert_eq!(info.state, SessionState::Running, "State should be Running");
    assert!(info.pid > 0, "PID should be non-zero");

    // Cleanup
    mgr.kill(id).await.unwrap();
}

/// Test 2 (PTY-02): spawn a process that prints known output;
/// wait briefly; read output via get_output() and verify it contains "hello"
#[tokio::test]
async fn test_spawn_captures_output() {
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join("sessions");
    let mgr = SessionManager::new(sessions_dir);

    let req = shell_spawn_request("echo hello && sleep 2", tmp.path());
    let id = mgr.spawn(req).await.unwrap();

    // Wait for output to be captured
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let output = mgr.get_output(id).await.expect("Should have output");
    let output_str = String::from_utf8_lossy(&output);
    assert!(
        output_str.contains("hello"),
        "Output should contain 'hello', got: {:?}",
        output_str
    );

    // Cleanup
    mgr.kill(id).await.unwrap();
}

/// Test 3 (PTY-03): spawn `cat` (reads stdin, echoes to stdout);
/// write "test input" via write(); wait briefly; verify output contains "test input"
#[tokio::test]
async fn test_write_sends_input_to_pty() {
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join("sessions");
    let mgr = SessionManager::new(sessions_dir);

    let req = shell_spawn_request("cat", tmp.path());
    let id = mgr.spawn(req).await.unwrap();

    // Give cat a moment to start
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // Write input
    mgr.write(id, "test input").await.unwrap();

    // Wait for echo
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let output = mgr.get_output(id).await.expect("Should have output");
    let output_str = String::from_utf8_lossy(&output);
    assert!(
        output_str.contains("test input"),
        "Output should contain 'test input', got: {:?}",
        output_str
    );

    // Cleanup
    mgr.kill(id).await.unwrap();
}

/// Test 4 (PTY-04): spawn a process; call resize(id, 40, 120);
/// verify no error (resize is fire-and-forget)
#[tokio::test]
async fn test_resize_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join("sessions");
    let mgr = SessionManager::new(sessions_dir);

    let req = shell_spawn_request("sleep 10", tmp.path());
    let id = mgr.spawn(req).await.unwrap();

    // Resize should succeed without error
    mgr.resize(id, 40, 120).await.unwrap();

    // Cleanup
    mgr.kill(id).await.unwrap();
}

/// Test 5 (PTY-06): spawn a process; verify session info contains correct PID
#[tokio::test]
async fn test_session_tracks_pid() {
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join("sessions");
    let mgr = SessionManager::new(sessions_dir);

    let req = shell_spawn_request("sleep 10", tmp.path());
    let id = mgr.spawn(req).await.unwrap();

    let info = mgr.get(id).await.expect("Session should exist");
    assert!(info.pid > 0, "PID should be non-zero, got: {}", info.pid);

    // Verify the PID belongs to a real process by checking /proc
    let proc_path = PathBuf::from(format!("/proc/{}", info.pid));
    assert!(
        proc_path.exists(),
        "Process with PID {} should exist in /proc",
        info.pid
    );

    // Cleanup
    mgr.kill(id).await.unwrap();
}

/// Test 6: kill() terminates a session; get() returns None (session removed)
#[tokio::test]
async fn test_kill_terminates_session() {
    let tmp = tempfile::tempdir().unwrap();
    let sessions_dir = tmp.path().join("sessions");
    let mgr = SessionManager::new(sessions_dir);

    let req = shell_spawn_request("sleep 10", tmp.path());
    let id = mgr.spawn(req).await.unwrap();

    let info = mgr.get(id).await.expect("Session should exist before kill");
    let pid = info.pid;

    // Kill the session
    mgr.kill(id).await.unwrap();

    // Session should be removed from the registry
    assert!(
        mgr.get(id).await.is_none(),
        "Session should not exist after kill"
    );

    // Process should no longer be running
    // Give a moment for process cleanup
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    let proc_path = PathBuf::from(format!("/proc/{}", pid));
    assert!(
        !proc_path.exists(),
        "Process {} should not exist after kill",
        pid
    );
}
