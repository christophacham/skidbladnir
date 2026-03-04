use std::sync::Arc;

use agtx_core::config::GlobalConfig;
use agtxd::config_watcher;
use agtxd::logging;
use tokio::sync::RwLock;

/// Helper: write a valid config.toml with a given log level
fn write_config(path: &std::path::Path, log_level: &str) {
    let content = format!(
        r#"
default_agent = "claude"

[daemon]
port = 3742
bind = "127.0.0.1"
log_level = "{}"
"#,
        log_level
    );
    std::fs::write(path, content).unwrap();
}

#[tokio::test]
async fn test_config_reload_updates_log_level() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");
    let log_dir = tmp.path().join("logs");

    // Write initial config
    write_config(&config_path, "info");

    // Build a subscriber for reload handle (not global)
    let (_subscriber, reload_handle, _guard) = logging::build_logging(&log_dir, "info").unwrap();

    let initial_config = GlobalConfig::default();
    let shared_config = Arc::new(RwLock::new(initial_config));

    // Spawn the watcher
    let watcher_config = shared_config.clone();
    let watcher_path = config_path.clone();
    let watcher_handle = tokio::spawn(async move {
        config_watcher::watch_config(watcher_path, watcher_config, reload_handle).await;
    });

    // Give the watcher time to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Change the log level in the config file
    write_config(&config_path, "debug");

    // Wait for the watcher to pick up the change (up to 3 seconds)
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let config = shared_config.read().await;
            if config.daemon.log_level == "debug" {
                break;
            }
            drop(config);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await;

    watcher_handle.abort();
    assert!(
        result.is_ok(),
        "Config should reload with new log_level within 3 seconds"
    );
}

#[tokio::test]
async fn test_invalid_config_retains_old_values() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");
    let log_dir = tmp.path().join("logs");

    // Write initial valid config
    write_config(&config_path, "info");

    let (_subscriber, reload_handle, _guard) = logging::build_logging(&log_dir, "info").unwrap();

    let mut initial_config = GlobalConfig::default();
    initial_config.daemon.log_level = "info".to_string();
    let shared_config = Arc::new(RwLock::new(initial_config));

    let watcher_config = shared_config.clone();
    let watcher_path = config_path.clone();
    let watcher_handle = tokio::spawn(async move {
        config_watcher::watch_config(watcher_path, watcher_config, reload_handle).await;
    });

    // Give the watcher time to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Write invalid TOML
    std::fs::write(&config_path, "this is not valid toml {{{{").unwrap();

    // Wait a bit for the watcher to attempt reload
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    // Config should retain old valid values
    let config = shared_config.read().await;
    assert_eq!(
        config.daemon.log_level, "info",
        "Invalid config should not change the stored values"
    );

    watcher_handle.abort();
}

#[tokio::test]
async fn test_port_change_updates_config() {
    let tmp = tempfile::tempdir().unwrap();
    let config_path = tmp.path().join("config.toml");
    let log_dir = tmp.path().join("logs");

    // Write initial config with default port
    write_config(&config_path, "info");

    let (_subscriber, reload_handle, _guard) = logging::build_logging(&log_dir, "info").unwrap();

    let shared_config = Arc::new(RwLock::new(GlobalConfig::default()));

    let watcher_config = shared_config.clone();
    let watcher_path = config_path.clone();
    let watcher_handle = tokio::spawn(async move {
        config_watcher::watch_config(watcher_path, watcher_config, reload_handle).await;
    });

    // Give the watcher time to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Write config with different port
    let content = r#"
default_agent = "claude"

[daemon]
port = 9999
bind = "127.0.0.1"
log_level = "info"
"#;
    std::fs::write(&config_path, content).unwrap();

    // Wait for the watcher to pick up the change
    let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let config = shared_config.read().await;
            if config.daemon.port == 9999 {
                break;
            }
            drop(config);
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    })
    .await;

    watcher_handle.abort();
    assert!(
        result.is_ok(),
        "Port change should be stored in shared config (even though restart is needed)"
    );
}
