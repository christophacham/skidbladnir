# Phase 1: Daemon Foundation - Context

**Gathered:** 2026-03-03
**Status:** Ready for planning

<domain>
## Phase Boundary

Axum daemon (`agtxd`) with REST API skeleton, structured logging, health checks, graceful shutdown, and hot config reload. This phase delivers the backend process foundation — no PTY management, no WebSocket streaming, no frontend.

Requirements covered: INFRA-01, INFRA-03, INFRA-04, INFRA-05, INFRA-06

</domain>

<decisions>
## Implementation Decisions

### Crate structure
- Cargo workspace with three members: `crates/agtx-core/` (shared library), `crates/agtxd/` (daemon binary), and root crate (existing TUI binary)
- Shared core logic (db, config, git, agent, skills) moves to `agtx-core`
- Both `agtx` (TUI) and `agtxd` (daemon) depend on `agtx-core`
- TUI binary must continue to compile and work throughout development
- Daemon binary name: `agtxd`

### Config reload
- File watcher using `notify` crate to detect changes to config.toml
- Reuse existing config file `~/.config/agtx/config.toml` — add `[daemon]` section for port, bind address, log level, and daemon-specific settings
- Shared settings (default_agent, theme, agents) stay common between TUI and daemon

### API conventions
- Versioned path prefix: `/api/v1/...` (e.g., `/api/v1/tasks`, `/api/v1/projects`)
- Direct JSON responses — no envelope wrapper (GET /tasks returns `[...tasks]`, GET /tasks/:id returns `{task}`)
- Health endpoint at `/health` (outside API prefix, standard for load balancers/monitoring)

### Logging
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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `GlobalConfig` / `ProjectConfig` / `MergedConfig` in `src/config/mod.rs` — config parsing and merging logic, reusable in daemon
- `Database` in `src/db/schema.rs` — SQLite operations for tasks and projects, directly reusable for REST API
- `Task`, `Project`, `TaskStatus` models in `src/db/models.rs` — data models with serde derive, ready for JSON serialization
- Trait-based DI pattern (`GitOperations`, `TmuxOperations`, etc.) — established pattern to follow for new daemon services
- `tokio` 1.44 already in dependencies with full features — async runtime ready

### Established Patterns
- `anyhow::Result` with `.context()` for error handling throughout
- Serde defaults pattern (`#[serde(default = "fn_name")]`) for backwards-compatible config
- `Arc<dyn Trait>` for injectable dependencies
- `#[cfg_attr(feature = "test-mocks", automock)]` for mockable traits
- UUID v4 strings for IDs, RFC3339 strings for datetimes in SQLite

### Integration Points
- Config file at `~/.config/agtx/config.toml` — daemon adds `[daemon]` section
- Database at `~/.local/share/agtx/` (Linux) — daemon serves same data the TUI uses
- `Cargo.toml` — transforms from single crate to workspace

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-daemon-foundation*
*Context gathered: 2026-03-03*
