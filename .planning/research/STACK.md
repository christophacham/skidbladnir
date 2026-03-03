# Technology Stack

**Project:** AGTX Web (daemon + web frontend for agent session management)
**Researched:** 2026-03-03

## Context

This stack covers the **new web-native components** being added to AGTX: a Rust daemon (agtxd) with WebSocket/REST API, a SvelteKit frontend, PTY process management, structured logging, and an external auth gateway. The existing Rust codebase (rusqlite, tokio, serde, anyhow, etc.) is preserved and reused directly.

---

## Recommended Stack

### HTTP/WebSocket Server

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **axum** | 0.8 | HTTP routing, WebSocket upgrade, REST API | Already in tokio ecosystem (AGTX uses tokio 1.44). Most popular Rust web framework since 2024. Native WebSocket support via `extract::ws`. Tower middleware integration means CORS, compression, tracing are plug-and-play. | HIGH |
| **tower-http** | 0.6 | CORS, compression, request tracing, timeout middleware | Official companion to axum. Provides `CorsLayer`, `CompressionLayer`, `TraceLayer` as composable middleware. Version 0.6 is the axum 0.8 compatible release. | HIGH |
| **axum-extra** | 0.10 | TypedHeader extraction, SSE support | Moved from axum core. Needed for typed header extraction and structured server-sent events if used alongside WebSocket. | HIGH |
| **tokio-tungstenite** | 0.28 | WebSocket protocol implementation (transitive) | Used internally by axum's `ws` feature. Not imported directly -- axum abstracts it. Listed for awareness. | HIGH |

**Key axum 0.8 changes to note:**
- Path parameter syntax changed: `/{id}` not `/:id`
- `Option<T>` extractors require `OptionalFromRequestParts` trait
- Feature flag `ws` required for WebSocket support

**axum feature flags needed:**
```toml
axum = { version = "0.8", features = ["ws", "macros"] }
```

### PTY Process Management

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **portable-pty** | 0.9 | Spawn agent processes in pseudo-terminals, read/write streams | Part of wezterm (battle-tested terminal emulator). Cross-platform. Provides `MasterPty`/`CommandBuilder` API for spawn, read, write, resize. No real competitor at this abstraction level in Rust. | MEDIUM |

**Why not alternatives:**

| Alternative | Why Not |
|-------------|---------|
| `pty-process` (doy) | Implements `tokio::io::AsyncRead/AsyncWrite` natively (advantage), but smaller community, less battle-tested than wezterm's crate. Could be revisited if portable-pty's blocking I/O proves painful. |
| `rustix::pty` | Too low-level. Provides raw `openpty`/`login_tty` syscalls. Would require building the entire process management layer manually. |
| `tokio-pty-process` | Abandoned (last update 2019). Do not use. |
| Raw `nix::pty` | Same problem as rustix -- too low-level, no process lifecycle management. |

**Critical portable-pty integration pattern:**

portable-pty's reader/writer are **blocking** (`std::io::Read`/`std::io::Write`). Integration with tokio requires:

1. **Reader**: Spawn a dedicated `tokio::task::spawn_blocking` thread that reads from `pty.try_clone_reader()` in a loop, sending bytes through an `mpsc` channel to async consumers.
2. **Writer**: Wrap `pty.take_writer()` behind a `tokio::sync::mpsc` channel where the async side sends bytes and a blocking thread consumes them.
3. **Drop semantics**: When the writer is dropped, EOF is sent to the child process. Must hold writer reference even if not writing, or risk deadlock.

This is a well-known pattern (wezterm discussions confirm it). Not elegant, but reliable. If this becomes a pain point, `pty-process` with native async I/O is the fallback.

### Frontend

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **Svelte** | 5 (latest: ~5.53) | Component framework with runes reactivity | Svelte 5's runes (`$state`, `$derived`, `$effect`) provide explicit, fine-grained reactivity perfect for real-time dashboard state. Compiles to minimal JS -- small bundle, fast load. No virtual DOM overhead. | HIGH |
| **SvelteKit** | 2 (latest: ~2.53) | Application framework, routing, build tooling | SvelteKit 2 with adapter-static builds to a pure SPA -- no Node server needed at runtime. File-based routing, prerendering for shell, client-side navigation. | HIGH |
| **@sveltejs/adapter-static** | 3.x | Build output adapter | Generates static HTML/CSS/JS that axum serves directly. No Node.js runtime dependency in production. SPA fallback mode (`fallback: 'index.html'`) handles client-side routing. | HIGH |
| **Vite** | 7.x | Dev server, HMR, bundling | Ships with SvelteKit. Vite 7 + Rolldown support landed in SvelteKit mid-2025 -- faster builds. | HIGH |

**SPA mode configuration:**
```javascript
// svelte.config.js
import adapter from '@sveltejs/adapter-static';
export default {
  kit: {
    adapter: adapter({ fallback: 'index.html' })
  }
};

// src/routes/+layout.js
export const prerender = false;
export const ssr = false;
```

**State management approach -- Svelte 5 runes, no external library:**
```svelte
<script>
  // Reactive state with $state rune
  let tasks = $state([]);
  let connected = $state(false);

  // Derived values auto-update
  let backlogCount = $derived(tasks.filter(t => t.status === 'backlog').length);

  // Side effects for WebSocket
  $effect(() => {
    if (connected) {
      // reconnection logic
    }
  });
</script>
```

No need for external state management (Zustand, Redux, etc.). Svelte 5 runes handle reactive state, derived computations, and effects natively. For cross-component shared state, use Svelte's context API or module-level `$state` exports.

**WebSocket client pattern:**

SvelteKit gained experimental native WebSocket support in March 2025, but for our use case (connecting to a separate Rust backend, not SvelteKit's own server), a plain browser `WebSocket` API with a reconnection wrapper is simpler and more reliable:

```typescript
// lib/ws.ts - thin wrapper, no library needed
function createSocket(url: string) {
  let ws = $state<WebSocket | null>(null);
  let status = $state<'connecting' | 'open' | 'closed'>('closed');

  function connect() {
    const socket = new WebSocket(url);
    socket.onopen = () => { ws = socket; status = 'open'; };
    socket.onclose = () => { status = 'closed'; setTimeout(connect, 1000); };
    socket.onmessage = (e) => handleMessage(JSON.parse(e.data));
  }
  // ...
}
```

No WebSocket client library needed. The browser API is sufficient. Add exponential backoff for reconnection.

### Authentication Gateway

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **oauth2-proxy** | 7.8+ (latest: ~7.14) | GitHub OAuth authentication proxy | Proven, maintained by active community. Supports `--github-user` flag for single-user allowlist. Handles OAuth flow, session cookies, token refresh. Zero custom auth code needed. | HIGH |
| **Caddy** | 2.11 | Reverse proxy, TLS termination, `forward_auth` | Automatic HTTPS via Let's Encrypt. Native WebSocket passthrough (auto-detected, unlike nginx). `forward_auth` directive purpose-built for auth proxy pattern. Simpler config than nginx. | HIGH |

**Architecture:**
```
Browser --> Caddy (:443) --> oauth2-proxy (:4180) --> GitHub OAuth
                |                    |
                |  forward_auth      | (validates session)
                |<-------------------+
                |
                +--> agtxd REST API (:3000/api/*)
                +--> agtxd WebSocket (:3000/ws/*)
                +--> static files (:3000/*)   (SvelteKit build output)
```

**Caddy configuration pattern:**
```caddyfile
agtx.example.com {
  forward_auth /oauth2/* oauth2-proxy:4180 {
    uri /oauth2/auth
    copy_headers X-Auth-Request-User X-Auth-Request-Email
  }

  reverse_proxy /api/* localhost:3000
  reverse_proxy /ws/*  localhost:3000
  reverse_proxy /*     localhost:3000
}
```

**oauth2-proxy configuration:**
```
--provider=github
--github-user=<your-username>
--email-domain=*
--upstream=static://200
--cookie-secret=<random-32-bytes>
--client-id=<github-oauth-app-id>
--client-secret=<github-oauth-app-secret>
```

Key detail: `--upstream=static://200` tells oauth2-proxy to return 200 on successful auth (let Caddy handle the actual proxying) rather than proxying the request itself.

### Host Metrics

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **sysinfo** | 0.38 | CPU, RAM, disk, load average, process list | 83M+ downloads, actively maintained, cross-platform. Single dependency for all system metrics. MSRV 1.88 (compatible with current toolchain 1.93). | HIGH |

**Usage pattern:**
```rust
use sysinfo::System;

let mut sys = System::new_all();
sys.refresh_all();

// CPU, memory, disk, load
let cpu_usage = sys.global_cpu_usage();
let total_mem = sys.total_memory();
let used_mem = sys.used_memory();
let load_avg = System::load_average();
```

### Structured Logging

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **tracing** | 0.1 (latest: 0.1.44) | Structured event/span instrumentation | Tokio ecosystem standard. Spans with enter/exit timing (not just log lines). Integrates with axum via `TraceLayer`. | HIGH |
| **tracing-subscriber** | 0.3 (latest: 0.3.22) | Subscriber composition, formatting, filtering | `EnvFilter` for runtime log level control via `RUST_LOG`. `fmt::Layer` for human-readable console output. Composable with `Registry`. | HIGH |
| **tracing-appender** | 0.2 (latest: 0.2.3) | Non-blocking file writer, log rotation | `RollingFileAppender` with daily/hourly rotation. `non_blocking()` returns a guard + writer -- writer goes to tracing, guard must be held for flush on shutdown. | HIGH |

**Configuration pattern:**
```rust
use tracing_subscriber::{fmt, EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::rolling;

let file_appender = rolling::daily("/var/log/agtxd", "agtxd.log");
let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

tracing_subscriber::registry()
    .with(EnvFilter::from_default_env().add_directive("agtxd=info".parse().unwrap()))
    .with(fmt::layer().with_writer(std::io::stdout))   // console
    .with(fmt::layer().json().with_writer(non_blocking)) // file (JSON)
    .init();

// CRITICAL: _guard must be held for the lifetime of the program
// Drop it only at shutdown to ensure buffered logs are flushed
```

### Database (Async Wrapper)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **tokio-rusqlite** | 0.7 | Async wrapper for rusqlite | AGTX already uses rusqlite 0.34. tokio-rusqlite wraps it with a background thread + channel pattern (same pattern as portable-pty I/O). Avoids blocking the tokio runtime on DB calls. All 42 rusqlite feature flags available. | MEDIUM |

**Why not migrate to sqlx or sea-orm:** The existing AGTX schema, migrations, and query patterns all use rusqlite directly. Rewriting the data layer is unnecessary risk. tokio-rusqlite is a thin async wrapper that preserves the existing code while making it safe to call from async handlers.

**Alternative:** Keep using `rusqlite` directly with `tokio::task::spawn_blocking` for each DB call. Simpler, no new dependency, but more boilerplate. Either approach works. Recommend tokio-rusqlite for cleaner code.

### Serialization (New Dependencies)

| Technology | Version | Purpose | Why | Confidence |
|------------|---------|---------|-----|------------|
| **serde** | 1.0 | Struct serialization/deserialization | Already in use. No change. | HIGH |
| **serde_json** | 1.0 | JSON for WebSocket messages and REST API | Already in use. No change. | HIGH |

---

## New Dependencies Summary

### Daemon (Rust) -- new additions to Cargo.toml

```toml
# HTTP/WebSocket server
axum = { version = "0.8", features = ["ws", "macros"] }
axum-extra = { version = "0.10", features = ["typed-header"] }
tower-http = { version = "0.6", features = ["cors", "compression-full", "trace", "timeout"] }
tower = "0.5"

# PTY management
portable-pty = "0.9"

# Host metrics
sysinfo = "0.38"

# Structured logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt", "json"] }
tracing-appender = "0.2"

# Async SQLite (optional, alternative to manual spawn_blocking)
tokio-rusqlite = { version = "0.7", features = ["bundled"] }
```

### Frontend (npm)

```bash
# Create project
npx sv create agtx-web  # Svelte 5 + SvelteKit 2
cd agtx-web

# Core (included by template)
# svelte@^5.0  @sveltejs/kit@^2.0  vite@^7.0

# Adapter
npm install -D @sveltejs/adapter-static

# No additional runtime dependencies needed for:
# - State management (Svelte 5 runes built-in)
# - WebSocket client (browser API)
# - Routing (SvelteKit file-based routing)
```

### Infrastructure (system packages)

```bash
# Caddy (v2.11+)
# Install via official repo or static binary
# https://caddyserver.com/docs/install

# oauth2-proxy (v7.8+)
# Install via official binary release or container image
# https://github.com/oauth2-proxy/oauth2-proxy/releases
```

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Web framework | axum 0.8 | actix-web 4 | actix-web has its own actor runtime, conflicting with tokio. axum is native tokio, matching existing AGTX runtime. |
| Web framework | axum 0.8 | rocket 0.5 | Rocket uses its own async runtime. Less middleware ecosystem than tower. |
| PTY | portable-pty 0.9 | pty-process (doy) | pty-process has native async I/O (advantage), but less battle-tested. Revisit if blocking I/O pattern proves problematic. |
| PTY | portable-pty 0.9 | raw nix::pty | Too low-level. Would require implementing process lifecycle, resize, reader/writer management manually. |
| Frontend | SvelteKit 2 + Svelte 5 | React/Next.js | Heavier runtime, more boilerplate for reactive state. Svelte compiles away the framework. Overkill for single-user dashboard. |
| Frontend | SvelteKit 2 + Svelte 5 | plain Svelte (no SvelteKit) | Lose file-based routing, adapter system, and build tooling. SvelteKit provides these for free. |
| Frontend state | Svelte 5 runes | Zustand/Redux/nanostores | Unnecessary. Runes ($state, $derived, $effect) cover reactive state, derived values, and side effects natively. Adding a state library is overhead with no benefit. |
| WebSocket client | Browser WebSocket API | socket.io-client | socket.io adds reconnection/multiplexing, but also adds a large dependency and requires a compatible server. Browser WebSocket + manual reconnect is simpler for our single-connection use case. |
| Auth | oauth2-proxy + Caddy | Custom auth in axum | Custom auth means implementing OAuth flow, session management, CSRF protection, token refresh. oauth2-proxy does all of this as a standalone binary. Proven in production by thousands of deployments. |
| Auth proxy | Caddy | nginx | Caddy auto-manages TLS, has built-in `forward_auth`, auto-detects WebSocket upgrades. nginx requires manual `proxy_set_header Upgrade`, manual cert management (certbot), and more verbose config. |
| Database async | tokio-rusqlite 0.7 | sqlx (with SQLite) | Would require rewriting all queries from rusqlite API to sqlx API. Unnecessary migration risk for a working data layer. |
| Logging | tracing + appender | log + env_logger | tracing provides spans (enter/exit timing), structured fields, and tower integration. `log` is fire-and-forget messages only. tracing is strictly better for a long-running daemon. |
| Metrics | sysinfo 0.38 | procfs + manual parsing | sysinfo abstracts cross-platform differences. Direct procfs parsing is Linux-only and fragile across kernel versions. |

---

## What NOT to Use

| Technology | Reason |
|------------|--------|
| **xterm.js / terminal emulator in browser** | PROJECT.md explicitly scopes this out. Structured output replaces raw terminal rendering. |
| **socket.io** | Overkill for single WebSocket connection to known backend. Adds complexity and server-side dependency. |
| **Diesel ORM** | AGTX uses raw rusqlite queries. Adding an ORM to an existing schema with working migrations is unnecessary abstraction. |
| **warp** | Effectively unmaintained. axum superseded it in the tokio ecosystem. |
| **tmux (in daemon)** | The whole point of agtxd is replacing tmux with direct PTY control. |
| **Electron / Tauri** | Web browser is the client. No desktop app wrapper needed. |
| **GraphQL** | Single client, known schema, no need for query flexibility. REST + WebSocket is simpler. |
| **Redis / message queue** | Single-process daemon, single user. In-process channels (`tokio::sync::broadcast/mpsc`) are sufficient for pub/sub between PTY readers and WebSocket writers. |

---

## Version Verification

| Crate/Package | Verified Version | Source | Date Verified |
|---------------|-----------------|--------|---------------|
| axum | 0.8.8 | docs.rs | 2026-03-03 |
| tower-http | 0.6.8 | docs.rs | 2026-03-03 |
| portable-pty | 0.9.0 | crates.io, docs.rs | 2026-03-03 |
| sysinfo | 0.38.3 | docs.rs | 2026-03-03 |
| tracing | 0.1.44 | docs.rs | 2026-03-03 |
| tracing-subscriber | 0.3.22 | docs.rs | 2026-03-03 |
| tracing-appender | 0.2.3 | docs.rs, crates.io | 2026-03-03 |
| tokio-rusqlite | 0.7.0 | crates.io | 2026-03-03 |
| Svelte | 5.53.x | npm | 2026-03-03 |
| @sveltejs/kit | 2.53.x | npm | 2026-03-03 |
| @sveltejs/adapter-static | 3.0.x | npm | 2026-03-03 |
| Caddy | 2.11.1 | GitHub releases | 2026-03-03 |
| oauth2-proxy | 7.8+ (up to 7.14.x) | GitHub releases | 2026-03-03 |

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| axum + tower-http | HIGH | Verified versions on docs.rs. Most documented Rust web framework. Tokio-native. |
| portable-pty | MEDIUM | Verified version. Battle-tested in wezterm. Blocking I/O requires spawn_blocking wrapper -- known pattern but adds complexity. pty-process is async-native fallback. |
| SvelteKit + Svelte 5 | HIGH | Verified versions on npm. Runes reactivity is stable (released Oct 2024). adapter-static SPA mode is well-documented. |
| oauth2-proxy + Caddy | HIGH | Both actively maintained, well-documented integration. Caddy's forward_auth + oauth2-proxy is a canonical pattern. |
| sysinfo | HIGH | Verified version. 83M+ downloads, actively maintained, API is straightforward. |
| tracing stack | HIGH | Tokio ecosystem standard. Verified versions. Well-documented patterns for multi-layer subscriber setup. |
| tokio-rusqlite | MEDIUM | Verified version. Thin wrapper, low risk. Alternative (manual spawn_blocking) is always available as fallback. |

---

## Sources

- [axum 0.8.0 announcement](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) -- Official Tokio blog
- [axum docs.rs](https://docs.rs/axum/latest/axum/) -- API documentation
- [axum WebSocket module](https://docs.rs/axum/latest/axum/extract/ws/index.html) -- WebSocket handler patterns
- [axum WebSocket examples](https://github.com/tokio-rs/axum/blob/main/examples/websockets/src/main.rs) -- Official examples
- [portable-pty docs](https://docs.rs/portable-pty/0.9.0/portable_pty/) -- API documentation
- [portable-pty WebSocket discussion](https://github.com/wezterm/wezterm/discussions/6484) -- PTY-to-WebSocket pattern
- [pty-process crate](https://lib.rs/crates/pty-process) -- Async alternative
- [Svelte 5 runes docs](https://svelte.dev/docs/svelte/$state) -- $state, $derived, $effect
- [SvelteKit SPA mode](https://svelte.dev/docs/kit/single-page-apps) -- adapter-static SPA configuration
- [SvelteKit state management](https://svelte.dev/docs/kit/state-management) -- Official patterns
- [SvelteKit native WebSocket support](https://svelte.dev/blog/whats-new-in-svelte-march-2025) -- Experimental, March 2025
- [oauth2-proxy Caddy integration](https://oauth2-proxy.github.io/oauth2-proxy/next/configuration/integrations/caddy/) -- Official docs
- [oauth2-proxy GitHub provider](https://oauth2-proxy.github.io/oauth2-proxy/configuration/providers/github/) -- GitHub OAuth config
- [Caddy forward_auth](https://caddyserver.com/docs/caddyfile/directives/forward_auth) -- Official Caddy docs
- [Caddy reverse_proxy](https://caddyserver.com/docs/caddyfile/directives/reverse_proxy) -- WebSocket auto-detection
- [sysinfo crate](https://docs.rs/sysinfo/latest/sysinfo/) -- API documentation
- [tracing crate](https://docs.rs/tracing) -- Instrumentation framework
- [tracing-appender](https://docs.rs/tracing-appender/latest/tracing_appender/) -- Rolling file appender, non-blocking writer
- [tracing-subscriber](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/) -- EnvFilter, fmt, layered subscribers
- [tokio-rusqlite](https://docs.rs/tokio-rusqlite) -- Async rusqlite wrapper
- [tower-http](https://lib.rs/crates/tower-http) -- CORS, compression, tracing middleware

---

*Stack research: 2026-03-03*
