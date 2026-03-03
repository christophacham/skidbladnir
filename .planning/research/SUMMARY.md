# Research Summary: AGTX Web

**Domain:** Web-native agent session management dashboard (daemon + frontend)
**Researched:** 2026-03-03
**Overall confidence:** HIGH

## Executive Summary

The stack for adding a web frontend and daemon API to the existing AGTX Rust TUI is well-established and mature. Every major component has a clear best-in-class choice with active maintenance and strong community adoption: axum for the HTTP/WebSocket server, portable-pty for PTY management, Svelte 5 + SvelteKit 2 for the frontend, and oauth2-proxy + Caddy for the auth gateway.

The highest-risk area is the PTY-to-async bridge. portable-pty (from wezterm) is the most battle-tested Rust PTY crate, but its blocking I/O model requires a `spawn_blocking` wrapper pattern to integrate with tokio. This is a known, documented pattern, but it adds complexity and is the most likely source of subtle bugs (deadlocks from dropped writers, buffer pressure between blocking reads and async consumers). The alternative crate `pty-process` offers native `AsyncRead`/`AsyncWrite`, but is less proven. Starting with portable-pty and keeping pty-process as a fallback is the pragmatic choice.

The frontend story is straightforward. SvelteKit 2 with adapter-static builds to a pure SPA that axum can serve as static files. Svelte 5's runes system ($state, $derived, $effect) eliminates the need for external state management libraries. WebSocket client needs are simple enough for the browser API -- no socket.io or similar library required.

The auth gateway (oauth2-proxy + Caddy) is the biggest time saver. Rather than implementing OAuth flows, session management, and CSRF protection in Rust, the entire auth layer is handled by two well-maintained standalone services with a canonical integration pattern. Caddy's `forward_auth` directive + oauth2-proxy's `--upstream=static://200` mode is the textbook configuration.

## Key Findings

**Stack:** axum 0.8 + portable-pty 0.9 + SvelteKit 2 (Svelte 5) + oauth2-proxy + Caddy 2.11 + tracing + sysinfo 0.38
**Architecture:** Rust daemon serving both REST API and WebSocket on one port, SvelteKit SPA as static files, Caddy as edge proxy with forward_auth to oauth2-proxy
**Critical pitfall:** portable-pty blocking I/O requires careful async bridging -- dropped writers cause EOF/deadlock, buffer pressure can stall readers

## Implications for Roadmap

Based on research, suggested phase structure:

1. **Daemon foundation** - axum server with basic REST endpoints, structured logging, health checks
   - Addresses: HTTP server, logging infrastructure, daemon lifecycle
   - Avoids: Tackling PTY complexity before the server framework is proven
   - Rationale: Establishes the skeleton that everything else plugs into

2. **PTY process management** - Replace tmux with portable-pty, implement spawn/read/write/resize
   - Addresses: Agent process lifecycle, blocking-to-async bridge
   - Avoids: Coupling PTY work to WebSocket streaming (test PTY independently first)
   - Rationale: Highest-risk component, should be isolated and tested early

3. **WebSocket streaming** - Live agent output from PTY to browser, input from browser to PTY
   - Addresses: Real-time bidirectional communication, session history persistence
   - Avoids: Frontend complexity (test with CLI WebSocket client first)
   - Rationale: Depends on both daemon (phase 1) and PTY (phase 2)

4. **Frontend** - SvelteKit SPA with kanban board, agent output view, system metrics
   - Addresses: All UI requirements, structured output display
   - Avoids: Auth complexity (test behind no auth first, add gateway later)
   - Rationale: Requires working API and WebSocket to build against

5. **Auth gateway + deployment** - Caddy + oauth2-proxy, systemd services, production config
   - Addresses: Secure remote access, TLS, GitHub OAuth
   - Avoids: None -- this is the final integration layer
   - Rationale: Auth is orthogonal to functionality, add last

**Phase ordering rationale:**
- Server foundation must exist before anything else can integrate
- PTY management is highest-risk and most novel -- tackle early, fail fast
- WebSocket streaming depends on PTY being functional
- Frontend needs a working backend to develop against (can use mock data initially, but real integration should follow soon)
- Auth gateway is configuration, not code -- add after functionality is proven

**Research flags for phases:**
- Phase 2 (PTY): Likely needs deeper research on async bridging patterns, resize handling, signal propagation
- Phase 3 (WebSocket): May need research on message protocol design, history persistence format, reconnection semantics
- Phase 5 (Auth): Standard patterns, unlikely to need additional research

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack choices | HIGH | All libraries verified with current versions, clear community consensus |
| Frontend | HIGH | Svelte 5 runes and SvelteKit SPA mode are well-documented, stable |
| Auth gateway | HIGH | Canonical pattern with extensive documentation |
| PTY integration | MEDIUM | Library is solid but async bridging pattern adds complexity |
| Database async | MEDIUM | tokio-rusqlite is thin wrapper; manual spawn_blocking is viable fallback |

## Gaps to Address

- **PTY resize propagation**: How does `portable-pty` handle SIGWINCH / terminal resize when the "terminal" is a browser viewport? Needs investigation during PTY phase.
- **Structured output parsing**: How to detect agent status (working/idle/done) from PTY output streams? Parsing heuristics need design.
- **Session history format**: What serialization format for persisted PTY history? Raw bytes vs parsed structured events? Needs design during WebSocket phase.
- **WebSocket message protocol**: JSON message schema for PTY output, agent actions, system metrics. Needs design.
- **Reconnection semantics**: How much history to replay on WebSocket reconnect? Summary banner design needs UX thought.
- **Systemd integration**: Service file structure for agtxd + Caddy + oauth2-proxy. Standard but needs concrete configuration.

---

*Research summary: 2026-03-03*
