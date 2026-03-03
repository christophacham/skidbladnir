# Codebase Concerns

**Analysis Date:** 2026-03-03

## Tech Debt

**God Object: `src/tui/app.rs` (5315 lines):**
- Issue: Single file contains all application logic -- state management, rendering, key handling, tmux orchestration, git operations, PR creation, plugin resolution, worktree setup, agent switching, ANSI parsing, file search, and skill deployment. It is the largest file by a factor of 10x over any other.
- Files: `src/tui/app.rs`
- Impact: Extremely difficult to navigate, modify, or test. Nearly all new features require touching this file. High risk of merge conflicts when multiple changes are in flight.
- Fix approach: Extract into focused modules. Candidates for extraction:
  - Worktree setup logic (`setup_task_worktree`, `cleanup_task_for_done`, `cleanup_task_resources`, `delete_task_resources`) into `src/tui/worktree_setup.rs`
  - PR creation flow (`generate_pr_description`, `create_pr_with_content`, `push_changes_to_existing_pr`) into `src/tui/pr.rs`
  - Agent orchestration (`send_skill_and_prompt`, `wait_for_agent_ready`, `switch_agent_in_tmux`, `wait_for_prompt_trigger`, `is_agent_active`) into `src/tui/agent_orchestration.rs`
  - Skill/plugin resolution (`resolve_skill_command`, `resolve_prompt`, `resolve_prompt_trigger`, `write_skills_to_worktree`) into `src/tui/plugin_resolution.rs`
  - ANSI parsing (`parse_ansi_to_lines`) into `src/tui/ansi.rs`
  - Key handlers (11 `handle_*_key` functions) could be grouped by feature area

**Duplicated Worktree Setup Code Across Three Entry Points:**
- Issue: The functions `start_research()` (line 3250), `move_task_right()` (line 2986), and `move_backlog_to_running()` (line 3360) all contain nearly identical patterns: clone a dozen fields, build `SetupResult`, spawn a thread, call `setup_task_worktree`, then `send_skill_and_prompt`. The thread body is copy-pasted each time.
- Files: `src/tui/app.rs` (lines ~2986, ~3250, ~3360)
- Impact: Any change to the setup flow must be replicated in three places. Easy to miss one, causing subtle inconsistencies.
- Fix approach: Extract a shared `spawn_task_setup()` method that takes a phase/status enum and handles the common pattern.

**Duplicated Artifact Archival Code:**
- Issue: `cleanup_task_for_done()` (line 3875) and `cleanup_task_resources()` (line 3917) contain nearly identical artifact archival logic (copy .md files from `.agtx/` to archive dir).
- Files: `src/tui/app.rs` (lines ~3882-3900, ~3927-3945)
- Impact: Bug fixes or changes to archival logic must be applied twice.
- Fix approach: Extract `archive_task_artifacts()` helper.

**Unused `thiserror` Dependency:**
- Issue: `thiserror = "2.0"` is declared in `Cargo.toml` but never imported or used anywhere in the codebase. All error handling uses `anyhow`.
- Files: `Cargo.toml` (line 31)
- Impact: Unnecessary compile-time dependency. Minor, but signals forgotten cleanup.
- Fix approach: Remove `thiserror = "2.0"` from `Cargo.toml`.

**Unnecessary `tokio` Full Runtime:**
- Issue: `tokio` is declared with `features = ["full"]` and `#[tokio::main]` is used, but the only `async` code is the `run()` event loop which uses `std::thread::spawn` for all background work and `mpsc::channel` (std, not tokio) for communication. No tokio async features (spawn, channels, IO, timers) are used.
- Files: `Cargo.toml` (line 18), `src/main.rs` (line 13), `src/tui/app.rs` (line 412)
- Impact: Adds ~200KB+ to binary size and compilation time for an unused runtime. The `async fn run()` and `.await` are vestigial.
- Fix approach: Remove tokio dependency entirely. Change `async fn main()` to `fn main()`, `async fn run()` to `fn run()`.

**Unused `serde_json` Dependency:**
- Issue: `serde_json = "1.0"` is declared in `Cargo.toml` but grep shows no usage of `serde_json::` anywhere in source code. All serialization uses TOML.
- Files: `Cargo.toml` (line 26)
- Impact: Unnecessary dependency.
- Fix approach: Remove from `Cargo.toml`.

**`AppState` Has 37+ Fields:**
- Issue: The `AppState` struct (lines 63-139) has grown to contain 37+ fields covering board state, input state, UI popups (7 different popup types), channels, caches, and configuration. Many fields are `Option<T>` representing mutually exclusive states.
- Files: `src/tui/app.rs` (lines 63-139)
- Impact: Hard to reason about valid state combinations. Impossible to enforce state machine invariants at the type level.
- Fix approach: Group related fields into sub-structs (e.g., `PopupState`, `BackgroundTaskState`, `CacheState`). Consider using an enum for mutually exclusive popup states.

## Known Bugs

**`DefaultHasher` for Database Path Hashing is Not Stable Across Rust Versions:**
- Symptoms: If a user upgrades their Rust toolchain and `DefaultHasher`'s algorithm changes, the `hash_path()` function in `src/db/schema.rs` will produce a different hash for the same project path, causing the app to create a new empty database instead of finding the existing one. All tasks appear lost.
- Files: `src/db/schema.rs` (lines 41-48)
- Trigger: Rust toolchain upgrade that changes `DefaultHasher` internals (documented as not guaranteed stable).
- Workaround: None. Data is not lost but becomes orphaned.

**PR Number Defaults to 0 on Parse Failure:**
- Symptoms: If the `gh pr create` output URL format changes, `pr_number` silently defaults to 0 instead of reporting an error. Subsequent operations referencing this PR number will fail or query the wrong PR.
- Files: `src/git/provider.rs` (lines 89-93)
- Trigger: Any `gh` CLI output format change where the PR number isn't the last path segment.
- Workaround: None currently.

## Security Considerations

**All Agents Run with Maximum Permissions:**
- Risk: Agents are spawned with flags that bypass all safety prompts: `--dangerously-skip-permissions` (Claude), `--full-auto` (Codex), `--approval-mode yolo` (Gemini), `--allow-all-tools` (Copilot). This grants agents unrestricted file system access, arbitrary command execution, and network access in the worktree.
- Files: `src/agent/mod.rs` (lines 41-57)
- Current mitigation: Git worktree isolation provides some blast radius containment. Agents operate in `.agtx/worktrees/{slug}` rather than the main project directory.
- Recommendations: This is an intentional design choice for autonomous operation, but consider: (1) documenting the security model explicitly for users, (2) offering a "supervised" mode that uses safer flags, (3) warning users on first run about the permissions model.

**Shell Command Injection via Task Titles/Descriptions:**
- Risk: Task titles are used to generate branch names, session names, and worktree paths. While `generate_task_slug()` sanitizes to alphanumeric/hyphen/underscore, task descriptions are passed directly to `build_interactive_command()` which embeds them in shell commands using single-quote escaping. A description containing `'` followed by shell metacharacters could potentially escape the quoting.
- Files: `src/agent/mod.rs` (line 51: `prompt.replace('\'', "'\"'\"'")`), `src/tui/app.rs` (various `build_interactive_command` calls)
- Current mitigation: The single-quote escaping technique (`'` -> `'"'"'`) is the standard POSIX approach and should handle most cases. However, for agents with skill support, the prompt is sent via `tmux send-keys` instead, bypassing shell escaping entirely.
- Recommendations: Audit the `send_keys` path to ensure tmux special characters in task content don't cause issues. Consider using `tmux send-keys -l` (literal mode) consistently.

**Plugin `init_script` Runs Arbitrary Shell Commands:**
- Risk: Plugin `init_script` fields execute arbitrary shell commands via `sh -c` in the worktree directory. A malicious plugin.toml could execute destructive commands.
- Files: `src/tui/app.rs` (lines 4028-4050), `src/git/worktree.rs` (lines 145-163)
- Current mitigation: Plugins are loaded from project-local `.agtx/plugins/`, global `~/.config/agtx/plugins/`, or bundled. Only bundled plugins are trusted.
- Recommendations: Document that installing third-party plugins is equivalent to running untrusted code. Consider sandboxing init scripts or adding a confirmation prompt for non-bundled plugins.

## Performance Bottlenecks

**100ms Polling Loop with Per-Task Tmux Subprocess Spawns:**
- Problem: The main event loop polls every 100ms (`event::poll(Duration::from_millis(100))`). During `refresh_sessions()`, for each task in Planning/Running/Review, it spawns multiple tmux subprocesses (`window_exists`, `capture_pane`, `pane_current_command`) to check status. With N active tasks, this is O(N) subprocess spawns every ~2 seconds (cache TTL).
- Files: `src/tui/app.rs` (line 479), `src/tui/app.rs` (lines 3693-3775 in `refresh_sessions`)
- Cause: Each tmux operation spawns a new `tmux -L agtx` process. No batching of tmux queries.
- Improvement path: Use a single `tmux list-windows -F` call to get all window info at once instead of per-task queries. Alternatively, use tmux control mode (`-C`) for persistent connection.

**Shell Popup Captures 500 Lines Every 100ms:**
- Problem: When the shell popup is open, `capture_tmux_pane_with_history` is called every 100ms loop iteration, spawning a subprocess to capture 500 lines of tmux history.
- Files: `src/tui/app.rs` (lines 488-490)
- Cause: No debouncing or diff-based updates for shell popup content.
- Improvement path: Only re-capture when the user scrolls or after detecting pane content changes (content hash comparison).

**Blocking `thread::sleep` Calls in Agent Orchestration:**
- Problem: `wait_for_agent_ready()` blocks a thread for up to 30 seconds with 200ms sleep intervals. `wait_for_prompt_trigger()` blocks for up to 5 minutes. `switch_agent_in_tmux()` contains multiple blocking waits totaling up to ~12 seconds.
- Files: `src/tui/app.rs` (lines 5120-5175, 4831-4869, 5027-5102)
- Cause: Background threads with blocking sleeps. While these don't block the UI (they run in spawned threads), they tie up OS threads unnecessarily.
- Improvement path: These are acceptable since they run in background threads. However, if thread count becomes a concern with many tasks, consider using async or a shared polling thread.

## Fragile Areas

**Agent Detection via Tmux Pane Content Scraping:**
- Files: `src/tui/app.rs` (lines 4971-5013: `is_agent_active`), `src/tui/app.rs` (lines 5110-5175: `wait_for_agent_ready`), `src/tui/app.rs` (lines 5027-5102: `switch_agent_in_tmux`)
- Why fragile: These functions rely on detecting specific strings in tmux pane output (e.g., `"Type your message"`, `"Yes, I accept"`, `">"`, `"%"`) to determine agent state. Any agent CLI update that changes prompt text, loading messages, or TUI layout will silently break detection. The `AGENT_READY_INDICATORS` and `AGENT_ACTIVE_INDICATORS` constants must be updated whenever agents change their UI.
- Safe modification: Add new indicators to the const arrays. Never remove existing ones (old agent versions may still be in use). Test manually against each agent CLI version.
- Test coverage: No automated tests. These functions depend on live tmux sessions and cannot be unit tested without significant mocking.

**`send_skill_and_prompt` Agent-Specific Branching:**
- Files: `src/tui/app.rs` (lines 4720-4814)
- Why fragile: This function has special-case behavior for `"gemini"` and `"codex"` (combine skill+prompt into single message), versus `"claude"` and `"opencode"` (send separately). Any new agent requires analyzing its CLI behavior and adding appropriate branching. The Ink TUI wait (lines 4750-4758) is a timing hack for Gemini/Codex rendering.
- Safe modification: When adding a new agent, test all code paths: with skill+prompt, with prompt only, with neither. Check the agent handles tmux `send-keys` correctly.
- Test coverage: No unit tests. Only covered by integration/manual testing.

**Database Migration Strategy:**
- Files: `src/db/schema.rs` (lines 94-99)
- Why fragile: Migrations use `ALTER TABLE ... ADD COLUMN` with errors silently ignored (`let _ =`). This works for additive migrations but provides no mechanism for column type changes, data transformations, or schema version tracking. If a future migration requires modifying existing columns, there is no infrastructure for it.
- Safe modification: Only add new `ALTER TABLE ... ADD COLUMN` lines. Never modify existing schema or column types. For complex migrations, would need to implement a version-tracked migration system.
- Test coverage: `tests/db_tests.rs` covers basic CRUD but does not test migration paths (e.g., opening a database created by an older version).

## Scaling Limits

**Single SQLite Connection Per Database:**
- Current capacity: One connection object per project database. All operations are synchronous on the main thread (except background PR creation which opens its own connection).
- Limit: SQLite itself handles concurrent reads well but writes are serialized. With background threads also opening connections (line 2004: `Database::open_project` in PR creation thread, line 3161), concurrent write conflicts could occur under heavy use.
- Scaling path: Use WAL mode (`PRAGMA journal_mode=WAL`) for better concurrent access. Consider connection pooling if multi-threaded access increases.

**No Pagination for Tasks:**
- Current capacity: All tasks for a project are loaded into memory (`get_all_tasks`, `get_tasks_by_status`).
- Limit: With hundreds of tasks per project, rendering and memory usage will degrade. The board navigation (BoardState) stores all tasks in-memory as `Vec<Vec<Task>>`.
- Scaling path: Implement task archival (partially exists for Done tasks) or pagination. Filter Done tasks from the default view.

## Dependencies at Risk

**`DefaultHasher` Stability:**
- Risk: `std::collections::hash_map::DefaultHasher` is explicitly documented as not guaranteed to produce the same output across Rust versions. It is used for database filename generation (`src/db/schema.rs` line 42) and idle detection content hashing (`src/tui/app.rs` line 3732).
- Impact: Database filename hashing could silently break on Rust toolchain upgrade, orphaning user data. Content hashing for idle detection is transient and less critical.
- Migration plan: Replace `DefaultHasher` in `hash_path()` with a deterministic hasher like `std::hash::SipHasher` (deprecated but stable) or a proper hash function from `sha2`/`blake3` crate. For the content hash (idle detection), `DefaultHasher` is fine since it only needs consistency within a single process run.

## Missing Critical Features

**No "Reopen" for Done Tasks:**
- Problem: Once a task is moved to Done, its worktree is deleted and tmux window killed. The branch is preserved locally, but there is no mechanism to recreate the worktree from the branch and resume work.
- Blocks: Users who need to revisit completed work must manually create worktrees and re-associate them.

**No Error Recovery for Failed Background Operations:**
- Problem: Background thread failures (worktree setup, PR creation, agent switching) are reported as warning messages that auto-clear after 5 seconds. There is no retry mechanism, no error log, and no way to inspect past failures.
- Blocks: Users may miss transient errors. Failed setups leave tasks in inconsistent states (e.g., status changed but no worktree created).

**No Concurrent Task Setup:**
- Problem: `setup_rx` is a single `Option<Receiver>`, meaning only one worktree setup can be in progress at a time. Attempting to start a second task while one is setting up is silently ignored (`if self.state.setup_rx.is_some() { return Ok(()) }`).
- Blocks: Users wanting to start multiple tasks rapidly must wait for each setup to complete.

## Test Coverage Gaps

**Zero Test Coverage for `src/tui/app.rs` Core Logic (5315 lines):**
- What's not tested: The actual `App::new()`, `App::run()`, `draw_board()`, `handle_normal_key()`, `move_task_right()`, `start_research()`, `move_backlog_to_running()` -- essentially all orchestration logic that ties together the subsystems.
- Files: `src/tui/app.rs`
- Risk: The largest and most complex file has its tests in `src/tui/app_tests.rs` (2792 lines, ~145 tests), but these test extracted pure functions and helpers (slug generation, plugin resolution, skill deployment, prompt rendering). The integration between subsystems (database + tmux + git + agent) during task lifecycle transitions is untested.
- Priority: High. This is where most bugs would occur -- in the orchestration logic that sequences background operations, state transitions, and error handling.

**No Tests for Agent Orchestration Functions:**
- What's not tested: `send_skill_and_prompt()`, `wait_for_agent_ready()`, `switch_agent_in_tmux()`, `wait_for_prompt_trigger()`, `is_agent_active()`
- Files: `src/tui/app.rs` (lines ~4720-5175)
- Risk: These functions contain the most fragile logic in the codebase (timing-dependent tmux pane scraping). Any agent CLI update could break them silently.
- Priority: High. These are the most likely to regress and the hardest to debug in production.

**No Integration Tests for Task Lifecycle:**
- What's not tested: Full task lifecycle (Backlog -> Planning -> Running -> Review -> Done) with mocked infrastructure. The mock infrastructure tests in `tests/mock_infrastructure_tests.rs` verify individual trait mock behavior but not the end-to-end flow.
- Files: `tests/mock_infrastructure_tests.rs`
- Risk: State machine bugs (wrong status transitions, missing cleanup, dangling tmux windows) would not be caught.
- Priority: Medium. The unit tests for individual components are good, but the integration is untested.

**No Tests for Dashboard Mode:**
- What's not tested: `handle_dashboard_key()`, `draw_dashboard()`, project switching, sidebar navigation.
- Files: `src/tui/app.rs` (lines 2231-2280, 1528-1660)
- Risk: Dashboard features could regress without notice.
- Priority: Low. Dashboard is a secondary feature.

---

*Concerns audit: 2026-03-03*
