---
phase: 01-daemon-foundation
plan: 02
subsystem: infra
tags: [tracing, tracing-subscriber, tracing-appender, notify, json-logging, config-hot-reload]

# Dependency graph
requires:
  - phase: 01-daemon-foundation plan 01
    provides: "Cargo workspace, agtxd daemon binary, AppState, GlobalConfig"
provides:
  - Multi-layer structured logging (JSON file + pretty stderr)
  - Daily log rotation via tracing-appender
  - Runtime log level changes via tracing reload handle
  - Config file hot-reload via notify filesystem watcher
  - DaemonConfig with port, bind, log_level fields
  - Shared live config via Arc<RwLock<GlobalConfig>>
affects: [02-pty-management, 03-websocket-streaming]

# Tech tracking
tech-stack:
  added: [tracing-subscriber 0.3 (fmt/json/env-filter/registry), tracing-appender 0.2, notify 8]
  patterns: [non-blocking log writes, reloadable EnvFilter, directory-level watcher with debounce, Arc<RwLock<T>> shared state]

key-files:
  created:
    - crates/agtxd/src/logging.rs
    - crates/agtxd/src/config_watcher.rs
    - crates/agtxd/tests/logging_tests.rs
    - crates/agtxd/tests/config_reload_tests.rs
  modified:
    - crates/agtx-core/src/config/mod.rs
    - crates/agtxd/Cargo.toml
    - crates/agtxd/src/main.rs
    - crates/agtxd/src/state.rs
    - crates/agtxd/src/lib.rs
    - crates/agtxd/tests/api_tests.rs

key-decisions:
  - "build_logging() helper returns subscriber without setting global default, enabling parallel test execution"
  - "Watch parent directory (not file) to handle editor delete+recreate save patterns"
  - "200ms debounce window prevents duplicate reloads from rapid filesystem events"
  - "Port/bind changes stored in shared config but logged as restart-required warnings"

patterns-established:
  - "Non-blocking tracing pattern: tracing_appender::non_blocking wraps file writer, WorkerGuard kept alive in main scope"
  - "Config hot-reload pattern: notify watcher -> mpsc channel -> debounce -> parse -> apply"
  - "Shared config pattern: Arc<RwLock<GlobalConfig>> cloned to watcher and handlers"

requirements-completed: [INFRA-03, INFRA-06]

# Metrics
duration: 6min
completed: 2026-03-04
---

# Phase 1 Plan 02: Structured Logging and Config Hot-Reload Summary

**Multi-layer tracing with daily-rotating JSON log files, pretty stderr output, and notify-based config hot-reload with runtime log level changes**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-04T08:05:46Z
- **Completed:** 2026-03-04T08:12:11Z
- **Tasks:** 2 (TDD, 4 commits total)
- **Files modified:** 10

## Accomplishments
- DaemonConfig added to GlobalConfig with backwards-compatible serde defaults (port 3742, bind 127.0.0.1, log_level info)
- Multi-layer structured logging: JSON to daily-rotating files at `~/.local/share/agtx/logs/`, pretty ANSI to stderr
- Non-blocking file writes via tracing_appender to avoid slowing request handling
- Config file watcher detects changes within 1 second, applies log level dynamically via tracing reload handle
- Invalid config files produce warnings without crashing or corrupting shared state
- 6 new tests (3 logging, 3 config reload), 233 total workspace tests green

## Task Commits

Each task was committed atomically:

1. **Task 1: DaemonConfig + multi-layer logging (RED)** - `385a2b1` (test)
2. **Task 1: DaemonConfig + multi-layer logging (GREEN)** - `03bc159` (feat)
3. **Task 2: Config file watcher with hot-reload (RED)** - `34424b7` (test)
4. **Task 2: Config file watcher with hot-reload (GREEN)** - `f540350` (feat)

## Files Created/Modified
- `crates/agtx-core/src/config/mod.rs` - Added DaemonConfig struct with port/bind/log_level defaults
- `crates/agtxd/Cargo.toml` - Added tracing-subscriber, tracing-appender, notify, toml dependencies
- `crates/agtxd/src/logging.rs` - init_logging (global subscriber) and build_logging (test-friendly) with dual output layers
- `crates/agtxd/src/config_watcher.rs` - notify-based watcher with debounce, log level reload, invalid config resilience
- `crates/agtxd/src/main.rs` - Wired logging init, config watcher spawn, reads DaemonConfig for port/bind
- `crates/agtxd/src/state.rs` - Added Arc<RwLock<GlobalConfig>> for live config access
- `crates/agtxd/src/lib.rs` - Added logging and config_watcher module declarations
- `crates/agtxd/tests/logging_tests.rs` - 3 tests: dir creation, JSON output, reload handle
- `crates/agtxd/tests/config_reload_tests.rs` - 3 tests: log level reload, invalid config, port change
- `crates/agtxd/tests/api_tests.rs` - Updated AppState::new call for new config parameter

## Decisions Made
- Used `build_logging()` helper that returns subscriber without calling `set_global_default`, enabling tests to run in parallel without conflicting global state
- Watch parent directory instead of config file directly, because editors like vim delete and recreate files on save
- 200ms debounce window prevents duplicate reloads when editors write temp files then rename
- Port/bind changes are stored in shared config but logged as warnings since they require a restart to take effect

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated api_tests.rs for new AppState::new signature**
- **Found during:** Task 2 (state.rs update)
- **Issue:** AppState::new gained a third parameter (GlobalConfig), breaking existing api_tests.rs
- **Fix:** Updated `build_test_app` helper to pass `GlobalConfig::default()` as third argument
- **Files modified:** crates/agtxd/tests/api_tests.rs
- **Verification:** All 9 existing API tests pass unchanged
- **Committed in:** f540350 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Straightforward signature update, no scope creep.

## Issues Encountered
None beyond the auto-fixed deviation above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Daemon has production-grade observability with structured JSON logs and pretty dev output
- Config hot-reload foundation ready for Phase 2 agent process management (can reload agent configs without restart)
- Shared config state pattern established for all future handler extensions
- All 233 workspace tests pass, safe to proceed

## Self-Check: PASSED

- All 10 claimed files exist on disk
- All 4 task commits verified in git log (385a2b1, 03bc159, 34424b7, f540350)
- logging.rs: 94 lines, contains `non_blocking` (7 occurrences)
- config_watcher.rs: 144 lines, contains `RecommendedWatcher` (3 occurrences)
- config/mod.rs: contains `DaemonConfig` (4 occurrences)
- All key_links verified (init_logging in main, reload_handle in watcher, GlobalConfig in watcher, watch_config in main)
- `cargo test --workspace`: 233 tests, 0 failures

---
*Phase: 01-daemon-foundation*
*Completed: 2026-03-04*
