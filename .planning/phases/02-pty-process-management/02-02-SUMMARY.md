---
phase: 02-pty-process-management
plan: 02
subsystem: api
tags: [rest-api, session-management, axum, shutdown, sigint, sigkill, base64, pty]

requires:
  - phase: 02-pty-process-management
    provides: "SessionManager, SessionOutput, session types from Plan 01"
  - phase: 01-daemon-foundation
    provides: "axum API router, AppState, AppError, shutdown signal handler"
provides:
  - "9 REST endpoints under /api/v1/sessions for full session lifecycle"
  - "AppState with Arc<SessionManager> wired at daemon startup"
  - "Graceful shutdown calling shutdown_all() after HTTP server stops"
  - "Drop safety net for SessionManager (defense in depth)"
  - "10 HTTP-level integration tests for session API"
affects: [03-websocket-streaming, 04-frontend-terminal]

tech-stack:
  added: []
  patterns: [shared-router-clone-for-testing, base64-encode-output, nix-signal-send]

key-files:
  created:
    - crates/agtxd/src/api/sessions.rs
    - crates/agtxd/tests/session_api_tests.rs
  modified:
    - crates/agtxd/src/api/mod.rs
    - crates/agtxd/src/state.rs
    - crates/agtxd/src/main.rs
    - crates/agtxd/src/session/manager.rs
    - crates/agtxd/tests/api_tests.rs
    - crates/agtxd/tests/session_tests.rs

key-decisions:
  - "Router cloned for each test request (axum Router is Clone) to share state across request sequences"
  - "Output endpoint returns base64-encoded JSON rather than raw octet-stream for API consumer convenience"
  - "Interrupt and kill-process endpoints use nix::sys::signal::kill directly on PID (no SessionManager method needed)"
  - "Drop impl uses try_read() to avoid blocking; sends SIGTERM as best-effort fallback"

patterns-established:
  - "Clone Router for multi-request test sequences sharing AppState"
  - "base64_encode/base64_decode helpers for binary output over JSON API"
  - "nix signal sending for SIGINT/SIGKILL to session processes via REST API"

requirements-completed: [PTY-05]

duration: 8min
completed: 2026-03-04
---

# Phase 2 Plan 2: REST API Endpoints and Shutdown Integration Summary

**9 session REST endpoints (spawn/list/get/write/resize/interrupt/kill/output) wired into axum with graceful shutdown calling shutdown_all() and Drop safety net**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-04T09:33:54Z
- **Completed:** 2026-03-04T09:42:11Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- All 9 session REST endpoints accessible under /api/v1/sessions with correct HTTP status codes
- AppState includes Arc<SessionManager>, created at daemon startup in main.rs
- Daemon shutdown sequence: signal received -> axum stops -> shutdown_all() kills all sessions -> exit
- Drop impl on SessionManager provides defense-in-depth SIGTERM to any surviving sessions
- 10 new HTTP-level integration tests and 1 new shutdown test, all passing (37 total agtxd tests)

## Task Commits

Each task was committed atomically:

1. **Task 1: REST API endpoints for session operations** - `538a007` (feat, TDD)
2. **Task 2: Process cleanup on daemon shutdown** - `a6fbb64` (feat)

## Files Created/Modified
- `crates/agtxd/src/api/sessions.rs` - 9 REST endpoint handlers + router, request/response types, base64 encoding
- `crates/agtxd/src/api/mod.rs` - Added sessions module and nested route
- `crates/agtxd/src/state.rs` - AppState now includes Arc<SessionManager> field
- `crates/agtxd/src/main.rs` - Creates SessionManager at startup, calls shutdown_all() on exit
- `crates/agtxd/src/session/manager.rs` - Added Drop impl for defense-in-depth cleanup
- `crates/agtxd/tests/api_tests.rs` - Updated build_test_app to provide SessionManager
- `crates/agtxd/tests/session_api_tests.rs` - 10 HTTP tests for all session endpoints
- `crates/agtxd/tests/session_tests.rs` - Added test_shutdown_all_kills_all_sessions

## Decisions Made
- **Router cloning for tests:** axum Router implements Clone; cloning it for each request in a test preserves shared state (SessionManager via Arc), unlike rebuilding the app which creates a new SessionManager.
- **Base64 output over JSON:** Output endpoint encodes PTY ring buffer as base64 in JSON `{ "data": "<b64>", "total_bytes": N }` rather than raw `application/octet-stream`. Simpler for API consumers to handle.
- **Direct nix signal for interrupt/kill:** Interrupt and kill-process endpoints call `nix::sys::signal::kill()` directly on the PID rather than adding methods to SessionManager. The PID is obtained via `session_manager.get()` and the signal is sent inline.
- **Drop uses try_read():** The Drop impl uses `try_read()` (non-blocking) on the sessions RwLock to avoid deadlocking if dropped while a write lock is held. This is a best-effort fallback.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Pre-commit hook `no-ai-in-commit-msg` blocks Co-Authored-By lines containing agent references. Commit messages omit the line.
- Test pattern mismatch: initial test approach used `build_test_app` per request which created fresh SessionManagers. Fixed by sharing a single Router instance (clone per request) for test sequences requiring state persistence.
- Clippy 1.93.0 introduced `manual_div_ceil` lint -- applied `data.len().div_ceil(3)` in base64_encode.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All session operations are HTTP-accessible, ready for WebSocket streaming (Phase 3)
- Shutdown lifecycle is complete: graceful HTTP drain + session cleanup + kernel safety net
- Resource monitoring (Plan 03) can add /api/v1/sessions/{id}/metrics endpoint following the same pattern

## Self-Check: PASSED

- All 8 created/modified files exist on disk
- Both commits found in git log (538a007, a6fbb64)
- Artifact min_lines met: sessions.rs=267/100, session_api_tests.rs (80+ lines)
- Required content found: spawn_session in sessions.rs, SessionManager in state.rs, shutdown_all in main.rs
- All 37 agtxd tests pass (9 API + 3 config + 3 logging + 10 session API + 12 session)

---
*Phase: 02-pty-process-management*
*Completed: 2026-03-04*
