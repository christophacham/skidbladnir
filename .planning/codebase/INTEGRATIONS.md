# External Integrations

**Analysis Date:** 2026-03-03

## APIs & External Services

**AI Coding Agents:**
- Claude Code (Anthropic) - Primary agent
  - CLI: `claude` executable
  - Usage: `.build_interactive_command()` in `src/agent/mod.rs`
  - Flags: `--dangerously-skip-permissions` (auto-approval mode)
  - Session tracking: `--session {task_id}` support via `build_spawn_args()`

- Codex (OpenAI) - Alternative agent
  - CLI: `codex` executable
  - Flags: `--full-auto` (auto-approval mode)

- GitHub Copilot - Alternative agent
  - CLI: `copilot` executable
  - Flags: `--allow-all-tools` (tool approval)

- Google Gemini - Alternative agent
  - CLI: `gemini` executable
  - Flags: `--approval-mode yolo` (full auto-approval)

- OpenCode - Alternative agent
  - CLI: `opencode` executable
  - No special flags

**GitHub Integration:**
- GitHub CLI (`gh`) via `src/git/provider.rs:RealGitHubOps`
  - PR creation: `gh pr create --title ... --body ... --head {branch}`
  - PR state polling: `gh pr view {pr_number} --json state`
  - Returns PR number and URL from create operation
  - State tracking: Open, Merged, Closed, Unknown
  - Used in `src/tui/app.rs` for PR creation workflow

## Data Storage

**Databases:**
- SQLite (local filesystem)
  - Connection: Bundled via `rusqlite` (no external server)
  - Global index: `~/.config/agtx/index.db`
  - Per-project: `~/.config/agtx/projects/{hash}.db` (hash of project path)
  - Location function: `GlobalConfig::data_dir()` via `directories` crate
  - Schema: `src/db/schema.rs`
    - Global schema: Project index table
    - Project schema: Tasks table with columns: id, title, description, status, agent, project_id, session_name, worktree_path, branch_name, pr_number, pr_url, plugin, created_at, updated_at, cycle

**File Storage:**
- Local filesystem only
  - Worktrees: `.agtx/worktrees/{task-slug}/` (git worktree directories)
  - Configuration: `~/.config/agtx/` (global config)
  - Plugins: `.agtx/plugins/{name}/` (project-local) or `~/.config/agtx/plugins/{name}/` (global)
  - Database files: `~/.config/agtx/` (all SQLite databases)

**Caching:**
- Phase artifact polling with 2-second TTL cache in `src/tui/app.rs`
- Plugin instance cache: `HashMap<Option<String>, Option<WorkflowPlugin>>` per task to avoid repeated disk reads

## Authentication & Identity

**Auth Provider:**
- Custom GitHub authentication
  - Method: Uses `gh` CLI for authentication (relies on user's existing GitHub auth)
  - No API keys in config (leverages system `gh` session)
  - Credentials: Stored in user's existing GitHub CLI config (`~/.config/gh/`)

**Session Management:**
- No centralized auth service
- Agents manage their own authentication via their installed CLIs
- Each agent CLI handles auth independently (Claude, Copilot, Gemini, Codex, OpenCode)

## Monitoring & Observability

**Error Tracking:**
- Not detected - No external error tracking service

**Logs:**
- Console output only (piped through terminal UI)
- No persistent logging framework
- Errors propagated via `anyhow::Result` with context
- All task state visible in kanban board UI

## CI/CD & Deployment

**Hosting:**
- Single binary (Rust statically compiled)
- No server component
- No deployment service required

**CI Pipeline:**
- Not detected - No CI/CD integration configured

**Artifact Management:**
- Phase artifacts detected by file glob patterns (configurable per plugin)
- Artifacts signal phase completion:
  - Research phase: Plugin-defined artifact path
  - Planning phase: Plugin-defined artifact path
  - Running phase: Plugin-defined artifact path
  - Review phase: Plugin-defined artifact path
- Default artifacts for bundled "agtx" plugin: `.agtx/{research|plan|execute|review}.md`

## Environment Configuration

**Required env vars:**
- `HOME` - Used for config/data directory resolution (standard Unix requirement)

**Secrets location:**
- No secrets in agtx itself
- Depends on installed agent CLIs for their secrets:
  - Claude: Uses ANTHROPIC_API_KEY (managed by claude CLI)
  - OpenAI Codex: Uses OPENAI_API_KEY (managed by codex CLI)
  - GitHub Copilot: Uses GitHub credentials (managed by copilot CLI)
  - Gemini: Uses GOOGLE_API_KEY (managed by gemini CLI)
  - OpenCode: Uses OpenCode credentials (managed by opencode CLI)

**Configuration files:**
- Global config: `~/.config/agtx/config.toml` (theme, default agent, worktree settings)
- Project config: `.agtx/config.toml` (project-specific overrides)
- Plugin config: `.agtx/plugins/{name}/plugin.toml` or `~/.config/agtx/plugins/{name}/plugin.toml`

## Webhooks & Callbacks

**Incoming:**
- Not detected - No webhook endpoints

**Outgoing:**
- PR creation via `gh pr create` (GitHub API via CLI)
- PR state polling via `gh pr view` (GitHub API via CLI)
- No custom webhooks

## External Tool Dependencies

**Required at runtime:**
- `tmux` - Terminal multiplexer for session management
  - Server name: "agtx" (isolated from user sessions)
  - Usage: `tmux -L agtx` commands in `src/tmux/mod.rs` and `src/tmux/operations.rs`
  - Session structure: Per-project sessions with per-task windows

- `git` - Version control
  - Worktree operations: `git worktree add/remove/list` in `src/git/worktree.rs`
  - Branch operations: Create, detect, list branches
  - Operations trait: `src/git/operations.rs` for testable abstraction

- `gh` - GitHub CLI
  - PR operations: Create, view state
  - Implementation: `src/git/provider.rs:RealGitHubOps`
  - Fallback: Returns `PullRequestState::Unknown` if `gh` command fails

- One of: `claude`, `codex`, `copilot`, `gemini`, `opencode`
  - Agent discovery: `which {agent_name}` via `which` crate
  - Detection: `src/agent/mod.rs:detect_available_agents()`
  - At least one must be available to run tasks

## Plugin System

**Bundled Plugins:**
- agtx - Default workflow with phase skills and basic artifact detection
- gsd - Get Shit Done framework (spec-driven development)
  - Init: `npx get-shit-done-cc@latest --{agent} --local --non-interactive`
  - Supported agents: claude, codex, gemini, opencode
  - Cyclic: Yes (allows Review → Planning transitions)

- spec-kit - Specification-driven development by GitHub
  - Requires `.specify` directory in project

- void - Plain agent session (no prompting)

**Plugin Resolution Order:**
1. Project-local: `.agtx/plugins/{name}/plugin.toml`
2. Global: `~/.config/agtx/plugins/{name}/plugin.toml`
3. Bundled: Compiled into binary

## Skill System

**Deployment:**
- Markdown files with YAML frontmatter deployed to agent-native discovery paths
- Canonical path: `.agtx/skills/agtx-{phase}/SKILL.md`
- Agent-specific paths (auto-deployed):
  - Claude: `.claude/commands/agtx/{phase}.md`
  - Gemini: `.gemini/commands/agtx/{phase}.toml` (auto-converted to TOML)
  - Codex: `.codex/skills/agtx-{phase}/SKILL.md`
  - OpenCode: `.config/opencode/command/agtx-{phase}.md` (frontmatter stripped)
  - Copilot: `.github/agents/agtx/{phase}.md`

**Command Translation:**
- Canonical format: `/ns:command` (slash-colon)
- Transformed per-agent:
  - Claude/Gemini: `/ns:command` (unchanged)
  - OpenCode: `/ns-command` (colon → hyphen)
  - Codex: `$ns-command` (slash → dollar, colon → hyphen)
  - Copilot: Prompt-only (no interactive skill invocation)

---

*Integration audit: 2026-03-03*
