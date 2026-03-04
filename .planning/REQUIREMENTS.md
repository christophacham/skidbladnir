# Requirements: AGTX Web

**Defined:** 2026-03-03
**Core Value:** Manage multiple coding agent sessions from any browser with full reconnectable history, without losing any of the workflow semantics that make AGTX useful.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Daemon & Infrastructure

- [x] **INFRA-01**: Daemon serves REST API endpoints for task and project CRUD via axum
- [ ] **INFRA-02**: Daemon serves WebSocket endpoint for bidirectional real-time streaming
- [x] **INFRA-03**: Structured logging with tracing + tracing-appender (rotation, non-blocking writes)
- [x] **INFRA-04**: Health check endpoint returns daemon status
- [x] **INFRA-05**: Daemon handles graceful shutdown on SIGTERM/SIGINT with active process cleanup
- [x] **INFRA-06**: Daemon reloads configuration changes without restart

### PTY Process Management

- [x] **PTY-01**: Daemon spawns agent processes with PTY pairs via portable-pty
- [x] **PTY-02**: Daemon reads agent PTY output as continuous byte stream
- [x] **PTY-03**: Daemon writes to agent PTY stdin (commands, text input from browser)
- [x] **PTY-04**: Daemon resizes PTY on browser viewport change
- [x] **PTY-05**: Daemon cleans up agent processes on exit with PR_SET_PDEATHSIG to prevent zombies
- [x] **PTY-06**: Daemon tracks PIDs for all managed agent processes
- [x] **PTY-07**: Daemon reports per-agent resource usage (CPU/memory per PID via /proc)

### WebSocket & Sessions

- [ ] **WS-01**: Browser receives live agent output via WebSocket as it is produced
- [ ] **WS-02**: Daemon persists session output to disk as PTY bytes arrive
- [ ] **WS-03**: User reconnects and sees full history via lazy-loaded virtualized scrollback
- [ ] **WS-04**: User sees connection status indicator (connected/disconnected/reconnecting)
- [ ] **WS-05**: Output auto-scrolls to bottom; pauses on manual scroll-up with "jump to bottom" button
- [ ] **WS-06**: User sees reconnect summary banner showing status since last disconnect
- [ ] **WS-07**: User can full-text search within session output history
- [ ] **WS-08**: Timeline markers appear at phase transitions and user inputs

### Kanban Board

- [ ] **BOARD-01**: User sees 5-column kanban layout (Backlog/Planning/Running/Review/Done)
- [ ] **BOARD-02**: Task cards display title, agent badge, and phase status indicator
- [ ] **BOARD-03**: User can create tasks with title and description
- [ ] **BOARD-04**: User can delete tasks with confirmation dialog
- [ ] **BOARD-05**: User can search/filter tasks across title and description
- [ ] **BOARD-06**: User can switch between projects via multi-project sidebar
- [ ] **BOARD-07**: User can invoke command palette (Ctrl+K) for fuzzy action search

### Task Detail & Agent Output

- [ ] **OUTPUT-01**: Clicking a task opens split-view detail panel (board left, detail right)
- [ ] **OUTPUT-02**: Detail panel streams live agent output in real time
- [ ] **OUTPUT-03**: Agent text, tool calls, and errors are visually distinct
- [ ] **OUTPUT-04**: Task cards and detail panel show phase status (Working/Idle/Ready/Exited)
- [ ] **OUTPUT-05**: Agent output is parsed into collapsible semantic sections (thinking, tool use, file edits)
- [ ] **OUTPUT-06**: Detected agent prompts render action buttons (approve/reject) with free text fallback
- [ ] **OUTPUT-07**: Phase progress timeline shows transitions with timestamps and durations
- [ ] **OUTPUT-08**: Artifact detection shows expected vs detected artifacts from plugin config

### Workflow Engine

- [ ] **FLOW-01**: Phase transitions trigger side effects (worktree creation, agent spawn, skill deployment)
- [ ] **FLOW-02**: Plugins resolve via project-local → global → bundled precedence
- [ ] **FLOW-03**: Skills deploy to agent-native paths in worktrees per agent type
- [ ] **FLOW-04**: Commands and prompts resolve per agent type with format translation
- [ ] **FLOW-05**: Artifact files detected via polling with glob support
- [ ] **FLOW-06**: Cyclic phases supported (Review → Planning with incrementing phase counter)
- [ ] **FLOW-07**: User can create GitHub PRs from browser (title, description, base branch)
- [ ] **FLOW-08**: User can view syntax-highlighted git diffs for task worktrees

### Auth & Deployment

- [ ] **AUTH-01**: Access gated by GitHub OAuth via oauth2-proxy restricted to single username
- [ ] **AUTH-02**: Caddy reverse proxies to daemon with forward_auth and WebSocket passthrough
- [ ] **AUTH-03**: systemd service files manage agtxd, web, and proxy lifecycle
- [ ] **AUTH-04**: TLS enabled via Caddy automatic HTTPS

### System Monitoring

- [ ] **SYS-01**: System tab shows live host metrics (CPU/RAM/disk/load) via sysinfo
- [ ] **SYS-02**: System tab shows service health status indicators (running/stopped)
- [ ] **SYS-03**: System tab streams service logs (journalctl) for agtxd/web/proxy
- [ ] **SYS-04**: System tab shows per-agent resource usage (CPU/memory per process)

### UX Polish

- [ ] **UX-01**: Keyboard shortcuts match TUI muscle memory (j/k navigation, Enter, o, m, /, q)
- [ ] **UX-02**: Dark theme is default with CSS custom properties for theming
- [ ] **UX-03**: Layout is responsive for desktop viewports (1280px-2560px+)
- [ ] **UX-04**: Toast notifications appear for background events (agent finished, PR created, errors)
- [ ] **UX-05**: Keyboard shortcut cheat sheet shown via ? key
- [ ] **UX-06**: Browser tab title updates to reflect current task status

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Session Intelligence

- **SESSION-01**: Per-agent resource tracking with historical graphs
- **SESSION-02**: Session comparison (diff two agent sessions)
- **SESSION-03**: Export session transcript to markdown

### Advanced Workflow

- **ADV-01**: Custom plugin editor in browser (TOML visual editor)
- **ADV-02**: Agent capability matrix (which agents support which features)
- **ADV-03**: Batch operations (move/delete multiple tasks)

### Theming

- **THEME-01**: Light theme option
- **THEME-02**: Import existing AGTX TUI color schemes

## Out of Scope

| Feature | Reason |
|---------|--------|
| Raw terminal emulator (xterm.js) | Structured output is the value proposition -- raw PTY rendering undermines it |
| Multi-user / team access | Single-user by design; oauth2-proxy allowlist enforces this |
| Mobile-responsive layout | Agent output is wide; desktop-first, no phone optimization |
| Drag-and-drop task reordering | Phase transitions have side effects; arbitrary movement conflicts with workflow engine |
| Real-time collaborative editing | Single-user tool; no concurrent editors |
| Custom dashboard widget layout | Fixed purposeful layout; customizable layouts add unnecessary complexity |
| Plugin marketplace | Plugins are TOML files; file-based system is simple and sufficient |
| Email / push notifications | In-app toasts sufficient for single-user single-browser |
| Chat interface with agent | Agents are autonomous workers; structured input box covers needed interaction |
| Undo/redo for state changes | Phase transitions have real-world side effects (worktrees, processes, PRs) |
| Embedded code editor | Users have VS Code/Neovim; show diffs read-only instead |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| INFRA-01 | Phase 1: Daemon Foundation | Complete |
| INFRA-02 | Phase 3: WebSocket Streaming | Pending |
| INFRA-03 | Phase 1: Daemon Foundation | Complete |
| INFRA-04 | Phase 1: Daemon Foundation | Complete |
| INFRA-05 | Phase 1: Daemon Foundation | Complete |
| INFRA-06 | Phase 1: Daemon Foundation | Complete |
| PTY-01 | Phase 2: PTY Process Management | Complete |
| PTY-02 | Phase 2: PTY Process Management | Complete |
| PTY-03 | Phase 2: PTY Process Management | Complete |
| PTY-04 | Phase 2: PTY Process Management | Complete |
| PTY-05 | Phase 2: PTY Process Management | Complete |
| PTY-06 | Phase 2: PTY Process Management | Complete |
| PTY-07 | Phase 2: PTY Process Management | Complete |
| WS-01 | Phase 3: WebSocket Streaming | Pending |
| WS-02 | Phase 3: WebSocket Streaming | Pending |
| WS-03 | Phase 3: WebSocket Streaming | Pending |
| WS-04 | Phase 3: WebSocket Streaming | Pending |
| WS-05 | Phase 3: WebSocket Streaming | Pending |
| WS-06 | Phase 7: Structured Output & Session Intelligence | Pending |
| WS-07 | Phase 7: Structured Output & Session Intelligence | Pending |
| WS-08 | Phase 7: Structured Output & Session Intelligence | Pending |
| BOARD-01 | Phase 4: Frontend Kanban Board | Pending |
| BOARD-02 | Phase 4: Frontend Kanban Board | Pending |
| BOARD-03 | Phase 4: Frontend Kanban Board | Pending |
| BOARD-04 | Phase 4: Frontend Kanban Board | Pending |
| BOARD-05 | Phase 4: Frontend Kanban Board | Pending |
| BOARD-06 | Phase 4: Frontend Kanban Board | Pending |
| BOARD-07 | Phase 4: Frontend Kanban Board | Pending |
| OUTPUT-01 | Phase 5: Task Detail & Live Output | Pending |
| OUTPUT-02 | Phase 5: Task Detail & Live Output | Pending |
| OUTPUT-03 | Phase 5: Task Detail & Live Output | Pending |
| OUTPUT-04 | Phase 5: Task Detail & Live Output | Pending |
| OUTPUT-05 | Phase 7: Structured Output & Session Intelligence | Pending |
| OUTPUT-06 | Phase 7: Structured Output & Session Intelligence | Pending |
| OUTPUT-07 | Phase 7: Structured Output & Session Intelligence | Pending |
| OUTPUT-08 | Phase 7: Structured Output & Session Intelligence | Pending |
| FLOW-01 | Phase 6: Workflow Engine | Pending |
| FLOW-02 | Phase 6: Workflow Engine | Pending |
| FLOW-03 | Phase 6: Workflow Engine | Pending |
| FLOW-04 | Phase 6: Workflow Engine | Pending |
| FLOW-05 | Phase 6: Workflow Engine | Pending |
| FLOW-06 | Phase 6: Workflow Engine | Pending |
| FLOW-07 | Phase 6: Workflow Engine | Pending |
| FLOW-08 | Phase 6: Workflow Engine | Pending |
| AUTH-01 | Phase 9: Auth & Deployment | Pending |
| AUTH-02 | Phase 9: Auth & Deployment | Pending |
| AUTH-03 | Phase 9: Auth & Deployment | Pending |
| AUTH-04 | Phase 9: Auth & Deployment | Pending |
| SYS-01 | Phase 8: System Monitoring | Pending |
| SYS-02 | Phase 8: System Monitoring | Pending |
| SYS-03 | Phase 8: System Monitoring | Pending |
| SYS-04 | Phase 8: System Monitoring | Pending |
| UX-01 | Phase 10: UX Polish | Pending |
| UX-02 | Phase 10: UX Polish | Pending |
| UX-03 | Phase 10: UX Polish | Pending |
| UX-04 | Phase 10: UX Polish | Pending |
| UX-05 | Phase 10: UX Polish | Pending |
| UX-06 | Phase 10: UX Polish | Pending |

**Coverage:**
- v1 requirements: 58 total
- Mapped to phases: 58
- Unmapped: 0

---
*Requirements defined: 2026-03-03*
*Last updated: 2026-03-03 after roadmap creation*
