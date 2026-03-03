# Domain Pitfalls

**Domain:** Web-native agent management dashboard (Rust daemon + SvelteKit frontend + PTY + WebSocket)
**Researched:** 2026-03-03

## Critical Pitfalls

Mistakes that cause rewrites, data loss, or architectural dead ends.

---

### Pitfall 1: Blocking PTY Reads on the Tokio Runtime

**What goes wrong:** `portable-pty` provides synchronous `Read`/`Write` traits. Calling `.read()` on a PTY master fd inside an async task blocks the tokio worker thread. With multiple agents running, this starves the async runtime -- WebSocket handlers stop responding, health checks time out, and the entire daemon appears hung.

**Why it happens:** PTY file descriptors are not natively async-compatible. `portable-pty` does not implement `AsyncRead`/`AsyncWrite`. Developers wrap reads in `tokio::spawn_blocking`, but `spawn_blocking` tasks cannot be cancelled and will keep blocking threads alive during shutdown. With 5+ agents, the blocking thread pool fills up.

**Consequences:** Daemon becomes unresponsive under load. Graceful shutdown hangs indefinitely waiting for blocked PTY reads to return. WebSocket connections drop because the event loop is starved.

**Prevention:**
- **Use `pty-process` with the `async` feature instead of `portable-pty`.** The `pty-process` crate wraps `tokio::process::Command` and its PTY master implements `tokio::io::AsyncRead` and `tokio::io::AsyncWrite` natively. This integrates directly with tokio's event loop without blocking threads.
- If `portable-pty` must be used (e.g., for cross-platform needs later), wrap reads in a dedicated OS thread (not `spawn_blocking`) with a channel back to async code, and use `tokio::select!` with a cancellation token so the reader can be signaled to stop.
- Never call synchronous PTY read/write from inside a `tokio::spawn` task.

**Detection:** Monitor tokio runtime metrics (thread utilization). If blocking thread count approaches the limit (default 512), PTY reads are leaking into the blocking pool. Integration test: spawn 5 agents simultaneously and verify WebSocket latency stays under 100ms.

**Confidence:** HIGH -- `pty-process` async support verified via official docs. `spawn_blocking` limitations documented in tokio docs.

**Phase:** Must be decided in Phase 1 (daemon foundation). Wrong choice here requires a rewrite.

---

### Pitfall 2: Zombie Agent Processes After Daemon Crash/Restart

**What goes wrong:** When `agtxd` crashes, gets SIGKILL'd, or is restarted by systemd, child agent processes (claude, codex, gemini) become orphans. They keep running, consuming resources, holding git worktree locks, and writing to files. On next daemon start, the daemon has no record of these processes and spawns duplicates.

**Why it happens:** PTY child processes are not automatically killed when their parent dies on Linux. Unlike tmux (which manages its own server process), direct PTY management means the daemon IS the parent. No parent = zombie/orphan.

**Consequences:** Runaway agent processes consuming API credits. Duplicate agents writing to the same worktree causing git corruption. Resource exhaustion on the server. Users see "phantom" agents they cannot control.

**Prevention:**
- Set `PR_SET_PDEATHSIG` (Linux) on child processes at spawn time so they receive SIGTERM when the parent dies. Use `unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) }` in the pre-exec hook.
- Use `PR_SET_CHILD_SUBREAPER` on the daemon so it becomes the reaper for all descendant processes, catching grandchild processes too.
- Write a PID file per agent session to disk. On daemon startup, scan for stale PIDs and kill them before spawning new sessions.
- Implement graceful shutdown: on SIGTERM, iterate all managed PTYs, send SIGTERM to children, wait up to 5 seconds, then SIGKILL.
- Consider a process group per agent: `setsid()` before exec, then `killpg()` the entire group on cleanup.

**Detection:** After daemon restart, run `ps aux | grep -E 'claude|codex|gemini|opencode|copilot'` and check for orphans. Automated: startup health check that scans for PID file mismatches.

**Confidence:** HIGH -- `PR_SET_PDEATHSIG` behavior well-documented in Linux man pages. Orphan process problem is fundamental to Unix process management.

**Phase:** Phase 1 (daemon foundation). Process lifecycle must be correct from day one.

---

### Pitfall 3: Caddy `forward_auth` Sends WebSocket Upgrade to Auth Server

**What goes wrong:** When a browser opens a WebSocket connection to `wss://agtx.example.com/ws`, Caddy's `forward_auth` directive forwards the full request -- including `Connection: Upgrade` and `Upgrade: websocket` headers -- to oauth2-proxy's `/oauth2/auth` endpoint. oauth2-proxy does not expect WebSocket upgrade requests and returns an error or unexpected response, breaking WebSocket authentication entirely.

**Why it happens:** Caddy's `forward_auth` passes hop-by-hop headers to the auth backend. This is [documented as issue #5430](https://github.com/caddyserver/caddy/issues/5430) and was closed as "not planned" -- Caddy considers this expected behavior.

**Consequences:** WebSocket connections fail to authenticate. Users see the dashboard load (static assets pass auth) but live agent output never appears. Debugging is painful because HTTP requests work fine.

**Prevention:**
- Strip the `Connection` header before forwarding to the auth endpoint:
  ```caddy
  forward_auth localhost:4180 {
      uri /oauth2/auth
      header_up -Connection
      header_up -Upgrade
  }
  ```
- Alternatively, use a split routing strategy: authenticate WebSocket connections via a session cookie validated by the daemon itself (check the `_oauth2_proxy` cookie), bypassing `forward_auth` for the `/ws` path entirely.
- Test WebSocket auth separately from HTTP auth during development. Use `websocat` to manually test authenticated WebSocket connections.

**Detection:** If HTTP endpoints work but WebSocket connections fail with 401/403/502, this is almost certainly the cause. Check oauth2-proxy logs for unusual request patterns.

**Confidence:** HIGH -- Caddy issue #5430 confirmed and workaround verified from GitHub discussion and community reports.

**Phase:** Phase 3 (auth gateway) or whenever Caddy + oauth2-proxy are configured. Must be tested immediately.

---

### Pitfall 4: Unbounded WebSocket Send Buffers Causing OOM

**What goes wrong:** Agent processes produce output at variable rates -- sometimes a burst of thousands of lines per second (e.g., during `cargo build` output or large file diffs). The daemon reads PTY output and pushes it to WebSocket connections. If the browser tab is backgrounded, on a slow network, or the client cannot consume fast enough, outbound WebSocket messages queue unboundedly in server memory. With multiple agents streaming simultaneously, the daemon OOMs.

**Why it happens:** WebSocket `send()` on the server side is decoupled from actual network transmission. The kernel TCP send buffer fills, axum/tungstenite queues messages internally, and there is no default backpressure mechanism. The server gets no signal that the client is falling behind.

**Consequences:** Daemon crashes with OOM. All agent sessions are lost. On restart, history must be replayed from disk (if it was persisted).

**Prevention:**
- Use a bounded channel (`tokio::sync::mpsc::channel` with a capacity, NOT unbounded) between the PTY reader task and the WebSocket sender task. When the channel is full, drop the oldest messages (or batch/summarize them).
- Implement a "slow consumer" detection: if the send channel has been full for more than N seconds, disconnect the WebSocket with a close frame indicating the client is too slow, and let it reconnect.
- Set `max_write_buffer_size` on the WebSocket configuration to cap memory per connection. axum/tungstenite supports this.
- Batch PTY output: instead of sending every byte individually, accumulate output for 16-50ms and send as a single message. This reduces message count dramatically.
- Since this project uses structured output (not raw terminal), output parsing acts as a natural throttle -- parsed structured events are lower volume than raw bytes.

**Detection:** Monitor per-connection send queue depth. Alert if any connection's queue exceeds 1000 messages. Load test with `tc netem` to simulate slow network and verify daemon memory stays bounded.

**Confidence:** HIGH -- WebSocket backpressure is a well-documented pattern. axum/tungstenite buffer configuration documented in tungstenite crate docs.

**Phase:** Phase 2 (WebSocket streaming). Must be designed into the streaming architecture from the start, not bolted on later.

---

### Pitfall 5: Structured Output Parsing Assumes Consistent Agent Format

**What goes wrong:** The project plans to parse agent output into structured views (status, file changes, phase progress, action buttons). But each agent (Claude, Codex, Gemini, Copilot, OpenCode) has different output formats, different ANSI escape usage, different progress indicators, and different interactive prompts. Parsing breaks silently when agents update their CLI output format, which happens frequently.

**Why it happens:** Agent CLIs are designed for human consumption, not machine parsing. Their output format is an implementation detail, not a stable API. Claude Code outputs rich ANSI with spinners and tool-use blocks. Codex has different formatting. Gemini supports `--output-format stream-json` but others may not. There is no stable cross-agent output contract.

**Consequences:** Structured views show garbled or incomplete data. Action buttons (approve/reject) trigger on wrong content. Phase detection based on output parsing produces false positives. Users lose trust in the structured view and want raw terminal back.

**Prevention:**
- **Use `--output-format stream-json` (or equivalent) where available.** Claude Code supports `--output-format stream-json`, Gemini supports `--output-format json`. This gives structured, typed events instead of ANSI scraping.
- **Investigate ACP (Agent Client Protocol)** as an alternative to PTY scraping entirely. ACP provides structured JSON-RPC messages (thinking, tool calls, diffs) and is supported by Claude Code, Codex CLI, and Gemini. This could replace PTY output parsing for agents that support it.
- For agents without structured output: maintain a "raw" fallback view that shows unstyled text (strip ANSI with `strip-ansi-escapes` crate). Do not attempt deep parsing of unstructured output.
- Build the parsing layer with per-agent parser implementations behind a trait. Each parser can be updated independently when an agent changes its format.
- Version-pin agent CLIs in deployment and test parsing against specific versions.

**Detection:** If the structured view shows "unknown" status for more than 30 seconds while the raw output shows the agent is clearly working, the parser has broken. Automated: compare structured parse results against raw output heuristics.

**Confidence:** MEDIUM -- Agent output formats verified for Claude Code and Gemini. Other agents' structured output capabilities need phase-specific research. ACP is promising but adoption is still early (2025-2026).

**Phase:** Phase 2 (structured output). Design the parser trait and raw fallback early. Deep per-agent parsing can be iterative.

---

### Pitfall 6: Session History Disk I/O Blocks the Event Loop

**What goes wrong:** The project requires full session history persisted to disk for reconnect/scrollback. Naive implementation writes every PTY output chunk to SQLite or a file synchronously. Under heavy agent output (build logs, test output), this creates I/O pressure that blocks async tasks, causing WebSocket latency spikes and dropped frames.

**Why it happens:** `rusqlite` is synchronous. File I/O in Rust is synchronous. Both block the calling thread. If called from an async context without proper isolation, they block the tokio runtime.

**Consequences:** Latency spikes during heavy agent output. WebSocket messages arrive in bursts instead of smoothly. In extreme cases, the event loop stalls and connections drop.

**Prevention:**
- **Use `tokio-rusqlite` for all database operations.** It wraps rusqlite calls in a dedicated thread and provides an async `.call()` interface. This is the standard pattern for rusqlite + axum.
- Batch history writes: accumulate output for 100-500ms in memory, then write a single batch to disk. This converts many small writes into fewer large writes.
- Use WAL mode for SQLite (`PRAGMA journal_mode=WAL`). This allows concurrent reads during writes, so history queries (for reconnect scrollback) don't block ongoing writes.
- For raw session history, consider append-only files (one per session) instead of SQLite. File appends are cheaper than INSERT statements. Load into SQLite only for queries.
- Use `tracing-appender`'s `non_blocking` writer for daemon logs. The `WorkerGuard` must be held for the daemon's entire lifetime or logs will be silently dropped.

**Detection:** Monitor SQLite write latency. If p99 exceeds 10ms, batching is insufficient. Monitor tokio task poll times -- if any task takes >1ms to poll, blocking I/O is leaking into async context.

**Confidence:** HIGH -- `tokio-rusqlite` pattern well-documented. WAL mode benefits are standard SQLite knowledge.

**Phase:** Phase 1 (daemon foundation) for the async database wrapper. Phase 2 for history persistence design.

---

## Moderate Pitfalls

---

### Pitfall 7: Caddy Config Reload Kills Active WebSocket Connections

**What goes wrong:** When Caddy reloads its configuration (e.g., after a cert renewal, Caddyfile change, or `caddy reload`), it forcibly closes all active streaming connections including WebSockets. All connected browsers lose their live agent output streams simultaneously.

**Prevention:**
- Set `stream_close_delay 5m` in the Caddy reverse_proxy config to delay closing streaming connections during reload.
- Implement robust reconnection on the client side (see Pitfall 8) so connection drops are transparent.
- Avoid frequent Caddy reloads in production. Use `caddy validate` before `caddy reload`.

**Confidence:** HIGH -- documented in Caddy reverse_proxy docs (`stream_close_delay` option).

**Phase:** Phase 3 (deployment/infrastructure).

---

### Pitfall 8: WebSocket Reconnection Storms

**What goes wrong:** When the daemon restarts or Caddy reloads, all browser clients reconnect simultaneously. If reconnection uses a fixed delay (e.g., "retry every 1 second"), all clients hit the server at the exact same moment, creating a thundering herd that can overwhelm the auth proxy or the daemon itself.

**Prevention:**
- Use exponential backoff with jitter for WebSocket reconnection on the client side. Start at 1s, double up to 30s, add random jitter of 0-1s.
- On reconnect, do not replay full session history immediately. Send a lightweight "summary" first, then lazy-load history as the user scrolls.
- Set a connection limit in the daemon (e.g., max 10 concurrent WebSocket connections for single-user) to reject excess connections during storms.

**Detection:** Monitor connection rate. If more than 10 connections arrive within 1 second, a reconnection storm is occurring.

**Confidence:** HIGH -- standard distributed systems pattern.

**Phase:** Phase 2 (WebSocket client implementation).

---

### Pitfall 9: oauth2-proxy Cookie Expiry Silently Breaks Long-Running Sessions

**What goes wrong:** A user has the dashboard open for hours while agents work. The oauth2-proxy session cookie expires (default 168 hours, but the access token may expire sooner). The next HTTP request (e.g., creating a task, moving a card) fails with a redirect to the OAuth login page. Worse, the WebSocket connection may continue working (it was established before expiry) while REST calls fail, creating a confusing split-brain state.

**Prevention:**
- Set `--cookie-refresh` to a value slightly less than the access token lifetime. This causes oauth2-proxy to transparently refresh the session on regular HTTP requests.
- For WebSocket connections, implement a periodic "heartbeat" that hits an authenticated HTTP endpoint (e.g., `GET /api/health`). If this returns a redirect (302), prompt the user to re-authenticate.
- On the frontend, intercept 401/302 responses globally and show a "session expired, click to re-login" banner instead of silently failing.

**Detection:** If REST API calls start returning 302 redirects while the WebSocket is still alive, cookie expiry is the cause.

**Confidence:** MEDIUM -- oauth2-proxy cookie behavior documented, but exact interaction with WebSocket sessions needs testing.

**Phase:** Phase 3 (auth gateway).

---

### Pitfall 10: PTY Window Size (TIOCSWINSZ) Never Set or Updated

**What goes wrong:** Agent CLIs detect terminal dimensions to format output. If the PTY is spawned without setting a window size, or if the size is never updated, agents output text wrapped at the wrong width (often 80 columns by default). For structured output parsing, this means unexpected line breaks corrupt the parsed data. For agents using interactive TUI elements (spinners, progress bars), they render incorrectly.

**Prevention:**
- Set an explicit PTY size at spawn time: 200 columns by 50 rows (wide enough to avoid wrapping for most agent output).
- If using structured output modes (`--output-format stream-json`), the PTY size matters less since output is not formatted for terminal display.
- If the frontend ever supports resizable views, propagate resize events via WebSocket to the daemon, which calls `TIOCSWINSZ` on the PTY fd.

**Detection:** If agent output has unexpected `\n` characters mid-line or ANSI cursor movement sequences appear in places that break parsing, the PTY size is likely wrong.

**Confidence:** HIGH -- standard PTY behavior, well-documented in POSIX.

**Phase:** Phase 1 (PTY management).

---

### Pitfall 11: `tracing-appender` Non-Blocking Writer Drops Logs Silently

**What goes wrong:** `tracing-appender::non_blocking` returns a `WorkerGuard` that must be held for the program's entire lifetime. If the guard is dropped (e.g., moved into a scope that ends, or not stored in a long-lived variable), the background writer thread stops and all subsequent log writes are silently discarded. Additionally, if the log volume exceeds the bounded queue capacity, messages are dropped without warning.

**Prevention:**
- Store the `WorkerGuard` in a variable that lives for the entire `main()` scope. Common pattern: `let _guard = non_blocking(file_appender);` at the top of main.
- Configure the queue capacity appropriately for expected log volume. Use `NonBlockingBuilder::default().lossy(false)` only if backpressure is acceptable (it will slow down the caller).
- Set up a `lost_event_count` metric or periodic log that reports how many events were dropped.

**Detection:** If log files suddenly stop growing while the daemon is running, the guard was dropped. Check that the guard variable is not shadowed or moved.

**Confidence:** HIGH -- documented in `tracing-appender` official docs.

**Phase:** Phase 1 (daemon foundation, logging setup).

---

### Pitfall 12: systemd Service Ordering Causes Startup Failures

**What goes wrong:** The deployment has three systemd services: `agtxd.service`, `caddy.service`, and `oauth2-proxy.service`. If `agtxd` starts before Caddy, the daemon binds to its port but external requests fail. If oauth2-proxy starts before DNS/network is up, GitHub OAuth callbacks fail. Wrong ordering leads to intermittent startup failures after reboot.

**Prevention:**
- Define explicit ordering in systemd units:
  ```ini
  # agtxd.service
  [Unit]
  After=network-online.target
  Wants=network-online.target

  # caddy.service
  [Unit]
  After=network-online.target agtxd.service
  Wants=network-online.target

  # oauth2-proxy.service
  [Unit]
  After=network-online.target
  Wants=network-online.target
  ```
- Use `Type=notify` for `agtxd` if possible (requires sd-notify integration) so Caddy only starts after the daemon is actually ready to accept connections.
- Add health check endpoints and `Restart=on-failure` with `RestartSec=5` to all services.
- Use `caddy reload` instead of restart when changing configuration to avoid downtime.

**Detection:** After system reboot, if the dashboard shows a 502 error for the first 30 seconds then works, service ordering is wrong.

**Confidence:** HIGH -- standard systemd patterns.

**Phase:** Phase 4 (deployment).

---

## Minor Pitfalls

---

### Pitfall 13: SvelteKit Component Cleanup on Navigation Leaks WebSocket Connections

**What goes wrong:** If a Svelte component opens a WebSocket connection and the user navigates away (SvelteKit client-side routing), the component unmounts but the WebSocket stays open if `onDestroy` cleanup is missing. Over many navigations, the browser accumulates dozens of orphaned WebSocket connections consuming server resources.

**Prevention:**
- Always close WebSocket connections in `onDestroy` lifecycle hook.
- Centralize WebSocket management in a Svelte store or module-level singleton, not in individual components. The store persists across navigation and manages the single connection.
- Use SvelteKit's `onNavigate` or `beforeNavigate` callbacks as additional cleanup triggers.

**Confidence:** HIGH -- standard Svelte lifecycle management.

**Phase:** Phase 2 (frontend WebSocket client).

---

### Pitfall 14: SQLite Database Migration From Existing AGTX

**What goes wrong:** The current AGTX stores databases at `~/.config/agtx/` with an `index.db` and per-project `projects/{hash}.db` files. The web daemon needs to read and extend these same databases (add columns for PTY session state, history references, etc.). If migrations are not idempotent, running the daemon against existing data corrupts it.

**Prevention:**
- The existing codebase already uses `ALTER TABLE ... ADD COLUMN` with error suppression for migrations. Continue this pattern.
- Add a `schema_version` field to the database. Check on startup and refuse to run if the version is from the future (prevents old daemon from corrupting new schema).
- Back up the database directory before first web daemon startup.
- Test migrations against a copy of production data before deploying.

**Confidence:** HIGH -- existing migration pattern visible in codebase.

**Phase:** Phase 1 (daemon foundation).

---

### Pitfall 15: Agent CLI Version Drift Breaks Spawn Commands

**What goes wrong:** Agent CLIs update frequently. Claude Code might rename `--dangerously-skip-permissions` to a different flag. Codex might change `--full-auto` syntax. The daemon hardcodes these flags (as the current TUI does in `build_interactive_command`), and after an agent update, spawning fails silently or with confusing errors.

**Prevention:**
- On daemon startup, detect agent versions (e.g., `claude --version`) and log them.
- Maintain a compatibility table mapping agent versions to flag sets.
- When spawn fails, capture stderr from the PTY child process and surface it to the user in the dashboard instead of silently failing.
- Consider using `--output-format stream-json` launch modes that are more likely to remain stable than interactive flags.

**Detection:** If an agent task moves to "Planning" but immediately shows "Exited" status, check the PTY output for flag/argument errors.

**Confidence:** MEDIUM -- flag names verified against current CLIs, but future stability is uncertain.

**Phase:** Phase 1 (agent spawning).

---

### Pitfall 16: History Replay on Reconnect Overwhelms the Client

**What goes wrong:** User reconnects after being away for an hour. The daemon sends the full session history (potentially megabytes of agent output across multiple sessions) over the WebSocket. The browser freezes trying to render it all, or the WebSocket send queue backs up (see Pitfall 4).

**Prevention:**
- On reconnect, send only a summary banner (last 20 lines per active session + phase status).
- Implement lazy loading: the frontend requests history in pages (e.g., 100 lines at a time) as the user scrolls up.
- Use a separate REST endpoint for history retrieval (HTTP with pagination) rather than streaming it over the WebSocket.
- Mark old history segments as "stale" so the frontend can render a "load more" button instead of auto-fetching.

**Confidence:** HIGH -- standard pagination/lazy-load pattern.

**Phase:** Phase 2 (reconnection + history).

---

### Pitfall 17: PTY Output Contains Incomplete UTF-8 Sequences

**What goes wrong:** PTY reads return arbitrary byte chunks that may split a multi-byte UTF-8 character across two reads. If each chunk is naively converted to a string with `String::from_utf8`, the split character causes an error or is replaced with the Unicode replacement character, corrupting the output.

**Prevention:**
- Use a `Utf8Chunks` iterator or buffer incomplete sequences between reads. The `bstr` crate handles this gracefully.
- If using structured output (`stream-json`), JSON output is always valid UTF-8, making this less of a concern for the primary output path. But raw output fallback still needs handling.

**Confidence:** HIGH -- fundamental UTF-8 encoding behavior.

**Phase:** Phase 1 (PTY reader implementation).

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Phase 1: Daemon Foundation | Blocking PTY reads starving tokio (P1), zombie processes (P2), database blocking (P6) | Use `pty-process` async, `PR_SET_PDEATHSIG`, `tokio-rusqlite` |
| Phase 1: Daemon Foundation | Worker guard dropped (P11), UTF-8 splits (P17) | Store guard at top of main, use `bstr` or `Utf8Chunks` |
| Phase 2: WebSocket + Streaming | Backpressure/OOM (P4), reconnection storms (P8), output parsing fragility (P5) | Bounded channels, exponential backoff, per-agent parser trait with raw fallback |
| Phase 2: Frontend | Component cleanup leaks (P13), history replay overwhelm (P16) | Centralized WebSocket store, lazy pagination |
| Phase 3: Auth Gateway | forward_auth + WebSocket (P3), cookie expiry (P9) | Strip Connection header, cookie-refresh + heartbeat |
| Phase 3: Deployment | Caddy reload kills WS (P7), systemd ordering (P12) | stream_close_delay, explicit After= dependencies |
| Phase 1-2: Agent Integration | Agent version drift (P15), PTY window size (P10), output format variations (P5) | Version detection, explicit PTY size, structured output modes where available |

## Sources

- [Caddy forward_auth + WebSocket issue #5430](https://github.com/caddyserver/caddy/issues/5430)
- [Caddy forward_auth directive docs](https://caddyserver.com/docs/caddyfile/directives/forward_auth)
- [Caddy reverse_proxy directive docs](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy)
- [oauth2-proxy Caddy integration docs](https://oauth2-proxy.github.io/oauth2-proxy/next/configuration/integrations/caddy/)
- [oauth2-proxy session storage docs](https://oauth2-proxy.github.io/oauth2-proxy/configuration/session_storage/)
- [tokio spawn_blocking docs](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)
- [pty-process crate (async PTY)](https://docs.rs/pty-process/latest/pty_process/)
- [portable-pty crate](https://docs.rs/portable-pty/latest/portable_pty/)
- [tokio-rusqlite crate](https://lib.rs/crates/tokio-rusqlite)
- [tracing-appender non_blocking docs](https://docs.rs/tracing-appender/latest/tracing_appender/non_blocking/index.html)
- [WebSocket backpressure patterns](https://skylinecodes.substack.com/p/backpressure-in-websocket-streams)
- [axum WebSocket backpressure discussion](https://github.com/tokio-rs/axum/discussions/242)
- [Rust child process graceful shutdown](https://www.fiveonefour.com/blog/Fixing-ctrl-c-in-terminal-apps-child-process-management)
- [tokio graceful shutdown guide (2026)](https://oneuptime.com/blog/post/2026-01-07-rust-graceful-shutdown/view)
- [Claude Code headless/structured output docs](https://code.claude.com/docs/en/headless)
- [Agent Client Protocol (ACP) introduction](https://block.github.io/goose/blog/2025/10/24/intro-to-agent-client-protocol-acp/)
- [acpx headless ACP client](https://github.com/openclaw/acpx)
- [SvelteKit memory leak issues](https://github.com/sveltejs/kit/issues/12405)
- [strip-ansi-escapes crate](https://docs.rs/strip-ansi-escapes)
- [axum 0.8.0 announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0)
