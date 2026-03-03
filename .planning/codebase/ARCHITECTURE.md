# Architecture

**Analysis Date:** 2026-03-03

## Pattern Overview

**Overall:** Monolithic TUI application with module-based layering and trait-based dependency injection for external systems (tmux, git, agents).

**Key Characteristics:**
- Single-binary Rust TUI application using Ratatui + Crossterm
- State machine architecture: `AppMode` (Dashboard/Project) and `InputMode` (Normal/InputTitle/InputDescription) govern behavior
- External system interactions (tmux, git, agent CLIs) abstracted behind traits for testability via `mockall`
- Background work offloaded to `std::thread::spawn` with `mpsc` channels for result communication
- Plugin system customizes per-phase behavior through TOML configuration + embedded skill files
- Centralized SQLite storage (global index + per-project task databases)

## Layers

**Entry Point / CLI:**
- Purpose: Parse CLI arguments, handle first-run setup, launch TUI
- Location: `src/main.rs`
- Contains: Arg parsing (`AppMode` resolution), config migration, agent selection prompt
- Depends on: `config`, `agent`, `git`, `tui`
- Used by: Binary execution

**TUI Layer:**
- Purpose: Terminal UI rendering, event loop, user input handling, workflow orchestration
- Location: `src/tui/`
- Contains: `App` struct (owns terminal + state), `AppState` (all runtime state), drawing functions, key handlers, task lifecycle orchestration
- Depends on: `db`, `config`, `git`, `tmux`, `agent`, `skills`
- Used by: `main.rs`
- Key files:
  - `src/tui/app.rs` (5315 lines) - Main application logic, event loop, rendering, key handling, task workflow
  - `src/tui/board.rs` - `BoardState` kanban column/row navigation
  - `src/tui/input.rs` - `InputMode` enum (Normal/InputTitle/InputDescription)
  - `src/tui/shell_popup.rs` - Shell popup state, ANSI content trimming, rendering helpers
  - `src/tui/app_tests.rs` - Unit tests for app.rs (2792 lines, included via `#[path]`)

**Database Layer:**
- Purpose: SQLite persistence for tasks and projects
- Location: `src/db/`
- Contains: Schema creation, CRUD operations, data models
- Depends on: `rusqlite`, `chrono`, `uuid`
- Used by: `tui` (via `AppState.db` and `AppState.global_db`)
- Key files:
  - `src/db/schema.rs` - `Database` struct, SQL operations, migrations
  - `src/db/models.rs` - `Task`, `Project`, `TaskStatus`, `PhaseStatus`, `RunningAgent` structs

**Git Layer:**
- Purpose: Git worktree management, diff operations, branch management
- Location: `src/git/`
- Contains: Worktree create/remove/initialize, diff helpers, branch operations
- Depends on: `std::process::Command` (git CLI), `config::WorkflowPlugin`
- Used by: `tui` (via `AppState.git_ops`)
- Key files:
  - `src/git/mod.rs` - `is_git_repo`, `repo_root`, `current_branch`, `diff_stat`, `merge_branch`, `delete_branch`
  - `src/git/worktree.rs` - `create_worktree`, `remove_worktree`, `initialize_worktree`, `copy_dir_recursive`
  - `src/git/operations.rs` - `GitOperations` trait + `RealGitOps` implementation
  - `src/git/provider.rs` - `GitProviderOperations` trait + `RealGitHubOps` (GitHub PR operations via `gh` CLI)

**Tmux Layer:**
- Purpose: Tmux session/window management for agent processes
- Location: `src/tmux/`
- Contains: Session lifecycle, pane capture, key sending, window management
- Depends on: `std::process::Command` (tmux CLI)
- Used by: `tui` (via `AppState.tmux_ops`)
- Key files:
  - `src/tmux/mod.rs` - `spawn_session`, `list_sessions`, `capture_pane`, `send_keys`, `kill_session`, `SessionInfo`
  - `src/tmux/operations.rs` - `TmuxOperations` trait + `RealTmuxOps` implementation

**Agent Layer:**
- Purpose: Coding agent detection, command building, registry for multi-agent workflows
- Location: `src/agent/`
- Contains: Agent definitions, availability detection, interactive command construction, text generation
- Depends on: `which` (binary detection), `std::process::Command`
- Used by: `tui` (via `AppState.agent_registry`), `main.rs` (first-run detection)
- Key files:
  - `src/agent/mod.rs` - `Agent` struct, `known_agents()`, `detect_available_agents()`, `build_interactive_command()`, `build_spawn_args()`
  - `src/agent/operations.rs` - `AgentOperations` trait, `CodingAgent` impl, `AgentRegistry` trait, `RealAgentRegistry`

**Skills Layer:**
- Purpose: Skill file management, agent-native path mapping, command translation
- Location: `src/skills.rs`
- Contains: Compile-time embedded skills (`include_str!`), bundled plugin loading, agent-native directory mapping, command format translation
- Depends on: `config::WorkflowPlugin`, `toml`
- Used by: `tui` (skill deployment, command resolution)

**Config Layer:**
- Purpose: Global/project/merged configuration, workflow plugin definitions
- Location: `src/config/mod.rs`
- Contains: `GlobalConfig`, `ProjectConfig`, `MergedConfig`, `WorkflowPlugin`, `ThemeConfig`, phase agent overrides
- Depends on: `serde`, `toml`, `directories`
- Used by: All other layers

## Data Flow

**Task Lifecycle (Backlog to Done):**

1. User creates task in Backlog (title + description entered via TUI)
2. Task persisted to project SQLite database (`db.create_task()`)
3. User presses `m` to move to Planning:
   - Git worktree created at `.agtx/worktrees/{slug}` (background thread via `setup_task_worktree`)
   - Agent config dirs + plugin files copied to worktree (`initialize_worktree`)
   - Skills deployed to agent-native paths (`write_skills_to_worktree`)
   - Tmux window created with agent command (`tmux_ops.create_window`)
   - Skill command + prompt sent to agent via tmux (`send_skill_and_prompt`)
   - `SetupResult` sent back via `mpsc::channel` to update task in DB
4. Phase artifact detection: `refresh_sessions()` polls every 100ms (2s cache TTL) checking for artifact files defined in plugin config
5. User presses `m` to move Planning to Running: execute skill command sent to agent
6. User presses `m` to move Running to Review: optionally creates PR (commit, push, `gh pr create`)
7. User presses `m` to move Review to Done: archives artifacts, kills tmux window, removes worktree (branch preserved)

**Background Operations Flow:**

1. Worktree setup: `std::thread::spawn` -> `setup_task_worktree()` -> `mpsc::Sender<SetupResult>`
2. PR description generation: `std::thread::spawn` -> `generate_pr_description()` -> `mpsc::Sender<(String, String)>`
3. PR creation: `std::thread::spawn` -> `create_pr_with_content()` -> `mpsc::Sender<Result<(i32, String), String>>`
4. Main event loop checks `try_recv()` on each channel every 100ms poll cycle

**Agent Communication Flow:**

1. Agent started in tmux window with interactive command (`build_interactive_command`)
2. `wait_for_agent_ready()` polls pane content + `pane_current_command` for readiness
3. Skill command sent via `tmux_ops.send_keys()` (format translated per agent)
4. Prompt sent separately or combined depending on agent type and `prompt_trigger` config
5. Shell popup captures pane content with ANSI colors for live viewing

**State Management:**
- All mutable state lives in `AppState` struct inside `App`
- `App` owns both `terminal: Terminal` and `state: AppState` (split for borrow checker)
- Drawing functions are static (`fn draw_*(state: &AppState, frame: &mut Frame, area: Rect)`)
- Board navigation state in `BoardState` (selected column/row)
- Multiple popup states tracked as `Option<T>` fields on `AppState`
- Phase status cached in `HashMap<String, (PhaseStatus, Instant)>` with 2s TTL
- Plugin instances cached per task to avoid repeated disk reads

## Key Abstractions

**Trait-Based Dependency Injection:**
- Purpose: Enable testing without real external systems
- Examples: `src/tmux/operations.rs`, `src/git/operations.rs`, `src/git/provider.rs`, `src/agent/operations.rs`
- Pattern: Trait defined with `#[cfg_attr(feature = "test-mocks", automock)]`, real impl wraps CLI commands, mock generated by `mockall` behind `test-mocks` feature flag
- Injected via `Arc<dyn Trait>` in `AppState`

**WorkflowPlugin:**
- Purpose: Customizes task lifecycle behavior per phase
- Examples: `plugins/agtx/plugin.toml`, `plugins/gsd/plugin.toml`, `plugins/void/plugin.toml`
- Pattern: TOML configuration with commands, prompts, artifacts, prompt_triggers, copy_back, cyclic flag
- Resolution: project-local `.agtx/plugins/{name}/` -> global `~/.config/agtx/plugins/{name}/` -> bundled (compile-time embedded)

**MergedConfig:**
- Purpose: Unified configuration merging global + project settings
- Examples: `src/config/mod.rs`
- Pattern: Project overrides take precedence over global defaults, per-phase agent overrides fall back to default agent

**AgentRegistry:**
- Purpose: Maps agent names to `AgentOperations` implementations
- Examples: `src/agent/operations.rs`
- Pattern: `HashMap<String, Arc<dyn AgentOperations>>` with fallback to default agent

## Entry Points

**Binary Entry (`main`):**
- Location: `src/main.rs`
- Triggers: User runs `agtx` or `agtx -g` or `agtx /path`
- Responsibilities: Parse CLI args into `AppMode`, handle first-run config, create `App`, call `app.run()`

**Library Entry (`lib.rs`):**
- Location: `src/lib.rs`
- Triggers: Integration tests
- Responsibilities: Re-exports all modules + `AppMode` enum

**TUI Event Loop:**
- Location: `src/tui/app.rs` line 412 (`App::run`)
- Triggers: Called from `main()` after setup
- Responsibilities: Draw loop (100ms poll), check background channel results, handle key events, refresh sessions, clear expired warnings

**Key Handler Dispatch:**
- Location: `src/tui/app.rs` line 1597 (`App::handle_key`)
- Triggers: Any key press during event loop
- Responsibilities: Route to appropriate handler based on active popup/input mode

## Error Handling

**Strategy:** `anyhow::Result` with `.context()` for all fallible operations. Graceful degradation for missing tmux sessions, worktrees, or agent binaries.

**Patterns:**
- Database operations return `anyhow::Result`, errors propagated to TUI
- Git/tmux operations silently ignore failures where appropriate (e.g., cleanup on Done)
- Background thread errors sent via `mpsc` channel, displayed as transient warning messages (5s auto-clear)
- Worktree initialization collects warnings as `Vec<String>` instead of failing fatally
- Plugin loading failures fall back to bundled `agtx` plugin via `load_bundled_plugin("agtx")`

## Cross-Cutting Concerns

**Logging:** No structured logging framework. `eprintln!` used for non-fatal warnings (worktree init, plugin script failures). Transient warning messages displayed in TUI footer.

**Validation:** Minimal explicit validation. `TaskStatus::from_str` returns `Option` with `Backlog` fallback. Config parsing uses serde defaults.

**Authentication:** No internal auth. Relies on `gh` CLI being authenticated for GitHub PR operations. Agent CLIs handle their own auth.

**Configuration Layering:** Three-tier: global (`~/.config/agtx/config.toml`) -> project (`.agtx/config.toml`) -> plugin (`plugin.toml`). Merged at app startup via `MergedConfig::merge()`.

**Phase Status Detection:** Artifact file polling with glob support. Four states: Working (spinner), Idle (15s no output), Ready (artifact found), Exited (no tmux window). Idle detection uses content hashing.

---

*Architecture analysis: 2026-03-03*
