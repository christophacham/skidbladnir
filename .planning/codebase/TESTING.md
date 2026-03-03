# Testing Patterns

**Analysis Date:** 2026-03-03

## Test Framework

**Runner:**
- Rust built-in test harness (`cargo test`)
- No external test runner; uses standard `#[test]` macro
- Test infrastructure located in `tests/` directory and inline via `#[path]` includes

**Assertion Library:**
- Standard Rust assertions: `assert!()`, `assert_eq!()`, `assert_ne!()`
- No external assertion library; all assertions are built-in

**Run Commands:**
```bash
cargo test                              # Run all tests
cargo test --features test-mocks        # Run tests with mock infrastructure
cargo test --lib                        # Run unit tests only (exclude integration tests)
cargo test --test '*'                   # Run integration tests only
```

**Feature flag:**
- `test-mocks` feature optional, gates mockall dependency and mock implementations
- When disabled, mock code is stripped via `#[cfg(feature = "test-mocks")]`

## Test File Organization

**Location:**
- **Unit tests:** Inline with modules, included via `#[path]` attribute
- **Integration tests:** Separate files in `tests/` directory
- Pattern: Tests are part of module or co-located in `tests/` but not both

**Naming:**
- Integration test files: `{module}_tests.rs` → `db_tests.rs`, `agent_tests.rs`, `board_tests.rs`
- Inline test modules: included file `app_tests.rs` via `#[path = "app_tests.rs"]` in `app.rs`
- All test functions prefixed with `test_`: `test_task_status_as_str()`, `test_board_state_new()`

**Test File Locations:**
```
src/
├── tui/
│   ├── app.rs
│   ├── app_tests.rs          # Included via #[path] in app.rs
│   ├── board.rs
│   └── ...
└── ...

tests/
├── db_tests.rs               # Integration tests for database
├── agent_tests.rs            # Integration tests for agent module
├── board_tests.rs            # Integration tests for board navigation
├── config_tests.rs           # Integration tests for configuration
├── git_tests.rs              # Integration tests for git operations
├── mock_infrastructure_tests.rs  # Tests for mock framework itself
└── shell_popup_tests.rs      # Integration tests for shell popup
```

## Test Structure

**Suite Organization:**

Unit test pattern from `tests/db_tests.rs`:
```rust
// === TaskStatus Tests ===

#[test]
fn test_task_status_as_str() {
    assert_eq!(TaskStatus::Backlog.as_str(), "backlog");
    assert_eq!(TaskStatus::Planning.as_str(), "planning");
    // ... more assertions
}

#[test]
fn test_task_status_roundtrip() {
    for status in TaskStatus::columns() {
        let s = status.as_str();
        let parsed = TaskStatus::from_str(s);
        assert_eq!(parsed, Some(*status));
    }
}

// === Task Tests ===

#[test]
fn test_task_new() {
    let task = Task::new("Test Task", "claude", "project-123");
    assert!(!task.id.is_empty());
    assert_eq!(task.title, "Test Task");
    // ... more assertions
}
```

**Patterns:**
- **Setup:** Create test data inline (no setup functions; data construction is straightforward)
- **Execution:** Call single function or method being tested
- **Assertion:** Multiple assertions per test to verify all aspects

**Minimal fixtures:**
Test helper from `tests/board_tests.rs`:
```rust
fn create_test_task(title: &str, status: TaskStatus) -> Task {
    let mut task = Task::new(title, "claude", "test-project");
    task.status = status;
    task
}
```

## Mocking

**Framework:** `mockall` crate (version 0.13)

**Mock generation:**
- Traits marked with `#[cfg_attr(feature = "test-mocks", automock)]`
- Generates `Mock{TraitName}` struct automatically
- Example:
  ```rust
  // src/git/operations.rs
  #[cfg_attr(feature = "test-mocks", automock)]
  pub trait GitOperations: Send + Sync {
    fn create_worktree(&self, project_path: &Path, task_slug: &str) -> Result<String>;
    fn remove_worktree(&self, project_path: &Path, worktree_path: &str) -> Result<()>;
    // ...
  }
  ```

**Mocking pattern from `tests/mock_infrastructure_tests.rs`:**
```rust
#[test]
fn test_git_operations_mock_for_pr_workflow() {
    let mut mock_git = MockGitOperations::new();

    // Setup expectations
    mock_git.expect_add_all()
        .times(1)
        .returning(|_| Ok(()));

    mock_git.expect_has_changes()
        .times(1)
        .returning(|_| true);

    mock_git.expect_commit()
        .times(1)
        .withf(|_: &Path, msg: &str| msg.contains("Test commit"))
        .returning(|_, _| Ok(()));

    // Execute
    let worktree = Path::new("/tmp/worktree");
    mock_git.add_all(worktree).unwrap();
    assert!(mock_git.has_changes(worktree));
    mock_git.commit(worktree, "Test commit message").unwrap();
}
```

**Call expectations:**
- `.times(N)` - Expect exactly N calls
- `.times(1)` - Most common for single operation tests
- `.withf(|args| condition)` - Validate argument conditions
- `.with(predicate::eq(value))` - Exact value match
- `.returning(|args| result)` - Return value or Result
- `.return_const(value)` - Return same value for all calls

**Predicate matching:**
```rust
mock_tmux.expect_create_window()
    .with(
        mockall::predicate::eq("my-project"),
        mockall::predicate::eq("/home/user/project"),
    )
    .times(1)
    .returning(|_, _, _, _| Ok(()));
```

**What to Mock:**
- I/O operations: git commands, tmux sessions, file system access
- External services: agent command execution, GitHub API calls
- Any operation that would be slow, have side effects, or depend on environment

**What NOT to Mock:**
- Pure business logic: `Task::generate_session_name()`, `TaskStatus::from_str()`, `BoardState::move_left()`
- Configuration parsing and validation
- String/type conversions
- Data model methods (only mock the trait boundaries)

**Test-only trait implementations:**
Mock implementations are separate from real implementations:
```rust
// Real in src/git/operations.rs
pub struct RealGitOps;
impl GitOperations for RealGitOps { ... }

// Mock auto-generated by #[automock]
// pub struct MockGitOperations { ... }
// impl GitOperations for MockGitOperations { ... }
```

## Fixtures and Factories

**Test Data Creation:**
- Minimal factory functions for reusable test objects
- Example from `tests/board_tests.rs`:
  ```rust
  fn create_test_task(title: &str, status: TaskStatus) -> Task {
      let mut task = Task::new(title, "claude", "test-project");
      task.status = status;
      task
  }
  ```

- Direct instantiation for simple cases:
  ```rust
  #[test]
  fn test_task_new() {
      let task = Task::new("Test Task", "claude", "project-123");
      // ... assertions
  }
  ```

**Location:**
- Test fixtures defined at top of test file or integrated inline
- No separate fixtures directory; data small and straightforward
- Mock configuration built inline in each test

**Reusable test setup from `tests/mock_infrastructure_tests.rs`:**
```rust
#[test]
fn test_mocks_can_be_arc_wrapped() {
    let mut mock_git = MockGitOperations::new();
    mock_git.expect_list_files()
        .returning(|_| vec!["file1.rs".to_string()]);

    let arc_git: Arc<dyn GitOperations> = Arc::new(mock_git);
    let arc_git_clone: Arc<dyn GitOperations> = Arc::clone(&arc_git);
    let files = arc_git_clone.list_files(Path::new("/tmp"));
    assert_eq!(files.len(), 1);
}
```

## Coverage

**Requirements:** No enforced coverage target

**Coverage goals (inferred from test count):**
- Core models: Comprehensive unit tests (TaskStatus, Task, Project, etc.)
- Business logic: High coverage for board navigation, configuration merging
- I/O boundaries: Tested via mocks to verify contract
- TUI/render logic: Limited (complex to test, primarily manual verification)

**View Coverage:**
```bash
# No built-in coverage command; would require external tool like tarpaulin
cargo tarpaulin --out Html  # If installed: cargo install cargo-tarpaulin
```

## Test Types

**Unit Tests:**
- **Scope:** Individual functions, enums, small structs
- **Approach:** Test one aspect per test function
- **Examples:**
  - `test_task_status_as_str()` - Enum method for all variants
  - `test_task_generate_session_name()` - String generation logic
  - `test_board_state_move_left()` - State navigation

**Integration Tests:**
- **Scope:** Module interactions, mock trait boundaries
- **Approach:** Test workflows across components
- **Examples from `tests/mock_infrastructure_tests.rs`:**
  - `test_git_operations_mock_for_pr_workflow()` - Multi-step git sequence
  - `test_tmux_session_management()` - Create → window → verify session exists
  - `test_agent_registry_mock()` - Registry lookup and agent method chaining

**Mock/Trait Tests:**
- **Purpose:** Verify mock infrastructure works before using in real tests
- **Scope:** Mock setup, expectation matching, Arc wrapping for thread safety
- **File:** `tests/mock_infrastructure_tests.rs` (required for test-mocks feature)

**App Logic Tests (with mocks):**
- **Location:** `src/tui/app_tests.rs` (included inline)
- **Pattern:** Test PR generation, error handling
- **Requires:** `#[cfg(feature = "test-mocks")]` guard
- **Example:**
  ```rust
  #[test]
  #[cfg(feature = "test-mocks")]
  fn test_generate_pr_description_with_diff_and_agent() {
      let mut mock_git = MockGitOperations::new();
      let mut mock_agent = MockAgentOperations::new();

      // Setup expectations
      mock_git.expect_diff_stat_from_main()
          .returning(|_| " src/main.rs | 10 +++++++---\n".to_string());
      mock_agent.expect_generate_text()
          .returning(|_, _| Ok("Description".to_string()));

      // Execute
      let (title, body) = generate_pr_description("Task", Some("/tmp"), None, &mock_git, &mock_agent);

      // Verify
      assert_eq!(title, "Task");
      assert!(body.contains("src/main.rs"));
  }
  ```

**No E2E Tests:**
- Application is TUI with tmux integration (difficult to test end-to-end)
- Workflows tested via mock components instead
- Manual testing and integration tests cover actual git/tmux behavior

## Common Patterns

**Async Testing:**
- Application uses `#[tokio::main]` in `main.rs`
- Integration tests are synchronous (no async test functions found)
- No `#[tokio::test]` usage; mock infrastructure doesn't require async

**Error Testing:**
Most tests verify success path. Error cases tested through:
- Option/Result assertions: `assert_eq!(parse_hex("#FFF"), None)`
- Expected failures in error handling paths
- Example from `config_tests.rs`:
  ```rust
  #[test]
  fn test_parse_hex_invalid() {
      assert_eq!(ThemeConfig::parse_hex("#FFF"), None);      // Too short
      assert_eq!(ThemeConfig::parse_hex("#GGGGGG"), None);   // Invalid hex
      assert_eq!(ThemeConfig::parse_hex(""), None);          // Empty
  }
  ```

**Roundtrip/Encoding Tests:**
Verify bidirectional conversions:
```rust
#[test]
fn test_task_status_roundtrip() {
    for status in TaskStatus::columns() {
        let s = status.as_str();
        let parsed = TaskStatus::from_str(s);
        assert_eq!(parsed, Some(*status));
    }
}
```

**State Transition Tests:**
Board navigation tests verify state changes:
```rust
#[test]
fn test_board_move_left_with_clamp() {
    let mut board = BoardState::new();
    // ... setup ...
    board.selected_column = 1;
    board.move_left();
    assert_eq!(board.selected_column, 0);
}
```

## Test Dependencies

**Dev Dependencies:**
- `tempfile` 3.x - For temporary file/directory creation in tests
- `mockall` 0.13 - For trait mocking (behind `test-mocks` feature)

**Running tests with full infrastructure:**
```bash
cargo test --features test-mocks
```

**Testing without mocks:**
```bash
cargo test
```
(Tests not requiring `test-mocks` feature still run)

---

*Testing analysis: 2026-03-03*
