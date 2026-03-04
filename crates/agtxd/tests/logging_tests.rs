use std::io::BufRead;

use agtxd::logging;
use tracing_subscriber::EnvFilter;

#[test]
fn test_init_logging_creates_log_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let log_dir = tmp.path().join("logs");

    // Directory should not exist yet
    assert!(!log_dir.exists());

    let (_subscriber, _reload, _guard) = logging::build_logging(&log_dir, "info").unwrap();

    // After init, the log directory must exist
    assert!(
        log_dir.exists(),
        "init_logging should create the log directory"
    );
}

#[test]
fn test_logging_writes_json_to_file() {
    let tmp = tempfile::tempdir().unwrap();
    let log_dir = tmp.path().join("logs");

    let (subscriber, _reload, guard) = logging::build_logging(&log_dir, "info").unwrap();

    // Use with_default so we don't conflict with other tests
    tracing::subscriber::with_default(subscriber, || {
        tracing::info!(test_marker = "json_check", "hello from test");
    });

    // Drop the guard to flush the non-blocking writer
    drop(guard);

    // Find the log file in the directory
    let entries: Vec<_> = std::fs::read_dir(&log_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert!(
        !entries.is_empty(),
        "Expected at least one log file in {:?}",
        log_dir
    );

    // Read the log file and verify JSON content
    let log_file = &entries[0].path();
    let file = std::fs::File::open(log_file).unwrap();
    let reader = std::io::BufReader::new(file);
    let mut found_marker = false;
    for line in reader.lines() {
        let line = line.unwrap();
        if line.trim().is_empty() {
            continue;
        }
        // Each line should be valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&line).expect("Log line should be valid JSON");
        // Check for our test marker
        if let Some(fields) = parsed.get("fields") {
            if fields.get("test_marker") == Some(&serde_json::json!("json_check")) {
                found_marker = true;
            }
        }
    }
    assert!(
        found_marker,
        "Should find test_marker=json_check in JSON log output"
    );
}

#[test]
fn test_reload_handle_accepts_new_filter() {
    let tmp = tempfile::tempdir().unwrap();
    let log_dir = tmp.path().join("logs");

    let (_subscriber, reload_handle, _guard) = logging::build_logging(&log_dir, "info").unwrap();

    // Changing the filter should not panic
    let new_filter = EnvFilter::try_new("debug").unwrap();
    reload_handle
        .reload(new_filter)
        .expect("Reload handle should accept new EnvFilter");
}
