# Phase 5: Task Detail & Live Output - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Clicking a task opens a split-view detail panel showing live agent output with visual distinction between output types and phase status. This phase connects the SvelteKit frontend (Phase 4) to the WebSocket streaming backend (Phase 3), delivering the first real-time agent interaction in the browser.

Requirements covered: OUTPUT-01, OUTPUT-02, OUTPUT-03, OUTPUT-04

Note: Structured semantic parsing (OUTPUT-05), action buttons (OUTPUT-06), phase timeline (OUTPUT-07), and artifact display (OUTPUT-08) are Phase 7. This phase delivers raw-but-styled output with basic type distinction.

</domain>

<decisions>
## Implementation Decisions

### Split-view layout
- Slide-in from right: board compresses to ~40% width, detail panel takes ~60%
- Smooth CSS transition for panel open/close
- Board columns shrink but remain usable while panel is open
- Clicking a different task card switches detail content in-place (no close/reopen)
- Close via Escape key + close button in panel header
- Panel header shows: task title, agent badge pill, live phase status indicator, close button

### Output rendering
- Plain monospace text in a scrollable container
- Decode base64 WebSocket bytes to UTF-8 and display as-is
- No terminal emulation (no ANSI cursor/escape sequences) — structured parsing is Phase 7
- Auto-scroll to bottom as new data arrives; pauses when user scrolls up
- Floating "Jump to bottom" button appears when auto-scroll is paused (WS-05)
- On reconnect/open: show recent tail (~64KB ring buffer via WS), lazy-load older history on scroll-up via REST endpoint with offset/limit
- Fixed input bar at bottom of detail panel for sending text to agent PTY stdin via WebSocket write message

### Output type styling
- Left-border color coding per output type: normal text (no border or subtle), tool calls (blue/cyan left border), errors (red left border)
- Simple heuristic regex patterns to classify output blocks (detect "Error:", tool use patterns, agent-specific markers)
- Not collapsible in Phase 5 — collapsible sections are Phase 7 (OUTPUT-05)
- Errors in output stream cause task card status dot to reflect error state for at-a-glance visibility

### Phase status display
- Task cards: colored dot with animation — green pulsing (Working), yellow steady (Idle), green checkmark (Ready), gray dot (Exited)
- Replaces placeholder `statusDotColor` in existing TaskCard component
- Detail panel header: colored status badge next to task title showing current state
- Status sourced via WebSocket `state` messages for tasks with active sessions; static status for Backlog/Done tasks (no unnecessary polling)
- Connection status indicator in NavBar showing WebSocket state: green (connected), orange (reconnecting), red (disconnected) — first phase using WS in frontend

### Claude's Discretion
- Exact CSS transition timing and easing for panel slide-in
- Monospace font choice and line height for output area
- "Jump to bottom" button positioning and styling
- Input bar design (placeholder text, send button vs Enter-only)
- Heuristic patterns for output type classification (agent-specific tuning)
- How to handle very long output lines (wrap vs horizontal scroll)
- Reconnection retry strategy and backoff timing
- Whether to show a loading skeleton while initial history loads

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `TaskCard.svelte`: Already has `onclick` prop and placeholder `statusDotColor` comment ("Phase 5: replace with live PhaseStatus")
- `Board.svelte`: CSS grid layout — detail panel adds a new grid region alongside columns
- `UiStore` (ui.svelte.ts): Pattern for UI state management — extend for selected task and panel open state
- `TaskStore` (tasks.svelte.ts): Svelte 5 class-based store with `$state`/`$derived` — extend for phase status tracking
- `api/client.ts`: `api<T>()` fetch wrapper — reuse for REST history endpoint calls
- `types/index.ts`: `PhaseStatus` type already defined ('working' | 'idle' | 'ready' | 'exited')
- WebSocket backend: `ws.rs` with `ServerMessage` (output/state/metrics/error/connected) and `ClientMessage` (write/resize) types
- REST output endpoint: `GET /sessions/{id}/output` with offset/limit for history loading

### Established Patterns
- Svelte 5 `$state` for DOM refs (`bind:this` requires reactive declaration)
- Modal overlay pattern: fixed inset-0 z-50 with backdrop click-to-close and Escape key
- CSS custom properties for all colors (dark theme, `--color-surface`, `--color-border`, etc.)
- Agent badge colors per agent type in TaskCard (agentColors Record)
- Board uses collapsible CSS grid columns — panel can follow similar grid pattern

### Integration Points
- WebSocket endpoint: `/api/v1/sessions/{id}/ws?cursor={offset}` — connect when detail panel opens for a task with a session
- REST endpoint: `GET /api/v1/sessions/{id}/output?offset=N&limit=M` — lazy-load history on scroll-up
- `+page.svelte`: Add detail panel alongside Board, conditionally rendered
- NavBar: Add connection status indicator
- TaskCard: Replace static statusDotColor with live phase status from store

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

- Collapsible semantic output sections (thinking, tool use, file edits) — Phase 7 (OUTPUT-05)
- Action buttons for agent prompts (approve/reject) — Phase 7 (OUTPUT-06)
- Phase progress timeline with timestamps — Phase 7 (OUTPUT-07)
- Artifact detection display — Phase 7 (OUTPUT-08)
- Full-text search within session output — Phase 7 (WS-07)
- Reconnect summary banner — Phase 7 (WS-06)

</deferred>

---

*Phase: 05-task-detail-live-output*
*Context gathered: 2026-03-04*
