# Phase 6: Workflow Engine - Research

**Researched:** 2026-03-04
**Domain:** Full-stack workflow orchestration (Rust daemon + Svelte 5 frontend), git worktree management, agent PTY lifecycle, syntax-highlighted diff viewing, PR creation flow
**Confidence:** HIGH

## Summary

Phase 6 ports the TUI's complete workflow semantics to the web interface. The TUI's `move_task_right()` function (app.rs line 3211) is the authoritative reference implementation -- it handles five distinct phase transitions, each with unique side effects (worktree creation, agent spawning, skill deployment, command/prompt sending, PR workflow). The daemon must orchestrate these same side effects using its existing PTY session manager rather than tmux, while the frontend adds advance buttons, a tabbed detail panel (Output | Diff | PR), and a PR creation modal.

The critical architectural insight is that the daemon currently has no concept of "workflow" -- it has raw session/task CRUD. This phase introduces a workflow service layer between the API endpoints and the session manager that coordinates multi-step transitions (create worktree, deploy skills, spawn agent PTY, wait for readiness, send commands). The `Task` model also needs a `session_id` column to bridge daemon PTY sessions to task records (currently missing from both the Rust model and DB schema, though the frontend TypeScript type already has it).

**Primary recommendation:** Build a `WorkflowService` in the daemon that encapsulates all transition logic, reusing existing agtx-core traits (`GitOperations`, `AgentOperations`, `WorkflowPlugin::load()`) and the daemon's `SessionManager`. Expose it through a single `POST /api/v1/tasks/{id}/advance` endpoint. Use Shiki for syntax-highlighted diff rendering on the frontend.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Advance buttons in both task cards (forward-arrow icon) and detail panel header (prominent button) -- multiple entry points like TUI's 'm' key
- Backlog to Planning: card moves to Planning column immediately with a "setting up..." spinner overlay while worktree creation, skill deployment, and agent spawn run in the background
- Running to Review: task moves to Review immediately -- no forced PR creation prompt. PR creation available as an optional action in the detail panel when ready
- Review to Done: check PR status (merged/open/closed) via `gh pr view` and warn if PR is still open. User can still proceed -- warning, not blocker. Matches TUI behavior
- Review to Planning (cyclic): available when plugin.cyclic is true, increments phase counter
- AI-first PR creation with edit: PR modal opens with AI-generated title and body already populated from the diff. User reviews and edits before creating. If AI generation fails, fields are empty for manual entry
- PR action available via both the detail panel toolbar and the command palette (Ctrl+K "Create PR for [task]")
- Base branch dropdown in modal for selecting target branch
- After PR is created, detail panel header shows clickable PR URL link + status badge (Open/Merged/Closed) that updates on refresh
- Unified inline diff (GitHub-style) with + (green) and - (red) line coloring
- Full language-aware syntax highlighting in diff hunks
- Displayed in the detail panel as a tab -- panel becomes tabbed view: Output | Diff | PR
- Diff tab shows diff relative to the base/main branch for the task's worktree
- Project-level default plugin settable via project settings or command palette
- Per-task plugin override available in the task creation modal
- Read-only plugin listing -- no web-based plugin editing. Plugin files managed on disk via TOML
- Available plugins discovered from bundled + global + project-local directories

### Claude's Discretion
- Daemon-side workflow service architecture (new endpoints for transition orchestration, how side effects are executed)
- Artifact detection mechanism (daemon-side file polling, WebSocket push, or REST polling from frontend)
- How agent readiness detection works in the daemon PTY model (replacing TUI's tmux pane content polling)
- How `send_skill_and_prompt()` sequencing works via daemon PTY write (replacing tmux send-keys + prompt trigger polling)
- Syntax highlighting library choice for diffs
- Tabbed detail panel implementation details (component structure, routing between tabs)
- How `session_id` bridges daemon PTY sessions to task records (DB migration, linking strategy)
- Background setup result delivery (WebSocket state messages, REST polling, or server-sent events)
- PR description AI generation approach (which agent's `generate_text()` to use, fallback behavior)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FLOW-01 | Phase transitions trigger side effects (worktree creation, agent spawn, skill deployment) | WorkflowService architecture with TUI reference implementation analysis; daemon session_id bridge; GitOperations/AgentOperations reuse |
| FLOW-02 | Plugins resolve via project-local -> global -> bundled precedence | WorkflowPlugin::load() already in agtx-core handles project-local/global; bundled via skills::load_bundled_plugin(); daemon needs project_path in AppState |
| FLOW-03 | Skills deploy to agent-native paths in worktrees per agent type | write_skills_to_worktree() reference impl with all 5 agent formats; move to agtx-core or call from daemon |
| FLOW-04 | Commands and prompts resolve per agent type with format translation | resolve_skill_command() + resolve_prompt() + transform_plugin_command() in skills.rs; pure functions, easily reusable |
| FLOW-05 | Artifact files detected via polling with glob support | phase_artifact_exists() + glob_path_exists() reference impl; daemon-side tokio::fs polling with WebSocket push |
| FLOW-06 | Cyclic phases supported (Review -> Planning with incrementing phase counter) | plugin.cyclic flag check + task.cycle increment; advance endpoint handles this as special transition |
| FLOW-07 | User can create GitHub PRs from browser (title, description, base branch) | RealGitHubOps::create_pr() in agtx-core; AgentOperations::generate_text() for AI description; daemon endpoints + frontend PR modal |
| FLOW-08 | User can view syntax-highlighted git diffs for task worktrees | Shiki v3 for frontend highlighting; daemon endpoint returns raw unified diff via git CLI; collect_task_diff() reference |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| shiki | ^3.23.0 | Syntax-highlighted diff rendering in browser | VSCode-quality grammar-based highlighting, ESM-native, lazy-loaded languages, supports diff notation and line decorations |
| @shikijs/transformers | ^3.23.0 | Line-level diff decorations (add/remove classes) | Official Shiki package for notation-based line highlighting |
| axum (existing) | workspace | Daemon API endpoints for workflow operations | Already in use, add new routes to existing router |
| agtx-core (existing) | workspace | Shared workflow logic (plugin loading, git ops, agent ops, skills) | Contains all traits and implementations already used by TUI |
| glob (Rust) | ^0.3 | Artifact file detection with wildcard patterns | Standard Rust globbing crate, replaces hand-rolled glob_path_exists |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tokio::fs | (in tokio) | Async file operations for artifact polling | Daemon-side artifact detection without blocking |
| serde_json (existing) | workspace | JSON serialization for new API responses | Already in use |
| fuse.js (existing) | ^7.0.0 | Fuzzy search in command palette for workflow actions | Already in use |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Shiki | diff2html | diff2html is a full diff viewer widget (harder to customize styling) vs Shiki which gives raw highlighted HTML (full CSS control matching existing theme) |
| Shiki | highlight.js | highlight.js has no built-in diff line decoration support; Shiki's transformer API handles it natively |
| Rust glob crate | Hand-rolled glob | TUI has hand-rolled glob_path_exists; the Rust `glob` crate handles edge cases better but the existing impl is only ~25 lines and works |

**Installation:**
```bash
# Frontend
cd web && npm install shiki @shikijs/transformers

# Daemon -- no new Rust crates needed (glob is optional)
```

## Architecture Patterns

### Recommended Project Structure
```
crates/agtxd/src/
├── api/
│   ├── mod.rs           # Add workflow routes
│   ├── workflow.rs      # NEW: advance, diff, plugins, pr endpoints
│   └── ...existing...
├── workflow/
│   ├── mod.rs           # NEW: WorkflowService
│   ├── transitions.rs   # NEW: Per-transition side effect logic
│   └── artifacts.rs     # NEW: Artifact polling task
├── ...existing...

crates/agtx-core/src/
├── workflow/
│   ├── mod.rs           # NEW: Shared workflow functions (extracted from TUI)
│   ├── skills.rs        # Move write_skills_to_worktree here
│   ├── commands.rs      # Move resolve_skill_command, resolve_prompt here
│   └── setup.rs         # Move setup_task_worktree logic here
├── ...existing...

web/src/lib/
├── api/
│   └── workflow.ts      # NEW: advanceTask, getDiff, getPlugins, createPr
├── components/
│   ├── DetailPanel.svelte    # EXTEND: tabbed view (Output | Diff | PR)
│   ├── DiffView.svelte       # NEW: syntax-highlighted diff viewer
│   ├── PrTab.svelte           # NEW: PR creation/status view
│   ├── PrModal.svelte         # NEW: PR creation modal
│   ├── TabBar.svelte          # NEW: tab navigation component
│   ├── PluginSelect.svelte    # NEW: plugin dropdown for task creation
│   └── TaskCard.svelte        # EXTEND: advance button
├── stores/
│   └── tasks.svelte.ts        # EXTEND: advance(), createPr() methods
```

### Pattern 1: WorkflowService on Daemon
**What:** A service struct held in `AppState` that encapsulates all workflow transition logic, coordinating between `SessionManager`, `GitOperations`, `AgentOperations`, and `WorkflowPlugin`.
**When to use:** Every `POST /api/v1/tasks/{id}/advance` call.
**Example:**
```rust
// crates/agtxd/src/workflow/mod.rs
pub struct WorkflowService {
    session_manager: Arc<SessionManager>,
    git_ops: Arc<dyn GitOperations>,
    agent_registry: Arc<dyn AgentRegistry>,
}

impl WorkflowService {
    pub async fn advance_task(
        &self,
        task: &mut Task,
        project: &Project,
    ) -> Result<AdvanceResult> {
        let current = task.status;
        let next = next_status(current)
            .ok_or_else(|| anyhow::anyhow!("Cannot advance from {:?}", current))?;

        match (current, next) {
            (TaskStatus::Backlog, TaskStatus::Planning) => {
                self.backlog_to_planning(task, project).await
            }
            (TaskStatus::Planning, TaskStatus::Running) => {
                self.planning_to_running(task, project).await
            }
            (TaskStatus::Running, TaskStatus::Review) => {
                self.running_to_review(task).await
            }
            (TaskStatus::Review, TaskStatus::Done) => {
                self.review_to_done(task, project).await
            }
            _ => Err(anyhow::anyhow!("Invalid transition")),
        }
    }
}
```

### Pattern 2: Immediate Status Update + Background Setup
**What:** The API endpoint immediately updates the task status in the DB and returns success, then spawns an async task for the heavy side effects (worktree creation, agent spawn). Progress is pushed via WebSocket state messages.
**When to use:** Backlog -> Planning transition (takes seconds due to worktree + agent spawn).
**Example:**
```rust
// Advance endpoint handler
async fn advance_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Task>, AppError> {
    let mut task = /* load task from DB */;

    // Immediate: update status
    task.status = TaskStatus::Planning;
    /* save to DB */

    // Background: heavy setup
    let workflow = state.workflow.clone();
    tokio::spawn(async move {
        match workflow.setup_planning(&mut task, &project).await {
            Ok(session_id) => {
                // Update task.session_id in DB
                // Push state update via WebSocket
            }
            Err(e) => {
                // Push error via WebSocket
                // Optionally revert status
            }
        }
    });

    Ok(Json(task)) // Return immediately with new status
}
```

### Pattern 3: PTY Command Sequencing (Replacing tmux send_keys)
**What:** In the daemon PTY model, sending commands to agents uses `session_manager.write()` instead of tmux send_keys. The key difference is that there's no pane capture polling -- instead, use output broadcast channel to watch for readiness patterns.
**When to use:** After spawning an agent, before sending skill commands + prompts.
**Example:**
```rust
// Wait for agent ready by watching output stream
async fn wait_for_agent_ready(
    session_manager: &SessionManager,
    session_id: Uuid,
    timeout: Duration,
) -> Result<()> {
    let (mut rx, _, _) = session_manager.subscribe(session_id).await
        .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

    let deadline = Instant::now() + timeout;
    let mut decoder = String::new();

    while Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Ok(OutputEvent::Data { bytes, .. })) => {
                decoder.push_str(&String::from_utf8_lossy(&bytes));
                // Check for agent ready indicators (prompt character, etc.)
                if agent_appears_ready(&decoder) {
                    return Ok(());
                }
            }
            Ok(Ok(OutputEvent::StateChange(SessionState::Exited(_)))) => {
                return Err(anyhow::anyhow!("Agent exited during setup"));
            }
            _ => continue,
        }
    }
    // Timeout is not necessarily an error -- agent may already be ready
    Ok(())
}

// Then send command + prompt via PTY write
async fn send_skill_and_prompt_pty(
    session_manager: &SessionManager,
    session_id: Uuid,
    skill_cmd: &Option<String>,
    prompt: &str,
    agent_name: &str,
) -> Result<()> {
    // For agents that need combined command+prompt (gemini, codex)
    if matches!(agent_name, "gemini" | "codex") {
        let combined = match skill_cmd {
            Some(cmd) if !prompt.is_empty() => format!("{}\n\n{}", cmd, prompt),
            Some(cmd) => cmd.clone(),
            None => prompt.to_string(),
        };
        session_manager.write(session_id, &combined).await?;
    } else {
        // Send skill command first, then prompt
        if let Some(cmd) = skill_cmd {
            session_manager.write(session_id, cmd).await?;
            // Brief delay for agent to process
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        if !prompt.is_empty() {
            session_manager.write(session_id, prompt).await?;
        }
    }
    Ok(())
}
```

### Pattern 4: Frontend Diff Rendering with Shiki
**What:** Fetch raw unified diff from daemon, parse into hunks on frontend, render each hunk with Shiki syntax highlighting using file extension for language detection.
**When to use:** When user clicks the Diff tab in the detail panel.
**Example:**
```typescript
// web/src/lib/components/DiffView.svelte
import { codeToHtml } from 'shiki';

async function highlightDiff(rawDiff: string): Promise<string> {
    // Parse unified diff into file sections
    const files = parseDiffFiles(rawDiff);
    let html = '';

    for (const file of files) {
        const lang = detectLanguage(file.path); // .rs -> rust, .ts -> typescript

        // Render each hunk with line-level add/remove classes
        for (const hunk of file.hunks) {
            const highlighted = await codeToHtml(hunk.content, {
                lang,
                theme: 'github-dark',
                decorations: hunk.lines.map((line, i) => ({
                    start: { line: i, character: 0 },
                    end: { line: i, character: line.length },
                    properties: {
                        class: line.startsWith('+') ? 'diff-add'
                             : line.startsWith('-') ? 'diff-remove'
                             : ''
                    }
                }))
            });
            html += highlighted;
        }
    }
    return html;
}
```

### Anti-Patterns to Avoid
- **Moving workflow logic into API handlers:** Keep endpoint handlers thin -- delegate all transition logic to WorkflowService. The TUI's `move_task_right()` is 400+ lines because it mixes UI logic with workflow orchestration.
- **Synchronous worktree setup:** Never block the API response waiting for git worktree creation + agent spawn. Always return immediately with updated status, push results via WebSocket.
- **Polling artifact files from the frontend:** Don't have the browser poll `GET /api/v1/tasks/{id}/artifacts` every second. Have the daemon poll the filesystem and push results via WebSocket state messages.
- **Duplicating TUI workflow logic:** Extract `resolve_skill_command()`, `resolve_prompt()`, `write_skills_to_worktree()`, and `phase_artifact_exists()` into agtx-core rather than reimplementing in the daemon.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Syntax highlighting | Custom regex-based highlighter | Shiki v3 | 200+ language grammars, VSCode-quality, handles edge cases in string interpolation, nested languages |
| Unified diff parsing | Custom diff parser | Simple line-by-line parser (prefix-based) | Unified diff format is simple enough (lines start with +, -, or space) that a 30-line parser suffices -- full libraries like diff2html are overkill |
| Plugin resolution | Custom file search | `WorkflowPlugin::load()` + `skills::load_bundled_plugin()` | Already implemented in agtx-core with correct precedence chain |
| Git operations | Raw `std::process::Command` in daemon | `RealGitOps` from agtx-core | Already has worktree, diff, push, branch operations behind a mockable trait |
| PR creation | Custom GitHub API client | `RealGitHubOps::create_pr()` via `gh` CLI | Already in agtx-core, handles URL parsing and error cases |
| Agent command building | Agent-specific command strings | `Agent::build_interactive_command()` | Already in agtx-core with per-agent flags (--dangerously-skip-permissions, --full-auto, etc.) |
| Command translation | Per-agent command formatting | `skills::transform_plugin_command()` | Already handles Claude/Gemini/OpenCode/Codex format differences |

**Key insight:** The TUI already has battle-tested implementations of every workflow operation. The job is extraction (move to agtx-core) and async adaptation (std::thread -> tokio::spawn), not reimplementation.

## Common Pitfalls

### Pitfall 1: session_id Column Missing from Rust Task Model
**What goes wrong:** The frontend TypeScript `Task` type has `session_id: string | null` but the Rust `Task` struct and SQLite schema have no such column. Attempting to serialize/deserialize tasks across the API boundary will either silently drop the field or fail.
**Why it happens:** The TypeScript type was forward-declared in Phase 4 anticipating this phase.
**How to avoid:** Add `session_id: Option<String>` to the Rust `Task` struct in `crates/agtx-core/src/db/models.rs`. Add migration in `schema.rs` with `ALTER TABLE tasks ADD COLUMN session_id TEXT`. Update `task_from_row` and `update_task` in schema.rs.
**Warning signs:** Tasks appear without session_id in API responses; WebSocket connections fail because session_id is always null.

### Pitfall 2: Blocking the Tokio Runtime with Synchronous Git Operations
**What goes wrong:** `RealGitOps` methods use `std::process::Command` which blocks the current thread. Calling these directly from async handlers can starve the tokio thread pool.
**Why it happens:** agtx-core git operations were designed for the synchronous TUI event loop.
**How to avoid:** Wrap all git operations in `tokio::task::spawn_blocking()`, exactly as the existing task CRUD handlers do with `Database` calls.
**Warning signs:** Daemon becomes unresponsive during worktree creation; WebSocket messages stop flowing.

### Pitfall 3: Race Condition Between Immediate Status Update and Background Setup
**What goes wrong:** Frontend shows task in Planning column, user clicks it expecting a session, but the background setup hasn't finished yet -- no session_id, no output.
**Why it happens:** Advance endpoint returns immediately with new status before worktree/session are ready.
**How to avoid:** Use a "setup_status" field or WebSocket state message to communicate setup progress. Frontend shows "Setting up..." overlay until session_id is populated. Poll or watch for the task update.
**Warning signs:** Clicking a just-advanced task shows "No active session" instead of output.

### Pitfall 4: Agent Readiness Detection Without tmux capture_pane
**What goes wrong:** The TUI uses `tmux capture_pane` + `is_agent_active()` to detect when an agent is ready to receive commands. The daemon has no tmux -- it has a PTY output stream.
**Why it happens:** Fundamentally different process management model.
**How to avoid:** Subscribe to the session's broadcast channel and watch for agent-specific ready indicators (e.g., Claude shows its prompt after startup). Use a timeout-based approach -- wait up to N seconds, then send commands regardless (most agents are ready within 2-3 seconds).
**Warning signs:** Skill commands sent before agent is ready get swallowed; agent starts but receives no instructions.

### Pitfall 5: Shiki Bundle Size Explosion
**What goes wrong:** Importing all Shiki languages and themes bundles 6.9 MB of grammars, making the web app slow to load.
**Why it happens:** Default Shiki import includes everything.
**How to avoid:** Use `createHighlighter()` with explicit language list. Only load languages that appear in the diff (detect from file extensions). Lazy-load languages on demand. Start with: rust, typescript, javascript, python, svelte, toml, yaml, json, markdown, css, html, bash.
**Warning signs:** Initial page load takes 5+ seconds; bundle size exceeds 2 MB.

### Pitfall 6: Daemon Does Not Know Project Path
**What goes wrong:** The daemon's `AppState` has `db_path` and `global_db_path` but no `project_path`. Workflow operations need the project path for git worktree creation, plugin resolution, and skill deployment.
**Why it happens:** The daemon was designed as a generic session manager, not a workflow engine. The TUI gets its project path from `std::env::current_dir()`.
**How to avoid:** The project path can be resolved from the `Project` record in the database (it has a `path` field). The advance endpoint should load the project by `task.project_id`, get the project path, and pass it to the WorkflowService.
**Warning signs:** Worktree creation fails with "not a git repository"; plugin resolution always falls through to bundled.

## Code Examples

### Advance Task API Endpoint
```rust
// crates/agtxd/src/api/workflow.rs
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AdvanceRequest {
    /// For cyclic: "next" (default) or "cycle" (Review -> Planning)
    #[serde(default = "default_direction")]
    pub direction: String,
}

fn default_direction() -> String { "next".to_string() }

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/{id}/advance", post(advance_task))
        .route("/{id}/diff", axum::routing::get(get_task_diff))
        .route("/{id}/pr", post(create_pr))
        .route("/{id}/pr/status", axum::routing::get(get_pr_status))
}

async fn advance_task(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<AdvanceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 1. Load task from DB
    // 2. Load project (for project_path)
    // 3. Call workflow_service.advance_task()
    // 4. Return updated task
    todo!()
}
```

### Get Task Diff Endpoint
```rust
// Returns raw unified diff for the task's worktree vs main branch
async fn get_task_diff(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DiffResponse>, AppError> {
    let db_path = state.db_path.clone();

    let diff = tokio::task::spawn_blocking(move || {
        let db = Database::open_at(&db_path)?;
        let task = db.get_task(&id)?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        let worktree = task.worktree_path
            .ok_or_else(|| anyhow::anyhow!("No worktree for task"))?;

        let git_ops = RealGitOps;
        let wt = std::path::Path::new(&worktree);

        // Get diff from main branch (all changes)
        let diff = std::process::Command::new("git")
            .current_dir(wt)
            .args(["diff", "main", "--unified=3"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        Ok::<String, anyhow::Error>(diff)
    }).await
    .map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?
    .map_err(AppError::from)?;

    Ok(Json(DiffResponse { diff }))
}
```

### Frontend Tabbed Detail Panel
```svelte
<!-- web/src/lib/components/DetailPanel.svelte (extended) -->
<script lang="ts">
    type Tab = 'output' | 'diff' | 'pr';
    let activeTab = $state<Tab>('output');

    const showDiffTab = $derived(
        task && task.status !== 'Backlog' && task.worktree_path
    );
    const showPrTab = $derived(
        task && (task.status === 'Review' || task.pr_number)
    );
</script>

<!-- Tab bar -->
{#if task?.session_id || showDiffTab || showPrTab}
    <div class="flex border-b" style="border-color: var(--color-border);">
        <button class:active={activeTab === 'output'} onclick={() => activeTab = 'output'}>
            Output
        </button>
        {#if showDiffTab}
            <button class:active={activeTab === 'diff'} onclick={() => activeTab = 'diff'}>
                Diff
            </button>
        {/if}
        {#if showPrTab}
            <button class:active={activeTab === 'pr'} onclick={() => activeTab = 'pr'}>
                PR
            </button>
        {/if}
    </div>
{/if}

<!-- Tab content -->
{#if activeTab === 'output'}
    <OutputView />
    <InputBar />
{:else if activeTab === 'diff'}
    <DiffView taskId={task.id} />
{:else if activeTab === 'pr'}
    <PrTab task={task} />
{/if}
```

### Plugin List Endpoint
```rust
// GET /api/v1/plugins - List available plugins
async fn list_plugins(
    State(state): State<AppState>,
) -> Result<Json<Vec<PluginInfo>>, AppError> {
    let project_path = /* resolve from active project */;

    let plugins = tokio::task::spawn_blocking(move || {
        let mut list = Vec::new();

        // 1. Bundled plugins
        for (name, description, _) in skills::BUNDLED_PLUGINS {
            list.push(PluginInfo {
                name: name.to_string(),
                description: description.to_string(),
                source: "bundled".to_string(),
            });
        }

        // 2. Global plugins (~/.config/agtx/plugins/)
        // 3. Project-local plugins (.agtx/plugins/)
        // (scan directories, parse plugin.toml for name + description)

        list
    }).await.map_err(|e| AppError::Internal(anyhow::anyhow!("{}", e)))?;

    Ok(Json(plugins))
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| highlight.js for diff highlighting | Shiki v3 with decorations API | 2024 (Shiki v1.0) | VSCode-quality grammar-based highlighting with native diff line decoration support |
| tmux send_keys for agent commands | Direct PTY write via pty-process | Phase 2 (daemon) | No tmux dependency; direct byte-level control; but loses pane capture for readiness detection |
| Synchronous workflow in TUI event loop | Async workflow with tokio::spawn | This phase | Non-blocking transitions; WebSocket progress reporting; multiple concurrent transitions |
| session_name (tmux target) for task-session link | session_id (UUID) for daemon PTY link | This phase | Direct session manager lookup; no tmux string parsing |

**Deprecated/outdated:**
- `tmux send_keys` / `tmux capture_pane`: Replaced by daemon PTY read/write for agent communication
- `session_name` field: Still used by TUI, but daemon uses `session_id` (UUID) instead
- Hand-rolled `glob_path_exists`: Works but could be replaced by the `glob` crate for correctness

## Open Questions

1. **Prompt trigger replacement in PTY model**
   - What we know: TUI uses `wait_for_prompt_trigger()` to poll tmux pane content for specific text before sending prompts (e.g., GSD plugin waits for "What do you want to build?")
   - What's unclear: Whether watching the PTY output broadcast channel is reliable enough for this pattern, given output buffering and timing
   - Recommendation: Subscribe to broadcast channel and scan accumulated output for the trigger string. Use 30-second timeout. If the trigger pattern is simple text matching (not regex), this should work reliably since we get the same bytes the agent produces

2. **Agent readiness heuristic**
   - What we know: TUI's `wait_for_agent_ready()` polls tmux pane content until it stabilizes (no changes for 2s). Claude shows a specific prompt, other agents vary
   - What's unclear: Exact output patterns for each agent's "ready" state in a PTY context
   - Recommendation: Use a simple heuristic: wait for 3 seconds of output silence after spawn, then send commands. This matches the TUI's approach (10 polls * 200ms stabilization). For correctness, watch for the agent process to still be running (not exited)

3. **PR description AI generation in daemon context**
   - What we know: `AgentOperations::generate_text()` uses the agent CLI's print mode (e.g., `claude --print`). This is a blocking call that can take 10-30 seconds
   - What's unclear: Whether to run this in the daemon process (spawn_blocking) or delegate to a PTY session
   - Recommendation: Use `tokio::task::spawn_blocking()` with the task's agent. The agent CLI is already installed on the server. If it fails, return empty fields for manual entry (matches locked decision)

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | vitest 3.x (frontend), cargo test (daemon) |
| Config file | `web/vitest.config.ts`, `crates/agtxd/Cargo.toml` |
| Quick run command | `cd web && npx vitest run --reporter=verbose` / `cargo test -p agtxd` |
| Full suite command | `cd web && npx vitest run && cd .. && cargo test` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| FLOW-01 | Phase transitions trigger side effects | integration | `cargo test -p agtxd advance_task` | No -- Wave 0 |
| FLOW-02 | Plugin resolution precedence | unit | `cargo test -p agtx-core plugin_resolution` | Partial (config_tests.rs) |
| FLOW-03 | Skills deploy to agent-native paths | unit | `cargo test -p agtx-core skill_deployment` | No -- Wave 0 |
| FLOW-04 | Command/prompt resolution per agent | unit | `cargo test -p agtx-core command_resolution` | Partial (agent_tests.rs) |
| FLOW-05 | Artifact detection with glob | unit | `cargo test -p agtxd artifact_detection` | No -- Wave 0 |
| FLOW-06 | Cyclic phase support | unit | `cargo test -p agtxd cyclic_advance` | No -- Wave 0 |
| FLOW-07 | PR creation from browser | integration | `cd web && npx vitest run pr` | No -- Wave 0 |
| FLOW-08 | Syntax-highlighted diff viewing | unit | `cd web && npx vitest run diff` | No -- Wave 0 |

### Sampling Rate
- **Per task commit:** `cd web && npx vitest run --reporter=verbose` + `cargo test -p agtxd`
- **Per wave merge:** Full suite: `cd web && npx vitest run && cd .. && cargo test`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/agtxd/tests/workflow_tests.rs` -- covers FLOW-01, FLOW-05, FLOW-06
- [ ] `crates/agtx-core/tests/workflow_core_tests.rs` -- covers FLOW-02, FLOW-03, FLOW-04
- [ ] `web/src/lib/stores/__tests__/workflow.test.ts` -- covers frontend advance/PR actions
- [ ] `web/src/lib/components/__tests__/DiffView.test.ts` -- covers FLOW-08 diff rendering

## Sources

### Primary (HIGH confidence)
- Source code: `src/tui/app.rs` lines 3211-3650 -- TUI `move_task_right()` reference implementation
- Source code: `src/tui/app.rs` lines 4552-4677 -- TUI `setup_task_worktree()` reference implementation
- Source code: `src/tui/app.rs` lines 5274-5500 -- `resolve_skill_command()`, `resolve_prompt()`, `send_skill_and_prompt()`
- Source code: `src/tui/app.rs` lines 5551-5632 -- `phase_artifact_exists()`, `glob_path_exists()`
- Source code: `src/tui/app.rs` lines 5895-6030 -- `write_skills_to_worktree()`
- Source code: `crates/agtx-core/src/config/mod.rs` lines 426-566 -- `WorkflowPlugin` struct and `load()` method
- Source code: `crates/agtx-core/src/skills.rs` -- All skill deployment and command translation logic
- Source code: `crates/agtx-core/src/git/operations.rs` -- `GitOperations` trait with all git methods
- Source code: `crates/agtx-core/src/git/provider.rs` -- `GitProviderOperations` with PR creation
- Source code: `crates/agtx-core/src/agent/mod.rs` -- Agent definitions and `build_interactive_command()`
- Source code: `crates/agtx-core/src/agent/operations.rs` -- `AgentOperations` trait with `generate_text()`
- Source code: `crates/agtxd/src/session/manager.rs` -- `SessionManager` with spawn/write/subscribe
- Source code: `crates/agtxd/src/state.rs` -- `AppState` struct (needs extending)
- Source code: `crates/agtxd/src/api/mod.rs` -- API router (needs new routes)
- Source code: `web/src/lib/types/index.ts` -- Frontend Task type (already has session_id)
- Source code: `web/src/lib/stores/tasks.svelte.ts` -- TaskStore (needs advance/PR methods)
- Source code: `web/src/lib/components/DetailPanel.svelte` -- Detail panel (needs tabs)

### Secondary (MEDIUM confidence)
- [Shiki Guide](https://shiki.style/guide/) -- Shiki v3.23.0 documentation, ESM-native, decorations API for diff line highlighting
- [Shiki Transformers](https://shiki.style/packages/transformers) -- `transformerNotationDiff()` for diff line classes
- [diff2html](https://diff2html.xyz/) -- Alternative diff viewer (not recommended, less customizable)

### Tertiary (LOW confidence)
- Agent readiness detection heuristic -- based on TUI patterns, not verified in PTY-only context
- Prompt trigger reliability via broadcast channel -- theoretical, needs integration testing

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all core libraries are either already in use (axum, agtx-core) or well-established (Shiki v3)
- Architecture: HIGH -- directly porting from TUI reference implementation with async adaptation
- Pitfalls: HIGH -- identified from direct code analysis of both TUI and daemon implementations
- Workflow semantics: HIGH -- every transition's side effects are documented in the TUI source code
- PTY command sequencing: MEDIUM -- replacing tmux send_keys with direct PTY write is conceptually straightforward but readiness detection is untested

**Research date:** 2026-03-04
**Valid until:** 2026-04-04 (stable -- dependencies and architecture are under project control)
