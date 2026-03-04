# Phase 6: Workflow Engine - Context

**Gathered:** 2026-03-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Full AGTX workflow semantics through the web interface. Phase transitions trigger side effects (worktree creation, agent spawn, skill deployment), plugins resolve via project-local/global/bundled precedence, commands and prompts translate per agent type, artifact files are detected via polling, cyclic phases work (Review to Planning with incrementing phase counter), and users can create GitHub PRs and view syntax-highlighted diffs from the browser.

Requirements covered: FLOW-01, FLOW-02, FLOW-03, FLOW-04, FLOW-05, FLOW-06, FLOW-07, FLOW-08

</domain>

<decisions>
## Implementation Decisions

### Task advancement UX
- Advance buttons in both task cards (forward-arrow icon) and detail panel header (prominent button) — multiple entry points like TUI's 'm' key
- Backlog to Planning: card moves to Planning column immediately with a "setting up..." spinner overlay while worktree creation, skill deployment, and agent spawn run in the background. Feels fast even though setup takes seconds
- Running to Review: task moves to Review immediately — no forced PR creation prompt. PR creation available as an optional action in the detail panel when ready
- Review to Done: check PR status (merged/open/closed) via `gh pr view` and warn if PR is still open. User can still proceed — warning, not blocker. Matches TUI behavior
- Review to Planning (cyclic): available when plugin.cyclic is true, increments phase counter

### PR creation flow
- AI-first with edit: PR modal opens with AI-generated title and body already populated from the diff (generated in background during modal open). User reviews and edits before creating. If AI generation fails, fields are empty for manual entry
- PR action available via both the detail panel toolbar (visible when task is in Review) and the command palette (Ctrl+K "Create PR for [task]")
- Base branch dropdown in modal for selecting target branch
- After PR is created, detail panel header shows clickable PR URL link + status badge (Open/Merged/Closed) that updates on refresh

### Diff viewing
- Unified inline diff (GitHub-style) with + (green) and - (red) line coloring
- Full language-aware syntax highlighting in diff hunks (recognize Rust, TypeScript, Python, etc.)
- Displayed in the detail panel as a tab — panel becomes tabbed view: Output | Diff | PR
- Diff tab shows diff relative to the base/main branch for the task's worktree

### Plugin selection
- Project-level default plugin settable via project settings or command palette — all new tasks inherit this plugin
- Per-task plugin override available in the task creation modal — dropdown with plugin name + short description
- Read-only plugin listing — no web-based plugin editing or management. Plugin files managed on disk via TOML
- Available plugins discovered from bundled + global + project-local directories and listed with name + description

### Claude's Discretion
- Daemon-side workflow service architecture (new endpoints for transition orchestration, how side effects are executed)
- Artifact detection mechanism (daemon-side file polling, WebSocket push, or REST polling from frontend)
- How agent readiness detection works in the daemon PTY model (replacing TUI's tmux pane content polling)
- How `send_skill_and_prompt()` sequencing works via daemon PTY write (replacing tmux send-keys + prompt trigger polling)
- Syntax highlighting library choice for diffs
- Tabbed detail panel implementation details (component structure, routing between tabs)
- How `session_id` bridges daemon PTY sessions to task records (DB migration, linking strategy)
- Background setup result delivery (WebSocket state messages, REST polling, or server-sent events)
- PR description AI generation approach (which agent's `generate_text()` to use, fallback behavior)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `move_task_right()` in `src/tui/app.rs` (line 3211): Complete reference implementation of all phase transitions with side effects — port logic to daemon-side workflow service
- `setup_task_worktree()` in `src/tui/app.rs`: Background thread that creates worktree, copies files, deploys skills, spawns agent — decompose into daemon async steps
- `write_skills_to_worktree()` in `src/tui/app.rs` (line 5895): Skill deployment to canonical + agent-native paths — move to agtx-core for reuse
- `resolve_skill_command()` / `resolve_prompt()` in `src/tui/app.rs`: Command translation per agent + prompt template substitution — move to agtx-core
- `WorkflowPlugin::load()` in `crates/agtx-core/src/config/mod.rs` (line 518): Plugin resolution chain already in agtx-core, reusable by daemon
- `RealGitHubOps` in `crates/agtx-core/src/git/provider.rs`: PR creation and status checking via `gh` CLI — reusable by daemon
- `Agent.build_interactive_command()` in `crates/agtx-core/src/agent/mod.rs`: Per-agent command construction — reusable by daemon
- `SessionManager` in `crates/agtxd/src/session/manager.rs`: PTY session spawn/manage — extend with workflow-aware spawning
- `TaskStore` in `web/src/lib/stores/tasks.svelte.ts`: Svelte 5 store with `byStatus` derivation — extend with `advance()`, `createPr()` methods
- `DetailPanel.svelte`: Split-view panel with output — extend with tabs for Diff and PR views
- `CommandPalette` in `web/src/lib/stores/commands.svelte.ts`: Action registry — add workflow actions (advance, create PR, view diff)

### Established Patterns
- Daemon REST API: `/api/v1/...` prefix, direct JSON responses, axum extractors
- Background thread results via `mpsc` channel (TUI) — daemon equivalent: async tasks with WebSocket state updates
- Plugin instances cached per task in `HashMap` to avoid repeated disk reads
- Svelte 5 `$state`/`$derived` runes for reactive store state
- Modal overlay: fixed inset-0 z-50 with backdrop click-to-close and Escape key
- Agent badge colors per agent type in `TaskCard` (agentColors Record)

### Integration Points
- New daemon endpoints needed for workflow transitions: `POST /api/v1/tasks/{id}/advance`, `POST /api/v1/tasks/{id}/pr`, `GET /api/v1/tasks/{id}/diff`, `GET /api/v1/plugins`
- `AppState` in `crates/agtxd/src/state.rs`: extend with git ops, agent registry, plugin loading, project path
- `Task` model in `crates/agtx-core/src/db/models.rs`: add `session_id` column (daemon PTY UUID)
- `Task` type in `web/src/lib/types/index.ts`: already has `session_id: string | null`
- WebSocket `state` messages: extend to carry artifact detection results and setup progress
- Detail panel: add tab navigation component (Output | Diff | PR)

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 06-workflow-engine*
*Context gathered: 2026-03-04*
