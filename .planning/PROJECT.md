# AGTX Web

## What This Is

A web-native version of AGTX that preserves the exact current workflow semantics — plugins, phase lifecycle, artifact tracking, worktree isolation, per-phase agent routing — but replaces terminal/tmux interaction with a browser UI backed by a persistent Rust daemon. Designed for remote server deployment with secure single-user access via GitHub OAuth.

## Core Value

Manage multiple coding agent sessions from any browser with full reconnectable history, without losing any of the workflow semantics that make AGTX useful.

## Requirements

### Validated

<!-- Existing capabilities in the current AGTX TUI codebase -->

- ✓ Kanban board with Backlog → Planning → Running → Review → Done workflow — existing
- ✓ Git worktree isolation per task (`.agtx/worktrees/{slug}`) — existing
- ✓ Plugin system (TOML config) with commands, prompts, artifacts, prompt_triggers, copy_back, cyclic phases — existing
- ✓ Phase lifecycle with artifact detection and status polling — existing
- ✓ Multi-agent support (Claude, Codex, Copilot, Gemini, OpenCode) — existing
- ✓ Agent-native skill deployment with per-agent path mapping — existing
- ✓ Command translation per agent type — existing
- ✓ Per-phase agent routing via config overrides — existing
- ✓ SQLite persistence (global project index + per-project task databases) — existing
- ✓ PR creation workflow via gh CLI — existing
- ✓ Task search — existing
- ✓ Theme/color configuration — existing
- ✓ Dashboard mode (multi-project view) — existing

### Active

<!-- Web-native capabilities to build -->

- [ ] Rust daemon (agtxd) exposing WebSocket + REST API via axum
- [ ] Direct PTY process management replacing tmux dependency (portable-pty)
- [ ] SvelteKit frontend with kanban board matching current workflow semantics
- [ ] Structured agent output view (status, file changes, phase progress)
- [ ] Structured action buttons for known agent interactions (approve, reject) with free text fallback
- [ ] WebSocket streaming of live agent output to browser
- [ ] Full session history persisted to disk, lazy-loaded with virtualized infinite scrollback
- [ ] Summary banner on reconnect for quick orientation
- [ ] GitHub OAuth via oauth2-proxy restricted to a single allowed username
- [ ] Caddy reverse proxy with forward_auth and WebSocket passthrough
- [ ] System tab: live host metrics (CPU/RAM/disk/load) via sysinfo
- [ ] System tab: service log streaming (agtxd/web/proxy) via journalctl
- [ ] Structured logging with tracing + tracing-appender (rotation, non-blocking writes)
- [ ] Retire TUI — web is the sole interface

### Out of Scope

- Mobile-native app — browser is sufficient, responsive if needed later
- Multi-user / team access — single-user by design (GitHub username allowlist)
- Real-time collaboration — one user, one dashboard
- Raw terminal emulator in browser — structured output replaces xterm.js-style PTY rendering
- Backward compatibility with TUI mode — web fully replaces it

## Context

AGTX is an existing ~8K line Rust codebase with a well-structured module system. The core logic (config, plugin, artifact, worktree, phase lifecycle, agent routing, database) is already separated from the TUI layer via traits and clean module boundaries. The web version reuses this core engine, replacing only the presentation and process management layers.

The current TUI uses tmux for agent process management. The web version replaces tmux with direct PTY control, which enables richer structured output parsing and eliminates a runtime dependency.

The existing trait-based architecture (`TmuxOperations`, `GitOperations`, `AgentOperations`) was designed for testability but conveniently enables swapping implementations — the PTY-based process manager can implement the same interface patterns.

Codebase map available at `.planning/codebase/` (7 documents, mapped 2026-03-03).

## Constraints

- **Tech stack (core)**: Rust for daemon — reuses existing AGTX core logic directly
- **Tech stack (frontend)**: SvelteKit 5 — chosen for lightweight DX and fast builds
- **Auth**: External oauth2-proxy + Caddy gateway — avoids building custom auth/session management
- **Deployment**: Remote Linux server (systemd services) — must work headless
- **Process control**: portable-pty for agent PTY management — must handle spawn, read/write, resize, exit
- **Real-time**: WebSocket for bidirectional streaming — agents produce continuous output
- **Data**: Existing SQLite schema preserved — migration path from current AGTX databases

## Suggested Defaults

These are implementation suggestions, not mandated decisions. They are proposed for speed and low risk, and we can replace any of them if planning/research finds a better fit.

- **Suggested backend**: axum + WebSockets, portable-pty, sysinfo, rusqlite, tracing + tracing-appender
- **Suggested frontend**: SvelteKit 5 + structured UI components
- **Suggested auth shortcut**: oauth2-proxy + GitHub allowlist (`--github-user`)
- **Suggested edge**: Caddy with `forward_auth` + `reverse_proxy` (native WebSocket support)
- **Suggested observability**: sysinfo for host metrics, journalctl for service logs

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Separate daemon + frontend (not monolith) | Clean separation of concerns, frontend can iterate independently | — Pending |
| Direct PTY over tmux | Eliminates runtime dependency, enables structured output parsing, richer process control | ✓ Confirmed — pty-process 0.5 with native async I/O, Phase 2 |
| Structured output over raw terminal | Better UX for web, enables action buttons and status parsing | — Pending |
| oauth2-proxy over custom auth | Avoids custom session/token management, proven component, minutes to configure | — Pending |
| Caddy over nginx | Native WebSocket support, automatic HTTPS, simpler config, forward_auth built-in | — Pending |
| Web replaces TUI entirely | Single UI to maintain, web is strictly more capable (remote access, richer rendering) | — Pending |
| Full history persisted to disk | Fidelity over convenience — lazy loading + virtualized scroll handles performance | — Pending |
| pty-process over portable-pty | Native tokio AsyncRead/AsyncWrite, built-in setsid(), pre_exec support, returns tokio::process::Child | ✓ Confirmed — Phase 2 |
| Base64 output encoding over raw octet-stream | Simpler for API consumers, JSON-native transport | ✓ Confirmed — Phase 2 |
| procfs crate for /proc reading | Type-safe Rust API vs hand-parsing, provides Process/Stat/StatM structs | ✓ Confirmed — Phase 2 |
| Delta CPU% over cumulative average | Instantaneous usage via tick deltas more useful for detecting runaway processes | ✓ Confirmed — Phase 2 |

---
*Last updated: 2026-03-04 after Phase 2*
