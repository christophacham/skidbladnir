# Phase 3: WebSocket Streaming - Research

**Researched:** 2026-03-04
**Domain:** WebSocket bidirectional streaming, broadcast channels, session persistence & reconnection
**Confidence:** HIGH

## Summary

Phase 3 adds WebSocket-based real-time streaming of PTY output from the daemon to browser clients, with bidirectional communication (input/resize), client multiplexing via `tokio::sync::broadcast`, and reconnection with history replay. The existing codebase already has all foundational pieces in place: `SessionOutput` (ring buffer + append-only file), `SessionManager` with `Arc<RwLock<SessionHandle>>`, and axum 0.8.8 which supports WebSocket via its `"ws"` feature flag.

The core implementation adds a `broadcast::Sender<Vec<u8>>` to each `SessionHandle`, modifies the reader task to publish output to both the broadcast channel and the existing output storage, creates a WebSocket handler at `/api/v1/sessions/{id}/ws`, and extends the output endpoint with offset/limit for lazy history loading. No external WebSocket libraries are needed -- axum's built-in `axum::extract::ws` module provides `WebSocketUpgrade`, `WebSocket`, and `Message` types, backed by tungstenite internally.

**Primary recommendation:** Use axum's built-in WebSocket support with `tokio::sync::broadcast` for fan-out. Keep the WebSocket message protocol as typed JSON messages wrapping base64-encoded PTY bytes. Add `tokio-tungstenite` only as a dev-dependency for integration testing.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- JSON-wrapped typed messages over WebSocket (not raw binary frames)
- Message types: `output` (PTY bytes as base64), `state` (session state changes), `metrics` (resource usage updates), `error`
- Input messages: `write` (text to PTY stdin), `resize` (new dimensions)
- axum's built-in WebSocket support (`axum::extract::ws`)
- On WebSocket connect, client sends optional `cursor` (byte offset from previous session)
- If cursor provided: daemon streams delta from cursor position using the append-only log file
- If no cursor: daemon sends ring buffer contents (last 64KB) for fast initial load
- Full history available via REST endpoint (GET /api/v1/sessions/{id}/output with offset/limit query params) for lazy-loaded scrollback
- Connection status conveyed via WebSocket ping/pong and explicit `state` messages
- Multiple browser tabs can watch the same session simultaneously
- Use `tokio::sync::broadcast` channel per session for fan-out
- Reader task publishes to broadcast channel in addition to ring buffer + file
- Each WebSocket handler subscribes to the broadcast channel
- Lagged receivers handled gracefully (skip missed messages, client re-syncs from file)
- WebSocket endpoint: `/api/v1/sessions/{id}/ws`
- Auto-scroll behavior is purely frontend (Phase 4/5) -- daemon just streams continuously
- "Jump to bottom" button is frontend-only UI
- Daemon-side: no special flow control needed

### Claude's Discretion
- Broadcast channel buffer size (balancing memory vs lag tolerance)
- Whether to compress WebSocket frames (permessage-deflate)
- Exact reconnection handshake protocol details
- Error handling for WebSocket upgrade failures
- Whether metrics updates are pushed via WebSocket or polled via REST

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| INFRA-02 | Daemon serves WebSocket endpoint for bidirectional real-time streaming | axum `"ws"` feature provides `WebSocketUpgrade` extractor; route at `/api/v1/sessions/{id}/ws` |
| WS-01 | Browser receives live agent output via WebSocket as it is produced | `tokio::sync::broadcast` channel per session; reader task publishes to broadcast; WS handler subscribes |
| WS-02 | Daemon persists session output to disk as PTY bytes arrive | Already implemented via `SessionOutput` (append-only file + ring buffer) in Phase 2; this phase connects it to WS |
| WS-03 | User reconnects and sees full history via lazy-loaded virtualized scrollback | Cursor-based reconnection reads delta from append-only file; REST endpoint with offset/limit for lazy loading |
| WS-04 | User sees connection status indicator (connected/disconnected/reconnecting) | WebSocket ping/pong (automatic in axum) + explicit `state` messages on connect/disconnect |
| WS-05 | Output auto-scrolls to bottom; pauses on manual scroll-up with "jump to bottom" button | Daemon-side: stream continuously, no flow control. Frontend behavior (Phase 4/5) |

</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.8.8 (workspace) | WebSocket support via `"ws"` feature | Already in use; built-in WS avoids extra deps |
| tokio | 1.44 (workspace) | `broadcast` channel, async runtime | Already in use; broadcast is purpose-built for fan-out |
| serde_json | 1.0 (workspace) | JSON message serialization | Already in use for REST API |
| futures-util | 0.3.32 (transitive) | `StreamExt::split()` for WS sender/receiver | Already in dependency tree via axum |
| base64 | 0.22.1 (transitive) | Encode PTY bytes for JSON transport | Already in dependency tree; replace hand-rolled base64 |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio-tungstenite | 0.26 | WebSocket client for integration tests | Dev-dependency only |
| bytes | (transitive) | `Bytes` type for WS message payloads | Already in tree via axum |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| axum built-in ws | axum-tungstenite | Extra dep, same underlying tungstenite; built-in is simpler |
| broadcast channel | watch channel | Watch only keeps latest value; broadcast preserves ordering and fan-out |
| broadcast channel | mpsc per client | More complex client management; broadcast handles subscribe/unsubscribe automatically |
| No compression | permessage-deflate | axum/tungstenite does not natively support permessage-deflate; base64 JSON is small enough for LAN use; defer to Phase 9 if needed |

**Installation:**
```toml
# Cargo.toml workspace dependencies - add "ws" feature to axum
axum = { version = "0.8", features = ["tracing", "ws"] }

# agtxd dev-dependencies - add WebSocket test client
[dev-dependencies]
tokio-tungstenite = "0.26"
futures-util = "0.3"
```

## Architecture Patterns

### Modified Session Infrastructure
```
crates/agtxd/src/
├── session/
│   ├── types.rs          # Add broadcast::Sender to SessionHandle
│   ├── manager.rs        # Modify reader_task to publish to broadcast
│   ├── output.rs         # Add read_from_offset() for history replay
│   └── mod.rs            # Re-export new types
├── api/
│   ├── sessions.rs       # Add WebSocket route + handler
│   ├── ws.rs             # NEW: WebSocket handler, message types, protocol
│   └── mod.rs            # Register WS route
└── state.rs              # No changes needed (AppState already has SessionManager)
```

### Pattern 1: Broadcast Channel Per Session
**What:** Each `SessionHandle` gets a `broadcast::Sender<OutputEvent>` created at spawn time. The reader task publishes every PTY read to this channel. Each WebSocket handler calls `sender.subscribe()` to get its own `Receiver`.
**When to use:** Always -- this is the core fan-out mechanism.
**Example:**
```rust
// In session/types.rs -- extend SessionHandle
use tokio::sync::broadcast;

pub struct SessionHandle {
    // ... existing fields ...
    /// Broadcast sender for live output fan-out to WebSocket clients
    pub output_tx: broadcast::Sender<OutputEvent>,
}

/// Event broadcast to WebSocket subscribers
#[derive(Debug, Clone)]
pub enum OutputEvent {
    /// New PTY output bytes
    Data(Vec<u8>),
    /// Session state changed
    StateChange(SessionState),
}
```

### Pattern 2: WebSocket Handler with Split Socket
**What:** The WebSocket handler splits the socket into sender/receiver halves. A send task subscribes to broadcast and forwards output. A receive task handles incoming write/resize messages. `tokio::select!` or separate spawned tasks coordinate the two halves.
**When to use:** Every WebSocket connection.
**Example:**
```rust
// In api/ws.rs
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};

async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let uuid = Uuid::parse_str(&id).unwrap(); // validate first
    ws.on_upgrade(move |socket| handle_ws(socket, uuid, state))
}

async fn handle_ws(socket: WebSocket, session_id: Uuid, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to broadcast channel for this session
    let mut rx = state.session_manager.subscribe(session_id).await;

    // Send initial state (ring buffer contents)
    // ... send initial output ...

    // Spawn send task: broadcast -> WebSocket
    let send_task = tokio::spawn(async move {
        while let Ok(event) = rx.recv().await {
            let msg = serialize_event(&event);
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Receive task: WebSocket -> PTY
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Parse and handle write/resize commands
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}
```

### Pattern 3: Cursor-Based Reconnection
**What:** Client sends cursor (byte offset) in the initial WebSocket message or as a query parameter. Server reads the append-only file from that offset and streams the delta before subscribing to live broadcast.
**When to use:** When a client reconnects after a disconnect.
**Example:**
```rust
// Reading delta from append-only log file
use tokio::io::{AsyncReadExt, AsyncSeekExt};

async fn read_from_offset(path: &std::path::Path, offset: u64) -> anyhow::Result<Vec<u8>> {
    let mut file = tokio::fs::File::open(path).await?;
    let metadata = file.metadata().await?;
    let file_size = metadata.len();
    if offset >= file_size {
        return Ok(vec![]); // Nothing new
    }
    let mut buf = vec![0u8; (file_size - offset) as usize];
    file.seek(std::io::SeekFrom::Start(offset)).await?;
    file.read_exact(&mut buf).await?;
    Ok(buf)
}
```

### Pattern 4: REST History Endpoint with Offset/Limit
**What:** Extend the existing `/api/v1/sessions/{id}/output` endpoint to accept `offset` and `limit` query parameters for paginated history loading.
**When to use:** Frontend lazy-loads scrollback beyond the ring buffer.
**Example:**
```rust
#[derive(Debug, Deserialize)]
pub struct OutputQuery {
    /// Byte offset to start reading from (default: 0)
    #[serde(default)]
    pub offset: u64,
    /// Maximum bytes to return (default: 65536)
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    65_536
}
```

### Anti-Patterns to Avoid
- **Holding sessions read lock during WebSocket lifetime:** Subscribe to broadcast and clone output Arc before releasing the lock. Never hold `sessions.read().await` across `.await` points in the WS handler.
- **Unbounded broadcast channel:** Always specify a capacity. Lagged receivers get `RecvError::Lagged` which is recoverable.
- **Sending raw binary frames:** The decision is typed JSON messages. Even though it adds overhead, it enables Phase 7's structured parsing layer.
- **Blocking on file I/O in async context:** Use `tokio::fs` for all file reads during reconnection, not `std::fs`.
- **Publishing lock-guarded data through broadcast:** The reader task already has exclusive access to the read half of the PTY. Publish the raw bytes directly, not through the RwLock.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| WebSocket protocol | Raw TCP frame parsing | `axum::extract::ws` | Handles upgrade, ping/pong, close frames, fragmentation |
| Fan-out to multiple clients | Manual subscriber list with mpsc | `tokio::sync::broadcast` | Automatic subscribe/unsubscribe, handles lag detection |
| Base64 encoding | Custom `base64_encode()` in sessions.rs | `base64` crate (0.22, already transitive dep) | Handles all edge cases, padding modes, constant-time options |
| WebSocket client for tests | Raw HTTP upgrade requests | `tokio-tungstenite` | Full WebSocket client with proper handshake |
| JSON serialization of WS messages | Manual string formatting | `serde_json::to_string()` with `#[derive(Serialize)]` | Type-safe, handles escaping |

**Key insight:** axum's WebSocket support handles ping/pong automatically (responds to client pings, clients respond to server pings). You do not need to implement heartbeat logic manually. The `Message::Ping` and `Message::Pong` variants are available for inspection but are auto-handled.

## Common Pitfalls

### Pitfall 1: Broadcast Channel Capacity Too Small
**What goes wrong:** With a small broadcast buffer (e.g., 16), high-throughput PTY output (agent producing lots of text) causes frequent `RecvError::Lagged` errors, forcing clients into constant re-sync cycles.
**Why it happens:** Each PTY read (up to 4096 bytes) produces one broadcast message. A fast agent can produce dozens of reads per second.
**How to avoid:** Use a buffer size of 256 or 512. This holds ~1MB of output references at peak, which is acceptable. The broadcast channel stores the value once and clones per receiver.
**Warning signs:** Clients repeatedly receiving partial output or re-syncing from file.
**Recommendation:** 256 messages (approximately 1MB at 4KB per message).

### Pitfall 2: Deadlock from Nested Lock Acquisition
**What goes wrong:** The WebSocket handler acquires `sessions.read()` to look up a session, then tries to access `session.output.read()` while holding the sessions lock. If a writer needs `sessions.write()`, deadlock occurs.
**Why it happens:** `RwLock` readers can starve writers when held across async yield points.
**How to avoid:** Clone the `Arc<broadcast::Sender>` and `Arc<RwLock<SessionOutput>>` references immediately, then drop the sessions lock before doing any async work.
**Warning signs:** Server hangs under concurrent load, WebSocket connections stop responding.

### Pitfall 3: WebSocket Send After Close
**What goes wrong:** The send task continues trying to send after the client has disconnected, producing spurious error logs.
**Why it happens:** The broadcast receiver keeps producing values even after the WebSocket is closed.
**How to avoid:** Check the return value of `sender.send()`. If it returns `Err`, break the loop. Use `tokio::select!` to cancel the send task when the receive task detects a close.
**Warning signs:** Error logs about "connection reset" or "broken pipe" after client disconnects.

### Pitfall 4: Race Between Initial State and Broadcast Subscription
**What goes wrong:** Client misses output that arrives between reading the ring buffer and subscribing to broadcast. Result: gap in output.
**Why it happens:** Subscribe to broadcast first, then read initial state. The broadcast receiver queues messages from the moment of subscription.
**How to avoid:** Subscribe to broadcast BEFORE reading ring buffer/file. Send the initial state, then drain broadcast from that point. Include `total_bytes` in initial state so client knows its cursor position.
**Warning signs:** Intermittent missing lines of output on connect.

### Pitfall 5: Cursor Drift on Reconnection
**What goes wrong:** Client provides a cursor offset, but the file has been truncated or rotated, causing seek to fail or return wrong data.
**Why it happens:** Not applicable in current design (append-only, never truncated), but worth defending against.
**How to avoid:** Validate that the cursor offset is less than the file size. If cursor > file_size, treat as "no cursor" and send ring buffer contents.
**Warning signs:** Garbled output on reconnection, client shows data from wrong position.

### Pitfall 6: Forgetting to Handle RecvError::Lagged
**What goes wrong:** `broadcast::Receiver::recv()` returns `Err(RecvError::Lagged(n))` when messages were dropped. If unhandled, the receive loop terminates.
**Why it happens:** The receiver falls behind the sender's production rate.
**How to avoid:** On `Lagged`, log the count of missed messages, then continue receiving. Optionally notify the client via a `state` message that some output was skipped and they should re-sync from the file.
**Warning signs:** WebSocket connections silently drop during high-output periods.

## Code Examples

### WebSocket Message Protocol
```rust
// Source: Derived from CONTEXT.md decisions
use serde::{Deserialize, Serialize};

/// Server -> Client messages
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// PTY output bytes (base64-encoded)
    #[serde(rename = "output")]
    Output {
        data: String,        // base64-encoded bytes
        offset: u64,         // byte offset in the session log
    },
    /// Session state change
    #[serde(rename = "state")]
    State {
        session_state: String,  // "running", "exited"
        #[serde(skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
    },
    /// Resource metrics update
    #[serde(rename = "metrics")]
    Metrics {
        cpu_percent: f32,
        rss_bytes: u64,
        uptime_secs: f64,
    },
    /// Error notification
    #[serde(rename = "error")]
    Error {
        message: String,
    },
    /// Initial connection state (sent on connect)
    #[serde(rename = "connected")]
    Connected {
        session_id: String,
        total_bytes: u64,    // total bytes in session log (client's new cursor)
    },
}

/// Client -> Server messages
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Write text to PTY stdin
    #[serde(rename = "write")]
    Write { input: String },
    /// Resize PTY dimensions
    #[serde(rename = "resize")]
    Resize { rows: u16, cols: u16 },
}
```

### WebSocket Route Registration
```rust
// Source: axum 0.8.8 routing pattern
// In api/sessions.rs or api/ws.rs
pub fn router() -> Router<AppState> {
    Router::new()
        // ... existing routes ...
        .route("/{id}/ws", get(ws_handler))
}
```

### Broadcast Channel Integration in SessionHandle
```rust
// Source: tokio::sync::broadcast documentation
use tokio::sync::broadcast;

const BROADCAST_CAPACITY: usize = 256;

// In SessionManager::spawn():
let (output_tx, _) = broadcast::channel::<OutputEvent>(BROADCAST_CAPACITY);

let handle = SessionHandle {
    // ... existing fields ...
    output_tx,
};

// In reader_task():
async fn reader_task(
    mut read_pty: OwnedReadPty,
    output: Arc<RwLock<SessionOutput>>,
    output_tx: broadcast::Sender<OutputEvent>,
    session_id: Uuid,
) {
    let mut buf = [0u8; 4096];
    loop {
        match read_pty.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let data = buf[..n].to_vec();
                let mut out = output.write().await;
                if let Err(e) = out.append(&data).await {
                    tracing::error!(session_id = %session_id, error = %e, "Output write failed");
                    break;
                }
                // Publish to broadcast (ignore error = no receivers)
                let _ = output_tx.send(OutputEvent::Data(data));
            }
            Err(e) if e.raw_os_error() == Some(libc::EIO) => break,
            Err(e) => {
                tracing::error!(session_id = %session_id, error = %e, "PTY read error");
                break;
            }
        }
    }
}
```

### Reading Session Output with Offset/Limit
```rust
// In session/output.rs -- add method for history reading
use tokio::io::{AsyncReadExt, AsyncSeekExt};

impl SessionOutput {
    /// Read bytes from the append-only log file starting at the given offset.
    /// Returns up to `limit` bytes.
    pub async fn read_range(path: &std::path::Path, offset: u64, limit: usize) -> anyhow::Result<Vec<u8>> {
        let mut file = tokio::fs::File::open(path).await?;
        let metadata = file.metadata().await?;
        let file_size = metadata.len();

        if offset >= file_size {
            return Ok(vec![]);
        }

        let available = (file_size - offset) as usize;
        let read_size = available.min(limit);
        let mut buf = vec![0u8; read_size];

        file.seek(std::io::SeekFrom::Start(offset)).await?;
        file.read_exact(&mut buf).await?;
        Ok(buf)
    }
}
```

### Integration Test Pattern with tokio-tungstenite
```rust
// In tests/ws_tests.rs
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite;

#[tokio::test]
async fn test_websocket_receives_output() {
    let tmp = tempfile::tempdir().unwrap();
    let app = build_test_app(tmp.path());

    // Bind to random port
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(axum::serve(listener, app).into_future());

    // Spawn a session via REST first
    // ... POST /api/v1/sessions ...

    // Connect WebSocket
    let (mut ws, _) = tokio_tungstenite::connect_async(
        format!("ws://{}/api/v1/sessions/{}/ws", addr, session_id)
    ).await.unwrap();

    // Receive the initial connected message
    let msg = ws.next().await.unwrap().unwrap();
    let text = match msg {
        tungstenite::Message::Text(t) => t,
        other => panic!("Expected text, got {:?}", other),
    };
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["type"], "connected");

    // Send input
    let write_msg = serde_json::json!({"type": "write", "input": "hello"});
    ws.send(tungstenite::Message::text(write_msg.to_string())).await.unwrap();

    // Should receive output message with the echoed text
    // ...
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| axum 0.7 `WebSocketUpgrade` | axum 0.8 `WebSocketUpgrade` (same API) | axum 0.8.0, Jan 2025 | Minor API changes; `Message::Text` now takes `Utf8Bytes` instead of `String` |
| Hand-rolled base64 | `base64` crate | Already in dep tree | More reliable, handles all RFC variants |
| `tokio::sync::mpsc` per client | `tokio::sync::broadcast` | Stable since tokio 1.0 | Purpose-built for fan-out; simpler client management |

**Deprecated/outdated:**
- `axum::extract::ws::Message::Text(String)` -- in axum 0.8, `Text` takes `Utf8Bytes`, not `String`. Use `.into()` for conversion.
- `tungstenite` 0.20 -- current is 0.26+. Ensure `tokio-tungstenite` version is compatible with axum's internal tungstenite.

## Discretion Recommendations

### Broadcast Channel Buffer Size
**Recommendation:** 256 messages. At 4KB max per PTY read, this is ~1MB of in-flight data. This provides sufficient buffer for a slow WebSocket client while not consuming excessive memory. A typical agent produces output in bursts; 256 messages covers several seconds of burst output.

### WebSocket Frame Compression
**Recommendation:** Do not implement permessage-deflate. axum's tungstenite integration does not support it natively. The messages are already compact (base64 adds ~33% overhead, typical message is a few KB). For LAN/localhost use this is negligible. If needed for remote deployment, this can be handled by a reverse proxy (Caddy) in Phase 9.

### Reconnection Handshake Protocol
**Recommendation:** Use query parameter `?cursor=<byte_offset>` on the WebSocket URL rather than an initial message. This simplifies the handler -- the cursor is available before the socket is established, allowing the handler to read the file delta synchronously before subscribing to broadcast.

Example: `ws://host/api/v1/sessions/{id}/ws?cursor=12345`

If no cursor parameter, send ring buffer contents (fast path). If cursor is 0 or absent, same behavior. If cursor > 0, read delta from file starting at offset.

### Error Handling for WebSocket Upgrade Failures
**Recommendation:** Return standard HTTP error responses before upgrade. Validate the session UUID and check session existence before calling `ws.on_upgrade()`. If session doesn't exist, return 404 JSON. If UUID is invalid, return 400 JSON. The upgrade only proceeds for valid, existing sessions.

### Metrics Push vs Poll
**Recommendation:** Push metrics via WebSocket. The metrics polling task already runs every 5 seconds. Add a second broadcast channel (or a variant of `OutputEvent`) for metrics updates. This avoids the client needing a separate polling loop and reduces HTTP overhead. The `metrics` message type is already defined in the protocol.

## Open Questions

1. **Output file path access from WebSocket handler**
   - What we know: `SessionOutput` stores the `File` handle but not the path. The path is constructed in `SessionManager::spawn()` as `sessions_dir.join(format!("{}.log", id))`.
   - What's unclear: The WebSocket handler needs the file path for cursor-based reconnection reads.
   - Recommendation: Store the output file path in `SessionHandle` (add a `output_path: PathBuf` field) so the WS handler can read the file directly for history replay.

2. **Session state change notification**
   - What we know: `SessionState` is currently only set at spawn time (always `Running`). There is no mechanism to detect when a child process exits and update the state.
   - What's unclear: How to detect process exit and broadcast a `StateChange` event.
   - Recommendation: The reader task already detects EOF/EIO (process exit). After the read loop ends, publish `OutputEvent::StateChange(SessionState::Exited(code))` to the broadcast channel. Retrieve the exit code via `child.try_wait()` or by checking the reader task's join result.

3. **Interaction between `write()` and WebSocket `write` messages**
   - What we know: The existing `SessionManager::write()` appends a newline after input. WebSocket `write` messages may or may not want a newline.
   - What's unclear: Should WebSocket write messages go through the same `SessionManager::write()` method?
   - Recommendation: Add a `write_raw()` method that does NOT append a newline. The WebSocket handler uses `write_raw()` for raw input, while the REST endpoint keeps `write()` for backward compatibility.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) |
| Config file | Cargo.toml `[dev-dependencies]` |
| Quick run command | `cargo test -p agtxd --test ws_tests` |
| Full suite command | `cargo test -p agtxd` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| INFRA-02 | WebSocket upgrade succeeds for valid session | integration | `cargo test -p agtxd --test ws_tests::test_ws_upgrade_succeeds -x` | Wave 0 |
| INFRA-02 | WebSocket upgrade returns 404 for missing session | integration | `cargo test -p agtxd --test ws_tests::test_ws_upgrade_404 -x` | Wave 0 |
| WS-01 | Client receives output message after PTY produces output | integration | `cargo test -p agtxd --test ws_tests::test_ws_receives_live_output -x` | Wave 0 |
| WS-01 | Multiple clients receive same output simultaneously | integration | `cargo test -p agtxd --test ws_tests::test_ws_multiple_clients -x` | Wave 0 |
| WS-02 | Output persisted to disk (already tested) | unit | `cargo test -p agtxd --test session_tests -x` | Exists |
| WS-03 | Reconnecting with cursor receives delta from file | integration | `cargo test -p agtxd --test ws_tests::test_ws_cursor_reconnection -x` | Wave 0 |
| WS-03 | REST output endpoint supports offset/limit | integration | `cargo test -p agtxd --test ws_tests::test_output_offset_limit -x` | Wave 0 |
| WS-04 | Client receives connected message on WebSocket open | integration | `cargo test -p agtxd --test ws_tests::test_ws_connected_message -x` | Wave 0 |
| WS-04 | Client receives state change on session exit | integration | `cargo test -p agtxd --test ws_tests::test_ws_state_change_on_exit -x` | Wave 0 |
| WS-05 | Daemon streams continuously (no flow control) | integration | Covered by WS-01 test | N/A |

### Sampling Rate
- **Per task commit:** `cargo test -p agtxd --test ws_tests -x`
- **Per wave merge:** `cargo test -p agtxd`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/agtxd/tests/ws_tests.rs` -- WebSocket integration tests (covers INFRA-02, WS-01, WS-03, WS-04)
- [ ] `Cargo.toml` dev-dependencies: `tokio-tungstenite = "0.26"`, `futures-util = "0.3"`
- [ ] axum `"ws"` feature flag added to workspace `Cargo.toml`

## Sources

### Primary (HIGH confidence)
- axum 0.8.8 -- currently resolved in `Cargo.lock`; WebSocket support via `"ws"` feature confirmed via [docs.rs axum::extract::ws](https://docs.rs/axum/latest/axum/extract/ws/index.html)
- tokio broadcast channel -- [official docs](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html) confirmed `RecvError::Lagged` behavior, capacity semantics
- [axum WebSocket example](https://github.com/tokio-rs/axum/blob/main/examples/websockets/src/main.rs) -- official example showing handler pattern, split, ping/pong
- [axum testing-websockets example](https://github.com/tokio-rs/axum/blob/main/examples/testing-websockets/src/main.rs) -- official integration test pattern with `tokio-tungstenite`
- Existing codebase: `SessionOutput`, `SessionManager`, `SessionHandle`, `AppState` -- read directly from source

### Secondary (MEDIUM confidence)
- [axum::extract::ws::Message](https://docs.rs/axum/latest/axum/extract/ws/enum.Message.html) -- `Text(Utf8Bytes)`, `Binary(Bytes)`, `Ping(Bytes)`, `Pong(Bytes)`, `Close(Option<CloseFrame>)` variants confirmed
- tokio-tungstenite latest version 0.26.x -- from [crates.io listing](https://crates.io/crates/tokio-tungstenite)

### Tertiary (LOW confidence)
- permessage-deflate support in axum/tungstenite -- could not confirm native support; recommendation to skip is based on absence of evidence

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries already in use or in transitive dependency tree; only need feature flag and dev-dep
- Architecture: HIGH -- broadcast channel pattern is well-documented; codebase structure is clear and extension points are obvious
- Pitfalls: HIGH -- broadcast lag handling, lock ordering, and race conditions are well-documented in tokio docs
- Message protocol: HIGH -- fully specified in CONTEXT.md decisions
- Test patterns: HIGH -- official axum testing-websockets example provides exact pattern needed

**Research date:** 2026-03-04
**Valid until:** 2026-04-04 (stable ecosystem, no fast-moving parts)
