# Phase 4: Frontend Kanban Board - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

SvelteKit 5 SPA with 5-column kanban layout (Backlog/Planning/Running/Review/Done), task CRUD via modal dialog, project sidebar, command palette, and text search with live filtering. This phase delivers the frontend board — live agent output streaming is Phase 5, workflow engine side effects are Phase 6, structured output parsing is Phase 7.

Requirements covered: BOARD-01, BOARD-02, BOARD-03, BOARD-04, BOARD-05, BOARD-06, BOARD-07

</domain>

<decisions>
## Implementation Decisions

### Board layout
- 5-column layout with equal-width columns by default
- Columns are collapsible — user can collapse Done/Backlog to icons to give more room to active work
- Collapsed state persisted in localStorage

### Task card density
- Minimal cards: title + agent badge + phase status colored dot
- Description and details shown in the detail panel (Phase 5), not on cards
- Agent badge: small colored pill with agent name (e.g., "claude", "codex")
- Phase status: colored dot — green pulsing (Working), yellow (Idle), checkmark (Ready), gray (Exited)

### Empty column states
- Claude's Discretion — pick whatever looks clean in context

### Task creation
- Modal dialog triggered from multiple entry points: 'o' keyboard shortcut, '+' button in Backlog column header, and command palette
- Modal fields: title (required), agent dropdown (required, pre-filled with project default), description textarea (optional)
- All three fields shown upfront

### Task deletion
- Confirmation dialog: press 'x' or click delete → modal asks "Delete 'task-name'?" with Cancel/Delete buttons
- Matches TUI behavior, prevents accidents

### Project sidebar
- Hidden by default, toggled with 'e' key (matches TUI shortcut)
- ~200px on the left when open, board takes remaining space
- Flat list of projects with task count per project
- Selected project highlighted

### Command palette
- Ctrl+K opens fuzzy search over actions: create task, switch project, search tasks, toggle sidebar, etc.
- Central action hub — like VS Code's Ctrl+Shift+P but for AGTX actions
- Also accessible as a navigation tool for projects and tasks

### Top navigation bar
- Project name on the left
- Search bar (always visible) in the center
- '+' create button on the right
- Subtle shortcuts hint area

### Search & filtering
- Always-visible search bar in the top navigation
- Press '/' to focus the search bar, Escape to clear
- Live filtering as you type — searches across title + description
- Non-matching cards dimmed/semi-transparent (not hidden) to maintain spatial context
- Text search only for Phase 4 — agent/status filter chips deferred to UX Polish (Phase 10)
- Client-side filtering — all tasks fetched on load, filtered in browser (sufficient for single-user with < 1000 tasks/project)

### Claude's Discretion
- SvelteKit project structure and component organization
- CSS approach (Tailwind vs scoped CSS vs CSS modules)
- State management pattern (stores, context, etc.)
- HTTP client for REST API calls
- Animation/transition details for collapsible columns
- Exact color values for dark theme (must use CSS custom properties per PROJECT.md)
- Responsive breakpoint handling within 1280px-2560px+ range

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- REST API endpoints already exist: GET/POST /api/v1/tasks, GET/PUT/DELETE /api/v1/tasks/{id}, GET /api/v1/projects, GET /api/v1/projects/{id}
- Task model with serde serialization: title, description, agent, status, project_id, phase, slug, worktree_path, created_at, updated_at
- TaskStatus enum: Backlog, Planning, Running, Review, Done — maps directly to kanban columns
- Project model: id, name, path, created_at
- Health endpoint at /health for connectivity checks
- WebSocket endpoint at /api/v1/sessions/{id}/ws for future Phase 5 integration

### Established Patterns
- API convention: /api/v1/... prefix, direct JSON responses (no envelope)
- UUID v4 strings for entity IDs
- RFC3339 datetime strings
- Dark theme default with CSS custom properties (ThemeConfig has color_selected, color_normal, color_dimmed, color_text, color_accent, etc.)

### Integration Points
- Frontend will be a separate SvelteKit project (likely in `web/` directory)
- Connects to agtxd daemon via REST API on configured port (default 3742)
- Future WebSocket connection for live output (Phase 5)
- Must handle daemon-down state gracefully (show connection error, retry)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

- Agent/status filter chips alongside search — Phase 10 (UX Polish)
- Drag-and-drop task reordering — explicitly out of scope per REQUIREMENTS.md
- Light theme option — v2 (THEME-01)

</deferred>

---

*Phase: 04-frontend-kanban-board*
*Context gathered: 2026-03-04*
