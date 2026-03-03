# Coding Conventions

**Analysis Date:** 2026-03-03

## Naming Patterns

**Files:**
- Rust source files use `snake_case`: `models.rs`, `operations.rs`, `app_tests.rs`
- Module files use `mod.rs` at directory root
- Test files follow pattern: `{module}_tests.rs` or included as `#[path = "module_tests.rs"]` within module
- Example: `src/tui/app_tests.rs` included in `src/tui/app.rs`

**Functions:**
- All functions use `snake_case`: `detect_available_agents()`, `generate_session_name()`, `build_interactive_command()`
- Constructor functions commonly named `new()`: `Task::new()`, `Agent::new()`, `Database::new()`
- Helper functions prefixed with purpose: `diff_stat_from_main()`, `list_untracked_files()`, `generate_pr_description()`
- Private helper functions follow same convention: `hex_to_color()`, `build_footer_text()`

**Variables:**
- Local variables use `snake_case`: `selected_column`, `pending_task_title`, `mock_git`
- Boolean flags use `should_`, `has_`, `is_` prefixes: `should_quit`, `has_changes`, `is_available()`
- Mutable variables prefixed or indicate mutability: `mut mock_tmux`, `mut board`
- Loop indices use `i` for outer loops, `idx` for indexed access

**Types:**
- Structs use `PascalCase`: `AppState`, `BoardState`, `WorkflowPlugin`, `TaskStatus`
- Enums use `PascalCase`: `PhaseStatus`, `TaskStatus`, `FirstRunAction`, `DoneConfirmPrState`
- Enum variants use `PascalCase`: `TaskStatus::Backlog`, `PhaseStatus::Working`
- Generic trait types use single capital letters: `T` for generic, or descriptive names like `S` for Sync/Send traits

**Traits:**
- Traits use `PascalCase` with `Operations` suffix for mockable traits: `GitOperations`, `AgentOperations`, `TmuxOperations`
- Trait methods use `snake_case`: `create_worktree()`, `generate_text()`, `send_keys()`

## Code Style

**Formatting:**
- Edition: Rust 2021
- Indentation: 4 spaces (enforced by `cargo fmt`)
- Line length: No hard limit, but readability preferred
- Imports grouped by: external crates → internal modules → relative imports

**Linting:**
- No `.clippy.toml` or `.rustfmt.toml` found; uses Rust defaults
- Code appears formatted with `cargo fmt` (no inconsistent spacing)
- Style is idiomatic Rust with modern patterns (Result types, trait objects, Arc for thread safety)

**Method visibility:**
- Public API methods: `pub fn`
- Implementation detail methods: private `fn`
- Trait implementations: `impl` without visibility (inherits from trait)
- Example visibility pattern from `src/db/schema.rs`:
  ```rust
  impl Database {
    pub fn open_project(project_path: &Path) -> Result<Self> { ... }  // Public API
    fn init_project_schema(&self) -> Result<()> { ... }               // Private helper
    fn hash_path(path: &str) -> String { ... }                        // Private static helper
  }
  ```

## Import Organization

**Order:**
1. External crate imports (`anyhow`, `rusqlite`, `ratatui`, `serde`, `std`)
2. Internal module imports (`crate::db`, `crate::agent`, etc.)
3. Conditional imports (`#[cfg(feature = "test-mocks")]` imports)
4. Super-module imports (`use super::*`)

**Pattern from actual code:**
```rust
// src/tui/app.rs
use anyhow::Result;
use crossterm::{...};
use ratatui::{prelude::*, widgets::*};
use std::collections::{HashMap, HashSet};
use std::io::{self, Stdout};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::Instant;

use crate::agent::{self, AgentOperations};
use crate::config::{GlobalConfig, MergedConfig, ProjectConfig, ThemeConfig, WorkflowPlugin};
use crate::db::{Database, PhaseStatus, Task, TaskStatus};
use crate::git::{self, GitOperations, GitProviderOperations, PullRequestState, RealGitHubOps, RealGitOps};

use super::board::BoardState;
use super::input::InputMode;
```

**Path Aliases:**
- No path aliases configured; all imports use absolute crate paths
- `crate::` prefix used for internal modules from any location
- `super::` used for relative imports within module hierarchy

## Error Handling

**Framework:** `anyhow` Result type throughout

**Patterns:**
- All fallible functions return `Result<T>` from `anyhow::Result`
- Error context added with `.context()` and `.with_context()` for debugging:
  ```rust
  // From src/config/mod.rs
  let content = std::fs::read_to_string(&config_path)
    .with_context(|| format!("Failed to read config from {:?}", config_path))?;
  ```
- Parse errors include field info: `"Failed to parse global config"`
- Directory/path errors include the actual path: `with_context(|| format!("Failed to open database at {:?}", db_path))`

**Bail pattern for explicit errors:**
```rust
// From src/agent/operations.rs
anyhow::bail!("{} command failed: {}", self.agent.name, stderr);

// From src/git/mod.rs
anyhow::bail!("Merge failed: {}", stderr);
```

**Graceful degradation:**
- Missing tmux sessions handled: `.is_ok()` checks without propagating errors
- File operations: `std::fs::remove_file(&old_path).ok()` to ignore non-critical failures
- Config migrations: Returns boolean to indicate success

**No panics in main code:**
- Uses Result/Option throughout
- `unwrap()` only used in tests and initialization paths
- `expect()` not used; errors bubble up as Result

## Logging

**Framework:** `println!()` and `eprintln!()` for TUI application

**Patterns:**
- No structured logging framework; uses direct println for user-facing messages
- CLI output for selection prompts uses `println!()` with styled output via `crossterm`
- Debug/error messages printed to stderr in main.rs error context

**Examples:**
```rust
// src/main.rs
println!("\n  Selected: {}\n", agents[idx].name);
stdout.execute(style::Print("Some message"))?;
```

## Comments

**When to Comment:**
- Module-level documentation: `//!` module docs describe purpose and usage
- Example from `src/agent/operations.rs`:
  ```rust
  //! Traits for agent operations to enable testing with mocks.
  //!
  //! This module provides a generic interface for interacting with coding agents
  //! like Claude Code, Aider, Codex, etc.
  ```
- Function-purpose comments before complex logic blocks
- Inline comments explaining non-obvious algorithm choices

**JSDoc/Documentation Comments:**
- Struct/trait documentation uses `///` comments:
  ```rust
  /// Operations for git worktree management
  pub trait GitOperations: Send + Sync {
    /// Create a worktree for a task
    fn create_worktree(&project_path: &Path, task_slug: &str) -> Result<String>;
  }
  ```
- Enum variant documentation:
  ```rust
  /// Agent is still working, no artifact yet
  Working,
  /// Agent output hasn't changed for 15s — may need user input
  Idle,
  ```

**Avoiding over-commenting:**
- Self-documenting code preferred: `fn is_git_repo()`, `fn worktree_exists()` need no comment
- Complex business logic (phase detection, polling) gets block comments
- TODO comments used sparingly: Found one example `// TODO: investigate CLI usage before enabling`

## Function Design

**Size:**
- Most functions under 50 lines, main exception is `App::handle_key()` and drawing functions which can be 100-200 lines
- Helper functions extracted when logic is reused
- Example: `build_footer_text()` extracted to avoid duplication across input modes

**Parameters:**
- Immutable references preferred: `&Path`, `&str`, `&Task`
- Owned types used when needed: `String`, `Vec<T>`, `PathBuf`
- Trait objects used for dependency injection: `&dyn GitOperations`, `Arc<dyn TmuxOperations>`
- Into trait used for ergonomic APIs:
  ```rust
  pub fn new(title: impl Into<String>, agent: impl Into<String>) -> Self
  ```

**Return Values:**
- Most public APIs return `Result<T>` for error handling
- Some queries return `Option<T>`: `selected_task()`, `get_agent()`
- Void operations return `Result<()>`: `fn add_all(&self, worktree_path: &Path) -> Result<()>`
- Boolean predicates: `is_available()`, `has_changes()`, `worktree_exists()`

## Module Design

**Exports:**
- `lib.rs` re-exports all public modules:
  ```rust
  // src/lib.rs
  pub mod agent;
  pub mod config;
  pub mod db;
  pub mod git;
  pub mod skills;
  pub mod tmux;
  pub mod tui;
  ```
- Submodules re-export key types in module `mod.rs`:
  ```rust
  // src/agent/mod.rs
  pub use operations::{AgentOperations, AgentRegistry, CodingAgent, RealAgentRegistry};
  #[cfg(feature = "test-mocks")]
  pub use operations::{MockAgentOperations, MockAgentRegistry};
  ```

**Barrel Files:**
- Minimal barrel exports; each module exposes what's needed
- `crate::` imports used throughout for clarity
- Test mocks gated behind `#[cfg(feature = "test-mocks")]`

**Module Structure:**
- Large modules split into submodules: `tui/app.rs`, `tui/board.rs`, `tui/input.rs`, `tui/shell_popup.rs`
- Tests included in module: `#[path = "app_tests.rs"]` in `app.rs`
- Traits live in `operations.rs`: `git/operations.rs`, `agent/operations.rs`, `tmux/operations.rs`
- Implementations in module root or specialized file: `git/worktree.rs` for worktree-specific logic

## Dependency Injection Pattern

**Traits for mockability:**
- Key I/O operations use trait-based dependency injection
- Traits: `GitOperations`, `TmuxOperations`, `AgentOperations`, `GitProviderOperations`
- Real implementations: `RealGitOps`, `RealTmuxOps`, `CodingAgent`, `RealGitHubOps`
- Mock implementations gated by `test-mocks` feature: `MockGitOperations`, `MockTmuxOperations`

**Injected into App:**
```rust
struct AppState {
  tmux_ops: Arc<dyn TmuxOperations>,
  git_ops: Arc<dyn git::GitOperations>,
  git_provider_ops: Arc<dyn GitProviderOperations>,
  agent_registry: Arc<dyn agent::AgentRegistry>,
}
```

---

*Convention analysis: 2026-03-03*
