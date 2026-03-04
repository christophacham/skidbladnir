use std::path::Path;

use anyhow::{Context, Result};
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload;
use tracing_subscriber::EnvFilter;

/// Type alias for the reload handle used to change log levels at runtime.
///
/// The generic parameters match the subscriber stack:
/// - `EnvFilter` is the reloadable filter layer
/// - `tracing_subscriber::Registry` is the base subscriber
pub type ReloadHandle = reload::Handle<EnvFilter, tracing_subscriber::Registry>;

/// Initialize multi-layer logging with JSON file output and pretty stderr output.
///
/// Creates a tracing subscriber with:
/// - A reloadable `EnvFilter` for dynamic log level changes
/// - A JSON-formatted layer writing to daily-rotating log files via `non_blocking`
/// - A pretty-printed layer writing to stderr for development
///
/// The log directory is created if it does not exist.
///
/// Returns a reload handle (for changing log level at runtime) and a worker guard
/// (must be kept alive for the entire program to ensure logs are flushed).
pub fn init_logging(
    log_dir: &Path,
    level: &str,
) -> Result<(ReloadHandle, tracing_appender::non_blocking::WorkerGuard)> {
    // Ensure the log directory exists
    std::fs::create_dir_all(log_dir)
        .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

    // Create daily-rotating file appender
    let file_appender = tracing_appender::rolling::daily(log_dir, "agtxd.log");

    // Wrap in non-blocking writer for async-safe, non-blocking log writes
    let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);

    // Create a reloadable env filter
    let env_filter =
        EnvFilter::try_new(level).with_context(|| format!("Invalid log level: {}", level))?;
    let (filter_layer, reload_handle) = reload::Layer::new(env_filter);

    // Build the subscriber with multiple layers:
    // 1. Reloadable filter (applied to all layers)
    // 2. JSON layer writing to non-blocking file writer
    // 3. Pretty layer writing to stderr
    let subscriber = tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt::layer().json().with_writer(non_blocking_writer))
        .with(fmt::layer().pretty().with_writer(std::io::stderr));

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set global tracing subscriber")?;

    Ok((reload_handle, guard))
}

/// Initialize logging for a specific log directory and level, returning the components
/// without setting a global default subscriber.
///
/// This is useful for tests where multiple init calls would conflict.
/// The caller must use `tracing::subscriber::with_default()` or similar
/// to install the subscriber for their scope.
pub fn build_logging(
    log_dir: &Path,
    level: &str,
) -> Result<(
    impl tracing::Subscriber + Send + Sync,
    ReloadHandle,
    tracing_appender::non_blocking::WorkerGuard,
)> {
    // Ensure the log directory exists
    std::fs::create_dir_all(log_dir)
        .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;

    // Create daily-rotating file appender
    let file_appender = tracing_appender::rolling::daily(log_dir, "agtxd.log");
    let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);

    // Create a reloadable env filter
    let env_filter =
        EnvFilter::try_new(level).with_context(|| format!("Invalid log level: {}", level))?;
    let (filter_layer, reload_handle) = reload::Layer::new(env_filter);

    let subscriber = tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt::layer().json().with_writer(non_blocking_writer))
        .with(fmt::layer().pretty().with_writer(std::io::stderr));

    Ok((subscriber, reload_handle, guard))
}
