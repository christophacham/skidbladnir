# Feature Landscape

**Domain:** Web-native coding agent management dashboard (browser-based replacement for terminal kanban + tmux)
**Researched:** 2026-03-03
**Overall confidence:** MEDIUM-HIGH

## Table Stakes

Features users expect from a web-based agent management dashboard. Missing = product feels broken or incomplete compared to the existing TUI or competitor tools like VS Code Agent HQ and Conductor.

### Core Kanban Board

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Column-based kanban layout (Backlog/Planning/Running/Review/Done) | Users already have this in the TUI; removing it is regression | Medium | Must preserve exact 5-column workflow semantics. Start with 3-5 columns as kanban best practice confirms. |
| Task cards with title, agent badge, status indicator | Minimum viable card. Users scan boards visually. | Low | Include color-coded agent badge, phase status icon (Working/Idle/Ready/Exited), and truncated description. |
| Click-to-open task detail panel | Split-view pattern is standard for dashboards (Gmail, Linear, Cloudscape). Clicking a card should show full details without navigating away. | Medium | Use split-view layout: board on left, detail panel on right. Avoids full page navigation which breaks flow. |
| Task creation with title + description | Existing TUI feature. Table stakes for any task board. | Low | Inline creation at column top, or modal. Support multi-line descriptions with file/skill references. |
| Task deletion with confirmation | Existing TUI feature. Destructive action needs guard. | Low | Confirmation dialog. |
| Move task forward/backward through workflow | Core workflow mechanic. The entire AGTX value prop depends on phase transitions triggering real actions (worktree creation, agent spawn, PR creation). | High | Each transition has side effects (worktree setup, agent launch, skill deployment). Must show progress/loading state during transitions. |
| Plugin selection per task | Existing feature. Different workflows need different plugins. | Low | Dropdown or selector at task creation time. Show active plugin on card. |

### Live Agent Output Streaming

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Real-time agent output visible in browser | This is THE core feature. Without it, users must SSH + tmux attach, defeating the purpose of a web UI. | High | WebSocket streaming from PTY to browser. Must handle high-throughput output (agent tool calls, file writes). |
| Auto-scroll to bottom with scroll-lock on manual scroll up | Standard pattern in every log viewer and terminal. Users expect output to follow the latest line unless they scroll up to read history. | Medium | Detect manual scroll-up, pause auto-scroll, show "Jump to bottom" button. Resume on click or new user input. |
| Visual distinction between agent output, tool calls, and errors | Structured output is a key differentiator over raw terminal, but basic visual distinction is table stakes for readability. | Medium | At minimum: different background colors for agent text vs tool output vs errors. Line-level parsing. |
| Loading/spinner indicator when agent is working | Users need to know if the agent is actively processing or idle. The TUI already has Working/Idle/Ready/Exited states. | Low | Animated spinner in task card and detail panel header. Map to existing PhaseStatus enum. |

### Session Persistence and Reconnection

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Full session history persisted to disk | Users close browser tabs. Agents run for hours. History must survive disconnects. PROJECT.md explicitly requires this. | High | Write PTY output to disk as it arrives. Lazy-load on reconnect. This is non-negotiable for a remote tool. |
| Reconnect and see full history on page load | Without this, refreshing the page loses context. Every CI/CD dashboard, log viewer, and chat app handles this. | High | On WebSocket connect, server sends current state snapshot. Client then subscribes to live stream. Use stale-while-revalidate pattern: show cached state immediately, revalidate in background. |
| Virtualized infinite scrollback | Agent sessions produce thousands of lines. Rendering all DOM nodes kills performance. | Medium | Virtual scrolling (only render visible viewport + buffer). Combined with infinite scroll for lazy-loading older history from disk. TanStack Virtual or similar. |
| Connection status indicator | Users need to know if they are connected, disconnected, or reconnecting. | Low | Small indicator (green dot / yellow dot / red dot) in header or status bar. Standard WebSocket health pattern. |

### Authentication and Access Control

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| GitHub OAuth login gate | Tool is deployed on a remote server. Unauthenticated access to agent sessions is a security hole. PROJECT.md specifies oauth2-proxy. | Low (config) | Not building custom auth. Using oauth2-proxy with GitHub provider + single-username allowlist. Configuration, not code. |
| Single-user access restriction | This is a personal tool, not SaaS. Must reject all users except the configured owner. | Low (config) | oauth2-proxy `--github-user` flag. |

### Navigation and Usability

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Keyboard shortcuts (j/k navigation, o to create, Enter to open) | Existing TUI users have muscle memory. Developer tools without keyboard shortcuts feel sluggish. | Medium | Mirror existing TUI shortcuts where possible. Use vim-style j/k for list navigation. Ctrl+K or `/` for search. Single-char shortcuts only when no input is focused. |
| Task search / filter | Existing TUI feature (`/` to search). Finding tasks across columns is essential with many tasks. | Medium | Fuzzy search across title and description. Highlight matching tasks or filter board. |
| Multi-project sidebar | Existing TUI feature. Users manage multiple repos. | Medium | Collapsible sidebar listing projects. Click to switch. Show task counts per project. |
| Responsive layout (desktop-first) | Web apps must work at reasonable desktop viewport sizes. Not mobile, but must handle 1280px-2560px+ widths. | Low | CSS grid/flexbox. Columns should reflow or scroll horizontally. Not a mobile app. |

### System Visibility

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Host metrics overview (CPU, RAM, disk) | Agents consume significant resources. Users running 3-5 agents need to see if the server is overloaded. PROJECT.md requires this. | Medium | sysinfo crate on backend, push metrics via WebSocket every 2-5 seconds. Simple gauges or spark lines. Inspired by Beszel/Glances lightweight approach. |
| Service health status | Users need to know if agtxd, the web server, and the proxy are running. | Low | Simple status dots in system tab. Pull from systemd or process checks. |


## Differentiators

Features that set AGTX Web apart from alternatives (Conductor, VS Code Agent HQ, raw tmux). Not expected, but create competitive advantage.

### Structured Agent Output (Primary Differentiator)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Parsed status sections (thinking, tool use, file edits, errors) | VS Code shows diffs and collapses tool output. Conductor shows raw output. AGTX Web can do better: parse agent output into collapsible semantic sections. | High | Agent-specific parsers that recognize patterns (Claude's thinking blocks, tool call boundaries, file edit markers). Falls back to plain text for unrecognized output. |
| Action buttons for known interactions (approve, reject, provide input) | Instead of typing "yes" into a terminal, click a button. VS Code 1.109 does this with "incremental approvals." AGTX Web should match or exceed. | High | Detect prompt patterns (permission requests, Y/N questions) and render contextual buttons. Must include free-text fallback input for arbitrary responses. |
| File change summary with inline diffs | Show which files the agent has modified, added, or deleted during the current phase. VS Code's background agents show diff statistics. | High | Watch worktree for file changes (inotify/polling), generate diffs against branch base. Display as expandable file list with unified diff view. |
| Phase progress timeline | Visual timeline showing phase transitions with timestamps, artifact detection, and cycle count. No existing tool does this. | Medium | Horizontal or vertical timeline component showing Backlog > Planning > Running > Review progression with timestamps and duration. |

### Session Intelligence

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Reconnect summary banner | When reconnecting after being away, show a brief "since you left" summary: what phase the agent is in, files changed, current status. No competitor does this well. | Medium | On reconnect, server generates a state snapshot: current phase, last N actions, files changed, time elapsed. Render as dismissible banner at top of output view. |
| Output search within session | Find specific text in agent output history. Critical for long sessions where the agent mentioned something 500 lines ago. | Medium | Full-text search across persisted session history. Highlight matches with jump-to navigation. |
| Session timeline markers | Bookmark important moments in output (phase transitions, errors, user inputs). Makes reviewing long sessions manageable. | Medium | Auto-insert markers at phase transitions and user inputs. Allow manual bookmarks. Display as clickable timeline scrubber alongside output. |

### Workflow Intelligence

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Artifact detection progress indicator | Show which artifacts the plugin expects and which have been detected. Current TUI shows a checkmark but no granularity. | Low | List expected artifact paths from plugin config, check mark each as detected. Show in task detail panel. |
| PR creation integrated in-browser | Existing TUI feature via `gh`. Web version should render a proper PR form (title, description, base branch) without shelling out to a terminal. | Medium | REST API call to GitHub. Pre-populate from task title/description. Show PR status after creation. |
| Git diff viewer for task worktrees | Existing TUI feature (`d` key). Web version should show a proper syntax-highlighted diff viewer. | Medium | Use a diff rendering library. Show changed files with expandable unified diffs. Color-coded additions/deletions. |

### Developer Experience

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Command palette (Ctrl+K) | Power user navigation pattern used by VS Code, GitHub, Linear, Notion. Makes every action accessible without mouse. | Medium | Fuzzy search across: tasks, projects, actions (create task, switch project, open settings). Single keypress to invoke. |
| Dark theme (default, with light option) | Developer tools are expected to be dark-themed. AGTX TUI already has a dark theme with configurable colors. | Low | Use CSS custom properties. Port existing theme colors as defaults. Dark-first design. |
| Toast notifications for background events | When an agent in another task finishes, or a PR is created, show a non-intrusive notification. The TUI cannot do this (single-focus). | Low | Toast component (bottom-right). Auto-dismiss after 4-5 seconds. Actionable toasts for events needing attention (agent finished, error occurred). |
| Keyboard shortcut cheat sheet (`?` key) | Discoverability for keyboard shortcuts. GitHub and many dev tools use `?` to show a shortcut overlay. | Low | Modal overlay listing all shortcuts grouped by context (board, task detail, output viewer). |

### System Tab (Operational Visibility)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Service log streaming (journalctl) | See agtxd/web/proxy logs without SSH. Essential for debugging deployment issues. | Medium | Stream journalctl output via WebSocket. Filter by service unit. Same virtualized scroll as agent output. |
| Per-agent resource usage | Show CPU/memory per agent process, not just system totals. Helps identify which agent is consuming resources. | Medium | Track PID per agent process, read /proc stats. Display per-task resource usage in task card or detail panel. |


## Anti-Features

Features to explicitly NOT build. These add complexity without proportional value, or actively harm the product.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Raw terminal emulator (xterm.js) | PROJECT.md explicitly rules this out. Raw terminal rendering in browser is complex (ANSI escape sequences, cursor positioning, resize handling) and the whole point of the web version is structured output that is BETTER than a terminal. | Build structured output viewer with semantic sections, action buttons, and searchable history. |
| Multi-user / team collaboration | Single-user tool by design. Adding user management, permissions, shared state, and real-time collaboration is a massive complexity multiplier with zero value for the target user. | oauth2-proxy with single-user allowlist. |
| Mobile-responsive layout | Agents produce wide output. Kanban boards need horizontal space. Optimizing for 320px screens adds design constraints that hurt the primary desktop experience. | Desktop-first (1280px+). Tolerable on tablet. Not a goal for phone. |
| Drag-and-drop task reordering | Tempting kanban pattern, but AGTX tasks have a strict linear workflow (Backlog > Planning > Running > Review > Done) with side effects at each transition. Drag-and-drop implies arbitrary movement which conflicts with the workflow engine. | Click-to-advance buttons with clear action labels ("Start Planning", "Begin Execution", "Move to Review"). |
| Real-time collaborative editing of task descriptions | Single-user tool. No concurrent editors. CRDT/OT complexity is unjustified. | Simple text input with save button. |
| Custom dashboard widgets / drag-and-drop layout builder | Over-engineering. The dashboard layout is fixed and purposeful: kanban board + task detail + system metrics. Customizable layouts add complexity without clear user value for a single-user tool. | Fixed, well-designed layout with collapsible panels. |
| Plugin marketplace / visual plugin editor | Plugins are TOML files. The existing file-based plugin system is simple and powerful. A visual editor adds UI complexity for a feature that changes rarely. | Document plugin TOML format well. Allow editing plugin.toml in task worktrees. |
| Email / push notifications | Single-user, single-browser. Toast notifications in-app are sufficient. Building push notification infrastructure (service workers, VAPID keys) is disproportionate effort. | In-app toast notifications for events. Browser tab title updates (e.g., "* AGTX - Task Ready"). |
| Chat interface with agent | Agents are autonomous workers, not chat partners. The interaction model is: give task + prompt, agent works, user reviews. Adding a chat UI implies conversational back-and-forth which doesn't match the AGTX workflow. | Free-text input box for sending commands/responses to agent PTY when needed (permission prompts, corrections). This is structured interaction, not chat. |
| Undo/redo for task state changes | Phase transitions have real-world side effects (worktree creation, agent processes, PRs). "Undo" would require reversing git operations and killing processes. Too dangerous and complex. | Confirmation dialogs before destructive actions. "Resume" from Review back to Running (existing feature). |
| Embedded code editor | Users have VS Code, Neovim, etc. Building a code editor in the dashboard duplicates existing tools. | Link to open files in user's preferred editor. Show diffs read-only. |


## Feature Dependencies

```
Authentication (oauth2-proxy)
  --> Everything (gate all access)

WebSocket infrastructure
  --> Live agent output streaming
  --> Session reconnection
  --> System metrics streaming
  --> Service log streaming
  --> Toast notifications

PTY process management (portable-pty)
  --> Agent spawning
  --> Live output capture
  --> Session history persistence
  --> Structured output parsing
  --> Action buttons (send input to PTY)

Session history persistence (disk)
  --> Reconnect with full history
  --> Output search within session
  --> Session timeline markers
  --> Reconnect summary banner

Kanban board (basic)
  --> Task creation/deletion
  --> Task detail panel (split view)
  --> Phase transitions
  --> Plugin selection

Phase transition engine
  --> Artifact detection progress
  --> Phase progress timeline
  --> Cyclic phase support

Structured output parser
  --> Semantic sections (collapsible)
  --> Action buttons (approve/reject)
  --> File change detection

Git integration
  --> Worktree management
  --> Diff viewer
  --> PR creation form

System metrics collection (sysinfo)
  --> Host metrics dashboard
  --> Per-agent resource usage
```

## MVP Recommendation

### Phase 1: Foundation (must work before anything else)
1. **Authentication gate** -- oauth2-proxy + Caddy. Without this, the tool is insecure.
2. **WebSocket infrastructure** -- bidirectional streaming. The backbone for everything real-time.
3. **PTY process management** -- replace tmux. Without this, no agent output.
4. **Session history persistence** -- write to disk. Without this, reconnection is useless.

### Phase 2: Core Board
5. **Kanban board with task cards** -- the primary UI. Column layout, status indicators, task creation.
6. **Task detail split panel** -- click task to see details + output.
7. **Live agent output streaming** -- the core feature. Structured text view with auto-scroll.
8. **Phase transitions** -- move tasks through workflow with real side effects.

### Phase 3: Intelligence Layer
9. **Structured output parsing** -- agent-specific parsers for semantic sections.
10. **Action buttons** -- detect permission prompts, render approve/reject buttons.
11. **Reconnect summary banner** -- orientation after disconnect.
12. **Artifact detection progress** -- show expected vs detected artifacts.

### Phase 4: Polish and Operations
13. **Command palette** -- power user navigation.
14. **Keyboard shortcuts** -- vim-style navigation matching TUI muscle memory.
15. **System metrics tab** -- CPU/RAM/disk gauges.
16. **Service log streaming** -- journalctl in browser.
17. **PR creation form** -- integrated GitHub PR workflow.
18. **Git diff viewer** -- syntax-highlighted diffs.
19. **Toast notifications** -- background event alerts.

**Defer:**
- Per-agent resource tracking: nice-to-have, can be added to system tab later.
- Session timeline markers: useful but not critical for launch.
- Output search within session: browser Ctrl+F covers basic case initially.

## Sources

- [Smashing Magazine: UX Strategies for Real-Time Dashboards](https://www.smashingmagazine.com/2025/09/ux-strategies-real-time-dashboards/) -- MEDIUM confidence, general UX patterns
- [UX Patterns for Developers: Kanban Board](https://uxpatterns.dev/patterns/data-display/kanban-board) -- MEDIUM confidence, kanban UI patterns
- [Ably: WebSocket Architecture Best Practices](https://ably.com/topic/websocket-architecture-best-practices) -- MEDIUM confidence, reconnection patterns
- [RingCentral: WebSocket Session Recovery](https://developers.ringcentral.com/guide/notifications/websockets/session-recovery) -- HIGH confidence, official docs on session recovery
- [web.dev: Infinite Scroll Pattern](https://web.dev/patterns/web-vitals-patterns/infinite-scroll/infinite-scroll/) -- HIGH confidence, Google official guidance
- [Cloudscape: Split View Pattern](https://cloudscape.design/patterns/resource-management/view/split-view/) -- HIGH confidence, AWS design system documentation
- [VS Code: Background Agents](https://code.visualstudio.com/docs/copilot/agents/background-agents) -- HIGH confidence, official VS Code docs on agent management
- [Conductor Docs](https://docs.conductor.build) -- MEDIUM confidence, competitor feature set
- [InfoQ: Stale-While-Revalidate UX Pattern](https://www.infoq.com/news/2020/11/ux-stale-while-revalidate/) -- MEDIUM confidence, reconnection UX
- [SWR: Automatic Revalidation](https://swr.vercel.app/docs/revalidation) -- HIGH confidence, official library docs
- [LogRocket: Toast Notification Best Practices](https://blog.logrocket.com/ux-design/toast-notifications/) -- MEDIUM confidence, UX guidance
- [Rob Dodson: Command Palettes for the Web](https://robdodson.me/posts/command-palettes/) -- MEDIUM confidence, implementation patterns
- [Golsteyn: Keyboard Shortcuts on the Web](https://golsteyn.com/writing/designing-keyboard-shortcuts/) -- MEDIUM confidence, shortcut design patterns
- [Google Developers: A2UI Agent-Driven Interfaces](https://developers.googleblog.com/introducing-a2ui-an-open-project-for-agent-driven-interfaces/) -- MEDIUM confidence, emerging standard for agent UIs
- [VS Code 1.109 Agent Infrastructure](https://visualstudiomagazine.com/articles/2025/12/12/vs-code-1-107-november-2025-update-expands-multi-agent-orchestration-model-management.aspx) -- MEDIUM confidence, competitor direction
