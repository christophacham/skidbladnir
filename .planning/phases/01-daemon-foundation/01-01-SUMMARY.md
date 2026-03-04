---
phase: 01-daemon-foundation
plan: 01
subsystem: infra
tags: [axum, cargo-workspace, rest-api, daemon, sqlite, graceful-shutdown]

# Dependency graph
requires: []
provides:
  - Cargo workspace with agtx-core shared library crate
  - agtxd daemon binary serving REST API on port 3742
  - Health endpoint with uptime and version reporting
  - Task CRUD REST endpoints (list, create, get, update, delete)
  - Project listing REST endpoint
  - Graceful shutdown on SIGTERM/SIGINT
  - Database::open_at and open_global_at constructors for explicit paths
affects: [02-pty-management, 03-websocket-streaming, 04-frontend-kanban, 01-02-logging-config]

# Tech tracking
tech-stack:
  added: [axum 0.8, tower-http 0.6, http-body-util 0.1]
  patterns: [workspace dependency inheritance, re-export facade, spawn_blocking for sync DB, AppError into IntoResponse]

key-files:
  created:
    - crates/agtx-core/Cargo.toml
    - crates/agtx-core/src/lib.rs
    - crates/agtxd/Cargo.toml
    - crates/agtxd/src/main.rs
    - crates/agtxd/src/lib.rs
    - crates/agtxd/src/state.rs
    - crates/agtxd/src/shutdown.rs
    - crates/agtxd/src/error.rs
    - crates/agtxd/src/api/mod.rs
    - crates/agtxd/src/api/health.rs
    - crates/agtxd/src/api/tasks.rs
    - crates/agtxd/src/api/projects.rs
    - crates/agtxd/tests/api_tests.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/lib.rs
    - crates/agtx-core/src/db/schema.rs

key-decisions:
  - "Re-export facade pattern: src/lib.rs uses pub use agtx_core::* so all existing TUI code and tests work unchanged"
  - "Default daemon port 3742 with --port/--bind CLI overrides"
  - "Database::open_at for daemon use - avoids hash-path derivation, takes explicit path"
  - "agtxd has both lib.rs and main.rs to support integration tests via use agtxd::"

patterns-established:
  - "Workspace dependency inheritance: shared deps declared in [workspace.dependencies], referenced with workspace = true"
  - "AppError enum with IntoResponse impl: NotFound/BadRequest/Internal map to HTTP status codes with JSON error bodies"
  - "spawn_blocking pattern for rusqlite: all DB calls in tokio::task::spawn_blocking closures"
  - "AppState with clone-friendly paths: handlers clone PathBuf for spawn_blocking moves"

requirements-completed: [INFRA-01, INFRA-04, INFRA-05]

# Metrics
duration: 8min
completed: 2026-03-04
---

# Phase 1 Plan 01: Cargo Workspace + agtxd Daemon Summary

**Cargo workspace conversion with agtx-core shared library and axum REST daemon serving health, task CRUD, and project endpoints with graceful shutdown**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-04T07:54:11Z
- **Completed:** 2026-03-04T08:02:31Z
- **Tasks:** 2
- **Files modified:** 20 created/modified

## Accomplishments
- Converted single-crate project to 3-member Cargo workspace (agtx, agtx-core, agtxd) with all 35 existing tests passing unchanged
- Built working axum daemon binary that starts on port 3742, serves health/task/project REST endpoints, and shuts down cleanly on SIGTERM
- Established workspace-wide dependency inheritance and re-export facade pattern that maintains backward compatibility
- 9 integration tests covering all daemon API endpoints and error paths

## Task Commits

Each task was committed atomically:

1. **Task 1: Convert to Cargo workspace** - `0011736` (feat)
2. **Task 2: TDD RED - Failing API tests** - `10ece2d` (test)
3. **Task 2: TDD GREEN - Daemon implementation** - `9daf25c` (feat)

## Files Created/Modified
- `Cargo.toml` - Workspace root with [workspace] section and dependency inheritance
- `src/lib.rs` - Re-export facade: `pub use agtx_core::*` + TUI modules
- `crates/agtx-core/Cargo.toml` - Shared library crate configuration
- `crates/agtx-core/src/lib.rs` - Module re-exports for shared code
- `crates/agtx-core/src/skills.rs` - Updated include_str! paths for workspace layout
- `crates/agtx-core/src/db/schema.rs` - Added open_at and open_global_at constructors
- `crates/agtxd/Cargo.toml` - Daemon binary with axum/tower-http deps
- `crates/agtxd/src/main.rs` - Entry point with CLI arg parsing, server startup, graceful shutdown
- `crates/agtxd/src/lib.rs` - Library exports for integration tests
- `crates/agtxd/src/state.rs` - AppState with db paths and start time
- `crates/agtxd/src/shutdown.rs` - SIGTERM/Ctrl+C signal handler
- `crates/agtxd/src/error.rs` - AppError with IntoResponse for HTTP error codes
- `crates/agtxd/src/api/mod.rs` - Router composition (health + tasks + projects)
- `crates/agtxd/src/api/health.rs` - GET /health with status, uptime_secs, version
- `crates/agtxd/src/api/tasks.rs` - Task CRUD handlers with spawn_blocking DB access
- `crates/agtxd/src/api/projects.rs` - Project list/get handlers
- `crates/agtxd/tests/api_tests.rs` - 9 integration tests for all endpoints

## Decisions Made
- Used `pub use agtx_core::*` re-export facade so all existing `use agtx::config`, `use agtx::db` imports work without changes
- Default daemon port 3742 (uncommon port, avoids conflicts with common dev servers)
- Added `Database::open_at` for explicit path databases (daemon and test use), keeping existing `open_project` hash-path derivation for TUI compatibility
- Created both `lib.rs` and `main.rs` in agtxd so integration tests can `use agtxd::` to access API router and state

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Installed Rust toolchain and build-essential**
- **Found during:** Task 1 (workspace compilation check)
- **Issue:** CI/build environment had no Rust compiler or C linker installed
- **Fix:** Installed rustup and build-essential (needed for rusqlite bundled SQLite compilation)
- **Files modified:** None (system packages)
- **Verification:** `cargo check --workspace` succeeded after installation

**2. [Rule 3 - Blocking] Created placeholder agtxd crate for workspace member**
- **Found during:** Task 1 (workspace compilation check)
- **Issue:** Workspace root listed `crates/agtxd` as member but it had no Cargo.toml yet
- **Fix:** Created minimal Cargo.toml and placeholder main.rs for agtxd in Task 1 so workspace compiles; fully implemented in Task 2
- **Files modified:** crates/agtxd/Cargo.toml, crates/agtxd/src/main.rs
- **Verification:** `cargo check --workspace` succeeded

---

**Total deviations:** 2 auto-fixed (2 blocking)
**Impact on plan:** Both auto-fixes were environment and build ordering issues. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Workspace structure established for all subsequent phases
- Daemon foundation ready for Phase 1 Plan 02 (structured logging and config hot-reload)
- REST API skeleton ready for Phase 2 (PTY process management endpoints)
- agtx TUI binary compiles and all tests pass, safe to develop in parallel

## Self-Check: PASSED

- All 15 claimed files exist on disk
- All 3 task commits verified in git log
- All must_have artifacts verified (workspace config, module exports, min lines, keyword presence)
- `cargo test --workspace` passes with 0 failures

---
*Phase: 01-daemon-foundation*
*Completed: 2026-03-04*
