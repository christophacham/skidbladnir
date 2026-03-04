# Roadmap: AGTX Web

## Overview

Transform AGTX from a terminal TUI into a web-native dashboard backed by a persistent Rust daemon. The build starts with the daemon foundation and PTY process management (highest-risk, fail-fast), layers on WebSocket streaming, then builds the SvelteKit frontend against a working backend. Workflow engine integration follows to preserve all existing AGTX semantics. Final phases add structured output intelligence, system monitoring, auth gateway, and UX polish. Each phase delivers a verifiable capability that unblocks the next.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Daemon Foundation** - Axum server with REST API skeleton, structured logging, health checks, and lifecycle management (completed 2026-03-04)
- [x] **Phase 2: PTY Process Management** - Agent process spawning and control via pty-process with async I/O, session lifecycle, and /proc resource monitoring (completed 2026-03-04)
- [ ] **Phase 3: WebSocket Streaming** - Bidirectional real-time agent output streaming with session persistence and reconnection
- [ ] **Phase 4: Frontend Kanban Board** - SvelteKit SPA with 5-column kanban layout, task CRUD, and project navigation
- [ ] **Phase 5: Task Detail & Live Output** - Split-view detail panel with live agent output streaming and phase status
- [ ] **Phase 6: Workflow Engine** - Phase transitions, plugin resolution, skill deployment, artifact detection, and PR workflow
- [ ] **Phase 7: Structured Output & Session Intelligence** - Semantic output parsing, action buttons, timeline, search, and reconnect summaries
- [ ] **Phase 8: System Monitoring** - System tab with host metrics, service health, log streaming, and per-agent resource usage
- [ ] **Phase 9: Auth & Deployment** - GitHub OAuth gateway via oauth2-proxy, Caddy reverse proxy, systemd services, and TLS
- [ ] **Phase 10: UX Polish** - Keyboard shortcuts, theming, responsive layout, toast notifications, and help overlay

## Phase Details

### Phase 1: Daemon Foundation
**Goal**: A running axum daemon that serves REST endpoints, logs structured output, reports health, and handles graceful lifecycle
**Depends on**: Nothing (first phase)
**Requirements**: INFRA-01, INFRA-03, INFRA-04, INFRA-05, INFRA-06
**Success Criteria** (what must be TRUE):
  1. Daemon starts and serves HTTP requests on a configured port
  2. Health endpoint returns daemon status with uptime and version
  3. Structured logs rotate to disk with configurable levels and non-blocking writes
  4. Daemon shuts down cleanly on SIGTERM/SIGINT without orphaned resources
  5. Configuration changes take effect without restarting the daemon process
**Plans:** 2/2 plans complete

Plans:
- [ ] 01-01-PLAN.md — Cargo workspace restructuring + agtxd daemon with REST API skeleton, health endpoint, and graceful shutdown
- [ ] 01-02-PLAN.md — Multi-layer structured logging and config file hot-reload

### Phase 2: PTY Process Management
**Goal**: Daemon can spawn agent processes with PTY pairs and manage their full lifecycle (read, write, resize, cleanup, tracking)
**Depends on**: Phase 1
**Requirements**: PTY-01, PTY-02, PTY-03, PTY-04, PTY-05, PTY-06, PTY-07
**Success Criteria** (what must be TRUE):
  1. Daemon spawns an agent process with a PTY and reads its continuous output stream
  2. Daemon writes input to a running agent's PTY stdin and the agent receives it
  3. PTY resizes when the client sends new dimensions and the agent's output reflows
  4. All agent processes are cleaned up on daemon exit with no zombie processes remaining
  5. Daemon reports PID and resource usage (CPU/memory) for each managed agent process
**Plans:** 3/3 plans complete

Plans:
- [x] 02-01-PLAN.md — Session module core: types, output persistence (ring buffer + file), SessionManager with spawn/read/write/resize/PID tracking
- [x] 02-02-PLAN.md — REST API endpoints for sessions + process cleanup on daemon shutdown (PTY-05)
- [x] 02-03-PLAN.md — Per-agent resource monitoring via /proc + metrics REST endpoint (PTY-07)

### Phase 3: WebSocket Streaming
**Goal**: Browser clients receive live agent output via WebSocket, can send input back, and reconnect to full persisted history
**Depends on**: Phase 2
**Requirements**: INFRA-02, WS-01, WS-02, WS-03, WS-04, WS-05
**Success Criteria** (what must be TRUE):
  1. Browser receives agent output in real time via WebSocket as the agent produces it
  2. All session output is persisted to disk and survives daemon restarts
  3. Reconnecting client loads full history via lazy-loaded virtualized scrollback
  4. Connection status indicator shows connected, disconnected, and reconnecting states
  5. Output auto-scrolls to bottom during live streaming and pauses when user scrolls up with a "jump to bottom" button
**Plans:** 2 plans

Plans:
- [ ] 03-01-PLAN.md — Broadcast channel infrastructure, WS message types, output read_range, write_raw, offset/limit on output endpoint
- [ ] 03-02-PLAN.md — WebSocket handler with split socket, cursor reconnection, input forwarding, and integration tests

### Phase 4: Frontend Kanban Board
**Goal**: Users see and manage tasks through a 5-column kanban board with full CRUD, search, and project switching
**Depends on**: Phase 1
**Requirements**: BOARD-01, BOARD-02, BOARD-03, BOARD-04, BOARD-05, BOARD-06, BOARD-07
**Success Criteria** (what must be TRUE):
  1. User sees tasks organized in five columns (Backlog, Planning, Running, Review, Done) matching current workflow
  2. Task cards display title, agent badge, and phase status indicator at a glance
  3. User can create and delete tasks with confirmation for destructive actions
  4. User can search and filter tasks by title and description content
  5. User can switch between projects via a sidebar and invoke actions via command palette (Ctrl+K)
**Plans**: TBD

Plans:
- [ ] 04-01: TBD
- [ ] 04-02: TBD

### Phase 5: Task Detail & Live Output
**Goal**: Clicking a task opens a split-view panel showing live agent output with visual distinction between output types and phase status
**Depends on**: Phase 3, Phase 4
**Requirements**: OUTPUT-01, OUTPUT-02, OUTPUT-03, OUTPUT-04
**Success Criteria** (what must be TRUE):
  1. Clicking a task card opens a split-view with the board on the left and detail panel on the right
  2. Detail panel streams live agent output in real time matching the WebSocket feed
  3. Agent text, tool calls, and errors are visually distinct (different styling or color coding)
  4. Task cards and detail panel both show current phase status (Working, Idle, Ready, Exited)
**Plans**: TBD

Plans:
- [ ] 05-01: TBD
- [ ] 05-02: TBD

### Phase 6: Workflow Engine
**Goal**: Full AGTX workflow semantics work through the web interface -- phase transitions trigger side effects, plugins resolve correctly, skills deploy, and PR workflow functions
**Depends on**: Phase 2, Phase 5
**Requirements**: FLOW-01, FLOW-02, FLOW-03, FLOW-04, FLOW-05, FLOW-06, FLOW-07, FLOW-08
**Success Criteria** (what must be TRUE):
  1. Moving a task forward triggers the correct side effects (worktree creation, agent spawn, skill deployment) matching TUI behavior
  2. Plugins resolve via project-local, global, then bundled precedence with correct command/prompt translation per agent type
  3. Artifact files are detected via polling with glob support and status updates appear on task cards
  4. Cyclic phases work (Review to Planning with incrementing phase counter)
  5. User can create GitHub PRs from the browser with title, description, and base branch selection and view syntax-highlighted diffs
**Plans**: TBD

Plans:
- [ ] 06-01: TBD
- [ ] 06-02: TBD
- [ ] 06-03: TBD

### Phase 7: Structured Output & Session Intelligence
**Goal**: Agent output is parsed into semantic sections with interactive elements, and users can search history and orient quickly on reconnect
**Depends on**: Phase 5
**Requirements**: OUTPUT-05, OUTPUT-06, OUTPUT-07, OUTPUT-08, WS-06, WS-07, WS-08
**Success Criteria** (what must be TRUE):
  1. Agent output is parsed into collapsible semantic sections (thinking, tool use, file edits) that users can expand/collapse
  2. Detected agent prompts render as action buttons (approve/reject) with a free text fallback input
  3. Phase progress timeline shows transitions with timestamps, durations, and timeline markers at phase boundaries and user inputs
  4. Artifact detection display shows expected vs detected artifacts from the task's plugin config
  5. User can full-text search within session output history and sees a reconnect summary banner showing changes since last disconnect
**Plans**: TBD

Plans:
- [ ] 07-01: TBD
- [ ] 07-02: TBD

### Phase 8: System Monitoring
**Goal**: System tab provides live visibility into host resources, service health, log output, and per-agent resource consumption
**Depends on**: Phase 3
**Requirements**: SYS-01, SYS-02, SYS-03, SYS-04
**Success Criteria** (what must be TRUE):
  1. System tab shows live-updating host metrics (CPU, RAM, disk, load) refreshing at a usable cadence
  2. Service health indicators show running/stopped status for agtxd, web, and proxy services
  3. Service logs stream live from journalctl for each managed service
  4. Per-agent resource usage (CPU and memory per process) is visible alongside host metrics
**Plans**: TBD

Plans:
- [ ] 08-01: TBD

### Phase 9: Auth & Deployment
**Goal**: The application is deployable on a remote Linux server with GitHub OAuth access control, TLS, and managed service lifecycle
**Depends on**: Phase 6
**Requirements**: AUTH-01, AUTH-02, AUTH-03, AUTH-04
**Success Criteria** (what must be TRUE):
  1. Access is gated by GitHub OAuth login restricted to a single configured username
  2. Caddy reverse proxies all traffic with forward_auth to oauth2-proxy and WebSocket connections pass through without interruption
  3. systemd service files manage agtxd, web frontend, and proxy lifecycle with restart-on-failure
  4. TLS is automatically provisioned and renewed via Caddy's built-in HTTPS
**Plans**: TBD

Plans:
- [ ] 09-01: TBD
- [ ] 09-02: TBD

### Phase 10: UX Polish
**Goal**: The web interface feels native and efficient with keyboard-driven navigation, consistent theming, responsive layout, and ambient notifications
**Depends on**: Phase 5
**Requirements**: UX-01, UX-02, UX-03, UX-04, UX-05, UX-06
**Success Criteria** (what must be TRUE):
  1. Keyboard shortcuts match TUI muscle memory (j/k navigation, Enter to open, o to create, m to move, / to search, q to quit)
  2. Dark theme is the default with all colors driven by CSS custom properties
  3. Layout renders correctly across desktop viewports from 1280px to 2560px+
  4. Toast notifications appear for background events (agent finished, PR created, errors) without blocking interaction
  5. User can press ? to see a keyboard shortcut cheat sheet and browser tab title reflects current task status
**Plans**: TBD

Plans:
- [ ] 10-01: TBD
- [ ] 10-02: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 9 -> 10

Note: Phase 4 (Frontend Kanban) depends only on Phase 1 and could theoretically parallel Phase 2/3, but sequential execution avoids context switching.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Daemon Foundation | 2/2 | Complete   | 2026-03-04 |
| 2. PTY Process Management | 3/3 | Complete   | 2026-03-04 |
| 3. WebSocket Streaming | 0/2 | Not started | - |
| 4. Frontend Kanban Board | 0/2 | Not started | - |
| 5. Task Detail & Live Output | 0/2 | Not started | - |
| 6. Workflow Engine | 0/3 | Not started | - |
| 7. Structured Output & Session Intelligence | 0/2 | Not started | - |
| 8. System Monitoring | 0/1 | Not started | - |
| 9. Auth & Deployment | 0/2 | Not started | - |
| 10. UX Polish | 0/2 | Not started | - |
