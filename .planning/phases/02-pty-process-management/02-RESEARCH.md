# Phase 2: PTY Process Management - Research

**Researched:** 2026-03-04
**Domain:** PTY pseudo-terminal management, async process lifecycle, Linux process cleanup
**Confidence:** HIGH

## Summary

This phase implements direct PTY process management in the daemon, replacing tmux as the agent process host. The core challenge is bridging pseudo-terminal I/O (inherently synchronous, blocking, file-descriptor-based) with tokio's async runtime while maintaining robust process lifecycle management (spawn, read, write, resize, signal, cleanup).

Two viable Rust PTY crates exist: `portable-pty` (synchronous, cross-platform, from wezterm) and `pty-process` (native tokio async, Unix-only). The CONTEXT.md mentions portable-pty, but the async bridging approach is explicitly Claude's discretion. Research strongly recommends `pty-process` because it provides native `AsyncRead`/`AsyncWrite` on the PTY, returns `tokio::process::Child` with proper async wait/kill, exposes `pre_exec` for process group configuration, and internally calls `setsid()` to make spawned processes session leaders -- all capabilities that portable-pty lacks and would require significant manual bridging work to replicate.

**Primary recommendation:** Use `pty-process` (with `async` feature) for PTY allocation and process spawning, `procfs` for /proc-based resource monitoring, and a simple `VecDeque<u8>` for the in-memory ring buffer. Implement explicit process cleanup in the shutdown handler rather than relying solely on `PR_SET_PDEATHSIG`.

<user_constraints>

## User Constraints (from CONTEXT.md)

### Locked Decisions
- 1:1 mapping between PTY sessions and tasks -- each task has exactly one active session
- Moving to a new phase kills the old session and spawns a new one (matches current tmux behavior)
- Session stays alive when moving Running -> Review (user can resume, send more input)
- Session killed only on Done (same cleanup that removes worktree)
- If agent process exits unexpectedly, mark session as Exited -- resuming creates a fresh session (new UUID, new output log)
- Each session gets a UUID (fresh on every creation)
- Task stores its current active session_id
- WebSocket clients (Phase 3) will connect to sessions by UUID
- Append-only file per session at `~/.local/share/agtx/sessions/{uuid}.log`
- Raw PTY bytes written as they arrive (no processing at this layer)
- Small in-memory ring buffer (~64KB) per active session for fast tail reads
- Output files deleted when task moves to Done (bounded disk usage)
- Structured commands only -- send text strings followed by newline to PTY stdin
- No raw keystroke forwarding
- Explicit interrupt endpoint: POST /api/v1/sessions/{id}/interrupt sends SIGINT to agent process
- Separate kill endpoint: POST /api/v1/sessions/{id}/kill sends SIGKILL as escalation
- Prompt detection (Y/N, approval requests) deferred to Phase 7 (OUTPUT-06)
- Track CPU%, RSS memory, and session uptime per agent process
- Read from /proc/{pid}/stat and /proc/{pid}/statm (Linux)
- Poll every 5 seconds
- Track agent PID only (not child processes)
- Expose via REST: GET /api/v1/sessions/{id}/metrics

### Claude's Discretion
- PTY async bridging approach (tokio integration with portable-pty's synchronous API)
- Process group management and PR_SET_PDEATHSIG implementation details
- Ring buffer sizing and eviction strategy
- Internal session state machine (spawning -> running -> exited states)
- REST API endpoint design for session CRUD (spawn, list, get, write, resize)
- Error handling for PTY spawn failures

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope

</user_constraints>

<phase_requirements>

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| PTY-01 | Daemon spawns agent processes with PTY pairs via portable-pty | `pty-process` crate with `async` feature provides `Pty::new()` + `Command::spawn(pts)` pattern; returns `tokio::process::Child` |
| PTY-02 | Daemon reads agent PTY output as continuous byte stream | `Pty` implements `AsyncRead`; spawn tokio task with read loop, forward bytes to ring buffer + append-only file |
| PTY-03 | Daemon writes to agent PTY stdin (commands, text input from browser) | `Pty` implements `AsyncWrite`; write bytes followed by newline via `WritePty` half |
| PTY-04 | Daemon resizes PTY on browser viewport change | `Pty::resize(Size::new(rows, cols))` -- synchronous call, safe from async context |
| PTY-05 | Daemon cleans up agent processes on exit with PR_SET_PDEATHSIG to prevent zombies | Dual strategy: `pre_exec` with `PR_SET_PDEATHSIG(SIGTERM)` as safety net + explicit `kill()` + `wait()` in shutdown handler |
| PTY-06 | Daemon tracks PIDs for all managed agent processes | `tokio::process::Child::id()` returns PID; stored in session registry `HashMap<Uuid, SessionHandle>` |
| PTY-07 | Daemon reports per-agent resource usage (CPU/memory per PID via /proc) | `procfs::process::Process::new(pid)` with `.stat()` for CPU and `.statm()` for memory; poll every 5s |

</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| pty-process | 0.5 | PTY allocation, process spawning with async I/O | Native tokio AsyncRead/AsyncWrite, setsid() built-in, pre_exec support, returns tokio::process::Child |
| procfs | 0.17 | Read /proc/{pid}/stat and /proc/{pid}/statm | Well-maintained Linux procfs interface, typed Stat/StatM structs, 1.70 MSRV |
| tokio | 1.44 (workspace) | Async runtime, process management, channels | Already in workspace; provides spawn, mpsc, RwLock, signal handling |
| uuid | 1.16 (workspace) | Session UUID generation | Already in workspace |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| libc | 0.2 | PR_SET_PDEATHSIG constant, prctl syscall | In pre_exec closure for zombie prevention safety net |
| serde | 1.0 (workspace) | Session/metrics serialization for API responses | All REST endpoint payloads |
| tracing | 0.1 (workspace) | Structured logging for session lifecycle events | Already configured in daemon |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| pty-process | portable-pty | portable-pty is cross-platform but synchronous -- requires spawn_blocking bridge, no pre_exec, no process_group, no kill_on_drop. Significant manual work for what pty-process provides natively. |
| procfs | Manual /proc parsing | procfs handles all parsing edge cases, clock tick conversion, page size multiplication. Hand-rolling is error-prone. |
| VecDeque ring buffer | ringbuffer crate | 64KB ring buffer is trivial -- VecDeque with manual cap avoids a dependency for ~15 lines of code |

**Installation:**
```bash
# Add to crates/agtxd/Cargo.toml
cargo add -p agtxd pty-process --features async
cargo add -p agtxd procfs
cargo add -p agtxd libc
```

## Architecture Patterns

### Recommended Project Structure
```
crates/agtxd/src/
├── session/
│   ├── mod.rs           # Re-exports
│   ├── manager.rs       # SessionManager: spawn, kill, list, get
│   ├── handle.rs        # SessionHandle: per-session state (child, reader task, ring buffer)
│   ├── output.rs        # Output persistence (append-only file + ring buffer)
│   ├── metrics.rs       # Per-process resource monitoring via /proc
│   └── state.rs         # SessionState enum (Spawning, Running, Exited)
├── api/
│   ├── sessions.rs      # REST endpoints for session CRUD
│   └── ... (existing)
└── ... (existing files)
```

### Pattern 1: Session Manager as Shared State
**What:** A `SessionManager` holds all active sessions behind `Arc<RwLock<HashMap<Uuid, SessionHandle>>>`. Registered in `AppState` alongside existing config.
**When to use:** All session operations route through this central registry.
**Example:**
```rust
// Source: Project pattern from existing AppState
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, SessionHandle>>>,
    sessions_dir: PathBuf, // ~/.local/share/agtx/sessions/
}

impl SessionManager {
    pub async fn spawn(
        &self,
        agent: &Agent,
        working_dir: &Path,
        prompt: &str,
        cols: u16,
        rows: u16,
    ) -> anyhow::Result<Uuid> {
        let session_id = Uuid::new_v4();
        let (pty, pts) = pty_process::Pty::new()?;
        pty.resize(pty_process::Size::new(rows, cols))?;

        let mut cmd = pty_process::Command::new(&agent.command);
        // Add agent-specific args
        // Set working directory
        // Configure pre_exec for PR_SET_PDEATHSIG

        let child = cmd.spawn(&pts)?;
        let pid = child.id().unwrap_or(0);

        // Split pty into read/write halves
        // Spawn reader task
        // Create SessionHandle

        let handle = SessionHandle::new(session_id, pid, child, write_pty, ...);
        self.sessions.write().await.insert(session_id, handle);

        Ok(session_id)
    }
}
```

### Pattern 2: Async Reader Task per Session
**What:** Each session spawns a dedicated tokio task that reads from the PTY's `AsyncRead` half in a loop, writing bytes to both the ring buffer and the append-only log file.
**When to use:** Always -- this is the core output capture mechanism.
**Example:**
```rust
// Source: pty-process AsyncRead + tokio patterns
async fn reader_task(
    mut read_pty: OwnedReadPty,
    output: Arc<RwLock<SessionOutput>>,
    session_id: Uuid,
) {
    let mut buf = [0u8; 4096];
    loop {
        match read_pty.read(&mut buf).await {
            Ok(0) => {
                tracing::info!(session_id = %session_id, "PTY EOF");
                break;
            }
            Ok(n) => {
                let bytes = &buf[..n];
                let mut out = output.write().await;
                out.append(bytes).await;
            }
            Err(e) => {
                tracing::error!(session_id = %session_id, error = %e, "PTY read error");
                break;
            }
        }
    }
}
```

### Pattern 3: Output Persistence with Ring Buffer
**What:** `SessionOutput` combines an append-only file writer and a fixed-capacity in-memory ring buffer. The ring buffer serves fast tail reads; the file provides full history.
**When to use:** Every session has one.
**Example:**
```rust
pub struct SessionOutput {
    file: tokio::fs::File,       // Append-only log
    ring: VecDeque<u8>,          // ~64KB ring buffer
    total_bytes: u64,            // Total bytes written
}

impl SessionOutput {
    const RING_CAPACITY: usize = 65_536; // 64KB

    pub async fn append(&mut self, data: &[u8]) -> std::io::Result<()> {
        // Write to file
        self.file.write_all(data).await?;

        // Write to ring buffer (evict oldest if over capacity)
        for &byte in data {
            if self.ring.len() >= Self::RING_CAPACITY {
                self.ring.pop_front();
            }
            self.ring.push_back(byte);
        }
        self.total_bytes += data.len() as u64;
        Ok(())
    }

    pub fn tail(&self) -> &[u8] {
        // VecDeque may not be contiguous -- use make_contiguous or return slices
        // For API responses, collect into Vec<u8>
    }
}
```

### Pattern 4: Dual-Layer Process Cleanup
**What:** Two independent mechanisms ensure no zombies: (1) `PR_SET_PDEATHSIG(SIGTERM)` in pre_exec as a kernel-level safety net, (2) explicit session cleanup in the daemon shutdown handler.
**When to use:** Always -- defense in depth.
**Example:**
```rust
// In pre_exec (runs in forked child before exec):
unsafe {
    cmd.pre_exec(|| {
        // Safety: prctl is async-signal-safe
        if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    });
}

// In shutdown handler (runs when daemon receives SIGTERM/SIGINT):
impl SessionManager {
    pub async fn shutdown_all(&self) {
        let mut sessions = self.sessions.write().await;
        for (id, handle) in sessions.drain() {
            tracing::info!(session_id = %id, pid = handle.pid, "Killing session");
            if let Err(e) = handle.child.kill().await {
                tracing::warn!(session_id = %id, error = %e, "Failed to kill");
            }
            // Wait to reap zombie
            let _ = handle.child.wait().await;
        }
    }
}
```

### Pattern 5: Resource Monitoring via /proc
**What:** A background tokio task polls `/proc/{pid}/stat` and `/proc/{pid}/statm` every 5 seconds for each active session, caching the latest metrics.
**When to use:** For PTY-07 resource reporting.
**Example:**
```rust
use procfs::process::Process;

pub struct ProcessMetrics {
    pub cpu_percent: f32,
    pub rss_bytes: u64,
    pub uptime_secs: f64,
}

fn read_metrics(pid: i32, prev_cpu: u64, elapsed_ticks: u64) -> Option<ProcessMetrics> {
    let proc = Process::new(pid).ok()?;
    let stat = proc.stat().ok()?;
    let statm = proc.statm().ok()?;

    let total_cpu = (stat.utime + stat.stime) as u64;
    let cpu_delta = total_cpu.saturating_sub(prev_cpu);
    let cpu_percent = if elapsed_ticks > 0 {
        (cpu_delta as f32 / elapsed_ticks as f32) * 100.0
    } else {
        0.0
    };

    let page_size = procfs::page_size();
    let rss_bytes = statm.resident * page_size;

    Some(ProcessMetrics {
        cpu_percent,
        rss_bytes,
        uptime_secs: stat.starttime as f64 / procfs::ticks_per_second() as f64,
    })
}
```

### Anti-Patterns to Avoid
- **Using `spawn_blocking` for PTY spawn:** This creates a tokio blocking thread as the "parent" for PR_SET_PDEATHSIG. Tokio reaps idle blocking threads after ~10 seconds, which sends SIGTERM to the child. Use regular tokio task spawning instead.
- **Dropping `tokio::process::Child` without waiting:** Creates zombie processes. Always call `child.wait().await` or `child.kill().await` followed by `wait()`.
- **Cloning PTY reader file descriptor:** portable-pty's `try_clone_reader()` can cause reader hangs because the kernel requires ALL references to the master FD to be closed for EOF. pty-process's `OwnedReadPty` avoids this by taking ownership.
- **Forgetting to drop the Pts (slave side):** After spawn, the parent process must drop its reference to the slave/pts side. Otherwise the PTY never delivers EOF to the reader. pty-process handles this in `spawn()` which consumes the `Pts`.
- **Storing output bytes without flush:** The append-only file must be flushed periodically or use `BufWriter` with reasonable buffer size to avoid data loss on crash.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| PTY allocation + async I/O | Raw openpty/forkpty + manual async bridge | `pty-process` crate | Handles setsid, controlling terminal, fd cleanup, split read/write, AsyncRead/AsyncWrite |
| /proc stat parsing | Manual string parsing of /proc/{pid}/stat | `procfs` crate | stat file has 52 space-delimited fields with edge cases (comm field can contain spaces/parens); procfs handles all of it |
| CPU% calculation | Manual clock tick math | `procfs::ticks_per_second()` | Clock tick rate varies by system; procfs reads sysconf(_SC_CLK_TCK) correctly |
| Process signal delivery | Manual unsafe kill() calls | `tokio::process::Child::kill()` / `start_kill()` | Handles error cases, async wait, zombie reaping |

**Key insight:** PTY management has many subtle fd lifecycle and signal-safety requirements. Using a purpose-built crate eliminates an entire class of bugs around file descriptor leaks, zombie processes, and blocking I/O on the async runtime.

## Common Pitfalls

### Pitfall 1: PR_SET_PDEATHSIG + Tokio Worker Threads
**What goes wrong:** Child process receives unexpected SIGTERM ~10 seconds after spawn.
**Why it happens:** PR_SET_PDEATHSIG fires on parent **thread** death, not parent **process** death. If the child is spawned from a `spawn_blocking` thread, tokio reaps that thread when idle.
**How to avoid:** Never spawn PTY processes from `spawn_blocking`. Use `pty-process::Command::spawn()` from a regular tokio task. The fork happens on a tokio worker thread (which is NOT reaped during runtime lifetime).
**Warning signs:** Agents dying after ~10 seconds of idle time with no apparent error.

### Pitfall 2: PTY EOF Never Arrives
**What goes wrong:** Reader task hangs forever after child process exits.
**Why it happens:** The kernel delivers EOF on the master PTY only when ALL file descriptors referencing the slave side are closed AND the reader holds the only reference to the master.
**How to avoid:** (1) `pty-process::Command::spawn()` consumes the `Pts`, closing the parent's slave reference. (2) Use `OwnedReadPty` (not borrowed) so the reader has exclusive ownership. (3) Do not clone the PTY file descriptor.
**Warning signs:** Reader task stays alive after `child.wait()` returns.

### Pitfall 3: Zombie Process Accumulation
**What goes wrong:** Exited child processes remain as zombies, consuming PID table entries.
**Why it happens:** On Unix, a parent must call `wait()` (or `waitpid()`) on exited children. Dropping a `tokio::process::Child` without waiting creates a zombie that tokio's background reaper handles on a best-effort basis.
**How to avoid:** Always `child.wait().await` after detecting exit or after `child.kill().await`. In the shutdown handler, kill then wait for every session.
**Warning signs:** `ps aux | grep Z` showing zombie processes owned by agtxd.

### Pitfall 4: Working Directory Not Set on PTY Spawn
**What goes wrong:** Agent starts in the wrong directory (typically $HOME).
**Why it happens:** PTY spawn does not inherit the daemon's cwd. The command builder must explicitly set cwd.
**How to avoid:** Always call `.current_dir(working_dir)` (or equivalent) on the command before spawn. For pty-process, the inner `tokio::process::Command` supports this.
**Warning signs:** Agent complaining about missing files or wrong project context.

### Pitfall 5: Ring Buffer Contiguity
**What goes wrong:** Returning ring buffer contents as a single `&[u8]` slice fails or returns partial data.
**Why it happens:** `VecDeque` stores data in two non-contiguous slices when it wraps around.
**How to avoid:** Use `VecDeque::make_contiguous()` before returning a slice, or collect into a `Vec<u8>` for API responses. For the ~64KB size this allocation is negligible.
**Warning signs:** Truncated output in tail responses.

### Pitfall 6: CPU% Calculation Requires Delta
**What goes wrong:** CPU usage shows cumulative time instead of current utilization percentage.
**Why it happens:** `/proc/{pid}/stat` reports cumulative CPU ticks since process start, not instantaneous usage.
**How to avoid:** Store previous `utime + stime` reading and compute delta between polls. Divide by elapsed wall-clock ticks for percentage.
**Warning signs:** CPU% monotonically increasing over time.

## Code Examples

Verified patterns from official sources:

### Spawning an Agent with PTY
```rust
// Source: pty-process docs + project Agent pattern
use pty_process::{Pty, Command, Size};

async fn spawn_agent(
    agent: &Agent,
    working_dir: &Path,
    prompt: &str,
    cols: u16,
    rows: u16,
) -> anyhow::Result<(Pty, tokio::process::Child)> {
    let mut pty = Pty::new()?;
    let pts = pty.pts()?;
    pty.resize(Size::new(rows, cols))?;

    let mut cmd = Command::new(&agent.command);

    // Add agent-specific arguments (from build_interactive_command pattern)
    match agent.name.as_str() {
        "claude" => { cmd.args(["--dangerously-skip-permissions"]); }
        "codex" => { cmd.args(["--full-auto"]); }
        "gemini" => { cmd.args(["--approval-mode", "yolo"]); }
        "copilot" => { cmd.args(["--allow-all-tools"]); }
        _ => {}
    }

    if !prompt.is_empty() {
        cmd.arg(prompt);
    }

    cmd.current_dir(working_dir);
    cmd.env("TERM", "xterm-256color");

    // Safety net: kill child if daemon dies
    unsafe {
        cmd.pre_exec(|| {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }

    let child = cmd.spawn(&pts)?;
    // pts is consumed by spawn -- slave fd closed in parent

    Ok((pty, child))
}
```

### Writing Input to PTY
```rust
// Source: pty-process AsyncWrite trait
use tokio::io::AsyncWriteExt;

async fn write_to_session(
    write_pty: &mut OwnedWritePty,
    input: &str,
) -> anyhow::Result<()> {
    // Structured command: text + newline
    write_pty.write_all(input.as_bytes()).await?;
    write_pty.write_all(b"\n").await?;
    write_pty.flush().await?;
    Ok(())
}
```

### Sending SIGINT to Agent
```rust
// Source: nix/libc signal handling
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

fn send_interrupt(pid: u32) -> anyhow::Result<()> {
    kill(Pid::from_raw(pid as i32), Signal::SIGINT)?;
    Ok(())
}

fn send_kill(pid: u32) -> anyhow::Result<()> {
    kill(Pid::from_raw(pid as i32), Signal::SIGKILL)?;
    Ok(())
}
```

### REST API Endpoint Design
```rust
// Source: Project axum patterns from existing API
use axum::{Router, routing::{get, post, delete}, extract::{State, Path, Json}};

pub fn session_router() -> Router<AppState> {
    Router::new()
        .route("/", post(spawn_session).get(list_sessions))
        .route("/{id}", get(get_session).delete(kill_session))
        .route("/{id}/write", post(write_to_session))
        .route("/{id}/resize", post(resize_session))
        .route("/{id}/interrupt", post(interrupt_session))
        .route("/{id}/kill", post(kill_session_process))
        .route("/{id}/metrics", get(get_session_metrics))
        .route("/{id}/output", get(get_session_output))
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| tmux for agent process hosting | Direct PTY management via pty-process | This phase | Eliminates tmux runtime dependency, enables structured output capture |
| portable-pty (sync) + spawn_blocking bridge | pty-process (native async) | pty-process 0.5 | No blocking thread bridge needed; AsyncRead/AsyncWrite directly on PTY |
| Manual /proc parsing | procfs crate | procfs 0.14+ | Typed structs, handles all 52 stat fields, page size calculation |
| VecDeque manual ring buffer | Same (still best approach) | N/A | 64KB is too small to justify a crate dependency |

**Deprecated/outdated:**
- `tokio-pty-process`: Unmaintained, superseded by `pty-process` with `async` feature
- `portable-pty` for async use cases: Works but requires significant manual bridging (spawn_blocking, channel forwarding, manual EOF detection)

## Open Questions

1. **pty-process Command access to inner tokio::process::Command**
   - What we know: pty-process::Command wraps tokio::process::Command and delegates methods like `arg`, `args`, `current_dir`, `env`, `pre_exec`
   - What's unclear: Whether `kill_on_drop(true)` can be set on the inner Command before spawn (documentation doesn't show this method)
   - Recommendation: The returned `tokio::process::Child` does not have `kill_on_drop` set by default. Implement explicit cleanup in SessionManager::shutdown_all() and Drop impl. PR_SET_PDEATHSIG provides the kernel-level safety net.

2. **pty-process split behavior**
   - What we know: `Pty` can be split into `OwnedReadPty` and `OwnedWritePty` for concurrent read/write
   - What's unclear: Whether resize works after splitting (resize is on the `Pty` or `WritePty`)
   - Recommendation: Test during implementation. If resize requires the original `Pty`, keep a reference or use `Pty` directly with tokio mutex for write operations.

3. **procfs availability guarantee**
   - What we know: /proc is always mounted on Linux, but container environments may restrict access
   - What's unclear: Whether all needed /proc files are accessible in Docker/Podman containers by default
   - Recommendation: Make metrics collection fallible -- return None/default when /proc is inaccessible. Log a warning once.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (built-in) + tokio::test for async |
| Config file | Cargo.toml [dev-dependencies] |
| Quick run command | `cargo test -p agtxd -- --lib` |
| Full suite command | `cargo test -p agtxd` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PTY-01 | Spawn agent process with PTY | integration | `cargo test -p agtxd test_spawn_session -x` | No -- Wave 0 |
| PTY-02 | Read continuous output stream | integration | `cargo test -p agtxd test_read_pty_output -x` | No -- Wave 0 |
| PTY-03 | Write input to PTY stdin | integration | `cargo test -p agtxd test_write_to_session -x` | No -- Wave 0 |
| PTY-04 | Resize PTY dimensions | integration | `cargo test -p agtxd test_resize_session -x` | No -- Wave 0 |
| PTY-05 | Clean up processes on exit | integration | `cargo test -p agtxd test_shutdown_kills_sessions -x` | No -- Wave 0 |
| PTY-06 | Track PIDs for managed processes | unit | `cargo test -p agtxd test_session_tracks_pid -x` | No -- Wave 0 |
| PTY-07 | Report CPU/memory per process | unit | `cargo test -p agtxd test_process_metrics -x` | No -- Wave 0 |

### Testing Strategy Notes
- PTY tests require spawning real processes (e.g., `echo`, `cat`, `sh -c "..."`) -- not agents
- Use simple shell commands that produce known output for deterministic assertions
- Process cleanup tests should spawn a child, kill the manager, and verify no zombies via `waitpid(WNOHANG)`
- Metrics tests can read /proc/self/stat to verify the procfs parsing logic
- SessionManager should use trait-based DI consistent with project patterns (`#[cfg_attr(feature = "test-mocks", automock)]`)
- API endpoint tests follow existing pattern in `tests/api_tests.rs` (tower oneshot + temp databases)

### Sampling Rate
- **Per task commit:** `cargo test -p agtxd`
- **Per wave merge:** `cargo test` (full workspace)
- **Phase gate:** Full suite green before /gsd:verify-work

### Wave 0 Gaps
- [ ] `crates/agtxd/tests/session_tests.rs` -- integration tests for session spawn/read/write/resize/kill
- [ ] `crates/agtxd/tests/metrics_tests.rs` -- unit tests for /proc parsing and CPU% calculation
- [ ] `crates/agtxd/tests/session_api_tests.rs` -- HTTP endpoint tests for session CRUD
- [ ] Dependencies: `pty-process` (features = ["async"]), `procfs`, `libc` added to Cargo.toml
- [ ] Optional: `nix` crate for typed signal delivery (SIGINT/SIGKILL)

## Sources

### Primary (HIGH confidence)
- [pty-process docs](https://docs.rs/pty-process/latest/pty_process/) - Pty, Command, Size API, AsyncRead/AsyncWrite, spawn behavior
- [pty-process GitHub](https://github.com/doy/pty-process) - Source confirming setsid() in spawn, pre_exec composition
- [procfs docs](https://docs.rs/procfs/latest/procfs/process/index.html) - Process, Stat, StatM structs for /proc reading
- [portable-pty docs](https://docs.rs/portable-pty/latest/portable_pty/) - CommandBuilder API (no pre_exec, no process_group)
- [tokio::process::Child docs](https://docs.rs/tokio/latest/tokio/process/struct.Child.html) - kill_on_drop, wait, zombie reaping

### Secondary (MEDIUM confidence)
- [Tokio + prctl = nasty bug](https://kobzol.github.io/rust/2025/02/23/tokio-plus-prctl-equals-nasty-bug.html) - PR_SET_PDEATHSIG + spawn_blocking thread reaping interaction
- [PTY + Cargo build article](https://developerlife.com/2025/08/10/pty-rust-osc-seq/) - Production patterns for portable-pty async bridging, fd lifecycle
- [Rust CommandExt](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html) - process_group, pre_exec, setsid std lib methods

### Tertiary (LOW confidence)
- Ring buffer sizing (64KB) - Based on CONTEXT.md decision, reasonable for tail reads, no external validation needed

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - pty-process and procfs are well-documented, actively maintained crates with clear APIs verified via official docs
- Architecture: HIGH - Patterns follow existing project conventions (trait DI, Arc<RwLock<>>, axum routing, anyhow errors)
- Pitfalls: HIGH - PR_SET_PDEATHSIG/tokio interaction verified via detailed blog post with reproduction; PTY fd lifecycle verified via multiple sources
- Process cleanup: HIGH - Dual strategy (PR_SET_PDEATHSIG + explicit shutdown) provides defense in depth

**Research date:** 2026-03-04
**Valid until:** 2026-04-04 (stable domain, 30-day validity)
