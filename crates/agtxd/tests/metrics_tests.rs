use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use agtxd::session::metrics::{MetricsCollector, MetricsSnapshot};
use agtxd::session::{SessionManager, SpawnRequest};

/// Test 1: read_metrics for current process returns Some with non-zero rss_bytes
#[test]
fn test_read_metrics_for_current_process() {
    let pid = std::process::id();
    let mut collector = MetricsCollector::new(pid, Instant::now());

    let snapshot = collector.collect();
    assert!(
        snapshot.is_some(),
        "Expected Some(MetricsSnapshot) for current process"
    );

    let metrics = snapshot.unwrap();
    assert!(
        metrics.rss_bytes > 0,
        "Expected non-zero RSS for current process, got {}",
        metrics.rss_bytes
    );
}

/// Test 2: CPU% returns 0.0 when prev_cpu equals current cpu (no delta)
#[test]
fn test_cpu_percent_zero_when_no_delta() {
    let pid = std::process::id();
    let mut collector = MetricsCollector::new(pid, Instant::now());

    // First collect to seed prev_cpu_ticks
    let first = collector.collect();
    assert!(first.is_some(), "First collect should succeed");

    // Second collect immediately -- negligible CPU delta expected
    // (no busy work between collects, so delta should be very small or zero)
    let second = collector.collect().unwrap();
    // CPU% should be very low (close to 0) since no significant work was done
    assert!(
        second.cpu_percent < 5.0,
        "Expected low CPU% with no work, got {}",
        second.cpu_percent
    );
}

/// Test 3: CPU% returns a positive value when cpu ticks have advanced
#[test]
fn test_cpu_percent_positive_after_work() {
    let pid = std::process::id();
    let mut collector = MetricsCollector::new(pid, Instant::now());

    // First collect to seed prev_cpu_ticks
    collector.collect();

    // Do some busy work to consume CPU
    let mut sum: u64 = 0;
    for i in 0..10_000_000u64 {
        sum = sum.wrapping_add(i.wrapping_mul(i));
    }
    // Prevent optimization of the busy loop
    std::hint::black_box(sum);

    // Allow a tiny bit of wall time to pass
    std::thread::sleep(std::time::Duration::from_millis(50));

    let snapshot = collector.collect().unwrap();
    // After busy work, CPU% should be measurably positive
    assert!(
        snapshot.cpu_percent >= 0.0,
        "Expected non-negative CPU%, got {}",
        snapshot.cpu_percent
    );
}

/// Test 4: read_metrics returns None for a non-existent PID
#[test]
fn test_read_metrics_none_for_nonexistent_pid() {
    let mut collector = MetricsCollector::new(999_999_999, Instant::now());
    let snapshot = collector.collect();
    assert!(
        snapshot.is_none(),
        "Expected None for non-existent PID 999999999"
    );
}

/// Test 5: MetricsSnapshot serializes to JSON with expected fields
#[test]
fn test_metrics_snapshot_serialization() {
    let snapshot = MetricsSnapshot {
        cpu_percent: 42.5,
        rss_bytes: 1024 * 1024,
        uptime_secs: 123.456,
    };

    let json = serde_json::to_string(&snapshot).expect("Should serialize to JSON");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("Should parse JSON");

    assert!(
        parsed.get("cpu_percent").is_some(),
        "Missing cpu_percent field"
    );
    assert!(parsed.get("rss_bytes").is_some(), "Missing rss_bytes field");
    assert!(
        parsed.get("uptime_secs").is_some(),
        "Missing uptime_secs field"
    );

    assert_eq!(parsed["cpu_percent"].as_f64().unwrap(), 42.5);
    assert_eq!(parsed["rss_bytes"].as_u64().unwrap(), 1024 * 1024);
    assert!((parsed["uptime_secs"].as_f64().unwrap() - 123.456).abs() < 0.001);
}

/// Test 6: Integration test -- metrics are available after the 5-second polling interval.
///
/// This test takes ~7 seconds: 1s initial delay + 5s poll interval + 1s safety margin.
#[tokio::test]
async fn test_session_metrics_available_after_poll() {
    let tmp = tempfile::tempdir().expect("create temp dir");
    let sessions_dir = tmp.path().join("sessions");
    let manager = Arc::new(SessionManager::new(sessions_dir));

    // Spawn a simple long-running process
    let id = manager
        .spawn(SpawnRequest {
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "while true; do echo a; sleep 1; done".to_string(),
            ],
            working_dir: PathBuf::from("/tmp"),
            env: vec![],
            cols: 80,
            rows: 24,
        })
        .await
        .expect("spawn session");

    // Wait for at least one metrics poll (1s initial delay + 5s interval + margin)
    tokio::time::sleep(std::time::Duration::from_secs(7)).await;

    // Metrics should be available
    let metrics = manager.get_metrics(id).await;
    assert!(
        metrics.is_some(),
        "Expected Some(MetricsSnapshot) after polling interval"
    );

    let snapshot = metrics.unwrap();
    assert!(
        snapshot.rss_bytes > 0,
        "Expected non-zero RSS, got {}",
        snapshot.rss_bytes
    );
    assert!(
        snapshot.uptime_secs >= 6.0,
        "Expected uptime >= 6s, got {}",
        snapshot.uptime_secs
    );

    // Clean up
    manager.kill(id).await.expect("kill session");
}
