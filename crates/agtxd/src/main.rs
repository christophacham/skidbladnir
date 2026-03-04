use std::path::PathBuf;

use anyhow::{Context, Result};
use tokio::net::TcpListener;

use agtx_core::config::GlobalConfig;
use agtx_core::db::Database;
use agtxd::api;
use agtxd::shutdown;
use agtxd::state::AppState;

const DEFAULT_PORT: u16 = 3742;
const DEFAULT_BIND: &str = "127.0.0.1";

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI args for optional --port and --bind overrides
    let args: Vec<String> = std::env::args().collect();
    let mut port = DEFAULT_PORT;
    let mut bind = DEFAULT_BIND.to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                i += 1;
                port = args
                    .get(i)
                    .context("--port requires a value")?
                    .parse()
                    .context("--port must be a number")?;
            }
            "--bind" => {
                i += 1;
                bind = args
                    .get(i)
                    .context("--bind requires a value")?
                    .clone();
            }
            _ => {}
        }
        i += 1;
    }

    // Determine database paths
    let data_dir = GlobalConfig::data_dir().unwrap_or_else(|_| PathBuf::from("."));

    // For the daemon, we use a default project database path
    // In the future, the daemon will serve multiple projects and route by project_id
    let db_path = data_dir.join("projects").join("daemon_default.db");
    let global_db_path = data_dir.join("index.db");

    // Ensure databases are initialized
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Database::open_at(&db_path).context("Failed to initialize project database")?;
    Database::open_global_at(&global_db_path).context("Failed to initialize global database")?;

    // Build application state and router
    let state = AppState::new(db_path, global_db_path);
    let app = api::api_router()
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    // Bind and serve
    let addr = format!("{}:{}", bind, port);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind to {}", addr))?;

    eprintln!("agtxd listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown::shutdown_signal())
        .await
        .context("Server error")?;

    eprintln!("agtxd shut down cleanly");
    Ok(())
}
