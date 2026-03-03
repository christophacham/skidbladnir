# Testing Patterns

**Analysis Date:** 2026-03-03

## Test Framework

**Runner:**
- Rust built-in test framework (`cargo test`)
- No external test runner (no `cargo-nextest`)

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_ne!` macros
- No third-party assertion library

**Mocking:**
- `mockall` 0.13 - conditional dependency via `test-mocks` feature flag
- Available as both dev-dependency and optional dependency

**Run Commands:**
```bash
cargo test                        # Run unit + integration tests (no mocks)
cargo test --features test-mocks  # Run all tests including mock-based tests
cargo build --verbose             # CI build step
```

**CI:** `.github/workflows/ci.yml` runs `cargo test --verbose --features test-mocks` on ubuntu-latest and macos-latest with stable Rust.

## Test File Organization

**Location:** Two patterns coexist:

1. **Integration tests in `tests/` directory** (7 files, 1809 lines total):
   - `tests/db_tests.rs` (114 lines) - Database model tests
   - `tests/config_tests.rs` (293 lines) - Configuration merge/parse tests
   - `tests/board_tests.rs` (225 lines) - Board navigation tests
   - `tests/git_tests.rs` (473 lines) - Git worktree operations (real git)
   - `tests/agent_tests.rs` (42 lines) - Agent selection parsing
   - `tests/mock_infrastructure_tests.rs` (190 lines) - Mock infrastructure validation
   - `tests/shell_popup_tests.rs` (472 lines) - Shell popup logic + rendering

2. **Unit tests in-module via `#[path]` attribute** (1 file, 2792 lines):
   - `src/tui/app_tests.rs` included at the bottom of `src/tui/app.rs`:
     ```rust
     #[cfg(test)]
     #[path = "app_tests.rs"]
     mod tests;
     ```
   - This gives tests access to private functions in `app.rs` (e.g., `generate_pr_description`, `create_pr_with_content`, `fuzzy_find_files`, `fuzzy_score`, `send_key_to_tmux`, `ensure_project_tmux_session`)

**Naming:**
- Integration test files: `{module}_tests.rs`
- Unit test module: `app_tests.rs` (included via `#[path]`)
- Test functions: `test_{what_is_tested}` or `test_{function_name}_{scenario}`

**Structure:**
```
tests/
├── agent_tests.rs                # Pure function tests for agent selection
├── board_tests.rs                # BoardState navigation logic
├── config_tests.rs               # Config parsing, merging, first-run logic
├── db_tests.rs                   # Task/Project model construction
├── git_tests.rs                  # Worktree create/remove (real git repos)
├── mock_infrastructure_tests.rs  # Validates mock setup patterns
└── shell_popup_tests.rs          # Popup scrolling, rendering, trimming

src/tui/
└── app_tests.rs                  # Unit tests for app.rs private functions
```

## Test Structure

**Suite Organization:**
- No `describe` or `mod tests {}` nesting in integration tests - flat list of `#[test]` functions
- Section comments using `// === Section Name ===` to group related tests
- Helper functions at the top of the file (e.g., `create_test_task()`, `setup_git_repo()`)

**Example pattern from `tests/board_tests.rs`:**
```rust
use agtx::db::{Task, TaskStatus};
use agtx::tui::board::BoardState;

fn create_test_task(title: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(title, "claude", "test-project");
    task.status = status;
    task
}

// === BoardState Tests ===

#[test]
fn test_board_state_new() {
    let board = BoardState::new();
    assert!(board.tasks.is_empty());
    assert_eq!(board.selected_column, 0);
    assert_eq!(board.selected_row, 0);
}

#[test]
fn test_tasks_in_column_with_tasks() {
    let mut board = BoardState::new();
    board.tasks = vec![
        create_test_task("Task 1", TaskStatus::Backlog),
        create_test_task("Task 2", TaskStatus::Backlog),
        create_test_task("Task 3", TaskStatus::Running),
    ];
    assert_eq!(board.tasks_in_column(0).len(), 2);
    assert_eq!(board.tasks_in_column(2).len(), 1);
}
```

**Patterns:**
- **Arrange/Act/Assert** - setup data, call function, assert result
- No explicit setup/teardown lifecycle hooks; helper functions handle setup
- `TempDir` from `tempfile` crate for filesystem tests (auto-cleanup on drop)

## Mocking

**Framework:** `mockall` 0.13

**Feature gate pattern:**
```rust
// In trait definition (src/git/operations.rs):
#[cfg_attr(feature = "test-mocks", automock)]
pub trait GitOperations: Send + Sync {
    fn create_worktree(&self, project_path: &Path, task_slug: &str) -> Result<String>;
    // ...
}

// In module re-exports (src/git/mod.rs):
#[cfg(feature = "test-mocks")]
pub use operations::MockGitOperations;

// In tests (src/tui/app_tests.rs):
#[cfg(feature = "test-mocks")]
use crate::git::{MockGitOperations, MockGitProviderOperations};

#[test]
#[cfg(feature = "test-mocks")]
fn test_something() { ... }
```

**Mock configuration patterns from `tests/mock_infrastructure_tests.rs` and `src/tui/app_tests.rs`:**

```rust
// Basic expectation with return value
let mut mock_tmux = MockTmuxOperations::new();
mock_tmux.expect_has_session()
    .returning(|_| false);

// Expectation with argument matching
mock_git.expect_commit()
    .withf(|path: &Path, msg: &str| {
        path == Path::new("/tmp/worktree") && msg.contains("Test commit")
    })
    .times(1)
    .returning(|_, _| Ok(()));

// Expectation with specific argument values (predicate::eq)
mock_tmux.expect_has_session()
    .with(mockall::predicate::eq("my-project"))
    .times(1)
    .returning(|_| false);

// Return constant (for methods returning &str)
mock_agent.expect_co_author_string()
    .return_const("Claude <noreply@anthropic.com>".to_string());

// Arc wrapping for thread-safe sharing
let arc_git: Arc<dyn GitOperations> = Arc::new(mock_git);
```

**What to Mock:**
- External system interactions: tmux, git CLI, GitHub API, coding agent CLI
- Every trait in the operations layer: `GitOperations`, `TmuxOperations`, `AgentOperations`, `GitProviderOperations`, `AgentRegistry`

**What NOT to Mock:**
- Pure data types: `Task`, `Project`, `TaskStatus`, `BoardState`, `ShellPopup`
- Configuration structs: `GlobalConfig`, `ProjectConfig`, `MergedConfig`
- Internal algorithms: fuzzy search scoring, string transformations, board navigation
- Rendering output: verified via ratatui's `TestBackend`

## Fixtures and Factories

**Test Data:**
```rust
// Task factory helper (tests/board_tests.rs)
fn create_test_task(title: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(title, "claude", "test-project");
    task.status = status;
    task
}

// Full Task struct construction for mock tests (src/tui/app_tests.rs)
let task = Task {
    id: "test-123".to_string(),
    title: "Test task".to_string(),
    description: None,
    status: TaskStatus::Running,
    agent: "claude".to_string(),
    project_id: "proj-1".to_string(),
    session_name: Some("test-session".to_string()),
    worktree_path: Some("/tmp/worktree".to_string()),
    branch_name: Some("feature/test".to_string()),
    pr_number: None,
    pr_url: None,
    plugin: None,
    cycle: 1,
    created_at: chrono::Utc::now(),
    updated_at: chrono::Utc::now(),
};

// Git repo setup helper (tests/git_tests.rs)
fn setup_git_repo() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    Command::new("git").current_dir(temp_dir.path()).args(["init"]).output().unwrap();
    Command::new("git").current_dir(temp_dir.path()).args(["config", "user.email", "test@test.com"]).output().unwrap();
    Command::new("git").current_dir(temp_dir.path()).args(["config", "user.name", "Test User"]).output().unwrap();
    std::fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
    Command::new("git").current_dir(temp_dir.path()).args(["add", "."]).output().unwrap();
    Command::new("git").current_dir(temp_dir.path()).args(["commit", "-m", "Initial commit"]).output().unwrap();
    Command::new("git").current_dir(temp_dir.path()).args(["branch", "-M", "main"]).output().unwrap();
    temp_dir
}
```

**Location:**
- Helpers are defined at the top of each test file that needs them
- No shared fixtures directory or test utilities module

## Coverage

**Requirements:** None enforced. No coverage threshold configured.

**View Coverage:**
```bash
# No built-in coverage command. Use cargo-tarpaulin or cargo-llvm-cov manually:
cargo install cargo-tarpaulin
cargo tarpaulin --features test-mocks
```

## Test Types

**Unit Tests (pure logic, no external dependencies):**
- `tests/db_tests.rs` - Task/Project model creation, status enum round-trips
- `tests/config_tests.rs` - Config defaults, merging, first-run action logic, serde round-trips
- `tests/board_tests.rs` - Board navigation (move left/right/up/down, clamping, selection)
- `tests/agent_tests.rs` - Agent selection parsing (number input validation)
- `tests/shell_popup_tests.rs` - Scroll math, line trimming, footer text, rendering with `TestBackend`
- `src/tui/app_tests.rs` (non-mock tests) - Fuzzy scoring algorithm

**Mock-based Unit Tests (require `--features test-mocks`):**
- `tests/mock_infrastructure_tests.rs` - Validates mock configuration patterns work correctly
- `src/tui/app_tests.rs` (mock tests) - PR description generation, PR creation workflow, session management, fuzzy file search, key forwarding to tmux

**Integration Tests (require real git):**
- `tests/git_tests.rs` - Creates real temporary git repos, creates/removes worktrees, tests branch detection, worktree initialization with file copying and init scripts

**Rendering Tests:**
- `tests/shell_popup_tests.rs` - Uses ratatui `TestBackend` to verify popup rendering:
```rust
let backend = TestBackend::new(80, 24);
let mut terminal = Terminal::new(backend).unwrap();
terminal.draw(|frame| {
    render_shell_popup(&popup, frame, area, lines, &colors);
}).unwrap();
let buffer = terminal.backend().buffer();
let buffer_content: String = buffer.content().iter().map(|c| c.symbol()).collect();
assert!(buffer_content.contains("Test Task"));
```

**E2E Tests:**
- Not used. No tmux-based or TUI interaction tests.

## Common Patterns

**Async Testing:**
- Not used. Despite `tokio` being a dependency, all tests are synchronous `#[test]` functions.
- The async runtime is only used in `main.rs` for the event loop.

**Error Testing:**
```rust
// Test that errors are returned (not panics)
#[test]
fn test_create_worktree_on_non_git_directory() {
    let temp_dir = TempDir::new().unwrap();
    let result = git::create_worktree(temp_dir.path(), "should-fail");
    assert!(result.is_err());
}

// Test error content
#[test]
#[cfg(feature = "test-mocks")]
fn test_create_pr_with_content_push_failure() {
    // ... setup mocks with error return ...
    mock_git.expect_push()
        .returning(|_, _, _| Err(anyhow::anyhow!("Permission denied")));

    let result = create_pr_with_content(/* ... */);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Permission denied"));
}

// Test graceful handling (no assertion on Ok/Err, just no panic)
#[test]
fn test_remove_worktree_nonexistent() {
    let temp_dir = setup_git_repo();
    let result = git::remove_worktree(temp_dir.path(), "does-not-exist");
    let _ = result; // Just verify no panic
}
```

**Testing with temporary directories:**
```rust
use tempfile::TempDir;

#[test]
fn test_something() {
    let temp_dir = TempDir::new().unwrap(); // Auto-cleaned on drop
    // ... use temp_dir.path() ...
}
```

**Feature-gated tests:**
```rust
#[test]
#[cfg(feature = "test-mocks")]
fn test_requiring_mocks() {
    let mut mock = MockGitOperations::new();
    // ...
}
```

## Test Count Summary

| File | Tests | Lines | Feature Gate |
|------|-------|-------|-------------|
| `tests/db_tests.rs` | 10 | 114 | None |
| `tests/config_tests.rs` | 17 | 293 | None |
| `tests/board_tests.rs` | 12 | 225 | None |
| `tests/git_tests.rs` | 18 | 473 | None |
| `tests/agent_tests.rs` | 6 | 42 | None |
| `tests/mock_infrastructure_tests.rs` | 6 | 190 | `test-mocks` |
| `tests/shell_popup_tests.rs` | 24 | 472 | None |
| `src/tui/app_tests.rs` | ~55 | 2792 | Most require `test-mocks` |

Total: ~148 tests across 8 files (~4,601 lines of test code)

## Untested Areas

- **TUI event loop and key dispatching** - `App::run()` and `App::handle_key()` are not tested; only extracted helper functions are
- **Database CRUD operations** - No tests for `Database::open_project()`, `create_task()`, `update_task()`, `get_task()` etc. (only model construction is tested)
- **Tmux real operations** - Only mock-verified; no integration tests with actual tmux
- **Agent detection** - `detect_available_agents()` depends on system state, not tested
- **Plugin loading from disk** - `WorkflowPlugin::load()` not tested
- **Skills deployment** - `write_skills_to_worktree()` (in `app.rs`) not tested
- **Config file I/O** - `GlobalConfig::load()`/`save()`, `ProjectConfig::load()`/`save()` not tested

---

*Testing analysis: 2026-03-03*
