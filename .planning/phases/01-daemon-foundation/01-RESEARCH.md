# Phase 1: Daemon Foundation - Research

**Researched:** 2026-03-03
**Domain:** Rust HTTP daemon (axum), structured logging, config hot-reload, graceful shutdown
**Confidence:** HIGH

## Summary

This phase establishes `agtxd`, a standalone HTTP daemon binary, as part of a Cargo workspace alongside the existing `agtx` TUI binary. The core technologies are well-established in the Rust ecosystem: axum 0.8 for HTTP, tracing/tracing-subscriber/tracing-appender for structured logging, notify 8.x for file-watching config reload, and tokio signals for graceful shutdown. All are mature, widely used, and well-documented.

The most significant structural change is converting the existing single-crate project into a Cargo workspace with three members. This requires careful handling to keep the TUI binary compiling throughout. The daemon itself is straightforward -- axum 0.8 provides built-in graceful shutdown, tracing-subscriber supports multi-layer output (JSON to file, pretty to stderr), and the reload module enables dynamic log level changes. The `notify` crate handles config file watching with debouncing.

**Primary recommendation:** Start with the Cargo workspace restructuring (highest structural risk), then layer in the daemon binary with axum, logging, health endpoint, graceful shutdown, and config reload in that order.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- Cargo workspace with three members: `crates/agtx-core/` (shared library), `crates/agtxd/` (daemon binary), and root crate (existing TUI binary)
- Shared core logic (db, config, git, agent, skills) moves to `agtx-core`
- Both `agtx` (TUI) and `agtxd` (daemon) depend on `agtx-core`
- TUI binary must continue to compile and work throughout development
- Daemon binary name: `agtxd`
- File watcher using `notify` crate to detect changes to config.toml
- Reuse existing config file `~/.config/agtx/config.toml` -- add `[daemon]` section for port, bind address, log level, and daemon-specific settings
- Shared settings (default_agent, theme, agents) stay common between TUI and daemon
- Versioned path prefix: `/api/v1/...` (e.g., `/api/v1/tasks`, `/api/v1/projects`)
- Direct JSON responses -- no envelope wrapper (GET /tasks returns `[...tasks]`, GET /tasks/:id returns `{task}`)
- Health endpoint at `/health` (outside API prefix, standard for load balancers/monitoring)
- `tracing` + `tracing-appender` for structured logging
- JSON format for log files (machine-parseable), pretty/colored format to stderr (development-friendly)
- Log directory: `~/.local/share/agtx/logs/` (alongside existing database files, via `directories` crate)
- Daily log rotation via `tracing-appender`
- Default log level: `info`

### Claude's Discretion
- Which config values are safe to hot-reload vs require restart (structural settings like port/bind likely restart-only)
- Default port number for the daemon
- Error response format (simple JSON or RFC 7807)
- Graceful shutdown implementation details (tokio signal handlers, cleanup ordering)
- Non-blocking log writer implementation

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INFRA-01 | Daemon serves REST API endpoints for task and project CRUD via axum | Axum 0.8.8 Router, State extractor, JSON responses; workspace structure for shared db/models |
| INFRA-03 | Structured logging with tracing + tracing-appender (rotation, non-blocking writes) | tracing-subscriber multi-layer (JSON file + pretty stderr), tracing-appender daily rotation with non_blocking |
| INFRA-04 | Health check endpoint returns daemon status | `/health` route returning JSON with uptime, version, status |
| INFRA-05 | Daemon handles graceful shutdown on SIGTERM/SIGINT with active process cleanup | axum `with_graceful_shutdown` + tokio signal handlers (ctrl_c + SIGTERM) |
| INFRA-06 | Daemon reloads configuration changes without restart | notify 8.x file watcher + Arc<RwLock<Config>> + tracing reload::Layer for dynamic log levels |

</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.8.8 | HTTP framework | De facto Rust web framework, tokio-native, tower middleware ecosystem |
| tokio | 1.44 | Async runtime | Already in project deps with full features |
| tracing | 0.1 | Structured instrumentation | Standard Rust instrumentation facade, used by axum/tokio internally |
| tracing-subscriber | 0.3.20 | Log formatting/filtering | Official tracing subscriber with fmt, JSON, layer composition |
| tracing-appender | 0.2.4 | File appender with rotation | Official companion for file output, daily rotation, non-blocking writes |
| tower-http | 0.6.8 | HTTP middleware | TraceLayer, TimeoutLayer, CorsLayer -- standard axum middleware |
| notify | 8.2.0 | Filesystem watcher | Cross-platform file change detection, used by rust-analyzer, watchexec, zed |
| serde | 1.0 | Serialization | Already in project deps |
| serde_json | 1.0 | JSON serialization | Already in project deps |

### Supporting (already in project)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| anyhow | 1.0 | Error handling | All fallible functions, matches existing pattern |
| chrono | 0.4 | DateTime handling | Uptime calculation, timestamps |
| rusqlite | 0.34 | SQLite | Database operations (moves to agtx-core) |
| directories | 6.0 | Platform paths | Config/data/log directory resolution |
| toml | 0.8 | Config parsing | Config file deserialization |
| uuid | 1.16 | ID generation | Task/project IDs |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| notify 8.x | notify 7.x | 7.x works but 8.x is current stable with better debouncing; MSRV 1.85 is fine with our Rust 1.93 |
| tower-http TraceLayer | Manual logging middleware | TraceLayer provides request/response tracing out of the box with span propagation |
| Simple JSON errors | RFC 7807 problem_details crate | RFC 7807 is more formal but adds a dependency; simple JSON `{"error": "...", "status": 404}` is sufficient for v1 single-user tool |

**Installation (new deps for daemon):**
```bash
# These will be added to workspace dependencies and inherited by crates/agtxd
cargo add axum --features tracing
cargo add tower-http --features trace,timeout
cargo add tracing
cargo add tracing-subscriber --features fmt,json,env-filter,registry
cargo add tracing-appender
cargo add notify
```

## Architecture Patterns

### Recommended Project Structure
```
Cargo.toml                    # Workspace root (virtual or root package)
crates/
  agtx-core/
    Cargo.toml                # Shared library: db, config, models, git, agent, skills
    src/
      lib.rs                  # Re-exports modules
      config/mod.rs           # GlobalConfig, ProjectConfig, MergedConfig, DaemonConfig (new)
      db/
        mod.rs
        schema.rs
        models.rs
      git/
        mod.rs
        worktree.rs
        operations.rs
        provider.rs
      agent/
        mod.rs
        operations.rs
      tmux/
        mod.rs
        operations.rs
      skills.rs
  agtxd/
    Cargo.toml                # Daemon binary
    src/
      main.rs                 # Entry point, logging init, server startup
      api/
        mod.rs                # Route composition
        health.rs             # GET /health
        tasks.rs              # Task CRUD endpoints (skeleton)
        projects.rs           # Project endpoints (skeleton)
      state.rs                # AppState, shared state construction
      config_watcher.rs       # notify-based config reload
      shutdown.rs             # Graceful shutdown signal handling
      logging.rs              # Multi-layer tracing setup
src/
  main.rs                     # TUI entry point (unchanged behavior)
  lib.rs                      # Re-exports from agtx-core
  tui/                        # TUI-specific code stays here
    mod.rs
    app.rs
    ...
```

### Pattern 1: Axum Application State
**What:** Shared state via `Arc` with `FromRef` for sub-state extraction
**When to use:** Every handler needs access to database, config, or other shared resources
**Example:**
```rust
// Source: https://docs.rs/axum/latest/axum/extract/struct.State.html
use axum::extract::State;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<DaemonConfig>>,
    pub start_time: std::time::Instant,
}

async fn health(State(state): State<AppState>) -> axum::Json<serde_json::Value> {
    let uptime = state.start_time.elapsed().as_secs();
    axum::Json(serde_json::json!({
        "status": "healthy",
        "uptime_secs": uptime,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

let app = Router::new()
    .route("/health", get(health))
    .with_state(state);
```

### Pattern 2: Multi-Layer Logging
**What:** JSON to file + pretty to stderr, with dynamic log level reload
**When to use:** Daemon startup -- configure once, reload filter level at runtime
**Example:**
```rust
// Source: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/reload/
// Source: https://docs.rs/tracing-appender/latest/tracing_appender/
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, reload};

fn init_logging(log_dir: &Path, level: &str) -> (reload::Handle<EnvFilter, impl tracing::Subscriber + Send + Sync>, tracing_appender::non_blocking::WorkerGuard) {
    // File appender: daily rotation, JSON format, non-blocking
    let file_appender = tracing_appender::rolling::daily(log_dir, "agtxd.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Reloadable filter
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    let (filter_layer, reload_handle) = reload::Layer::new(filter);

    // Compose: registry + reloadable filter + JSON file layer + pretty stderr layer
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt::layer().json().with_writer(non_blocking))
        .with(fmt::layer().pretty().with_writer(std::io::stderr))
        .init();

    (reload_handle, guard)
}
```

### Pattern 3: Graceful Shutdown
**What:** tokio signal handlers for SIGTERM/SIGINT with axum's built-in support
**When to use:** Daemon main function
**Example:**
```rust
// Source: https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
use tokio::signal;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

// In main:
let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
```

### Pattern 4: Config File Watcher
**What:** notify-based watcher that detects config.toml changes and updates shared state
**When to use:** Background task started at daemon init
**Example:**
```rust
// Source: https://docs.rs/notify/latest/notify/
use notify::{RecommendedWatcher, RecursiveMode, Watcher, Event};
use std::sync::Arc;
use tokio::sync::RwLock;

async fn watch_config(
    config_path: PathBuf,
    shared_config: Arc<RwLock<DaemonConfig>>,
    log_reload_handle: reload::Handle<EnvFilter, impl tracing::Subscriber>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, _>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    let _ = tx.blocking_send(());
                }
            }
        },
        notify::Config::default(),
    ).expect("failed to create file watcher");

    watcher.watch(&config_path, RecursiveMode::NonRecursive)
        .expect("failed to watch config file");

    while rx.recv().await.is_some() {
        // Debounce: small delay to batch rapid writes
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        match reload_config(&config_path).await {
            Ok(new_config) => {
                // Update log level if changed
                if let Ok(new_filter) = EnvFilter::try_new(&new_config.log_level) {
                    let _ = log_reload_handle.reload(new_filter);
                }
                *shared_config.write().await = new_config;
                tracing::info!("Configuration reloaded");
            }
            Err(e) => {
                tracing::warn!("Failed to reload config: {}", e);
            }
        }
    }
}
```

### Pattern 5: Cargo Workspace with Dependency Inheritance
**What:** Workspace root declares shared dependencies, members inherit them
**When to use:** The workspace Cargo.toml setup
**Example:**
```toml
# Root Cargo.toml
[workspace]
members = ["crates/agtx-core", "crates/agtxd"]
resolver = "2"

[workspace.dependencies]
# Shared across workspace
tokio = { version = "1.44", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
anyhow = "1.0"
thiserror = "2.0"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.16", features = ["v4"] }
rusqlite = { version = "0.34", features = ["bundled"] }
directories = "6.0"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "json", "env-filter", "registry"] }
tracing-appender = "0.2"

# Root package (TUI binary)
[package]
name = "agtx"
version = "0.1.0"
edition = "2021"

[dependencies]
agtx-core = { path = "crates/agtx-core" }
tokio = { workspace = true }
# ... TUI-specific deps (ratatui, crossterm)
```

```toml
# crates/agtx-core/Cargo.toml
[package]
name = "agtx-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
anyhow = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
rusqlite = { workspace = true }
directories = { workspace = true }
tracing = { workspace = true }
```

```toml
# crates/agtxd/Cargo.toml
[package]
name = "agtxd"
version = "0.1.0"
edition = "2021"

[dependencies]
agtx-core = { path = "../agtx-core" }
axum = { version = "0.8", features = ["tracing"] }
tower-http = { version = "0.6", features = ["trace", "timeout"] }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
tracing-appender = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
notify = "8"
```

### Anti-Patterns to Avoid
- **Passing Database directly in State:** rusqlite `Connection` is not `Send`. Use a wrapper that opens connections per-request or use a connection pool pattern. Since this is SQLite for a single-user tool, opening a new connection per-request via a factory in state is simplest and safe.
- **Blocking the tokio runtime with SQLite:** rusqlite operations are synchronous. Use `tokio::task::spawn_blocking` for database calls, or accept the overhead for a single-user tool where query latency is negligible.
- **Holding RwLock across await points:** Use `tokio::sync::RwLock` (not `std::sync::RwLock`) for the shared config since the lock may be held across `.await` points in handlers.
- **Forgetting the WorkerGuard:** The non-blocking writer's `WorkerGuard` must be kept alive (not dropped) for the duration of the program. Store it in main's scope or in the app state.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTTP routing/serving | Custom TCP listener + parser | axum 0.8 Router + serve | HTTP/1.1 compliance, connection management, middleware stack |
| Request tracing | Manual request logging middleware | tower-http TraceLayer | Span propagation, latency tracking, status codes, all automatic |
| Log rotation | Custom file rotation logic | tracing-appender RollingFileAppender | Thread-safe, handles midnight rollover, atomic file operations |
| Non-blocking writes | Manual background writer thread | tracing-appender non_blocking | WorkerGuard flush-on-drop, bounded channel, backpressure handling |
| File watching | Manual polling / inotify | notify 8.x RecommendedWatcher | Cross-platform (inotify/FSEvents/ReadDirectoryChanges), debouncing, event coalescing |
| Signal handling | Manual libc signal handlers | tokio::signal | Async-safe, no undefined behavior, integrates with select! |
| Request timeout | Manual timeout tracking | tower-http TimeoutLayer | Per-request timeout with proper status code, composes with graceful shutdown |

**Key insight:** Every item above has subtle edge cases (signal handler reentrancy, file rotation atomicity, non-blocking writer backpressure) that existing crates handle correctly and custom code almost always gets wrong.

## Common Pitfalls

### Pitfall 1: Workspace Restructuring Breaks TUI
**What goes wrong:** Moving modules from `src/` to `crates/agtx-core/` causes the existing TUI binary to stop compiling. Import paths change, `lib.rs` re-exports break, integration tests can't find modules.
**Why it happens:** Cargo workspace member crates have independent `lib.rs` roots. The root crate's `lib.rs` must re-export from `agtx-core` or the TUI code must update all `use` statements.
**How to avoid:** Keep root `src/lib.rs` as a thin re-export facade: `pub use agtx_core::*;` or selective re-exports. Run `cargo check` after every module move. Integration tests in `tests/` reference `agtx::` (the root crate name) which should continue to work if lib.rs re-exports.
**Warning signs:** `cargo test` fails after moving any module.

### Pitfall 2: rusqlite Connection Not Send
**What goes wrong:** Putting `Database` (which holds a `rusqlite::Connection`) directly in axum `State` fails to compile because `Connection` is `!Send`.
**Why it happens:** SQLite connections are not thread-safe by default. axum handlers must be `Send` because they run on the tokio runtime.
**How to avoid:** Store a factory/path in state and create `Database` instances per-request, or use `spawn_blocking`. For a single-user daemon, per-request connection creation is fine (SQLite open is fast with WAL mode). Alternatively, use `r2d2-sqlite` for connection pooling, but it's overkill for single-user.
**Warning signs:** Compile error about `Send` bound on handler future.

### Pitfall 3: WorkerGuard Dropped Too Early
**What goes wrong:** Log messages at shutdown are silently lost because the `WorkerGuard` from `non_blocking()` was dropped before the shutdown sequence completes.
**Why it happens:** If `guard` goes out of scope (e.g., stored in a variable that gets dropped when `main` transitions to the server loop), buffered log messages won't flush.
**How to avoid:** Declare `let _guard = ...;` at the top of `main()` so it lives for the entire program duration. The underscore-prefixed name prevents "unused variable" warnings while ensuring the guard is not dropped.
**Warning signs:** Missing log lines, especially near shutdown.

### Pitfall 4: Config File Watcher Event Storms
**What goes wrong:** Text editors save files by writing to a temp file and renaming, generating multiple events (create, modify, rename, modify). The watcher fires the reload handler multiple times.
**Why it happens:** Different editors use different save strategies. `vim` writes to a swap file then renames. VS Code may write directly.
**How to avoid:** Debounce events: when a change is detected, wait 100-200ms before reading the file. If more events arrive during the debounce window, reset the timer. The `notify-debouncer-mini` or `notify-debouncer-full` crates handle this, but a simple `tokio::time::sleep` after receiving the first event with a channel drain works fine.
**Warning signs:** "Configuration reloaded" appearing 3-5 times per save.

### Pitfall 5: Axum Path Parameter Syntax Changed in 0.8
**What goes wrong:** Using `/:id` path syntax (from axum 0.7 and earlier) causes 404s.
**Why it happens:** Axum 0.8 changed path parameter syntax from `/:single` to `/{single}` and `/*many` to `/{*many}`.
**How to avoid:** Always use the new syntax: `.route("/api/v1/tasks/{id}", get(get_task))`.
**Warning signs:** Routes not matching, 404 on parameterized paths.

### Pitfall 6: EnvFilter vs LevelFilter for Reload
**What goes wrong:** Using `LevelFilter` with the reload layer means you can only change the global level. Per-target filtering (e.g., `agtxd=debug,tower_http=info`) is lost.
**Why it happens:** `LevelFilter` is a simple level gate. `EnvFilter` supports directive-based filtering.
**How to avoid:** Wrap `EnvFilter` in the reload layer, not `LevelFilter`. This allows reloading the full filter string from config, supporting per-target levels.
**Warning signs:** Unable to set different levels for different modules after reload.

## Code Examples

Verified patterns from official sources:

### DaemonConfig Addition to Existing Config
```rust
// In agtx-core config module -- extends existing GlobalConfig
// Source: Existing src/config/mod.rs pattern with serde defaults

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_bind")]
    pub bind: String,

    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            bind: default_bind(),
            log_level: default_log_level(),
        }
    }
}

fn default_port() -> u16 { 3742 }       // "agtx" on phone keypad: 2489, or pick a memorable port
fn default_bind() -> String { "127.0.0.1".to_string() }
fn default_log_level() -> String { "info".to_string() }

// Add to GlobalConfig:
// #[serde(default)]
// pub daemon: DaemonConfig,
```

### Axum Router Composition
```rust
// Source: https://docs.rs/axum/latest/axum/struct.Router.html
use axum::{Router, routing::get};

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::handler))
        .nest("/api/v1", v1_router())
}

fn v1_router() -> Router<AppState> {
    Router::new()
        .nest("/tasks", tasks::router())
        .nest("/projects", projects::router())
}

// In tasks.rs:
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_tasks).post(create_task))
        .route("/{id}", get(get_task).put(update_task).delete(delete_task))
}
```

### Database Access Pattern for Axum
```rust
// SQLite Connection per-request via spawn_blocking
use agtx_core::db::Database;
use std::path::PathBuf;

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,         // Path to project database
    pub global_db_path: PathBuf,  // Path to global index database
    // ... other fields
}

async fn list_tasks(State(state): State<AppState>) -> Result<Json<Vec<Task>>, AppError> {
    let db_path = state.db_path.clone();
    let tasks = tokio::task::spawn_blocking(move || {
        let db = Database::open_at(&db_path)?;
        db.get_all_tasks()
    })
    .await
    .context("task panicked")?
    .context("database error")?;

    Ok(Json(tasks))
}
```

## Discretion Recommendations

### Default Port: 3742
Recommendation: **3742**. Not a well-known port, memorable (think "37" for 3+7=10 phases, "42" for the answer), unlikely to conflict with common dev tools. Falls in the registered port range (1024-49151).

### Error Response Format: Simple JSON
Recommendation: **Simple JSON** for v1. RFC 7807 adds a dependency and cognitive overhead for a single-user tool. Use a consistent structure:
```json
{"error": "Task not found", "status": 404}
```
If needed later, RFC 7807 can be adopted without breaking clients by adding the `type` and `title` fields.

### Hot-Reload Scope
Recommendation: These config values are **safe to hot-reload**:
- `log_level` -- via tracing reload handle
- `default_agent` -- only affects new task creation
- `theme` -- cosmetic, no structural impact
- `agents` (phase overrides) -- only affects new task creation

These require **restart**:
- `daemon.port` -- TcpListener already bound
- `daemon.bind` -- TcpListener already bound

### Non-Blocking Log Writer
Recommendation: Use `tracing_appender::non_blocking()` which provides a bounded channel between the logging thread and the I/O thread. The `WorkerGuard` returned must be stored in main scope to ensure flush-on-drop.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| axum 0.7 `/:param` | axum 0.8 `/{param}` | Jan 2025 | All route definitions use new syntax |
| axum 0.7 Option extractor | axum 0.8 OptionalFromRequestParts | Jan 2025 | Optional extractors require trait impl |
| notify 7.x | notify 8.x | Aug 2025 | New event API, better debouncing |
| tower-http 0.5 | tower-http 0.6 | 2025 | Updated for axum 0.8 compatibility |
| tracing-subscriber env-filter | Same, but reload::Layer improved | Stable | Dynamic filter reload without subscriber replacement |

**Deprecated/outdated:**
- `axum::Server` from hyper -- replaced by `axum::serve()` with `TcpListener` in axum 0.7+
- `hyper::Server::bind()` pattern -- replaced by `tokio::net::TcpListener::bind()`
- notify 5.x/6.x API -- significantly different from 7.x/8.x event model

## Open Questions

1. **Database open pattern for daemon**
   - What we know: Current `Database::open_project()` derives path from `ProjectDirs` and hashes the project path. Works for single-project TUI.
   - What's unclear: Daemon serves all projects. Need a way to open per-project databases on demand (by project path from the request).
   - Recommendation: Add a `Database::open_at(path: &Path)` method that skips the hash-path derivation and opens directly at a given path. The daemon can resolve project paths from the global index DB and cache open connections if needed.

2. **Workspace member discovery for tests**
   - What we know: Current integration tests in `tests/` use `use agtx::...`. After workspace conversion, they need to reference either `agtx` (root) or `agtx_core`.
   - What's unclear: Whether to keep tests at root or move them to per-crate `tests/` directories.
   - Recommendation: Move integration tests alongside their crate. Config tests go to `crates/agtx-core/tests/`, daemon tests to `crates/agtxd/tests/`. Root `tests/` can be kept for TUI-specific tests.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in, Rust 1.93) |
| Config file | None (cargo test uses Cargo.toml [dev-dependencies]) |
| Quick run command | `cargo test --workspace` |
| Full suite command | `cargo test --workspace --features test-mocks` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INFRA-01 | REST endpoints return correct JSON for tasks/projects | integration | `cargo test -p agtxd --test api_tests -- --nocapture` | No -- Wave 0 |
| INFRA-03 | Structured logging writes JSON to file, rotates daily | integration | `cargo test -p agtxd --test logging_tests -- --nocapture` | No -- Wave 0 |
| INFRA-04 | Health endpoint returns status, uptime, version | unit | `cargo test -p agtxd --test api_tests::health -- --nocapture` | No -- Wave 0 |
| INFRA-05 | Graceful shutdown completes in-flight requests | integration | `cargo test -p agtxd --test shutdown_tests -- --nocapture` | No -- Wave 0 |
| INFRA-06 | Config changes detected and applied without restart | integration | `cargo test -p agtxd --test config_reload_tests -- --nocapture` | No -- Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --workspace`
- **Per wave merge:** `cargo test --workspace --features test-mocks`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/agtxd/tests/api_tests.rs` -- covers INFRA-01, INFRA-04 (HTTP endpoint tests using axum test utilities)
- [ ] `crates/agtxd/tests/logging_tests.rs` -- covers INFRA-03 (verify JSON log output format)
- [ ] `crates/agtxd/tests/shutdown_tests.rs` -- covers INFRA-05 (verify graceful shutdown behavior)
- [ ] `crates/agtxd/tests/config_reload_tests.rs` -- covers INFRA-06 (verify config watcher triggers reload)
- [ ] Existing tests in root `tests/` must continue passing after workspace conversion

## Sources

### Primary (HIGH confidence)
- [axum 0.8.8 docs](https://docs.rs/axum/latest/axum/) - Router, State, serve, routing patterns
- [axum graceful-shutdown example](https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs) - Official shutdown pattern
- [tracing-appender 0.2.4 docs](https://docs.rs/tracing-appender/latest/tracing_appender/) - RollingFileAppender, non_blocking, WorkerGuard
- [tracing-subscriber reload module](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/reload/) - Dynamic filter reload
- [tracing-subscriber fmt JSON](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/format/struct.Json.html) - JSON format configuration
- [notify 8.2.0 docs](https://docs.rs/notify/latest/notify/) - RecommendedWatcher, Event, Config
- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html) - workspace.dependencies inheritance
- [axum 0.8.0 announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) - Breaking changes from 0.7

### Secondary (MEDIUM confidence)
- [tower-http 0.6.8 docs](https://docs.rs/crate/tower-http/latest) - TraceLayer, TimeoutLayer versions
- [Tokio graceful shutdown guide](https://tokio.rs/tokio/topics/shutdown) - General shutdown patterns

### Tertiary (LOW confidence)
- None -- all findings verified with primary sources

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries are official tokio-rs ecosystem crates or widely established (notify)
- Architecture: HIGH -- patterns directly from official axum examples and docs
- Pitfalls: HIGH -- based on known Rust type system constraints (Send bounds, drop semantics) and documented axum 0.8 breaking changes

**Research date:** 2026-03-03
**Valid until:** 2026-04-03 (30 days -- all libraries are stable releases)
