# Phase 2: PTY Process Management - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Daemon spawns agent processes with PTY pairs via portable-pty and manages their full lifecycle: read continuous output, write structured input, resize on demand, clean up on exit, track PIDs, and report per-agent resource usage. This phase delivers the process management layer — no WebSocket streaming, no frontend, no structured output parsing.

Requirements covered: PTY-01, PTY-02, PTY-03, PTY-04, PTY-05, PTY-06, PTY-07

</domain>

<decisions>
## Implementation Decisions

### Session lifecycle
- 1:1 mapping between PTY sessions and tasks — each task has exactly one active session
- Moving to a new phase kills the old session and spawns a new one (matches current tmux behavior)
- Session stays alive when moving Running → Review (user can resume, send more input)
- Session killed only on Done (same cleanup that removes worktree)
- If agent process exits unexpectedly, mark session as Exited — resuming creates a fresh session (new UUID, new output log)

### Session identity
- Each session gets a UUID (fresh on every creation)
- Task stores its current active session_id
- WebSocket clients (Phase 3) will connect to sessions by UUID

### Output persistence
- Append-only file per session at `~/.local/share/agtx/sessions/{uuid}.log`
- Raw PTY bytes written as they arrive (no processing at this layer)
- Small in-memory ring buffer (~64KB) per active session for fast tail reads
- Output files deleted when task moves to Done (bounded disk usage)

### Input handling
- Structured commands only — send text strings followed by newline to PTY stdin
- No raw keystroke forwarding (aligns with PROJECT.md "structured output, not raw terminal" vision)
- Explicit interrupt endpoint: POST /api/v1/sessions/{id}/interrupt sends SIGINT to agent process
- Separate kill endpoint: POST /api/v1/sessions/{id}/kill sends SIGKILL as escalation
- Prompt detection (Y/N, approval requests) deferred to Phase 7 (OUTPUT-06)

### Resource monitoring
- Track CPU%, RSS memory, and session uptime per agent process
- Read from /proc/{pid}/stat and /proc/{pid}/statm (Linux)
- Poll every 5 seconds
- Track agent PID only (not child processes — Phase 8 system monitoring covers host-level usage)
- Expose via REST: GET /api/v1/sessions/{id}/metrics

### Claude's Discretion
- PTY async bridging approach (tokio integration with portable-pty's synchronous API)
- Process group management and PR_SET_PDEATHSIG implementation details
- Ring buffer sizing and eviction strategy
- Internal session state machine (spawning → running → exited states)
- REST API endpoint design for session CRUD (spawn, list, get, write, resize)
- Error handling for PTY spawn failures

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TmuxOperations` trait (crates/agtx-core/src/tmux/operations.rs): Defines the interface pattern (create, kill, send_keys, capture, resize) — new `PtyOperations` trait can mirror this shape
- `Agent.build_interactive_command()` (crates/agtx-core/src/agent/mod.rs): Builds shell commands per agent type — reusable for PTY spawn
- `AppState` (crates/agtxd/src/state.rs): Daemon state with `Arc<RwLock<GlobalConfig>>` — extend with session registry
- `tracing` logging already configured with structured JSON output

### Established Patterns
- Trait-based DI: `#[cfg_attr(feature = "test-mocks", automock)]` on traits with `Real*` implementations
- `Arc<dyn Trait>` for injectable dependencies, `Send + Sync` bounds
- `anyhow::Result` with `.context()` for all fallible operations
- Background work via `std::thread::spawn` + `mpsc` channels (may evolve to tokio tasks for PTY)
- UUID v4 strings for IDs, RFC3339 for timestamps

### Integration Points
- `AppState` in crates/agtxd/src/state.rs — add session manager/registry
- API router in crates/agtxd/src/api/ — add session endpoints
- `known_agents()` and `build_interactive_command()` in agtx-core — used to spawn agents
- `~/.local/share/agtx/` — add sessions/ subdirectory for output files

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

*Phase: 02-pty-process-management*
*Context gathered: 2026-03-04*
