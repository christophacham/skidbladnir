---
phase: 03-websocket-streaming
plan: 01
subsystem: api
tags: [websocket, broadcast, tokio, base64, pty, streaming]

requires:
  - phase: 02-pty-process-management
    provides: SessionManager, SessionOutput, SessionHandle, PTY reader task
provides:
  - OutputEvent broadcast channel for WebSocket fan-out
  - ServerMessage/ClientMessage WebSocket protocol types
  - SessionOutput::read_range for history reads from log files
  - SessionManager::write_raw (no newline), subscribe, get_output_path
  - Output REST endpoint with offset/limit query parameters
affects: [03-websocket-streaming, 04-frontend-core]

tech-stack:
  added: [base64 0.22, tokio-tungstenite 0.26 (dev), futures-util 0.3 (dev), axum ws feature]
  patterns: [broadcast channel fan-out, serde tagged enum protocol, static file read_range]

key-files:
  created:
    - crates/agtxd/src/api/ws.rs
  modified:
    - Cargo.toml
    - crates/agtxd/Cargo.toml
    - crates/agtxd/src/session/types.rs
    - crates/agtxd/src/session/output.rs
    - crates/agtxd/src/session/manager.rs
    - crates/agtxd/src/session/mod.rs
    - crates/agtxd/src/api/sessions.rs
    - crates/agtxd/src/api/mod.rs

key-decisions:
  - "broadcast::channel capacity 256 for OutputEvent fan-out"
  - "OutputEvent::Data carries bytes + offset for client cursor tracking"
  - "read_range is a static method on SessionOutput (no instance needed)"
  - "Replaced hand-rolled base64_encode with base64 crate standard engine"
  - "Output endpoint uses ring buffer when offset absent/0, log file when offset > 0"

patterns-established:
  - "Broadcast fan-out: reader task publishes to broadcast, subscribers get Receiver"
  - "Serde tagged enums for WebSocket protocol: #[serde(tag = \"type\")] with rename"
  - "Static file readers for append-only logs (no lock contention)"

requirements-completed: [INFRA-02, WS-01, WS-02]

duration: 5min
completed: 2026-03-04
---

# Phase 3 Plan 01: WebSocket Streaming Infrastructure Summary

**Broadcast channel fan-out on SessionHandle, WS message protocol types, and output read_range for reconnection history**

## Performance

- **Duration:** 5 min
- **Started:** 2026-03-04T10:18:44Z
- **Completed:** 2026-03-04T10:23:55Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- Broadcast channel infrastructure added to sessions for real-time WebSocket fan-out
- WebSocket message protocol types (ServerMessage/ClientMessage) defined with serde serialization
- SessionOutput::read_range enables arbitrary byte range reads from append-only log files
- Output REST endpoint extended with offset/limit query parameters for paginated history

## Task Commits

Each task was committed atomically:

1. **Task 1: Broadcast channel infrastructure and output history reading** - `72ef09f` (feat)
2. **Task 2: WebSocket message protocol types and output endpoint offset/limit** - `7c9d2a3` (feat)

## Files Created/Modified
- `crates/agtxd/src/api/ws.rs` - ServerMessage and ClientMessage serde-tagged enums for WS protocol
- `crates/agtxd/src/session/types.rs` - OutputEvent enum, output_tx and output_path on SessionHandle
- `crates/agtxd/src/session/output.rs` - read_range static method for log file byte range reads
- `crates/agtxd/src/session/manager.rs` - write_raw, subscribe, get_output_path methods; broadcast in reader task
- `crates/agtxd/src/session/mod.rs` - Re-export OutputEvent
- `crates/agtxd/src/api/sessions.rs` - OutputQuery with offset/limit, base64 crate replacement
- `crates/agtxd/src/api/mod.rs` - Register ws module
- `Cargo.toml` - axum ws feature, base64 workspace dep
- `crates/agtxd/Cargo.toml` - base64 dep, tokio-tungstenite and futures-util dev-deps

## Decisions Made
- broadcast::channel capacity set to 256 (sufficient for bursts, bounded memory)
- OutputEvent::Data carries both bytes and offset so WS clients can track cursor position
- read_range is a static method (reads directly from file path, no SessionOutput instance needed)
- Replaced hand-rolled base64_encode with base64 crate standard engine for correctness and maintenance
- Output endpoint preserves backward compatibility: no query params = ring buffer behavior

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Removed unused base64_encode function**
- **Found during:** Task 2
- **Issue:** After replacing with base64 crate, the hand-rolled function triggered dead_code warning (clippy -D warnings)
- **Fix:** Removed the function entirely
- **Files modified:** crates/agtxd/src/api/sessions.rs
- **Committed in:** 7c9d2a3 (Task 2 commit)

**2. [Rule 1 - Bug] Fixed redundant closure in map_err**
- **Found during:** Task 2
- **Issue:** Clippy flagged `|e| AppError::Internal(e)` as redundant closure
- **Fix:** Changed to `AppError::Internal` (tuple variant directly)
- **Files modified:** crates/agtxd/src/api/sessions.rs
- **Committed in:** 7c9d2a3 (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 bug fixes from clippy)
**Impact on plan:** Minor code quality fixes. No scope creep.

## Issues Encountered
- Pre-commit hook blocks Co-Authored-By lines containing agent names -- commit messages adjusted accordingly

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Broadcast infrastructure ready for Plan 02 WebSocket handler to subscribe and stream
- ServerMessage/ClientMessage types ready for JSON serialization over WebSocket
- read_range ready for reconnection history delivery
- All 43 existing tests pass with zero regressions

---
*Phase: 03-websocket-streaming*
*Completed: 2026-03-04*
