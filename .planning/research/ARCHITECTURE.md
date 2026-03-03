# Architecture Patterns

**Domain:** Rust daemon + SvelteKit frontend for PTY-based agent management
**Researched:** 2026-03-03

## Recommended Architecture

### High-Level Overview

```
Browser <--HTTPS--> Caddy <--forward_auth--> oauth2-proxy
                      |
                      v
               +-----------+
               |  SvelteKit |  (static build served by Caddy)
               |  Frontend  |
               +-----------+
                      |
          REST (HTTP) | WebSocket (WS)
                      v
               +-----------+
               |   agtxd   |  (Rust daemon, axum)
               |  Daemon   |
               +-----------+
               /     |     \
              v      v      v
          [PTY    [SQLite] [Git/Agent
         Sessions]         Operations]
```

The system is a three-layer architecture:

1. **Edge layer** -- Caddy reverse proxy with oauth2-proxy for GitHub OAuth authentication
2. **Frontend layer** -- SvelteKit 5 static build (adapter-static), served directly by Caddy as files
3. **Backend layer** -- `agtxd` Rust daemon (axum), owning all state: PTY processes, SQLite databases, git worktrees, agent lifecycles

The daemon is the single source of truth. The frontend is a thin client that renders state received via REST and WebSocket.

### Component Boundaries

| Component | Responsibility | Communicates With |
|-----------|---------------|-------------------|
| **Caddy** | TLS termination, static file serving, forward_auth to oauth2-proxy, reverse proxy to agtxd (REST + WS) | oauth2-proxy (HTTP), agtxd (HTTP/WS), browser (HTTPS) |
| **oauth2-proxy** | GitHub OAuth flow, session cookies, username allowlist enforcement | GitHub OAuth (HTTPS), Caddy (HTTP response) |
| **SvelteKit frontend** | Kanban board UI, task management forms, agent output viewer, system metrics display | agtxd (REST for CRUD, WebSocket for streaming) |
| **agtxd daemon** | PTY lifecycle, task workflow engine, artifact polling, plugin resolution, git worktree management, agent spawning, session history persistence, system metrics collection | SQLite (disk), PTY processes (fd), git CLI (subprocess), agent CLIs (subprocess), filesystem (worktrees, history, artifacts) |
| **SQLite databases** | Task/project persistence (existing schema, unchanged) | agtxd (rusqlite) |
| **PTY sessions** | Agent process isolation, bidirectional I/O | agtxd (portable-pty read/write) |

### Data Flow

```
User Input Flow (browser -> agent):
  Browser
    -> WebSocket message (JSON: {type: "input", session_id, text})
    -> agtxd WS handler
    -> PTY master writer (session lookup)
    -> Agent process stdin

Agent Output Flow (agent -> browser):
  Agent process stdout
    -> PTY master reader (tokio::task per session)
    -> Session buffer (append to ring + write to disk history)
    -> tokio::broadcast channel (per session)
    -> All subscribed WebSocket connections
    -> Browser renders structured output

Task Lifecycle Flow:
  Browser REST POST /api/tasks/{id}/advance
    -> agtxd validates transition
    -> Creates worktree (if Planning)
    -> Spawns PTY + agent (if Planning)
    -> Sends skill command to PTY (if Running)
    -> Creates PR via gh CLI (if Review)
    -> Cleans up worktree + kills PTY (if Done)
    -> Updates SQLite
    -> Returns updated task JSON
    -> Browser updates kanban board

Artifact Polling Flow (daemon-internal):
  Periodic tokio::interval (2s, matching existing TTL)
    -> Check artifact file paths per active task
    -> Update PhaseStatus in memory
    -> Broadcast status change to subscribed WebSocket clients
```

## Daemon Architecture (agtxd)

### Axum Router Structure

```
Router::new()
    // REST API
    .nest("/api", api_router())
    // WebSocket endpoint
    .route("/ws", get(ws_handler))
    // Application state
    .with_state(app_state)

fn api_router() -> Router<AppState> {
    Router::new()
        // Projects
        .route("/projects", get(list_projects).post(create_project))
        .route("/projects/{id}", get(get_project))
        // Tasks (scoped to project)
        .route("/projects/{id}/tasks", get(list_tasks).post(create_task))
        .route("/projects/{id}/tasks/{task_id}", get(get_task).put(update_task).delete(delete_task))
        .route("/projects/{id}/tasks/{task_id}/advance", post(advance_task))
        .route("/projects/{id}/tasks/{task_id}/resume", post(resume_task))
        // Sessions
        .route("/sessions/{session_id}/history", get(get_session_history))
        .route("/sessions/{session_id}/resize", post(resize_session))
        // Config
        .route("/config", get(get_config))
        .route("/plugins", get(list_plugins))
        // System
        .route("/system/metrics", get(get_system_metrics))
}
```

### Application State

```rust
#[derive(Clone)]
struct AppState {
    // Shared, concurrent-safe state
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    // Session manager owns all PTY processes
    sessions: SessionManager,
    // Database access (rusqlite Connection is not Send, so wrap per-request)
    db_pool: DatabasePool,
    // Configuration (read-mostly, reload on SIGHUP)
    config: ArcSwap<MergedConfig>,
    // Agent registry (immutable after startup)
    agent_registry: Arc<dyn AgentRegistry>,
    // Git operations
    git_ops: Arc<dyn GitOperations>,
    git_provider_ops: Arc<dyn GitProviderOperations>,
    // System metrics collector
    system_monitor: SystemMonitor,
    // Shutdown signal
    shutdown: CancellationToken,
}
```

Use `arc-swap` for config (lock-free reads, rare writes). Use `tokio::sync::RwLock<HashMap<String, Session>>` inside `SessionManager` because session creation/destruction is infrequent relative to reads.

For SQLite: rusqlite's `Connection` is `!Send`, so use a dedicated thread with `tokio::sync::mpsc` commands (similar to the `r2d2` pattern but simpler for SQLite). Alternatively, use multiple connections behind `deadpool-sqlite` or a simple connection-per-request on `spawn_blocking`. The existing codebase already uses synchronous rusqlite -- wrapping calls in `spawn_blocking` is the lowest-friction migration path.

### WebSocket Handler Pattern

```rust
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_connection(socket, state))
}

async fn handle_ws_connection(socket: WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // Client sends subscription messages to indicate which sessions to follow
    // Daemon sends output chunks, status updates, metrics

    // Outbound: fan-out from broadcast channels
    let send_task = tokio::spawn(async move {
        // Subscribe to relevant broadcast channels
        // Forward messages to ws_tx
    });

    // Inbound: parse client commands (input, subscribe, resize)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match parse_ws_message(msg) {
                WsCommand::Subscribe(session_id) => { /* add broadcast rx */ }
                WsCommand::Input(session_id, text) => { /* write to PTY */ }
                WsCommand::Resize(session_id, cols, rows) => { /* resize PTY */ }
                WsCommand::Unsubscribe(session_id) => { /* drop broadcast rx */ }
            }
        }
    });

    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
}
```

### WebSocket Protocol

Use JSON messages for simplicity (agent output is text, not high-bandwidth binary). Define a typed message enum:

```rust
// Client -> Daemon
enum WsClientMsg {
    Subscribe { session_id: String },
    Unsubscribe { session_id: String },
    Input { session_id: String, data: String },
    Resize { session_id: String, cols: u16, rows: u16 },
}

// Daemon -> Client
enum WsDaemonMsg {
    Output { session_id: String, data: String },
    PhaseStatus { task_id: String, status: PhaseStatus },
    TaskUpdate { task: Task },
    SystemMetrics { cpu: f32, memory_used: u64, memory_total: u64, disk_used: u64, disk_total: u64, load: [f64; 3] },
    SessionEnded { session_id: String, exit_code: Option<i32> },
    Error { message: String },
}
```

### PTY Lifecycle Management

```rust
struct SessionManager {
    sessions: RwLock<HashMap<String, Session>>,
}

struct Session {
    // PTY handle
    pty_master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    // Output fan-out
    broadcast_tx: broadcast::Sender<OutputChunk>,
    // History
    history_writer: HistoryWriter,
    // Metadata
    task_id: String,
    created_at: Instant,
    last_output_at: AtomicInstant,
}
```

**Spawn flow:**

1. `native_pty_system().openpty(PtySize { rows: 24, cols: 120, .. })` -- creates PTY pair
2. Build `CommandBuilder` with agent command (from `AgentOperations::build_interactive_command`)
3. `pair.slave.spawn_command(cmd)` -- starts agent in PTY
4. `pair.master.try_clone_reader()` -- get async reader for output
5. `pair.master.take_writer()` -- get writer for input
6. Spawn tokio task for reader loop: read chunks, append to history file, broadcast to subscribers
7. Store session in `SessionManager`

**Reader task (per session):**

```rust
async fn pty_reader_loop(
    reader: Box<dyn Read + Send>,
    broadcast_tx: broadcast::Sender<OutputChunk>,
    history: HistoryWriter,
    last_output: Arc<AtomicInstant>,
) {
    // portable-pty reader is synchronous, so use spawn_blocking
    // with a small buffer and a channel back to async land
    let (chunk_tx, mut chunk_rx) = mpsc::channel(64);

    tokio::task::spawn_blocking(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,  // EOF: process exited
                Ok(n) => {
                    let data = buf[..n].to_vec();
                    if chunk_tx.blocking_send(data).is_err() {
                        break;  // receiver dropped
                    }
                }
                Err(_) => break,
            }
        }
    });

    while let Some(chunk) = chunk_rx.recv().await {
        last_output.store(Instant::now());
        history.append(&chunk).await;
        let _ = broadcast_tx.send(OutputChunk { data: chunk, timestamp: Instant::now() });
    }
}
```

**Key insight:** portable-pty's reader is synchronous (`impl Read`), not async. Bridge to tokio via `spawn_blocking` + `mpsc` channel. This is unavoidable with portable-pty and is the standard pattern.

**Resize:** `pty_master.resize(PtySize { rows, cols, .. })` -- called when frontend reports viewport change.

**Kill:** `child.kill()` or drop the session to close the PTY, which sends SIGHUP to the child process group.

### Session History Persistence

Each session's output is appended to a file on disk:

```
~/.config/agtxd/history/{task_id}.log
```

Format: raw bytes as produced by the PTY (includes ANSI escape sequences). This preserves full fidelity for replay. Append-only writes via `BufWriter` flushed on each chunk (or at 100ms intervals for batching).

**Reconnection protocol:**

1. Client connects via WebSocket, subscribes to session
2. Client sends `Subscribe { session_id, from_offset: Option<u64> }`
3. If `from_offset` is None: daemon sends nothing (client loads history via REST)
4. If `from_offset` is Some(n): daemon streams from offset n onward, then live
5. Client fetches history lazily via `GET /sessions/{id}/history?offset=0&limit=65536`
6. Frontend uses virtualized scroll for large histories

**Summary banner on reconnect:** The daemon does NOT parse/summarize output. The frontend receives raw history and renders it. A "summary banner" can be implemented frontend-side by looking at the last N lines of output and displaying them prominently at reconnection time.

### Artifact Polling

Reuse the existing artifact detection logic. The current TUI polls every 100ms with 2s cache TTL. The daemon uses a `tokio::time::interval(Duration::from_secs(2))` per active task (or a single sweeper task iterating all active tasks). When status changes, broadcast via WebSocket.

### System Metrics

```rust
struct SystemMonitor {
    sys: Mutex<sysinfo::System>,
}

impl SystemMonitor {
    fn snapshot(&self) -> SystemMetrics {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        sys.refresh_disks();
        SystemMetrics {
            cpu_usage: sys.global_cpu_usage(),
            memory_used: sys.used_memory(),
            memory_total: sys.total_memory(),
            // ...
        }
    }
}
```

Polled every 5 seconds by a background task, broadcast to all WebSocket subscribers.

## Frontend Architecture (SvelteKit)

### Build Strategy

Use `adapter-static` to produce a static SPA. Caddy serves the built files directly. No Node.js server in production. All data fetching happens client-side via REST and WebSocket to agtxd.

### Route Structure

```
src/routes/
  +layout.svelte          # Shell: sidebar nav, WebSocket provider
  +layout.ts              # Client-side: fetch projects list
  +page.svelte            # Dashboard (multi-project overview)
  projects/
    [id]/
      +layout.svelte      # Project layout: kanban board chrome
      +layout.ts           # Fetch project + tasks
      +page.svelte         # Kanban board view
      tasks/
        [taskId]/
          +page.svelte     # Task detail / agent output view
  system/
    +page.svelte           # System metrics + service logs
  settings/
    +page.svelte           # Plugin selection, agent config
```

### Component Hierarchy

```
App Shell (+layout.svelte)
  |-- Sidebar (project list, navigation)
  |-- WebSocket Provider (context, connection management)
  |
  +-- Dashboard (+page.svelte)
  |     |-- ProjectCard (per project summary)
  |
  +-- Project Layout (projects/[id]/+layout.svelte)
  |     |-- Kanban Board (+page.svelte)
  |     |     |-- Column (Backlog, Planning, Running, Review, Done)
  |     |     |     |-- TaskCard (title, status badge, phase indicator)
  |     |     |-- TaskCreateDialog
  |     |     |-- PluginSelector
  |     |
  |     +-- Task Detail (tasks/[taskId]/+page.svelte)
  |           |-- AgentOutputView (structured output renderer)
  |           |     |-- OutputBlock (per-chunk with timestamp)
  |           |     |-- VirtualScroller (windowed rendering)
  |           |-- ActionBar (approve, reject, free text input)
  |           |-- TaskMetadata (status, agent, plugin, phase)
  |           |-- DiffView (git diff display)
  |
  +-- System (+page.svelte)
        |-- MetricsPanel (CPU, RAM, disk gauges)
        |-- ServiceLogs (streaming log viewer)
```

### WebSocket Store Pattern

Use a single WebSocket connection managed at the layout level, with Svelte 5 runes ($state) for reactive state:

```typescript
// lib/stores/websocket.svelte.ts

class WebSocketStore {
    #socket: WebSocket | null = null;
    #reconnectTimer: number | null = null;

    connected = $state(false);

    // Per-session output buffers
    sessions: Record<string, SessionOutput> = $state({});

    // Task status updates
    taskStatuses: Record<string, PhaseStatus> = $state({});

    // System metrics
    metrics = $state<SystemMetrics | null>(null);

    connect(url: string) {
        this.#socket = new WebSocket(url);
        this.#socket.onopen = () => { this.connected = true; };
        this.#socket.onclose = () => {
            this.connected = false;
            this.#scheduleReconnect(url);
        };
        this.#socket.onmessage = (event) => {
            const msg = JSON.parse(event.data);
            this.#handleMessage(msg);
        };
    }

    subscribe(sessionId: string) {
        this.#send({ type: 'subscribe', session_id: sessionId });
    }

    sendInput(sessionId: string, data: string) {
        this.#send({ type: 'input', session_id: sessionId, data });
    }

    #scheduleReconnect(url: string) {
        // Exponential backoff: 1s, 2s, 4s, 8s, max 30s
        this.#reconnectTimer = setTimeout(() => this.connect(url), delay);
    }

    #handleMessage(msg: WsDaemonMsg) {
        switch (msg.type) {
            case 'output':
                this.sessions[msg.session_id]?.append(msg.data);
                break;
            case 'phase_status':
                this.taskStatuses[msg.task_id] = msg.status;
                break;
            case 'system_metrics':
                this.metrics = msg;
                break;
        }
    }
}

export const ws = new WebSocketStore();
```

Provide via Svelte context in the root layout. Components access it reactively.

### Virtualized Scroll for History

Use `svelte-virtual-infinite-list` or a custom intersection-observer-based scroller. The agent output view loads history in 64KB pages:

1. On mount: fetch `GET /sessions/{id}/history?offset=0&limit=65536` (last page)
2. Render with virtual list (only DOM nodes in viewport)
3. On scroll-to-top: fetch previous page, prepend to buffer
4. Live output appended at bottom via WebSocket

This matches the PROJECT.md requirement for "lazy-loaded with virtualized infinite scrollback."

## Monorepo Structure

Use a single git repository with a Cargo workspace for the Rust side and a top-level `web/` directory for SvelteKit:

```
agtx/
  Cargo.toml              # Workspace root
  crates/
    agtx-core/            # Shared core: config, db, models, skills, git ops, agent ops
      Cargo.toml
      src/
        lib.rs
        config/
        db/
        git/
        agent/
        skills.rs
    agtxd/                # Daemon binary: axum server, PTY management, WebSocket
      Cargo.toml
      src/
        main.rs
        api/              # REST handlers
        ws/               # WebSocket handler
        pty/              # PTY session manager
        system/           # sysinfo metrics, journalctl streaming
  web/                    # SvelteKit frontend
    package.json
    svelte.config.js
    src/
      routes/
      lib/
        stores/
        components/
        api/              # REST client functions
  deploy/                 # Deployment configs
    Caddyfile
    agtxd.service
    agtxd-web.service     # Optional: build step only
    oauth2-proxy.service
    oauth2-proxy.cfg
```

**Why monorepo:** The daemon and frontend share API type definitions (via TypeScript types generated from Rust structs or a shared OpenAPI spec). Co-locating them ensures API contract changes are atomic. The Cargo workspace means `agtx-core` is shared between the (now-retired) TUI and the daemon without code duplication.

**Why Cargo workspace with `crates/` directory:** Extract the core logic (`config`, `db`, `models`, `skills`, `git`, `agent`) into `agtx-core`. The daemon (`agtxd`) depends on `agtx-core` and adds axum, WebSocket, PTY management. This keeps the core testable and reusable. The existing TUI binary could remain as a separate crate during transition but is ultimately retired.

## Deployment Architecture

### Systemd Services

```ini
# /etc/systemd/system/agtxd.service
[Unit]
Description=AGTX Web Daemon
After=network.target

[Service]
Type=simple
User=agtx
ExecStart=/usr/local/bin/agtxd --config /etc/agtxd/config.toml
Restart=on-failure
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

### Caddy Configuration

```caddyfile
agtx.example.com {
    # Auth: forward to oauth2-proxy
    forward_auth localhost:4180 {
        uri /oauth2/auth
        header_up -Connection  # CRITICAL: strip Connection header for WS compat
        copy_headers X-Auth-Request-User X-Auth-Request-Email
    }

    # API + WebSocket -> daemon
    handle /api/* {
        reverse_proxy localhost:3000
    }
    handle /ws {
        reverse_proxy localhost:3000
    }

    # OAuth2 proxy endpoints (sign-in, callback)
    handle /oauth2/* {
        reverse_proxy localhost:4180
    }

    # Static frontend files
    handle {
        root * /var/www/agtxd/web
        try_files {path} /index.html
        file_server
    }
}
```

**Critical detail:** The `header_up -Connection` in forward_auth is required. Without it, Caddy sends the WebSocket upgrade headers to oauth2-proxy, which fails. This header manipulation only affects the auth check copy, not the actual proxied request.

### oauth2-proxy Configuration

```bash
oauth2-proxy \
  --provider=github \
  --github-user=<your-username> \
  --client-id=<github-oauth-app-id> \
  --client-secret=<github-oauth-app-secret> \
  --cookie-secret=<random-32-bytes-base64> \
  --upstream=static://202 \
  --http-address=127.0.0.1:4180 \
  --reverse-proxy=true \
  --set-xauthrequest=true
```

## Patterns to Follow

### Pattern 1: Trait-Based Core Extraction

**What:** Extract existing trait-based abstractions (`GitOperations`, `AgentOperations`, `AgentRegistry`) into `agtx-core` unchanged. The daemon imports them directly.

**When:** Always. This is the foundation for reuse.

**Why:** The existing codebase already designed these traits for testability. That same abstraction enables the daemon to use identical core logic without modification.

```rust
// agtx-core/src/lib.rs
pub mod agent;
pub mod config;
pub mod db;
pub mod git;
pub mod skills;
```

### Pattern 2: Broadcast Channel Per Session

**What:** Each PTY session owns a `tokio::broadcast::Sender<OutputChunk>`. WebSocket handlers subscribe by calling `sender.subscribe()`.

**When:** Any time a client wants live output from a session.

**Why:** Broadcast channels handle fan-out naturally. Multiple WebSocket connections (tabs, reconnections) can subscribe to the same session. Lagging receivers get `RecvError::Lagged` which is handled by catching up from disk history.

```rust
// In SessionManager::create_session
let (broadcast_tx, _) = broadcast::channel::<OutputChunk>(1024);
// Store broadcast_tx in Session

// In WebSocket handler
let mut rx = session.broadcast_tx.subscribe();
while let Ok(chunk) = rx.recv().await {
    ws_tx.send(Message::Text(serde_json::to_string(&chunk)?)).await?;
}
```

### Pattern 3: spawn_blocking Bridge for Synchronous I/O

**What:** Use `tokio::task::spawn_blocking` to bridge portable-pty's synchronous Read/Write with tokio's async runtime.

**When:** All PTY I/O operations.

**Why:** portable-pty's `MasterPty::try_clone_reader()` returns `Box<dyn Read>`, not async. Blocking on the tokio runtime thread pool is the correct bridge. The mpsc channel back to async land enables integration with broadcast.

### Pattern 4: REST for CRUD, WebSocket for Streaming

**What:** Use REST endpoints for task creation, updates, deletion, history fetching. Use WebSocket for live output streaming, status updates, metrics.

**When:** Always. Don't mix concerns.

**Why:** REST is simpler for request-response operations (creating tasks, fetching history pages). WebSocket is necessary only for real-time streaming. This separation makes the API testable with standard HTTP testing tools.

## Anti-Patterns to Avoid

### Anti-Pattern 1: Shared Mutable State with Fine-Grained Locks

**What:** Putting individual fields behind separate Mutex/RwLock instances scattered across the codebase.

**Why bad:** Lock ordering bugs, deadlocks, cognitive overhead. Difficult to reason about invariants.

**Instead:** Centralize mutable state in `SessionManager` with a single RwLock over the session map. Individual sessions are mostly append-only (output buffer) and use lock-free structures (broadcast channel, atomic timestamps).

### Anti-Pattern 2: Raw Terminal Emulation in Browser

**What:** Embedding xterm.js or similar terminal emulator to render PTY output pixel-perfectly.

**Why bad:** PROJECT.md explicitly states "Structured output over raw terminal" and "Raw terminal emulator in browser" is out of scope. Terminal emulation adds massive complexity (ANSI parsing, cursor management, alternate screen handling).

**Instead:** Parse agent output into structured blocks (status messages, file changes, progress indicators, plain text) and render with purpose-built Svelte components. Fall back to plain monospace text for unrecognized output.

### Anti-Pattern 3: Node.js Server in Production

**What:** Running SvelteKit in SSR mode with a Node.js server.

**Why bad:** Adds another runtime dependency, another process to manage, another failure point. The frontend has no server-side logic -- all data comes from agtxd.

**Instead:** Use `adapter-static` to build a pure SPA. Caddy serves the files. Zero Node.js in production.

### Anti-Pattern 4: Polling REST for Real-Time Data

**What:** Frontend polling `GET /tasks` every second to check for status changes.

**Why bad:** Wastes bandwidth, adds latency, scales poorly with many tasks.

**Instead:** Push status changes and output over the single WebSocket connection. REST is only for initial page loads and CRUD mutations.

## Scalability Considerations

| Concern | At 5 sessions | At 20 sessions | At 50 sessions |
|---------|---------------|----------------|----------------|
| PTY reader threads | 5 blocking threads (negligible) | 20 blocking threads (fine) | 50 blocking threads (monitor thread pool, increase `spawn_blocking` pool if needed) |
| Broadcast channels | 5 channels, ~1KB/s each | 20 channels, some idle | Memory: ~50MB ring buffers total, prune idle sessions |
| Disk history | ~10MB/session/day | ~200MB/day | ~500MB/day, add rotation/retention policy |
| WebSocket connections | 1-2 (one browser) | 1-2 (same user, few tabs) | 1-2 (single user system, not a scaling concern) |
| SQLite | Negligible load | Negligible load | Negligible load (single user, infrequent writes) |

This is a **single-user system**. The scaling concern is number of concurrent agent sessions, not number of users. 50 concurrent PTY sessions with history persistence is the practical ceiling, and well within system limits.

## Suggested Build Order

The build order reflects dependency chains. Later components depend on earlier ones.

```
Phase 1: Core Extraction
  Extract agtx-core from existing codebase
  (config, db, models, skills, git, agent modules)
    |
    v
Phase 2: Daemon Foundation
  agtxd binary with axum skeleton
  REST API for tasks/projects (using agtx-core)
  SQLite access via spawn_blocking
    |
    v
Phase 3: PTY Management
  SessionManager with portable-pty
  PTY spawn/read/write/kill lifecycle
  Session history persistence to disk
    |
    v
Phase 4: WebSocket Streaming
  WebSocket handler with split pattern
  Broadcast channel integration
  Subscribe/unsubscribe/input protocol
  Reconnection + history lazy loading
    |
    v
Phase 5: Frontend Foundation
  SvelteKit project with adapter-static
  WebSocket store with reconnection
  Kanban board (task CRUD via REST)
  Agent output view (WebSocket streaming)
    |
    v
Phase 6: Deployment + Auth
  Caddy + oauth2-proxy configuration
  systemd services
  Structured logging (tracing + tracing-appender)
    |
    v
Phase 7: System Monitoring
  sysinfo metrics collection + broadcast
  journalctl log streaming
  System tab in frontend
```

**Why this order:**
- Phase 1 is prerequisite for everything (shared core)
- Phase 2 must come before 3 (daemon exists before adding PTY)
- Phase 3 must come before 4 (PTY produces output before streaming it)
- Phase 4 and 5 could partially overlap (frontend REST can be built while WS is in progress)
- Phase 6 is deployment concern, can be deferred until the app works locally
- Phase 7 is enhancement, independent of core workflow

## Sources

### HIGH Confidence (official docs)
- [axum WebSocket extract docs](https://docs.rs/axum/latest/axum/extract/ws/index.html)
- [axum official WebSocket example](https://github.com/tokio-rs/axum/blob/main/examples/websockets/src/main.rs)
- [portable-pty API docs](https://docs.rs/portable-pty/latest/portable_pty/)
- [tokio broadcast channel docs](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)
- [tokio channels tutorial](https://tokio.rs/tokio/tutorial/channels)
- [tracing-appender docs](https://docs.rs/tracing-appender/latest/tracing_appender/)
- [sysinfo crate docs](https://docs.rs/sysinfo/latest/sysinfo/)
- [Caddy forward_auth directive](https://caddyserver.com/docs/caddyfile/directives/forward_auth)
- [Caddy reverse_proxy directive](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
- [OAuth2 Proxy Caddy integration](https://oauth2-proxy.github.io/oauth2-proxy/next/configuration/integrations/caddy/)
- [SvelteKit routing docs](https://svelte.dev/docs/kit/routing)
- [Caddy forward_auth + WebSocket issue #5430](https://github.com/caddyserver/caddy/issues/5430) -- workaround: `header_up -Connection`

### MEDIUM Confidence (verified via multiple sources)
- [axum shared state patterns](https://github.com/tokio-rs/axum/discussions/1756)
- [zmx daemon architecture](https://github.com/neurosnap/zmx) -- reference for PTY session persistence patterns
- [svelte-virtual-infinite-list](https://github.com/koheing/svelte-virtual-infinite-list)
- [SvelteKit WebSocket in v5](https://medium.com/@vinay.s.khanagavi/implementing-websockets-in-a-sveltekit-version-5-1d6c6041e9ca)
- [Rust Cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [rust-embed SvelteKit example](https://github.com/thanhnguyen2187/example-rust-embed-sveltekit)

### LOW Confidence (needs validation)
- Native WebSocket support in SvelteKit (was announced March 2025 for testing, current production status unverified)
- portable-pty async compatibility details (API is synchronous, spawn_blocking bridge is well-established pattern but specific performance characteristics under 50+ concurrent PTYs unverified)
