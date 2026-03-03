# Codebase Structure

**Analysis Date:** 2026-03-03

## Directory Layout

```
agtx/
├── src/
│   ├── main.rs              # Binary entry point, CLI arg parsing, first-run setup
│   ├── lib.rs               # Module exports + AppMode enum (for integration tests)
│   ├── skills.rs            # Skill constants, agent-native paths, command translation, bundled plugins
│   ├── tui/
│   │   ├── mod.rs           # Re-exports App, ShellPopup
│   │   ├── app.rs           # Main App struct, event loop, rendering, key handlers, workflow orchestration (5315 lines)
│   │   ├── app_tests.rs     # Unit tests for app.rs (2792 lines, included via #[path])
│   │   ├── board.rs         # BoardState - kanban column/row navigation
│   │   ├── input.rs         # InputMode enum (Normal, InputTitle, InputDescription)
│   │   └── shell_popup.rs   # ShellPopup state, content trimming, rendering helpers
│   ├── db/
│   │   ├── mod.rs           # Re-exports models and Database
│   │   ├── schema.rs        # Database struct, SQLite operations, migrations
│   │   └── models.rs        # Task, Project, TaskStatus, PhaseStatus, RunningAgent, AgentStatus
│   ├── tmux/
│   │   ├── mod.rs           # Tmux session management, pane capture, key sending
│   │   └── operations.rs    # TmuxOperations trait + RealTmuxOps implementation
│   ├── git/
│   │   ├── mod.rs           # Git helpers: is_git_repo, repo_root, current_branch, diff, merge, delete_branch
│   │   ├── worktree.rs      # Worktree create/remove/initialize, copy_dir_recursive
│   │   ├── operations.rs    # GitOperations trait + RealGitOps implementation
│   │   └── provider.rs      # GitProviderOperations trait + RealGitHubOps (PR operations via gh CLI)
│   ├── agent/
│   │   ├── mod.rs           # Agent struct, known_agents(), detect_available_agents(), build_interactive_command()
│   │   └── operations.rs    # AgentOperations/CodingAgent/AgentRegistry traits + real implementations
│   └── config/
│       └── mod.rs           # GlobalConfig, ProjectConfig, MergedConfig, WorkflowPlugin, ThemeConfig
├── plugins/
│   ├── agtx/
│   │   ├── plugin.toml      # Default workflow: skills + prompts for all phases
│   │   └── skills/
│   │       ├── research.md  # Research phase skill
│   │       ├── plan.md      # Planning phase skill
│   │       ├── execute.md   # Execution phase skill
│   │       └── review.md    # Review phase skill
│   ├── gsd/
│   │   └── plugin.toml      # Get Shit Done workflow (cyclic, research_required)
│   ├── spec-kit/
│   │   └── plugin.toml      # GitHub spec-kit workflow
│   └── void/
│       └── plugin.toml      # Plain agent session, no prompting
├── tests/
│   ├── db_tests.rs          # Database CRUD and model tests
│   ├── config_tests.rs      # Configuration loading, merging, first-run logic
│   ├── board_tests.rs       # Board navigation (column/row movement, clamping)
│   ├── git_tests.rs         # Git worktree and operations tests
│   ├── agent_tests.rs       # Agent detection and spawn argument tests
│   ├── mock_infrastructure_tests.rs  # Mock trait infrastructure validation
│   └── shell_popup_tests.rs # Shell popup content trimming and rendering logic
├── .github/
│   └── workflows/           # CI/CD workflows
├── .agtx/
│   └── plugins/             # Project-local plugin overrides
│       └── gsd/             # Local GSD plugin override (if present)
├── Cargo.toml               # Package manifest, dependencies, features
├── Cargo.lock               # Dependency lockfile
├── CLAUDE.md                # Project instructions for Claude Code
├── README.md                # Project documentation
├── install.sh               # Installation script
├── LICENSE                   # MIT license
└── .gitignore               # Ignores: /target/, .agtx/, IDE files, .DS_Store
```

## Directory Purposes

**`src/tui/`:**
- Purpose: Terminal user interface - rendering, event handling, workflow orchestration
- Contains: The main `App` struct (event loop owner), drawing functions, key handlers, popup states, task lifecycle transitions
- Key files: `app.rs` is the central hub (5315 lines) containing rendering, input handling, and all workflow orchestration logic

**`src/db/`:**
- Purpose: Data persistence via SQLite
- Contains: Schema definitions, CRUD operations, domain models
- Key files: `schema.rs` (Database struct with all SQL operations), `models.rs` (Task, Project, TaskStatus enums)

**`src/git/`:**
- Purpose: Git operations for worktree-based task isolation
- Contains: Worktree lifecycle, diff operations, branch management, GitHub PR operations
- Key files: `worktree.rs` (create/initialize/remove worktrees), `operations.rs` (mockable trait)

**`src/tmux/`:**
- Purpose: Tmux server/session/window management for agent processes
- Contains: Session creation, pane capture with ANSI support, key forwarding, window lifecycle
- Key files: `operations.rs` (mockable trait with 12 operations)

**`src/agent/`:**
- Purpose: Coding agent abstraction and registry
- Contains: Agent definitions, CLI binary detection, per-agent command construction, text generation
- Key files: `mod.rs` (agent definitions), `operations.rs` (AgentRegistry for multi-agent workflows)

**`src/config/`:**
- Purpose: Configuration management at global, project, and plugin levels
- Contains: Config structs, TOML serialization, theme colors, workflow plugin definition
- Key files: `mod.rs` (all config types, plugin loading, MergedConfig)

**`plugins/`:**
- Purpose: Bundled workflow plugin configurations (embedded at compile time via `include_str!`)
- Contains: TOML plugin definitions and skill markdown files
- Key files: Each plugin's `plugin.toml` defines commands, prompts, artifacts, and behavior flags

**`tests/`:**
- Purpose: Integration tests (run with `cargo test`)
- Contains: Tests for each module using real or mock implementations
- Key files: Each `*_tests.rs` corresponds to a source module

## Key File Locations

**Entry Points:**
- `src/main.rs`: Binary entry, CLI parsing, first-run setup
- `src/lib.rs`: Library exports for integration tests
- `src/tui/app.rs` line 412 (`App::run`): Main event loop

**Configuration:**
- `Cargo.toml`: Package manifest, dependencies, feature flags
- `src/config/mod.rs`: All config types (GlobalConfig, ProjectConfig, MergedConfig, ThemeConfig, WorkflowPlugin)
- Runtime: `~/.config/agtx/config.toml` (global), `.agtx/config.toml` (project)

**Core Logic:**
- `src/tui/app.rs`: Workflow orchestration, task state transitions, rendering (~5300 lines)
- `src/skills.rs`: Skill deployment, agent-native path mapping, command translation
- `src/git/worktree.rs`: Worktree creation and initialization

**Data Models:**
- `src/db/models.rs`: `Task`, `Project`, `TaskStatus`, `PhaseStatus`
- `src/db/schema.rs`: `Database` struct with all SQL operations

**Trait Definitions (for DI/mocking):**
- `src/tmux/operations.rs`: `TmuxOperations` trait
- `src/git/operations.rs`: `GitOperations` trait
- `src/git/provider.rs`: `GitProviderOperations` trait
- `src/agent/operations.rs`: `AgentOperations` and `AgentRegistry` traits

**Testing:**
- `src/tui/app_tests.rs`: App unit tests (included via `#[path]` in `app.rs`)
- `tests/*.rs`: Integration tests per module

## Naming Conventions

**Files:**
- `mod.rs` for module roots (re-exports)
- `snake_case.rs` for all source files
- `*_tests.rs` for test files
- Plugin configs always `plugin.toml`
- Skill files always `SKILL.md` (canonical) or `{phase}.md` (in plugins)

**Directories:**
- `snake_case` for source modules (`src/tui/`, `src/db/`)
- `kebab-case` for plugin names (`spec-kit`)
- `kebab-case` for skill directory names (`agtx-plan`, `agtx-execute`)

**Structs/Enums:**
- `PascalCase`: `AppState`, `BoardState`, `TaskStatus`, `PhaseStatus`
- Popup states suffixed with `Popup`: `PrConfirmPopup`, `DiffPopup`, `DeleteConfirmPopup`
- Search states suffixed with `State`: `FileSearchState`, `TaskSearchState`, `SkillSearchState`

**Functions:**
- `snake_case` throughout
- Drawing functions prefixed `draw_`: `draw_board`, `draw_sidebar`, `draw_shell_popup`
- Key handlers prefixed `handle_`: `handle_key`, `handle_normal_key`, `handle_shell_popup_key`
- Trait methods match the operation: `create_worktree`, `kill_window`, `send_keys`

**Constants:**
- `SCREAMING_SNAKE_CASE`: `AGENT_SERVER`, `DEFAULT_SKILLS`, `BUNDLED_PLUGINS`
- Compile-time skills: `RESEARCH_SKILL`, `PLAN_SKILL`, `EXECUTE_SKILL`, `REVIEW_SKILL`

## Where to Add New Code

**New Task Field:**
1. Add field to `Task` struct in `src/db/models.rs`
2. Add column to schema and migration in `src/db/schema.rs`
3. Update `create_task`, `update_task`, `task_from_row` in `src/db/schema.rs`
4. Update UI rendering in `src/tui/app.rs`
5. Add test coverage in `tests/db_tests.rs`

**New Keyboard Shortcut:**
1. Find the appropriate `handle_*_key` function in `src/tui/app.rs`
2. Add `KeyCode` match arm with the new behavior
3. Update footer text in `build_footer_text()` at top of `src/tui/app.rs`

**New Popup:**
1. Add popup state struct in `src/tui/app.rs` (near other popup structs, lines 141-281)
2. Add `Option<YourPopup>` field to `AppState` (line 63)
3. Initialize to `None` in `App::with_ops()` (line 346)
4. Add rendering in `draw_board()` function
5. Add `handle_your_popup_key()` method
6. Add dispatch in `handle_key()` to route to handler

**New Agent:**
1. Add to `known_agents()` in `src/agent/mod.rs`
2. Add `build_interactive_command()` match arm in `src/agent/mod.rs`
3. Add agent-native skill dir in `agent_native_skill_dir()` in `src/skills.rs`
4. Add command transform in `transform_plugin_command()` in `src/skills.rs`
5. Add skill deployment case in `write_skills_to_worktree()` in `src/tui/app.rs`
6. Add tests in `tests/agent_tests.rs`

**New Bundled Plugin:**
1. Create `plugins/<name>/plugin.toml` with commands, prompts, artifacts
2. Add entry to `BUNDLED_PLUGINS` array in `src/skills.rs`
3. Optionally add skill files in `plugins/<name>/skills/`
4. Optionally add `supported_agents` to restrict agent compatibility

**New Theme Color:**
1. Add field to `ThemeConfig` in `src/config/mod.rs`
2. Add `default_color_*` function and update `Default` impl
3. Use `hex_to_color(&state.config.theme.color_*)` in `src/tui/app.rs`
4. Add test in `tests/config_tests.rs`

**New Git Operation:**
1. Add method to `GitOperations` trait in `src/git/operations.rs`
2. Implement in `RealGitOps` in `src/git/operations.rs`
3. Optionally add helper function in `src/git/mod.rs` or `src/git/worktree.rs`
4. Add test in `tests/git_tests.rs`

**New Tmux Operation:**
1. Add method to `TmuxOperations` trait in `src/tmux/operations.rs`
2. Implement in `RealTmuxOps` in `src/tmux/operations.rs`
3. Add test in `tests/mock_infrastructure_tests.rs`

**Utility / Helper Functions:**
- Git helpers: `src/git/mod.rs` (standalone functions) or `src/git/worktree.rs` (worktree-specific)
- Skill helpers: `src/skills.rs`
- Config helpers: `src/config/mod.rs`
- TUI helpers: bottom of `src/tui/app.rs` (free functions after `impl App`)

## Special Directories

**`.agtx/` (project-level, gitignored):**
- Purpose: Runtime data for a project managed by agtx
- Contains: `worktrees/` (git worktrees for active tasks), `plugins/` (project-local plugin overrides), `config.toml` (project config), `archive/` (archived task artifacts)
- Generated: Yes, at runtime
- Committed: No (in `.gitignore`)

**`plugins/` (source tree):**
- Purpose: Bundled plugin configurations embedded at compile time
- Contains: TOML plugin configs and skill markdown files
- Generated: No
- Committed: Yes

**`target/` (build output):**
- Purpose: Rust build artifacts
- Generated: Yes, by cargo
- Committed: No (in `.gitignore`)

**`~/.config/agtx/` (user home, not in repo):**
- Purpose: Global user configuration and data
- Contains: `config.toml` (global config), `plugins/` (user-installed plugins), `projects/` (per-project SQLite databases), `index.db` (global project index)
- Generated: Yes, at runtime
- Committed: N/A (not in repo)

---

*Structure analysis: 2026-03-03*
