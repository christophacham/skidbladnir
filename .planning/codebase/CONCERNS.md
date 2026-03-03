# Codebase Concerns

**Analysis Date:** 2025-03-03

## Tech Debt

**Large monolithic app.rs file:**
- Issue: The main TUI application logic is concentrated in a single 5,315-line file (`src/tui/app.rs`)
- Files: `src/tui/app.rs`
- Impact: Makes code difficult to navigate, test, and modify. High cognitive load for maintainers. Tight coupling between concerns.
- Fix approach: Refactor into smaller modules with clear responsibilities (e.g., separate files for task operations, phase transitions, PR operations, phase status polling)

**Silent error suppression in copy_back_to_project:**
- Issue: Copy-back failures during Working→Ready transition are only logged to stderr via `eprintln!()`, not propagated to UI
- Files: `src/tui/app.rs:3836-3857` (copy_back_to_project function)
- Impact: Users won't see copy-back errors in the TUI. Critical artifacts may silently fail to copy back to project root. Phase transition will still complete.
- Fix approach: Return `Result<Vec<String>>` with warnings, display in UI like worktree initialization warnings. Make failures observable.

**Database schema migrations use silent failures:**
- Issue: Lines 95-99 in `src/db/schema.rs` use `let _ = ...` to silently ignore migration failures
- Files: `src/db/schema.rs:95-99`
- Impact: If a column already exists or migration fails, no error is logged. Silent failures make debugging difficult. Missing columns cause runtime panics.
- Fix approach: Use SQLite's `CREATE ... IF NOT EXISTS` where applicable. For required columns, verify they exist after migrations and log actual errors.

**Unwrap_or_default() silently loses data on parse failures:**
- Issue: DateTime parsing in database reads (lines 220-226, 305-309) uses `unwrap_or_else(|_| chrono::Utc::now())` to silently replace invalid timestamps
- Files: `src/db/schema.rs:220-226, 305-309`
- Impact: Corrupted or invalid timestamps in database are replaced with current time, losing task metadata. No visibility into data corruption.
- Fix approach: Log parse failures with task context. Return error if timestamp parsing fails. Make corrupted data visible to user.

**Phase status cache uses global TTL for all phases:**
- Issue: 2-second cache TTL in `refresh_sessions()` is shared across all phase types
- Files: `src/tui/app.rs:3675` (CACHE_TTL constant)
- Impact: Research phase artifacts (expensive disk operations) refresh every 2 seconds. No differentiation for phases with different artifact check costs.
- Fix approach: Use phase-aware TTLs. Research: longer (5-10s). Planning/Running/Review: shorter (2s for responsiveness).

**No validation on plugin.toml artifact paths:**
- Issue: Plugin artifact paths can contain `{phase}` placeholders that are expanded at runtime
- Files: `src/config/mod.rs` (WorkflowPlugin loading), `src/tui/app.rs:4872` (phase_artifact_exists)
- Impact: If plugin defines invalid artifact paths, phase detection silently returns false. Tasks appear stuck. No user feedback on misconfigured plugins.
- Fix approach: Validate artifact path templates when plugin loads. Error on invalid placeholders. Test plugin on creation/selection.

**Task cycle counter increments without validation:**
- Issue: Cycle counter is incremented (line 3519) when transitioning Review→Planning, but phase templates use `{phase}` placeholder
- Files: `src/tui/app.rs:3518-3519`
- Impact: If plugin doesn't support `{phase}` placeholder, artifact detection will fail silently. Cyclic phase will appear broken.
- Fix approach: Validate that plugin supports cyclic mode on load. Check that all artifact templates use `{phase}` if cyclic=true.

## Known Bugs

**PR description generation can fail silently in background thread:**
- Symptoms: PR creation popup shows "Loading..." indefinitely, no error visible
- Files: `src/tui/app.rs:1878-1914` (PR description generation), lines 4234-4265 (git diff generation)
- Trigger: Run large git diff that fails, or agent command execution error in background thread
- Workaround: Thread errors are caught but only logged to stderr. User sees spinning load indicator forever. Can close popup with Esc.

**Copy-back race condition with project changes:**
- Symptoms: Files may be lost or corrupted if project root is modified during phase transition
- Files: `src/tui/app.rs:3749-3767` (copy-back on Working→Ready transition), `src/git/worktree.rs:73-167` (initialize_worktree)
- Trigger: User manually edits project files while agent is running (Review phase)
- Workaround: None. Recommeded: don't edit project root while task is in Review phase with copy_back configured.

**Session name collision possible with parallel tasks:**
- Symptoms: Multiple tasks with similar slugs could potentially collide in tmux session names
- Files: `src/tui/app.rs:3859-3871` (generate_task_slug), variable task creation
- Trigger: Create two tasks with identical titles in same project in quick succession
- Workaround: Slugs include task ID prefix (8 chars), collision unlikely but not guaranteed unique

**Pane content hash stability assumption:**
- Symptoms: "Idle" detection triggers incorrectly if agent pauses for legitimate reasons
- Files: `src/tui/app.rs:3726-3745` (idle detection logic)
- Trigger: Agent working on task but not writing to terminal for 15+ seconds (e.g., thinking, API calls, file operations)
- Workaround: None. Task will show "Idle" status but still be working. Phase detection still polls artifact files.

## Security Considerations

**Single-quote escaping in shell commands:**
- Risk: Potential shell injection if malicious task titles or descriptions are used in worktree operations
- Files: `src/agent/mod.rs:51-59` (prompt escaping), `src/tmux/mod.rs:29` (session name escaping)
- Current mitigation: Single quotes are escaped as `'\"'\"'` (close quote, escaped quote, open quote pattern)
- Recommendations: Add unit tests for shell escaping with edge cases. Consider using argv arrays instead of shell strings where possible. Document shell escaping assumptions.

**Environment variables not validated:**
- Risk: `$` expansions in task descriptions could access environment variables
- Files: `src/tui/app.rs` (task creation/editing), `src/agent/mod.rs` (command building)
- Current mitigation: None explicit. Task descriptions are passed through escaping for tmux.
- Recommendations: Document that task descriptions are user input, not trusted. Add examples of safe patterns. Consider warning users about env var substitution in agent CLIs.

**Database file permissions:**
- Risk: Project database files in `~/.config/agtx/projects/` inherit system default permissions (may be world-readable)
- Files: `src/db/schema.rs:14-38` (database file creation)
- Current mitigation: No explicit chmod. Relies on umask.
- Recommendations: Set explicit file permissions (0600) after opening database. Harden config directory permissions.

**GitHub credentials passed to gh CLI:**
- Risk: PR creation uses `gh` CLI which requires GitHub authentication
- Files: `src/git/provider.rs:64-97` (create_pr function)
- Current mitigation: Relies on gh CLI's built-in credential handling
- Recommendations: Document credential requirements. Add check for `gh auth status` before attempting PR ops. Handle auth failures gracefully.

**Prompt text can contain arbitrary agent directives:**
- Risk: Plugin prompts can include agent skill invocations or commands that might be unsafe
- Files: `src/config/mod.rs` (WorkflowPlugin with prompts), `src/tui/app.rs:4646-4710` (resolve_prompt/resolve_skill_command)
- Current mitigation: Only bundled plugins are shipped. Project plugins loaded from `.agtx/plugins/`.
- Recommendations: Document plugin security model. Warn that custom plugins run arbitrary prompts. Add plugin signature validation if sharing becomes widespread.

## Performance Bottlenecks

**Phase status polling on every 100ms tick:**
- Problem: refresh_sessions() runs every 100ms in main loop, checks artifact existence for all active tasks
- Files: `src/tui/app.rs:493` (called every poll), 3673-3775 (refresh_sessions implementation)
- Cause: No debouncing of expensive operations. Disk I/O for glob matching happens frequently.
- Improvement path: Increase base poll interval to 200-250ms. Implement per-task debouncing with individual timestamps. Cache glob results more aggressively.

**Research artifact glob matching performance:**
- Problem: glob_path_exists() reads entire directory for each poll cycle
- Files: `src/tui/app.rs:4911-4935` (glob_path_exists function), called from research_artifact_exists
- Cause: No caching of directory contents. std::fs::read_dir is called repeatedly for same paths.
- Improvement path: Cache directory listings with TTL. Use inotify/FSEvents for change detection instead of polling.

**Full task list loaded on every project switch:**
- Problem: Database queries load all tasks even if only display column subset
- Files: `src/tui/app.rs:3799` (Database::open_project), board initialization
- Cause: No pagination or lazy loading of task columns
- Improvement path: Implement cursor pagination. Load only displayed tasks initially. Load adjacent columns on-demand.

**Shell popup content capture with 500-line buffer:**
- Problem: Capturing 500 lines from tmux pane happens every 100ms when popup is open
- Files: `src/tui/app.rs:489` (shell_popup cached_content refresh)
- Cause: No content hashing or change detection before re-capturing
- Improvement path: Hash pane content before capture. Only update if hash differs. Reduce capture interval when content stable.

**Copy-back directory recursion without progress feedback:**
- Problem: Large directory copies block UI during Working→Ready transition
- Files: `src/tui/app.rs:3836-3857` (copy_back_to_project), `src/git/worktree.rs:170-183` (copy_dir_recursive)
- Cause: Synchronous copy happens on main thread
- Improvement path: Move copy_back to background thread. Show progress indicator. Allow cancellation.

## Fragile Areas

**Task status transition state machine:**
- Files: `src/tui/app.rs:2000-2550` (handle_board_key, all status transitions)
- Why fragile: Complex branching logic for different phase transitions. Cyclic phases add another dimension (Research→Planning→Running→Review→Planning cycle). Manual state validation at each transition point.
- Safe modification: Add comprehensive unit tests for all transition paths. Use explicit state machine enum. Document preconditions for each transition.
- Test coverage: No dedicated state machine tests. Transitions are tested indirectly through integration tests.

**Phase artifact detection logic:**
- Files: `src/tui/app.rs:4872-4907` (phase_artifact_exists, research_artifact_exists), plugin artifact configuration
- Why fragile: Relies on consistent plugin configuration and artifact path templates. Silent failures if artifacts don't match expectations. glob_path_exists has single-wildcard limitation.
- Safe modification: Validate artifact paths on plugin load. Add logging for artifact checks. Support multiple wildcard levels. Test with actual plugin files.
- Test coverage: Basic artifact existence checks tested, but not glob matching edge cases or plugin misconfiguration scenarios.

**Background thread communication with mpsc channels:**
- Files: `src/tui/app.rs:1849-2070` (multiple spawn_session calls), 1978-2050 (PR creation), 3150-3195 (setup thread)
- Why fragile: Multiple background threads (PR generation, PR creation, setup, diff) use different channel patterns. Manual channel ownership and error handling. No timeout on channel recv_timeout.
- Safe modification: Extract background operation handling into utility function. Add timeout wrapper. Document channel lifecycle. Add tests for thread failures.
- Test coverage: Mock infrastructure tests exist but don't cover all thread failure scenarios.

**Copy files from project to worktree (initialize_worktree):**
- Files: `src/git/worktree.rs:73-167` (initialize_worktree), returns Vec<String> warnings
- Why fragile: Collects errors into warnings vector instead of failing. Partial failures aren't clearly visible. Silent skips for missing files. Complex nested directory logic with multiple failure modes.
- Safe modification: Clearly separate fatal errors from warnings. Return detailed error info. Add summary of what was copied. Test with missing directories, permissions issues, symlinks.
- Test coverage: Basic file copy tested. Edge cases (symlinks, permissions, large files) not tested.

**Task cycle counter state in database:**
- Files: `src/db/schema.rs:99` (cycle column), `src/tui/app.rs:3518-3519` (increment on transition)
- Why fragile: Cycle counter must stay in sync with actual phase progression. If task is manually moved or phase detection fails, counter gets out of sync. No validation.
- Safe modification: Derive cycle count from task history instead of storing. Or validate cycle before using in artifact path. Add migration helper to validate/fix cycle values.
- Test coverage: Cycle tracking has basic tests but not desync scenarios.

## Scaling Limits

**Single SQLite database per project:**
- Current capacity: Tested with ~1000 tasks per project (untested limit)
- Limit: SQLite performance degrades with large datasets. No partitioning or archiving. Index strategy optimizes by status, not by created_at range.
- Scaling path: Implement task archiving (move old Done tasks to archive DB). Add date-range indices. Consider migration to PostgreSQL for projects with 10k+ tasks. Implement pagination.

**Tmux session namespace:**
- Current capacity: Session names format is `task-{id}--{project}--{slug}` (e.g., 120-char max)
- Limit: Very long project paths or task slugs could exceed tmux session name limits (255 chars on most systems)
- Scaling path: Hash long paths instead of including full path. Document session naming limits. Validate on task creation.

**Phase status cache size unbounded:**
- Current capacity: HashMap grows with number of tasks in current project
- Limit: No eviction policy. Old task entries stay in cache even after task is deleted. In projects with 1000+ tasks being created/deleted, cache grows indefinitely.
- Scaling path: Use LRU cache with max size. Evict entries for deleted tasks. Periodically clean cache on task list refresh.

**Copy-back file I/O with large artifacts:**
- Current capacity: Synchronous copy blocks UI. Untested with artifacts >100MB
- Limit: Large file copies (1GB+) will freeze TUI for 10+ seconds
- Scaling path: Async copy in background. Progress indicator. Limit total copy_back size per task. Implement resume logic.

## Dependencies at Risk

**tokio full features enabled:**
- Risk: Heavy dependency with all features enabled, adds to compile time and binary size
- Impact: Overkill for current usage (only used for #[tokio::main]). Could cause maintenance burden.
- Migration plan: Audit tokio usage. Likely only need tokio for blocking spawn_session calls. Could replace with std::thread.

**Bundled SQLite via rusqlite:**
- Risk: SQLite bundled feature bypasses system SQLite. Binary size increases (~5MB). Missing security patches until recompilation.
- Impact: Can't benefit from system SQLite updates. Upgrade requires recompile.
- Migration plan: Switch to system SQLite by removing `bundled` feature. Requires SQLite dev package installed. Trade: smaller binary, easier security updates.

**No async runtime needed:**
- Risk: tokio pulled in but app is fundamentally synchronous. Main loop uses std::thread::spawn, not async/await.
- Impact: Unused dependency. Adds 10+ crates transitively.
- Migration plan: Remove tokio. Use std::thread for all background work. Keep #[tokio::main] for compatibility if needed, or switch to plain fn main().

## Missing Critical Features

**No task priority or ordering:**
- Problem: Tasks displayed only by creation order. No way to prioritize urgent work.
- Blocks: Triaging large backlogs becomes difficult. Can't surface high-impact tasks first.
- Improvement: Add priority field (High/Medium/Low). Sort columns by priority then creation date. Allow drag-reorder in backlog.

**No task search/filter beyond title:**
- Problem: Only `/` command allows text search on title. No filtering by agent, status, date, description.
- Blocks: Finding specific tasks in large backlogs requires scrolling through all tasks.
- Improvement: Add filter panel. Support queries like `agent:claude status:running`. Index description text.

**No task relationships or dependencies:**
- Problem: Can't express "task B depends on task A". Can't bulk-operate on related tasks.
- Blocks: Complex features that span multiple tasks can't be tracked as related units.
- Improvement: Add task linking. Parent/child relationships. Block detection (can't run task until dependency done).

**No bulk operations:**
- Problem: Can only modify/delete one task at a time.
- Blocks: Closing many tasks after sprint, archiving old tasks, agent/plugin bulk changes.
- Improvement: Add multi-select mode. Bulk move, delete, or update operations.

**No task history or audit trail:**
- Problem: Can't see who/when a task status changed. No undo for accidental deletions.
- Blocks: Can't audit work done. Can't recover accidentally deleted tasks.
- Improvement: Track all state changes. Keep soft-delete option. Show task timeline.

## Test Coverage Gaps

**app.rs key event handlers:**
- What's not tested: Complex keyboard shortcut logic, popup interactions, tile navigation, multi-select scenarios
- Files: `src/tui/app.rs:2400-2950` (all handle_*_key functions)
- Risk: Regressions in UI interactions won't be caught. Branch coverage likely <30%.
- Priority: **High** - Most user interactions go through these code paths

**PR creation workflow (background thread interactions):**
- What's not tested: PR description generation timeout, PR creation failure recovery, channel communication errors
- Files: `src/tui/app.rs:1878-2050` (PR background threads), `src/git/provider.rs:64-97`
- Risk: PR feature silently fails. Errors not visible to user. Can't test without mocking gh CLI.
- Priority: **High** - Feature is critical path but untested

**Database migration/schema evolution:**
- What's not tested: Schema changes on existing databases, column existence validation, migration rollback
- Files: `src/db/schema.rs:69-102` (migration code)
- Risk: Silent schema failures. Incompatibility with old database versions not detected.
- Priority: **Medium** - Affects long-term data integrity

**Worktree initialization with various copy_files configurations:**
- What's not tested: Missing source files, nested paths, symlinks, permission errors
- Files: `src/git/worktree.rs:73-167` (initialize_worktree returns Vec<String> warnings)
- Risk: Copy failures silently added to warnings list. Users may miss important setup failures.
- Priority: **Medium** - Affects reproducibility of task environments

**Plugin loading and artifact path resolution:**
- What's not tested: Invalid plugin TOML, missing artifact fields, {phase} placeholder substitution, glob matching edge cases
- Files: `src/config/mod.rs:WorkflowPlugin`, `src/tui/app.rs:4872-4935` (artifact existence checks)
- Risk: Misconfigured plugins cause phase detection to fail. Cyclic phases silently break if artifact templates wrong.
- Priority: **Medium** - Affects plugin ecosystem reliability

**Phase status polling with cache TTL edge cases:**
- What's not tested: Rapid task transitions, artifact appearing/disappearing during poll cycle, cache invalidation on task update
- Files: `src/tui/app.rs:3673-3775` (refresh_sessions with cache logic)
- Risk: Status can appear stale or ahead of reality. Copy-back timing issues.
- Priority: **Low** - Hard to reproduce, affects edge cases

---

*Concerns audit: 2025-03-03*
