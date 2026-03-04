# AGTX WEB — PROJECT BRIEF

## WHAT THIS IS

A web-native version of AGTX that preserves the exact current workflow semantics — plugins, phase lifecycle, artifact tracking, worktree isolation, per-phase agent routing — but replaces terminal/tmux interaction with a browser UI backed by a persistent Rust daemon. Designed for remote server deployment with secure single-user access via GitHub OAuth.

## CORE VALUE

Manage multiple coding agent sessions from any browser with full reconnectable history, without losing any of the workflow semantics that make AGTX useful.

## EXISTING CODEBASE

AGTX is an existing ~8K line Rust codebase with a well-structured module system. The core logic (config, plugin, artifact, worktree, phase lifecycle, agent routing, database) is already separated from the TUI layer via traits and clean module boundaries. The web version reuses this core engine, replacing only the presentation and process management layers.

The current TUI uses tmux for agent process management. The web version replaces tmux with direct PTY control, which enables richer structured output parsing and eliminates a runtime dependency.

The existing trait-based architecture (`TmuxOperations`, `GitOperations`, `AgentOperations`) was designed for testability but conveniently enables swapping implementations — the PTY-based process manager can implement the same interface patterns.

### WHAT ALREADY WORKS (VALIDATED, DON'T REBUILD)

- Kanban board with Backlog → Planning → Running → Review → Done workflow
- Git worktree isolation per task (`.agtx/worktrees/{slug}`)
- Plugin system (TOML config) with commands, prompts, artifacts, prompt_triggers, copy_back, cyclic phases
- Phase lifecycle with artifact detection and status polling
- Multi-agent support (Claude, Codex, Copilot, Gemini, OpenCode)
- Agent-native skill deployment with per-agent path mapping
- Command translation per agent type
- Per-phase agent routing via config overrides
- SQLite persistence (global project index + per-project task databases)
- PR creation workflow via gh CLI
- Task search
- Theme/color configuration
- Dashboard mode (multi-project view)

## WHAT TO BUILD

- Rust daemon (agtxd) exposing WebSocket + REST API via axum
- Direct PTY process management replacing tmux dependency (portable-pty)
- SvelteKit frontend with kanban board matching current workflow semantics
- Structured agent output view (status, file changes, phase progress)
- Structured action buttons for known agent interactions (approve, reject) with free text fallback
- WebSocket streaming of live agent output to browser
- Full session history persisted to disk, lazy-loaded with virtualized infinite scrollback
- Summary banner on reconnect for quick orientation
- GitHub OAuth via oauth2-proxy restricted to a single allowed username
- Caddy reverse proxy with forward_auth and WebSocket passthrough
- System tab: live host metrics (CPU/RAM/disk/load) via sysinfo
- System tab: service log streaming (agtxd/web/proxy) via journalctl
- Structured logging with tracing + tracing-appender (rotation, non-blocking writes)
- Retire TUI — web is the sole interface

## CONSTRAINTS

- **Tech stack (core)**: Rust for daemon — reuses existing AGTX core logic directly
- **Tech stack (frontend)**: SvelteKit 5 — chosen for lightweight DX and fast builds
- **Auth**: External oauth2-proxy + Caddy gateway — avoids building custom auth/session management
- **Deployment**: Remote Linux server (systemd services) — must work headless
- **Process control**: portable-pty for agent PTY management — must handle spawn, read/write, resize, exit
- **Real-time**: WebSocket for bidirectional streaming — agents produce continuous output
- **Data**: Existing SQLite schema preserved — migration path from current AGTX databases

## SUGGESTED DEFAULTS

These are implementation suggestions, not mandated decisions. Proposed for speed and low risk, replaceable if planning/research finds a better fit.

- **Backend**: axum + WebSockets, portable-pty, sysinfo, rusqlite, tracing + tracing-appender
- **Frontend**: SvelteKit 5 + structured UI components
- **Auth shortcut**: oauth2-proxy + GitHub allowlist (`--github-user`)
- **Edge**: Caddy with `forward_auth` + `reverse_proxy` (native WebSocket support)
- **Observability**: sysinfo for host metrics, journalctl for service logs

## KEY DECISIONS

- Separate daemon + frontend (not monolith) — clean separation of concerns, frontend can iterate independently
- Direct PTY over tmux — eliminates runtime dependency, enables structured output parsing, richer process control
- Structured output over raw terminal — better UX for web, enables action buttons and status parsing
- oauth2-proxy over custom auth — avoids custom session/token management, proven component, minutes to configure
- Caddy over nginx — native WebSocket support, automatic HTTPS, simpler config, forward_auth built-in
- Web replaces TUI entirely — single UI to maintain, web is strictly more capable (remote access, richer rendering)
- Full history persisted to disk — fidelity over convenience, lazy loading + virtualized scroll handles performance

## OUT OF SCOPE

- Mobile-native app — browser is sufficient, responsive if needed later
- Multi-user / team access — single-user by design (GitHub username allowlist)
- Real-time collaboration — one user, one dashboard
- Raw terminal emulator in browser (xterm.js) — structured output is the value proposition, raw PTY rendering undermines it
- Backward compatibility with TUI mode — web fully replaces it
- Drag-and-drop task reordering — phase transitions have side effects, arbitrary movement conflicts with workflow engine
- Real-time collaborative editing — single-user tool, no concurrent editors
- Custom dashboard widget layout — fixed purposeful layout, customizable layouts add unnecessary complexity
- Plugin marketplace — plugins are TOML files, file-based system is simple and sufficient
- Email / push notifications — in-app toasts sufficient for single-user single-browser
- Chat interface with agent — agents are autonomous workers, structured input box covers needed interaction
- Undo/redo for state changes — phase transitions have real-world side effects (worktrees, processes, PRs)
- Embedded code editor — users have VS Code/Neovim, show diffs read-only instead

## PHASING GUIDANCE

Preferred phase ordering rationale:

1. Daemon foundation first (establish HTTP server skeleton)
2. PTY management second (highest risk component, fail fast)
3. WebSocket streaming (depends on daemon + PTY)
4. Frontend kanban board (can develop against working API; depends only on Phase 1)
5. Task detail + live output (depends on WebSocket + frontend)
6. Workflow engine (depends on PTY + frontend, preserves all AGTX semantics)
7. Structured output & session intelligence (depends on live output view)
8. System monitoring (depends on WebSocket streaming)
9. Auth & deployment (orthogonal, add last before polish)
10. UX polish last (keyboard shortcuts, theming, responsive, toasts, help)

Note: Phase 4 (Frontend Kanban) depends only on Phase 1 and could theoretically parallel Phase 2/3, but sequential execution avoids context switching.

## GSD SETTINGS

- Mode: interactive
- Granularity: fine
- Model profile: quality
- All workflow agents enabled (research, plan_check, verifier, nyquist_validation)
