# Architecture

**Analysis Date:** 2026-03-03

## Pattern Overview

**Overall:** Multi-layered TUI application with event-driven architecture, decoupled domain logic through trait-based dependency injection, and persistent task management with git-based workflow orchestration.

**Key Characteristics:**
- State-separated architecture (terminal state independent from application state)
- Trait-based abstractions for all external operations (tmux, git, agents)
- Event loop driven by crossterm keyboard/event handling
- Persistent SQLite storage for task metadata across sessions
- Git worktree isolation for each task's execution context
- Skill deployment system for agent-native discovery

## Layers

**Presentation (TUI):**
- Purpose: Terminal UI rendering, keyboard input handling, state display
- Location: `src/tui/`
- Contains: App struct (event loop), board navigation, popup management, drawing functions
- Depends on: Database (read tasks), Git operations, Tmux operations, Agent operations
- Used by: main.rs entry point
- Notes: Terminal state (`App { terminal, state: AppState }`) is separated from application state for borrow checker compliance

**Application Logic:**
- Purpose: Task lifecycle management, phase transitions, validation
- Location: `src/tui/app.rs` (event handlers, state mutations)
- Contains: Move task, create task, advance phase, resume task logic
- Depends on: Database, Git operations, Tmux operations
- Used by: Presentation layer

**Domain Models:**
- Purpose: Core data structures and enums
- Location: `src/db/models.rs`
- Contains: `Task`, `TaskStatus`, `Project`, `RunningAgent`, `PhaseStatus` enums
- Depends on: chrono for timestamps, uuid for IDs
- Notes: TaskStatus enum defines the 5-column kanban board (Backlog → Planning → Running → Review → Done)

**Persistence:**
- Purpose: SQLite database operations
- Location: `src/db/`
- Contains: Schema initialization, CRUD operations for tasks and projects
- Depends on: rusqlite with bundled SQLite
- Storage:
  - Global: `~/.config/agtx/index.db` (project index)
  - Per-project: `~/.config/agtx/projects/{path_hash}.db` (tasks for specific project)

**External Integration Abstractions:**
- Purpose: Mockable traits for testing and agent/git/tmux operations
- Location: `src/git/`, `src/tmux/`, `src/agent/`, `src/config/`
- Traits: `GitOperations`, `GitProviderOperations`, `TmuxOperations`, `AgentOperations`, `AgentRegistry`
- Real implementations: `RealGitOps`, `RealGitHubOps`, `RealTmuxOps`, `RealAgentRegistry`
- Mock implementations: Behind `#[cfg(feature = "test-mocks")]` gates

**Configuration:**
- Purpose: Global config persistence, per-project overrides, theme customization, workflow plugins
- Location: `src/config/mod.rs`
- Contains: `GlobalConfig` (default agent, theme, worktree settings), `ProjectConfig`, `ThemeConfig`, `WorkflowPlugin`, `MergedConfig` (global + project merged)
- Storage: `~/.config/agtx/config.toml`

**Skill System:**
- Purpose: Deploy agent-native command/skill files to worktrees at phase entry
- Location: `src/skills.rs`
- Contains: Canonical skill content (embedded at compile time), agent-native path mapping, command transformations, plugin loading
- Agents supported: claude, codex, copilot, gemini, opencode
- Notes: Skills are stored in plugin directories and written to agent-native discovery paths (`.claude/commands/`, `.gemini/commands/`, etc.)

## Data Flow

**Task Creation Flow:**

1. User presses `o` in Backlog column → `InputTitle` mode
2. User enters title → `InputDescription` mode
3. User enters prompt, presses Enter → Save to database (status=Backlog)
4. Task appears in Backlog column

**Task Lifecycle (Planning Phase):**

1. User presses `m` on Backlog task → Move to Planning
2. `AppState.advance_task()` called:
   - Create git worktree at `.agtx/worktrees/{task_slug}/`
   - Copy agent config dirs (`.claude/`, `.gemini/`, etc.)
   - Write plugin skills to agent-native paths
   - Run plugin init script (if configured)
   - Spawn tmux session `task-{id}--{project}--{slug}` in worktree
   - Start agent with planning prompt/command
3. Task status → Planning, session_name stored in DB
4. Task appears in Planning column
5. Planning artifact (`.agtx/artifacts/plan.txt`) signals phase completion

**Task Lifecycle (Running Phase):**

1. User presses `m` on Planning task → Move to Running
2. `AppState.advance_task()` called:
   - Send `/execute` command (or equivalent per-agent) to tmux session
   - Task status → Running
3. Agent continues in same tmux window/session
4. Running artifact signals phase completion

**Task Lifecycle (Review Phase):**

1. User presses `m` on Running task → Move to Review
2. Task status → Review
3. Optionally create PR:
   - Fetch PR description in background thread
   - Show PR confirmation popup
   - Create PR via GitHub API (if PR number absent)
   - Store `pr_number` and `pr_url` in DB
   - Tmux session stays open for manual changes/rebasing

**Task Lifecycle (Done):**

1. User presses `m` on Review task → Move to Done
2. If task has cyclic plugin:
   - Offer to move back to Planning (increment cycle counter)
3. If task doesn't have PR or PR is merged:
   - Clean up worktree
   - Clean up tmux session
   - Keep git branch locally

**State Management:**

- `AppState` holds all task state and UI state (board selection, popups, search dropdowns)
- Database is single source of truth for task metadata
- Tmux sessions are living processes tracked via session names
- Git worktrees are persistent filesystems in `.agtx/worktrees/`
- Phase status (Working/Idle/Ready/Exited) is runtime-only, computed from artifact files and tmux window state

## Key Abstractions

**TaskStatus:**
- Purpose: 5-column kanban board enum with string conversion
- Examples: Backlog, Planning, Running, Review, Done
- Pattern: Static method `columns()` returns all statuses in board order

**Task:**
- Purpose: Core domain model for a unit of work
- Key fields: id, title, description, status, agent, plugin, session_name, worktree_path, branch_name, pr_number, cycle
- Key methods: `new()`, `generate_session_name()`
- Notes: `cycle` field enables cyclic workflows (Review → Planning with incrementing counter)

**WorkflowPlugin:**
- Purpose: Customize task lifecycle per phase
- Fields: commands, prompts, artifacts, prompt_triggers, init_script, copy_dirs, copy_files, copy_back, cyclic, supported_agents
- Resolution order: project-local `.agtx/plugins/{name}/` → global `~/.config/agtx/plugins/{name}/` → bundled
- Per-task persistence: Plugin name stored in DB at task creation time

**BoardState:**
- Purpose: Stateless navigation model for 5-column kanban board
- Contains: `tasks: Vec<Task>`, `selected_column: usize`, `selected_row: usize`
- Key methods: `tasks_in_column()`, `selected_task()`, `move_left()`, `move_right()`, `move_up()`, `move_down()`, `clamp_row()`
- Pattern: All draw functions take `&BoardState` as immutable reference

**InputMode:**
- Purpose: State machine for task entry UI
- States: Normal, InputTitle, InputDescription
- Usage: `AppState.input_mode` switches mode on each state change

**PhaseStatus:**
- Purpose: Runtime-only detection of phase completion
- States: Working (spinner), Idle (no output 15s), Ready (artifact found), Exited (no tmux window)
- Detection: Refresh every 100ms, cache results with 2-second TTL
- Artifact paths: Come from task's plugin or agtx defaults

**Trait Abstractions:**
- `GitOperations`: create_worktree, initialize_worktree, git commands
- `GitProviderOperations`: create_pull_request, get_pull_request
- `TmuxOperations`: spawn_session, send_keys, get_window_content, list_windows, kill_window
- `AgentOperations`: build_interactive_command, available agents
- `AgentRegistry`: detect_available_agents, get_agent

## Entry Points

**Main Entry Point:**
- Location: `src/main.rs`
- Triggers: `cargo run` or `./target/release/agtx`
- Responsibilities:
  - Parse CLI arguments (`-g` for dashboard, path for project)
  - Detect AppMode (Dashboard or Project)
  - Handle first-run configuration (prompt for default agent)
  - Initialize and run App event loop

**App Event Loop:**
- Location: `src/tui/app.rs`
- Method: `App::run()`
- Responsibilities:
  - Poll crossterm events every ~16ms (60 FPS)
  - Route keyboard events to appropriate handlers
  - Refresh phase status every 100ms
  - Render board and popups via ratatui
  - Handle background channel results (PR generation, PR creation)
  - Gracefully cleanup terminal state on quit

**Database Entry Points:**
- Project database: `Database::open_project(&project_path)`
- Global database: `Database::open_global()`
- Auto-initialize schema and run migrations on open

## Error Handling

**Strategy:** Use `anyhow::Result<T>` with `.context()` for error propagation. Gracefully handle missing resources (tmux sessions, worktrees, git branches).

**Patterns:**
- Git operations that fail on missing branches: Return empty results, not errors
- Tmux operations on missing sessions: Check session existence before operations
- Database schema migrations: Use `ALTER TABLE ... ADD COLUMN` with silent error ignore (column may exist)
- File operations: Wrap with context messages showing paths

## Cross-Cutting Concerns

**Logging:** None currently (no logger dependency). Errors bubble up to TUI via Result types.

**Validation:**
- Task title: Non-empty string required
- Agent: Must be in available agents list
- Plugin: Validated at load time, falls back to defaults
- Phase transitions: Validated by AppState before advancing

**Authentication:** GitHub PR operations use `gh` CLI (must be logged in). No explicit auth handling in app.

**Concurrency:**
- PR description generation runs in std::thread spawned from event loop
- Results communicated back via mpsc channel `pr_generation_rx`
- PR creation also in background thread
- Setup operations (worktree init) may run in background
- All state mutations still happen on main event loop thread (single-threaded TUI)

**Theming:** All colors stored in `ThemeConfig`, accessed via `hex_to_color()` helper, applied during rendering.
