use std::path::PathBuf;
use std::sync::Arc;

use agtx_core::config::GlobalConfig;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

use crate::logging::ReloadHandle;

/// Watch the config file for changes and reload configuration dynamically.
///
/// - Detects file modifications using `notify::RecommendedWatcher`
/// - Watches the parent directory (handles editor delete+recreate patterns)
/// - Debounces rapid changes (200ms window)
/// - Reloads log level via the tracing reload handle
/// - Updates shared config state for live reads by handlers
/// - Logs warnings for invalid config files and port/bind changes that require restart
pub async fn watch_config(
    config_path: PathBuf,
    shared_config: Arc<RwLock<GlobalConfig>>,
    reload_handle: ReloadHandle,
) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    // Watch the parent directory to catch editor delete+recreate patterns
    let watch_dir = config_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_path_buf();

    let config_file_name = config_path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();

    let _watcher = {
        let tx = tx.clone();
        let config_file_name = config_file_name.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<notify::Event, notify::Error>| {
                if let Ok(event) = result {
                    // Only trigger on write/create events for our config file
                    let dominated = matches!(
                        event.kind,
                        EventKind::Modify(_) | EventKind::Create(_)
                    );
                    if !dominated {
                        return;
                    }
                    // Check if the event is for our config file
                    let is_our_file = event.paths.iter().any(|p| {
                        p.file_name()
                            .map(|n| n == config_file_name)
                            .unwrap_or(false)
                    });
                    if is_our_file {
                        let _ = tx.try_send(());
                    }
                }
            },
            notify::Config::default(),
        )
        .expect("Failed to create file watcher");

        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .expect("Failed to watch config directory");

        watcher // keep alive
    };

    tracing::info!("Config watcher started for {:?}", config_path);

    // Event processing loop
    while rx.recv().await.is_some() {
        // Debounce: wait 200ms and drain any additional events
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        while rx.try_recv().is_ok() {
            // drain queued events
        }

        // Read and parse the config file
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read config file: {}", e);
                continue;
            }
        };

        let new_config: GlobalConfig = match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to reload config: {}", e);
                continue;
            }
        };

        // Compare and apply changes
        let mut current = shared_config.write().await;

        // Check log level changes
        if new_config.daemon.log_level != current.daemon.log_level {
            match EnvFilter::try_new(&new_config.daemon.log_level) {
                Ok(filter) => {
                    if let Err(e) = reload_handle.reload(filter) {
                        tracing::warn!("Failed to reload log filter: {}", e);
                    } else {
                        tracing::info!(
                            "Log level changed from {} to {}",
                            current.daemon.log_level,
                            new_config.daemon.log_level
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Invalid log level '{}': {}", new_config.daemon.log_level, e);
                }
            }
        }

        // Check for port/bind changes (require restart)
        if new_config.daemon.port != current.daemon.port {
            tracing::warn!(
                "Port changed from {} to {} - restart required to take effect",
                current.daemon.port,
                new_config.daemon.port
            );
        }
        if new_config.daemon.bind != current.daemon.bind {
            tracing::warn!(
                "Bind address changed from {} to {} - restart required to take effect",
                current.daemon.bind,
                new_config.daemon.bind
            );
        }

        // Update the shared config
        *current = new_config;

        tracing::info!("Configuration reloaded");
    }
}
