# Codebase Structure

**Analysis Date:** 2026-03-03

## Directory Layout

```
agtx/
├── src/                           # Rust source code
│   ├── main.rs                    # Entry point, CLI parsing, first-run config
│   ├── lib.rs                     # Module exports for library access
│   ├── skills.rs                  # Skill system: content, agent paths, plugin loading
│   ├── tui/                       # Terminal UI (Ratatui + Crossterm)
│   │   ├── mod.rs                 # Module exports
│   │   ├── app.rs                 # Main App struct, event loop, rendering (5315 lines)
│   │   ├── app_tests.rs           # Unit tests for app.rs (included via #[path])
│   │   ├── board.rs               # BoardState - kanban column/row navigation
│   │   ├── input.rs               # InputMode enum (Normal, InputTitle, InputDescription)
│   │   └── shell_popup.rs         # Shell popup rendering and content management
│   ├── db/                        # Database layer (SQLite)
│   │   ├── mod.rs                 # Module exports
│   │   ├── models.rs              # Task, TaskStatus, Project, RunningAgent, PhaseStatus
│   │   └── schema.rs              # Database struct, CRUD operations, migrations
│   ├── tmux/                      # Tmux session management
│   │   ├── mod.rs                 # tmux module with session spawning
│   │   └── operations.rs          # TmuxOperations trait, RealTmuxOps impl
│   ├── git/                       # Git operations and worktree management
│   │   ├── mod.rs                 # Helper functions (is_git_repo, diff_stat, etc.)
│   │   ├── worktree.rs            # Worktree creation, initialization, file copying
│   │   ├── operations.rs          # GitOperations trait, RealGitOps impl
│   │   └── provider.rs            # GitProviderOperations for GitHub PR creation
│   ├── agent/                     # Agent detection and integration
│   │   ├── mod.rs                 # Agent struct, known_agents(), detect_available_agents()
│   │   └── operations.rs          # AgentOperations trait, agent command building
│   └── config/                    # Configuration management
│       └── mod.rs                 # GlobalConfig, ProjectConfig, ThemeConfig, WorkflowPlugin
├── tests/                         # Integration tests
│   ├── db_tests.rs                # Task, Project, TaskStatus tests
│   ├── config_tests.rs            # GlobalConfig loading tests
│   ├── board_tests.rs             # BoardState navigation tests
│   ├── git_tests.rs               # Git worktree tests
│   ├── agent_tests.rs             # Agent detection and spawn args tests
│   ├── mock_infrastructure_tests.rs # Tests for mock traits
│   └── shell_popup_tests.rs       # Shell popup trimming logic tests
├── plugins/                       # Bundled workflow plugins (embedded at compile time)
│   ├── agtx/                      # Default agtx workflow
│   │   ├── plugin.toml            # Phase commands, prompts, artifacts, init script
│   │   └── skills/                # Skill markdown files
│   │       ├── research.md        # Research phase skill
│   │       ├── plan.md            # Planning phase skill
│   │       ├── execute.md         # Running phase skill
│   │       └── review.md          # Review phase skill
│   ├── gsd/                       # Get Shit Done workflow
│   │   └── plugin.toml            # GSD-specific phases and prompts
│   ├── spec-kit/                  # GitHub spec-kit workflow
│   │   └── plugin.toml            # Spec-driven development workflow
│   └── void/                      # Minimal workflow (no prompting)
│       └── plugin.toml            # Plain agent session
├── .agtx/                         # Project-local agtx data (git-ignored)
│   └── plugins/                   # Project-local plugin overrides
├── Cargo.toml                     # Package manifest and dependencies
├── Cargo.lock                     # Locked dependency versions
└── .planning/
    └── codebase/                  # GSD codebase analysis docs
```

## Directory Purposes

**src/:**
- Purpose: All Rust source code for the application
- Contains: Application logic, TUI, database, external integrations
- Key pattern: Module-per-concern architecture

**src/tui/:**
- Purpose: Terminal UI implementation using Ratatui (TUI framework) and Crossterm (terminal events)
- Contains: Event loop, rendering, input handling, popup management
- Key files:
  - `app.rs`: Core event loop (main thread), handlers for all keyboard shortcuts, state mutations
  - `board.rs`: Navigation logic for 5-column kanban (selected_column, selected_row, bounds checking)
  - `input.rs`: Input mode state machine (Normal, InputTitle, InputDescription)
  - `shell_popup.rs`: Tmux output popup (fetches window content, trims, renders with scrolling)

**src/db/:**
- Purpose: Persistent data storage using SQLite (bundled, no external binary required)
- Contains: Task CRUD, Project index, schema management
- Schema:
  - `tasks` table: id, title, description, status, agent, project_id, session_name, worktree_path, branch_name, pr_number, pr_url, plugin, cycle, created_at, updated_at
  - `projects` table: id, name, path, github_url, default_agent, last_opened
- Key pattern: Single Connection per database, auto-initialization on open

**src/tmux/:**
- Purpose: Interact with tmux server named `agtx` (separate from user's regular sessions)
- Contains: Session spawning, command sending (send-keys), window listing, content fetching
- Architecture: Dedicated tmux server `agtx` keeps agent sessions isolated from user shell

**src/git/:**
- Purpose: Git worktree management and repository operations
- Contains:
  - `worktree.rs`: Create worktrees at `.agtx/worktrees/{slug}/`, copy agent config, run init scripts
  - `operations.rs`: Trait-based abstraction for all git operations
  - `provider.rs`: GitHub API integration for PR creation/fetching
- Key pattern: Worktrees are created from main branch, new branches named `task/{slug}`

**src/agent/:**
- Purpose: Integration with coding agents (Claude, Codex, Copilot, Gemini, OpenCode)
- Contains: Agent detection, command building, spawn arguments per agent
- Key pattern: Each agent has different CLI flags and prompt formats (abstracted via traits)

**src/config/:**
- Purpose: Configuration management for global settings, project overrides, themes, plugins
- Contains:
  - `GlobalConfig`: Default agent, per-phase agent overrides, worktree settings, theme colors
  - `ProjectConfig`: Project-specific overrides (different from global)
  - `ThemeConfig`: 9 hex colors for UI elements (selected, normal, dimmed, text, accent, description, headers, popups)
  - `WorkflowPlugin`: Phase commands, prompts, artifacts, init scripts, file copies
- Storage: `~/.config/agtx/config.toml`

**src/skills.rs:**
- Purpose: Skill system for agent-native command discovery
- Contains:
  - Canonical skill content (embedded from `plugins/*/skills/*.md`)
  - Agent-native directory mapping (`.claude/commands/agtx/` vs `.gemini/commands/agtx/` vs `.codex/skills/agtx-*`, etc.)
  - Plugin loading from bundled and filesystem sources
  - Command transformation per agent (`/gsd:plan` → `/gsd-plan` for opencode, `$gsd-plan` for codex)
- Key pattern: Skills written once, auto-deployed to agent-native paths during worktree init

**plugins/:**
- Purpose: Workflow plugin definitions (bundled at compile time via `include_str!()`)
- Contains:
  - `plugin.toml`: Phase commands, prompts (with `{task}`, `{task_id}`, `{phase}` templates), artifacts, init scripts
  - `skills/`: Markdown files with YAML frontmatter (name, description)
- Key pattern: Each plugin customizes the task lifecycle

**tests/:**
- Purpose: Integration and unit tests for all modules
- Contains: Tests for models, database, configuration, board navigation, git operations, agent detection
- Key patterns:
  - Models tested for roundtrip serialization
  - Board navigation tested for boundary conditions
  - Git/tmux/agent operations use trait-based mocks via `feature = "test-mocks"`

**.agtx/ (project-local, git-ignored):**
- Purpose: Store generated worktrees and plugin overrides
- Contents:
  - `worktrees/`: Per-task working directories created from git worktrees
  - `plugins/`: Local plugin overrides (project-specific customizations)
  - `skills/`: Canonical skill copies (for reference)
  - `artifacts/`: Phase completion indicators (e.g., `plan.txt`, `execute.txt`)

## Key File Locations

**Entry Points:**
- `src/main.rs`: CLI entry point, mode detection (Dashboard vs Project), first-run config, app initialization
- `src/lib.rs`: Library interface (re-exports all public modules)
- `src/tui/app.rs`: Event loop implementation (App::run(), event handlers)

**Configuration:**
- `~/.config/agtx/config.toml`: Global configuration (colors, default agent, worktree settings)
- `src/config/mod.rs`: Config struct definitions and TOML parsing

**Core Logic:**
- `src/db/models.rs`: Task, TaskStatus, Project, PhaseStatus enums
- `src/db/schema.rs`: Database initialization and CRUD operations
- `src/tui/app.rs`: Task lifecycle handlers (advance_task, move_task, create_task, delete_task)
- `src/tui/board.rs`: Kanban board navigation
- `src/git/worktree.rs`: Worktree creation and initialization

**Testing:**
- `src/tui/app_tests.rs`: Unit tests for app.rs (included inline via `#[path]`)
- `tests/*.rs`: Integration tests (run with `cargo test`)

## Naming Conventions

**Files:**
- Rust modules: snake_case (e.g., `app.rs`, `shell_popup.rs`, `git_operations.rs`)
- Config files: kebab-case (e.g., `config.toml`, `plugin.toml`)
- Skill files: kebab-case with phase name (e.g., `research.md`, `plan.md`)

**Directories:**
- Source modules: lowercase (e.g., `src/tui/`, `src/db/`)
- Bundled plugins: lowercase (e.g., `plugins/agtx/`, `plugins/gsd/`)
- Project-local: dot-prefix for hidden dirs (e.g., `.agtx/`, `.claude/`)

**Code Identifiers:**
- Structs: PascalCase (e.g., `AppState`, `BoardState`, `WorkflowPlugin`)
- Enums: PascalCase (e.g., `TaskStatus`, `InputMode`, `PhaseStatus`)
- Functions: snake_case (e.g., `advance_task()`, `move_task()`, `hex_to_color()`)
- Constants: SCREAMING_SNAKE_CASE (e.g., `AGENT_SERVER`, `AGTX_DIR`, `SHELL_POPUP_HEIGHT_PERCENT`)
- Module names: snake_case (e.g., `mod agent`, `mod tui`)

**Trait Naming:**
- Suffixed with Operations or Registry (e.g., `GitOperations`, `TmuxOperations`, `AgentRegistry`)
- Mock implementations: Prefixed with Mock (e.g., `MockGitOperations`, `MockTmuxOperations`)
- Real implementations: Prefixed with Real (e.g., `RealGitOps`, `RealTmuxOps`)

## Where to Add New Code

**New Feature (e.g., new task field):**
- Add field to `Task` struct in `src/db/models.rs`
- Add column to schema and migration in `src/db/schema.rs`
- Update CRUD methods (create_task, update_task, task_from_row)
- Update UI rendering in `src/tui/app.rs` (draw_* functions)
- Add tests in `tests/db_tests.rs`

**New Keyboard Shortcut:**
- Find appropriate `handle_*_key()` function in `src/tui/app.rs`
- Add match arm for the new key
- Update footer help text in `build_footer_text()` function
- Update corresponding handler (task advance, move, create, delete, etc.)

**New UI Popup:**
- Define state struct in `src/tui/app.rs` (e.g., `MyPopup`)
- Add `Option<MyPopup>` field to `AppState`
- Add rendering function `draw_my_popup()` (call from `draw_board()`)
- Add key handler `handle_my_popup_key()`
- Add routing in `handle_key()` to dispatch to handler

**New External Integration (e.g., new git operation):**
- Add method to `GitOperations` trait in `src/git/operations.rs`
- Implement in `RealGitOps` struct
- Create mock in `#[cfg(feature = "test-mocks")]` section
- Use from `AppState` via `Arc<dyn GitOperations>` (already injected)
- Add tests in `tests/git_tests.rs`

**New Workflow Plugin:**
- Create `plugins/{name}/plugin.toml` with TOML config
- Add skill markdown files in `plugins/{name}/skills/`
- Add entry to `BUNDLED_PLUGINS` in `src/skills.rs`
- Optionally add `supported_agents` field to restrict compatibility

**New Theme Color:**
- Add field to `ThemeConfig` in `src/config/mod.rs` (e.g., `pub color_new_element: String`)
- Add default function and update `Default` impl
- Use `hex_to_color(&state.config.theme.color_new_element)` in `src/tui/app.rs` draw functions
- Document in config template

**New Agent Support:**
- Add to `known_agents()` vector in `src/agent/mod.rs`
- Add match arm in `build_interactive_command()` with agent-specific flags
- Add agent-native skill directory in `agent_native_skill_dir()` in `src/skills.rs`
- Add command transformation in `transform_plugin_command()` in `src/skills.rs`
- Add tests in `tests/agent_tests.rs`

## Special Directories

**`.agtx/` (project-local, git-ignored):**
- Purpose: Store task-specific worktrees and artifacts
- Generated: Yes (created by app when tasks enter Planning phase)
- Committed: No (git-ignored)
- Contents:
  - `worktrees/{slug}/`: Git worktrees created from main branch
  - `artifacts/`: Phase completion signals (e.g., `.agtx/artifacts/plan.txt` signals planning complete)
  - `skills/`: Canonical skill copies for reference
  - `plugins/`: Local plugin overrides

**`~/.config/agtx/` (global, user-specific):**
- Purpose: Store global configuration and per-project databases
- Generated: Yes (created on first run)
- Committed: No (user-specific)
- Contents:
  - `config.toml`: Global configuration
  - `index.db`: Project index database
  - `projects/{path_hash}.db`: Per-project task databases
  - `plugins/`: Global plugin overrides

**`.planning/codebase/` (repo-level documentation):**
- Purpose: GSD codebase analysis documents
- Generated: Yes (by GSD mapper agent)
- Committed: Yes (git-tracked for team reference)
- Contents: ARCHITECTURE.md, STRUCTURE.md, TESTING.md, CONVENTIONS.md, CONCERNS.md (as applicable)

**`plugins/` (bundled plugins):**
- Purpose: Workflow definitions bundled with the binary
- Generated: No (part of source repo)
- Committed: Yes
- Contents:
  - `agtx/`: Default workflow (research → plan → execute → review)
  - `gsd/`: Get Shit Done workflow (research → plan → execute → review → done)
  - `spec-kit/`: GitHub spec-driven development workflow
  - `void/`: Minimal workflow (plain agent session)

## File Organization Principles

**Module-per-concern:** Each module owns a specific layer (db, tui, git, etc.). No cross-module state except via traits.

**Separation of concerns:** TUI doesn't know about tmux details; it calls trait methods. Testable via trait mocks.

**Trait-based injection:** All external operations (git, tmux, agents) are injected as Arc<dyn Trait>. Allows testing without actual external tools.

**State separation:** `App { terminal, state: AppState }` keeps terminal and logic separate. Borrow checker friendly.

**Immutable rendering:** All `draw_*` functions take immutable references to state. No side effects during rendering.

**Config-driven behavior:** Workflow plugins customize task lifecycle. App loads plugins at task creation time, stored in DB.
