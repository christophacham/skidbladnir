# Phase 3: WebSocket Streaming - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Bidirectional real-time agent output streaming via WebSocket, with session persistence and reconnection support. This phase delivers the daemon-side WebSocket infrastructure — the frontend SvelteKit app is Phase 4, but a minimal integration test or demo client may be included to verify end-to-end WebSocket behavior.

Requirements covered: INFRA-02, WS-01, WS-02, WS-03, WS-04, WS-05

Note: WS-02 (output persistence to disk) is already implemented via Phase 2's SessionOutput (append-only file + ring buffer). This phase focuses on the WebSocket broadcasting and reconnection layers that consume that existing persistence.

</domain>

<decisions>
## Implementation Decisions

### WebSocket message protocol
- JSON-wrapped typed messages over WebSocket (not raw binary frames)
- Message types: `output` (PTY bytes as base64), `state` (session state changes), `metrics` (resource usage updates), `error`
- Input messages: `write` (text to PTY stdin), `resize` (new dimensions)
- This aligns with the "structured output, not raw terminal" vision — even though raw bytes are sent now, wrapping in typed JSON enables Phase 7's structured parsing layer
- axum's built-in WebSocket support (`axum::extract::ws`)

### Reconnection & history delivery
- On WebSocket connect, client sends optional `cursor` (byte offset from previous session)
- If cursor provided: daemon streams delta from cursor position using the append-only log file
- If no cursor: daemon sends ring buffer contents (last 64KB) for fast initial load
- Full history available via REST endpoint (GET /api/v1/sessions/{id}/output with offset/limit query params) for lazy-loaded scrollback
- Connection status conveyed via WebSocket ping/pong and explicit `state` messages

### Client multiplexing
- Multiple browser tabs can watch the same session simultaneously
- Use `tokio::sync::broadcast` channel per session for fan-out
- Reader task (already exists from Phase 2) publishes to broadcast channel in addition to ring buffer + file
- Each WebSocket handler subscribes to the broadcast channel
- Lagged receivers handled gracefully (skip missed messages, client re-syncs from file)

### Auto-scroll and flow control
- Daemon-side: no special flow control needed — WebSocket sends all output as it arrives
- Auto-scroll behavior is purely frontend (Phase 4/5) — daemon just streams continuously
- "Jump to bottom" button is frontend-only UI

### Connection lifecycle
- WebSocket endpoint: `/api/v1/sessions/{id}/ws`
- On connect: validate session exists, subscribe to broadcast, send initial state
- On disconnect: unsubscribe from broadcast (no state to clean up)
- Heartbeat via WebSocket ping frames (axum handles this)

### Claude's Discretion
- Broadcast channel buffer size (balancing memory vs lag tolerance)
- Whether to compress WebSocket frames (permessage-deflate)
- Exact reconnection handshake protocol details
- Error handling for WebSocket upgrade failures
- Whether metrics updates are pushed via WebSocket or polled via REST

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `SessionOutput` (crates/agtxd/src/session/output.rs): Ring buffer + append-only file — WebSocket handler reads from these for initial state and history
- `SessionManager` (crates/agtxd/src/session/manager.rs): Session registry with `Arc<RwLock<HashMap<Uuid, SessionHandle>>>` — WebSocket handler looks up sessions here
- `SessionHandle.output: Arc<RwLock<SessionOutput>>` — shareable reference for WebSocket readers
- Reader task in `SessionManager::spawn()` — already reads PTY output continuously, can be extended to publish to broadcast channel
- Base64 encoding in `api/sessions.rs` — established pattern for binary-over-JSON

### Established Patterns
- `Arc<RwLock<...>>` for concurrent access to session data
- axum router nesting under `/api/v1/sessions`
- `AppState` with `Arc<SessionManager>` for handler access
- Clone router for HTTP integration tests

### Integration Points
- Extend `SessionHandle` with `tokio::sync::broadcast::Sender<Vec<u8>>`
- Modify reader task to publish output bytes to broadcast channel
- Add WebSocket route to `api/sessions.rs` router
- WebSocket handler accesses `SessionManager` via `AppState`
- History endpoint extends existing output endpoint with offset/limit

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

*Phase: 03-websocket-streaming*
*Context gathered: 2026-03-04*
