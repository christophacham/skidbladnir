use agtxd::session::{SessionOutput, SessionState};

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
