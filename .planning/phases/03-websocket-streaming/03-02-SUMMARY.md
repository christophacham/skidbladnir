---
phase: 03-websocket-streaming
plan: 02
subsystem: api
tags: [websocket, axum, tokio, broadcast, pty, streaming, reconnection]

requires:
  - phase: 03-websocket-streaming/01
    provides: "ServerMessage/ClientMessage types, OutputEvent broadcast channel, SessionManager::subscribe/write_raw, SessionOutput::read_range"
provides:
  - "WebSocket endpoint at /api/v1/sessions/{id}/ws with bidirectional streaming"
  - "Broadcast fan-out for multi-client output delivery"
  - "Cursor-based reconnection for delta delivery from append-only log"
  - "Client write/resize forwarding to PTY stdin"
affects: [04-frontend-terminal, 05-frontend-dashboard]

tech-stack:
  added: []
  patterns: [mpsc-bridge-for-ws-split, cursor-reconnection, broadcast-to-mpsc-fanout]

key-files:
  created: [crates/agtxd/tests/ws_tests.rs]
  modified: [crates/agtxd/src/api/ws.rs, crates/agtxd/src/api/mod.rs]

key-decisions:
  - "mpsc channel bridge pattern instead of futures-util split to avoid adding futures-util as main dependency"
  - "tokio::select! loop for multiplexing inbound/outbound on single WebSocket without splitting"
  - "Cat-based cursor test with REST write API for deterministic timing (no sleep-based flakiness)"

patterns-established:
  - "WebSocket handler pattern: validate-before-upgrade, subscribe-before-read, mpsc bridge for send task"
  - "Integration test pattern: TcpListener::bind(0) + axum::serve + tokio_tungstenite::connect_async"

requirements-completed: [WS-03, WS-04, WS-05]

duration: 7min
completed: 2026-03-04
---

# Phase 3 Plan 02: WebSocket Handler Summary

**Bidirectional WebSocket streaming with broadcast fan-out, cursor reconnection, and 7 integration tests using tokio-tungstenite**

## Performance

- **Duration:** 7 min
- **Started:** 2026-03-04T10:26:41Z
- **Completed:** 2026-03-04T10:33:21Z
- **Tasks:** 1 (TDD: RED + GREEN)
- **Files modified:** 3

## Accomplishments
- WebSocket endpoint at /api/v1/sessions/{id}/ws accepts connections, validates session existence before upgrade
- Broadcast fan-out delivers live PTY output to multiple concurrent WebSocket clients simultaneously
- Cursor-based reconnection reads delta from append-only log file for seamless client reconnects
- Client write/resize messages forwarded to PTY stdin via SessionManager
- 7 integration tests covering: upgrade success, 404 for missing session, live output, multi-client fan-out, write input forwarding, cursor reconnection delta, state change on exit

## Task Commits

Each task was committed atomically:

1. **Task 1 (RED): Failing WebSocket integration tests** - `0baf54a` (test)
2. **Task 1 (GREEN): WebSocket handler implementation** - `4738282` (feat)

## Files Created/Modified
- `crates/agtxd/src/api/ws.rs` - WebSocket handler with ws_handler, handle_ws, send_msg functions (extended from Plan 01 types)
- `crates/agtxd/src/api/mod.rs` - Added WebSocket route registration at /api/v1/sessions/{id}/ws
- `crates/agtxd/tests/ws_tests.rs` - 7 integration tests using tokio-tungstenite WebSocket client

## Decisions Made
- Used mpsc channel bridge pattern instead of futures-util StreamExt::split() to avoid adding futures-util as a main dependency (it was only a dev-dependency). The broadcast listener spawns a task that sends JSON strings through an mpsc channel, and the main loop uses tokio::select! to multiplex mpsc receives with WebSocket recv().
- Cat-based cursor reconnection test with REST write API for deterministic timing control instead of sleep-based shell commands.
- WebSocket route registered at top level in api_router (before session nest) to ensure WebSocketUpgrade extractor works correctly with axum 0.8 routing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Avoided futures-util as main dependency**
- **Found during:** Task 1 GREEN phase
- **Issue:** Plan called for `futures_util::{SinkExt, StreamExt}` to split WebSocket, but futures-util was only a dev-dependency
- **Fix:** Used mpsc channel bridge pattern with tokio::select! loop instead of socket split
- **Files modified:** crates/agtxd/src/api/ws.rs
- **Verification:** cargo build -p agtxd compiles cleanly
- **Committed in:** 4738282

**2. [Rule 1 - Bug] Fixed timing-sensitive test flakiness**
- **Found during:** Task 1 GREEN phase (tests passed in isolation, failed under load)
- **Issue:** cursor_reconnection and state_change_on_exit tests had race conditions with shell command timing
- **Fix:** Rewrote cursor test to use cat + REST write API for deterministic output control; increased state change timeout
- **Files modified:** crates/agtxd/tests/ws_tests.rs
- **Verification:** All 50 tests pass reliably under full test suite load
- **Committed in:** 4738282

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both fixes necessary for correctness and reliability. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- WebSocket streaming endpoint fully functional with all planned features
- Ready for Plan 03 (flow control / backpressure) or frontend integration
- All 50 tests pass: 43 pre-existing + 7 new WebSocket tests

---
*Phase: 03-websocket-streaming*
*Completed: 2026-03-04*
