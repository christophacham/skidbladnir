use std::path::Path;

use anyhow::Result;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;

/// Type alias for the reload handle used to change log levels at runtime
pub type ReloadHandle = reload::Handle<EnvFilter, tracing_subscriber::Registry>;

/// Initialize multi-layer logging with JSON file output and pretty stderr output.
///
/// Returns a reload handle for dynamic log level changes and a worker guard
/// that must be kept alive for the duration of the program.
pub fn init_logging(
    _log_dir: &Path,
    _level: &str,
) -> Result<(ReloadHandle, tracing_appender::non_blocking::WorkerGuard)> {
    todo!("init_logging not yet implemented")
}
