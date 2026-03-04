# Phase 5: Task Detail & Live Output - Research

**Researched:** 2026-03-04
**Domain:** SvelteKit frontend / WebSocket client / real-time streaming UI
**Confidence:** HIGH

## Summary

Phase 5 connects the SvelteKit frontend (Phase 4) to the WebSocket streaming backend (Phase 3), delivering the first real-time agent interaction in the browser. The core work is: (1) a WebSocket client store that manages connections per session, (2) a split-view detail panel that renders live output with type classification, (3) phase status propagation to task cards, and (4) a connection status indicator in the NavBar.

The backend is already complete and well-structured. The WebSocket protocol (`ServerMessage`/`ClientMessage` in `ws.rs`) supports output streaming, state changes, metrics, errors, and bidirectional write/resize. The REST output endpoint (`GET /sessions/{id}/output?offset=N&limit=M`) supports lazy history loading. All frontend work is pure SvelteKit/Svelte 5 -- no new backend changes required.

**Primary recommendation:** Build a class-based `WebSocketStore` (`.svelte.ts`) following the existing store pattern, wire it into a `DetailPanel.svelte` component that renders output with left-border color coding, and extend `UiStore` with selected-task state for the split-view layout.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Split-view layout: slide-in from right, board compresses to ~40% width, detail panel takes ~60%
- Smooth CSS transition for panel open/close
- Board columns shrink but remain usable while panel is open
- Clicking a different task card switches detail content in-place (no close/reopen)
- Close via Escape key + close button in panel header
- Panel header shows: task title, agent badge pill, live phase status indicator, close button
- Plain monospace text in a scrollable container
- Decode base64 WebSocket bytes to UTF-8 and display as-is
- No terminal emulation (no ANSI cursor/escape sequences)
- Auto-scroll to bottom as new data arrives; pauses when user scrolls up
- Floating "Jump to bottom" button appears when auto-scroll is paused (WS-05)
- On reconnect/open: show recent tail (~64KB ring buffer via WS), lazy-load older history on scroll-up via REST endpoint with offset/limit
- Fixed input bar at bottom of detail panel for sending text to agent PTY stdin via WebSocket write message
- Left-border color coding per output type: normal text (no border), tool calls (blue/cyan left border), errors (red left border)
- Simple heuristic regex patterns to classify output blocks
- Not collapsible in Phase 5
- Errors in output stream cause task card status dot to reflect error state
- Task cards: colored dot with animation -- green pulsing (Working), yellow steady (Idle), green checkmark (Ready), gray dot (Exited)
- Replaces placeholder `statusDotColor` in existing TaskCard component
- Detail panel header: colored status badge next to task title
- Status sourced via WebSocket `state` messages for tasks with active sessions; static status for Backlog/Done tasks
- Connection status indicator in NavBar: green (connected), orange (reconnecting), red (disconnected)

### Claude's Discretion
- Exact CSS transition timing and easing for panel slide-in
- Monospace font choice and line height for output area
- "Jump to bottom" button positioning and styling
- Input bar design (placeholder text, send button vs Enter-only)
- Heuristic patterns for output type classification (agent-specific tuning)
- How to handle very long output lines (wrap vs horizontal scroll)
- Reconnection retry strategy and backoff timing
- Whether to show a loading skeleton while initial history loads

### Deferred Ideas (OUT OF SCOPE)
- Collapsible semantic output sections (thinking, tool use, file edits) -- Phase 7 (OUTPUT-05)
- Action buttons for agent prompts (approve/reject) -- Phase 7 (OUTPUT-06)
- Phase progress timeline with timestamps -- Phase 7 (OUTPUT-07)
- Artifact detection display -- Phase 7 (OUTPUT-08)
- Full-text search within session output -- Phase 7 (WS-07)
- Reconnect summary banner -- Phase 7 (WS-06)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| OUTPUT-01 | Clicking a task opens split-view detail panel (board left, detail right) | Split-view layout via CSS grid with transition; UiStore extension for selectedTask; Board grid-template-columns adjustment |
| OUTPUT-02 | Detail panel streams live agent output in real time | WebSocketStore manages per-session connections; base64 decode + TextDecoder for UTF-8; append to scrollable container |
| OUTPUT-03 | Agent text, tool calls, and errors are visually distinct | Left-border color coding with heuristic regex classification; OutputBlock type with variant styling |
| OUTPUT-04 | Task cards and detail panel show phase status (Working/Idle/Ready/Exited) | WebSocket `state` messages update phaseStatuses map in store; TaskCard reads from store; pulsing dot animations via CSS |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Svelte 5 | ^5.0.0 | Component framework with runes reactivity | Already installed, project standard |
| SvelteKit | ^2.0.0 | App framework with routing | Already installed, project standard |
| Tailwind CSS | ^4.0.0 | Utility-first styling | Already installed, project standard |
| Vite | ^6.0.0 | Build tool and dev server with WS proxy | Already installed, project standard |
| Vitest | ^3.0.0 | Test framework | Already installed, project standard |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| Native WebSocket API | Browser built-in | WebSocket client | Always -- no wrapper library needed |
| TextDecoder API | Browser built-in | Base64 bytes to UTF-8 string | Every output message decode |
| atob() | Browser built-in | Base64 string to binary string | Fallback if Uint8Array.fromBase64 unavailable |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Native WebSocket | reconnecting-websocket library | Adds dependency; project pattern prefers zero deps; manual reconnect is simple enough |
| TextDecoder | Custom UTF-8 decoder | No reason; TextDecoder is standard, performant, handles multi-byte correctly |

**Installation:**
```bash
# No new packages needed. All dependencies already installed.
```

## Architecture Patterns

### Recommended Project Structure
```
web/src/lib/
├── stores/
│   ├── websocket.svelte.ts     # WebSocket connection manager + phase status tracking
│   └── ui.svelte.ts            # Extended with selectedTask, detailPanelOpen
├── components/
│   ├── DetailPanel.svelte      # Split-view detail panel (output + input bar)
│   ├── OutputView.svelte       # Scrollable output with auto-scroll + classified blocks
│   ├── InputBar.svelte         # Fixed bottom input for PTY stdin writes
│   ├── StatusDot.svelte        # Animated phase status dot (reusable in card + panel)
│   ├── Board.svelte            # Modified: grid adjusts for detail panel
│   ├── TaskCard.svelte         # Modified: onclick + live status dot
│   ├── NavBar.svelte           # Modified: connection status indicator
│   └── Column.svelte           # Modified: pass onclick handler through
├── api/
│   └── sessions.ts             # REST helpers: fetchSessionOutput, listSessions
└── types/
    └── index.ts                # Extended with WebSocket message types
```

### Pattern 1: Class-Based Svelte 5 Store for WebSocket
**What:** A singleton `WebSocketStore` class using `$state` runes, exported as a module-level instance from a `.svelte.ts` file.
**When to use:** For all WebSocket connection state, output buffering, and phase status tracking.
**Example:**
```typescript
// web/src/lib/stores/websocket.svelte.ts
// Follows exact pattern of existing taskStore, projectStore, uiStore

class WebSocketStore {
  // Connection state
  socket = $state<WebSocket | null>(null);
  connectionStatus = $state<'connected' | 'reconnecting' | 'disconnected'>('disconnected');
  activeSessionId = $state<string | null>(null);

  // Output buffer (append-only text blocks)
  outputBlocks = $state<OutputBlock[]>([]);
  totalBytes = $state(0);

  // Phase status per session (map of session_id -> PhaseStatus)
  phaseStatuses = $state<Map<string, PhaseStatus>>(new Map());

  // Track cursor for reconnection
  private cursor = 0;

  connect(sessionId: string): void { /* ... */ }
  disconnect(): void { /* ... */ }
  send(message: ClientMessage): void { /* ... */ }
}

export const wsStore = new WebSocketStore();
```

### Pattern 2: Output Block Classification
**What:** Incoming output text is split into blocks classified by heuristic regex, each rendered with appropriate left-border styling.
**When to use:** Every time new output bytes arrive via WebSocket.
**Example:**
```typescript
// Classification types
type OutputBlockType = 'text' | 'tool_call' | 'error';

interface OutputBlock {
  id: number;
  text: string;
  type: OutputBlockType;
  timestamp: number;
}

// Heuristic classifier
function classifyBlock(text: string): OutputBlockType {
  // Error patterns (check first -- most specific)
  if (/^(Error|error|ERROR|FATAL|panic|FAIL)[:!]/.test(text)) return 'error';
  if (/\b(traceback|exception|failed|denied)\b/i.test(text)) return 'error';

  // Tool call patterns (agent-specific)
  if (/^(Read|Write|Edit|Bash|Glob|Grep|WebSearch|WebFetch)\(/.test(text)) return 'tool_call';
  if (/^\s*\$ /.test(text)) return 'tool_call';  // Shell commands
  if (/^(Creating|Updating|Deleting|Reading) /.test(text)) return 'tool_call';

  return 'text';
}
```

### Pattern 3: Auto-Scroll with User Override
**What:** Scroll container auto-scrolls to bottom on new content unless user has scrolled up. Shows "Jump to bottom" button when paused.
**When to use:** The output view component.
**Example:**
```typescript
// Track whether user is at bottom
let container = $state<HTMLDivElement | null>(null);
let userScrolledUp = $state(false);

function handleScroll() {
  if (!container) return;
  const { scrollTop, scrollHeight, clientHeight } = container;
  // Consider "at bottom" if within 50px of bottom
  userScrolledUp = scrollHeight - scrollTop - clientHeight > 50;
}

function scrollToBottom() {
  container?.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
  userScrolledUp = false;
}

// After appending new content:
$effect(() => {
  if (outputBlocks.length > 0 && !userScrolledUp && container) {
    // Use queueMicrotask to scroll after DOM update
    queueMicrotask(() => container?.scrollTo({ top: container.scrollHeight }));
  }
});
```

### Pattern 4: Split-View CSS Grid Layout
**What:** Board component uses CSS grid. When detail panel is open, grid gains a new column. CSS transition handles smooth animation.
**When to use:** The main page layout.
**Example:**
```svelte
<!-- +page.svelte or Board wrapper -->
<div
  class="h-full"
  style="
    display: grid;
    grid-template-columns: {detailOpen ? '40fr 60fr' : '1fr'};
    transition: grid-template-columns 0.3s ease-in-out;
  "
>
  <Board />
  {#if uiStore.selectedTask}
    <DetailPanel task={uiStore.selectedTask} />
  {/if}
</div>
```

### Pattern 5: Task-Session Linkage
**What:** Tasks have `session_name` (tmux-style name, may be null for Backlog tasks). The daemon's sessions are identified by UUID. Since Phase 6 (Workflow Engine) handles the actual task-to-session lifecycle, Phase 5 needs a bridge: query the sessions list endpoint and match by a convention or add a session lookup endpoint.
**When to use:** When opening a detail panel for a task that has an active agent session.
**Important design note:** The current daemon session system does not store task IDs. For Phase 5, the practical approach is:
- For tasks without sessions (Backlog, Done): show task details only (title, description, status), no output panel
- For tasks with sessions: the Workflow Engine (Phase 6) will assign session UUIDs to tasks. Phase 5 should define the interface (`task.session_id: string | null`) and wire up the WebSocket connection when a session_id is present
- Phase 5 can add a `session_id` field to the Task type in the frontend and have the detail panel gracefully handle missing sessions

### Anti-Patterns to Avoid
- **Creating one WebSocket per component mount:** Open one WS per selected session, reuse it. Close on panel close or task switch. Never create multiple connections to the same session.
- **Storing raw byte arrays in state:** Decode to string immediately on receipt. Storing Uint8Array in reactive state causes unnecessary re-renders.
- **Using innerHTML for output rendering:** Always use textContent or Svelte `{text}` bindings. Output contains raw PTY data that could include HTML-like sequences.
- **Polling for phase status:** Use WebSocket `state` messages for live status. Only use static status for tasks without sessions.
- **Blocking the main thread with large output processing:** If output is very large, use chunked processing or requestAnimationFrame batching.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Base64 decoding | Custom base64 decoder | `atob()` + `Uint8Array` then `TextDecoder` | Standard API, handles edge cases, performant |
| WebSocket reconnection | Complex state machine | Simple retry with exponential backoff (3 states) | The reconnection logic is straightforward; overengineering adds bugs |
| CSS animations for status dots | JavaScript-driven animation | CSS `@keyframes` pulse animation | Performant, declarative, no JS overhead |
| Output virtualization | Custom virtual scroll | Simple DOM append with max block limit | Phase 5 is raw output; virtualization complexity is Phase 7+ territory |
| UTF-8 multi-byte handling | Manual byte assembly | `TextDecoder` with `stream: true` option | TextDecoder handles incomplete multi-byte sequences at chunk boundaries |

**Key insight:** Phase 5 is explicitly "raw-but-styled output" -- the CONTEXT.md defers structured parsing to Phase 7. Keep the output rendering simple: decode, classify, append, scroll. Do not build infrastructure for collapsible sections, action buttons, or virtual scrolling yet.

## Common Pitfalls

### Pitfall 1: Multi-Byte UTF-8 Split Across WebSocket Messages
**What goes wrong:** A multi-byte UTF-8 character (e.g., emoji, CJK) gets split across two WebSocket messages. Decoding each independently produces garbled output or replacement characters.
**Why it happens:** WebSocket output messages carry raw PTY bytes. A 4-byte emoji might have 2 bytes in one message and 2 in the next.
**How to avoid:** Use `TextDecoder` with `stream: true` option. Create one decoder instance per session connection and reuse it across messages. The `stream` flag tells TextDecoder to buffer incomplete sequences.
**Warning signs:** Garbled characters at the end of output chunks, especially with emoji or non-ASCII output.

### Pitfall 2: Auto-Scroll Race with DOM Updates
**What goes wrong:** Calling `scrollTo()` before the DOM has rendered new content results in scrolling to the wrong position (before the new content).
**Why it happens:** Svelte updates the DOM asynchronously after state changes. Setting scrollTop immediately after pushing to the output array may fire before the DOM reflects the new items.
**How to avoid:** Use `$effect` or `queueMicrotask` / `requestAnimationFrame` to defer scroll until after DOM paint. Better yet, use `$effect` which runs after DOM updates in Svelte 5.
**Warning signs:** Output appears but viewport doesn't scroll to show it; intermittent scroll jumps.

### Pitfall 3: WebSocket Connection Leak on Task Switch
**What goes wrong:** Switching from task A to task B without closing the WebSocket for A leaves orphaned connections consuming memory and bandwidth.
**Why it happens:** The `connect()` call for task B doesn't automatically close the connection to task A.
**How to avoid:** In the `connect()` method, always call `disconnect()` first. In `UiStore.selectTask()`, trigger disconnect before connecting to the new session.
**Warning signs:** Multiple WebSocket connections visible in browser DevTools; growing memory usage over time.

### Pitfall 4: Vite Dev Server WebSocket Proxy Not Configured
**What goes wrong:** WebSocket connections fail during development because the Vite proxy only handles HTTP, not WebSocket upgrades.
**Why it happens:** The current `vite.config.ts` has `proxy: { '/api': { target: ..., changeOrigin: true } }` but no `ws: true` flag.
**How to avoid:** Add `ws: true` to the `/api` proxy config in `vite.config.ts`. This tells Vite's http-proxy to handle WebSocket upgrade requests.
**Warning signs:** WebSocket connection immediately closes or gets 404 in development mode; works in production.

### Pitfall 5: CSS Grid Transition Not Animating
**What goes wrong:** Setting `grid-template-columns` via a CSS transition doesn't animate in some browsers, causing an abrupt layout jump.
**Why it happens:** CSS transitions on `grid-template-columns` with `fr` units are supported in modern browsers but can be janky with complex grids.
**How to avoid:** Use percentage-based or pixel-based widths for the transition, or use a wrapper div with `width` transition instead of relying on grid-template-columns animation. Alternatively, use `flex` layout with `flex-basis` transition.
**Warning signs:** Panel appears/disappears instantly instead of sliding in; layout shifts during animation.

### Pitfall 6: Output Buffer Memory Growth
**What goes wrong:** Appending every WebSocket message to an in-memory array without bounds causes the browser tab to consume gigabytes of memory for long-running sessions.
**Why it happens:** Agent sessions can produce megabytes of output. Storing all of it as classified OutputBlock objects in reactive state is expensive.
**How to avoid:** Cap the in-memory block array (e.g., 2000 blocks). When the cap is hit, drop oldest blocks and adjust the stored cursor offset. The REST endpoint supports offset-based history loading for scroll-up access to older content.
**Warning signs:** Browser tab memory growing steadily; page becoming sluggish after hours of streaming.

## Code Examples

### Base64 WebSocket Message Decoding
```typescript
// Create a persistent TextDecoder with stream mode for the session
const decoder = new TextDecoder('utf-8', { fatal: false });

function decodeOutputMessage(data: string, isLastChunk: boolean = false): string {
  // data is base64-encoded bytes from ServerMessage::Output
  const binaryString = atob(data);
  const bytes = new Uint8Array(binaryString.length);
  for (let i = 0; i < binaryString.length; i++) {
    bytes[i] = binaryString.charCodeAt(i);
  }
  // stream: true keeps incomplete multi-byte sequences buffered
  return decoder.decode(bytes, { stream: !isLastChunk });
}
```

### WebSocket Connection with Reconnection
```typescript
connect(sessionId: string): void {
  this.disconnect(); // Close existing connection first

  this.activeSessionId = sessionId;
  this.connectionStatus = 'reconnecting';

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const wsUrl = `${protocol}//${window.location.host}/api/v1/sessions/${sessionId}/ws?cursor=${this.cursor}`;

  const ws = new WebSocket(wsUrl);

  ws.onopen = () => {
    this.connectionStatus = 'connected';
    this.retryCount = 0;
  };

  ws.onmessage = (event) => {
    const msg: ServerMessage = JSON.parse(event.data);
    this.handleMessage(msg);
  };

  ws.onclose = () => {
    this.connectionStatus = 'disconnected';
    this.socket = null;
    // Reconnect with exponential backoff if session is still selected
    if (this.activeSessionId === sessionId) {
      this.scheduleReconnect(sessionId);
    }
  };

  ws.onerror = () => {
    // onclose will fire after onerror; handle reconnection there
  };

  this.socket = ws;
}
```

### Vite WebSocket Proxy Configuration
```typescript
// vite.config.ts -- add ws: true to proxy config
export default defineConfig({
  // ...
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:3742',
        changeOrigin: true,
        ws: true  // Enable WebSocket proxy for /api/v1/sessions/{id}/ws
      },
      '/health': {
        target: 'http://localhost:3742',
        changeOrigin: true
      }
    }
  }
});
```

### Output Block Rendering with Left-Border Styling
```svelte
<!-- OutputView.svelte -->
{#each outputBlocks as block (block.id)}
  <div
    class="px-3 py-0.5 font-mono text-sm whitespace-pre-wrap break-words"
    class:border-l-2={block.type !== 'text'}
    class:border-cyan-500={block.type === 'tool_call'}
    class:border-red-500={block.type === 'error'}
    style="color: {block.type === 'error' ? 'var(--color-error, #f87171)' : 'var(--color-text)'};"
  >
    {block.text}
  </div>
{/each}
```

### Phase Status Dot Animation
```css
/* Status dot animations */
@keyframes pulse-working {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.status-dot-working {
  background-color: #4ade80; /* green */
  animation: pulse-working 1.5s ease-in-out infinite;
}

.status-dot-idle {
  background-color: #facc15; /* yellow */
}

.status-dot-ready {
  background-color: #4ade80; /* green */
  /* Checkmark rendered as content or SVG, no pulse */
}

.status-dot-exited {
  background-color: #6b7280; /* gray */
}
```

### UiStore Extension for Detail Panel
```typescript
// Extend existing UiStore in ui.svelte.ts
class UiStore {
  // ... existing fields ...
  selectedTask = $state<Task | null>(null);

  selectTask(task: Task): void {
    this.selectedTask = task;
    // WebSocket connection handled by DetailPanel reactive effect
  }

  closeDetail(): void {
    this.selectedTask = null;
    // WebSocket disconnection handled by DetailPanel reactive effect
  }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Svelte stores (writable/readable) | Svelte 5 $state/$derived runes | Svelte 5 (2024) | Stores work but runes are the recommended pattern; project already uses runes |
| atob() for base64 | Uint8Array.fromBase64() | 2025 (Chrome 120+) | Not yet universal; stick with atob() + Uint8Array manual conversion for now |
| flex-direction: column-reverse for auto-scroll | scroll + JS observer pattern | N/A | column-reverse trick has accessibility issues; JS approach is more reliable |

**Deprecated/outdated:**
- Svelte store `$` prefix auto-subscription: Still works but project uses class-based runes instead
- `window.btoa`/`window.atob` for binary: Works fine for ASCII base64; use TextDecoder for UTF-8

## Open Questions

1. **Task-to-Session Mapping**
   - What we know: Tasks have `session_name` (tmux name, nullable). Daemon sessions have UUID. No direct link exists yet.
   - What's unclear: How will Phase 6 (Workflow Engine) assign daemon session UUIDs to tasks? Will it add a `session_id` column to the Task model?
   - Recommendation: Add `session_id: string | null` to the frontend Task type now. Phase 5 renders the detail panel when session_id is present, shows task-only view otherwise. Phase 6 will populate this field when implementing spawn.

2. **Phase Status for Multi-Session View**
   - What we know: A user can click different task cards to switch the detail panel. Each task may have its own session.
   - What's unclear: Should Phase 5 maintain WebSocket connections to multiple sessions simultaneously (for status dots on all visible cards), or only connect to the selected task's session?
   - Recommendation: Connect WebSocket only for the selected task's session. For status dots on other cards, use the REST session info endpoint polled on a timer (e.g., every 5 seconds) or implement a lightweight status-only endpoint. This avoids the complexity of multiple concurrent WebSocket connections.

3. **Output Block Splitting Strategy**
   - What we know: Output arrives as continuous byte streams. Need to split into classifiable blocks.
   - What's unclear: What constitutes a "block"? Line-by-line? Paragraph-by-paragraph?
   - Recommendation: Split on newlines. Each line is classified independently. Adjacent lines of the same type can be merged visually via CSS (no gap between same-type blocks). This is simple and works well for Phase 5; Phase 7 will implement proper semantic parsing.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Vitest 3.x |
| Config file | `web/vitest.config.ts` |
| Quick run command | `cd web && npx vitest run --reporter=verbose` |
| Full suite command | `cd web && npx vitest run` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OUTPUT-01 | Split-view opens on task click, closes on Escape | unit | `cd web && npx vitest run src/lib/stores/__tests__/ui.test.ts -x` | Exists (extend) |
| OUTPUT-02 | WebSocket store connects, receives output, decodes base64 | unit | `cd web && npx vitest run src/lib/stores/__tests__/websocket.test.ts -x` | Wave 0 |
| OUTPUT-03 | Output classifier distinguishes text/tool_call/error | unit | `cd web && npx vitest run src/lib/stores/__tests__/websocket.test.ts -x` | Wave 0 |
| OUTPUT-04 | Phase status updates from WebSocket state messages | unit | `cd web && npx vitest run src/lib/stores/__tests__/websocket.test.ts -x` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cd web && npx vitest run --reporter=verbose`
- **Per wave merge:** `cd web && npx vitest run`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `web/src/lib/stores/__tests__/websocket.test.ts` -- covers OUTPUT-02, OUTPUT-03, OUTPUT-04 (WebSocket store connect/disconnect, message handling, output classification, phase status)
- [ ] Extend `web/src/lib/stores/__tests__/ui.test.ts` -- covers OUTPUT-01 (selectedTask, selectTask, closeDetail methods)
- [ ] `web/src/lib/api/__tests__/sessions.test.ts` -- covers REST session output fetching (if api/sessions.ts is created)

## Sources

### Primary (HIGH confidence)
- Project codebase: `crates/agtxd/src/api/ws.rs` -- WebSocket protocol (ServerMessage/ClientMessage types, cursor reconnection, broadcast fan-out)
- Project codebase: `crates/agtxd/src/api/sessions.rs` -- REST session endpoints (output with offset/limit, session info, metrics)
- Project codebase: `crates/agtxd/src/session/output.rs` -- Ring buffer (64KB) + append-only log architecture
- Project codebase: `web/src/lib/stores/*.svelte.ts` -- Existing Svelte 5 class-based store pattern
- Project codebase: `web/src/lib/components/TaskCard.svelte` -- Existing `statusDotColor` placeholder and `onclick` prop
- Project codebase: `web/src/lib/components/Board.svelte` -- CSS grid layout with dynamic `gridTemplate`
- Project codebase: `web/vite.config.ts` -- Current proxy configuration (missing `ws: true`)
- [MDN TextDecoder](https://developer.mozilla.org/en-US/docs/Web/API/TextDecoder) -- UTF-8 stream decoding with multi-byte safety

### Secondary (MEDIUM confidence)
- [Vite Server Options](https://vite.dev/config/server-options) -- WebSocket proxy configuration with `ws: true`
- [CSS-Tricks Pin Scrolling](https://css-tricks.com/books/greatest-css-tricks/pin-scrolling-to-bottom/) -- Auto-scroll techniques with user override
- [Svelte 5 Migration Guide](https://svelte.dev/docs/svelte/v5-migration-guide) -- Runes reactivity model confirmation

### Tertiary (LOW confidence)
- None -- all findings verified against project source code and official documentation

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- no new dependencies, all patterns from existing codebase
- Architecture: HIGH -- backend protocol fully implemented; frontend follows established store/component patterns
- Pitfalls: HIGH -- derived from direct code analysis of ws.rs protocol and Svelte 5 reactivity model
- Output classification: MEDIUM -- heuristic patterns are inherently fuzzy; will need tuning per agent

**Research date:** 2026-03-04
**Valid until:** 2026-04-04 (stable; no rapidly changing dependencies)
