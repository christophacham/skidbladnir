use std::path::PathBuf;
use std::sync::Arc;

use agtx_core::config::GlobalConfig;
use tokio::sync::RwLock;

use crate::logging::ReloadHandle;

/// Watch the config file for changes and reload configuration dynamically.
///
/// - Detects file modifications using `notify::RecommendedWatcher`
/// - Debounces rapid changes (200ms window)
/// - Reloads log level via the tracing reload handle
/// - Updates shared config state for live reads by handlers
/// - Logs warnings for invalid config files and port/bind changes that require restart
pub async fn watch_config(
    _config_path: PathBuf,
    _shared_config: Arc<RwLock<GlobalConfig>>,
    _reload_handle: ReloadHandle,
) {
    todo!("watch_config not yet implemented")
}
