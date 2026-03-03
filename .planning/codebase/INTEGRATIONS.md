# External Integrations

**Analysis Date:** 2026-03-03

## APIs & External Services

**No direct API calls.** All external service interaction is through CLI subprocess invocations (`std::process::Command`). The application never makes HTTP requests or uses SDK clients directly.

## CLI Tool Integrations

### tmux (Required)

Session and window management for coding agents. All calls go through `tmux -L agtx` (dedicated server).

- **Module:** `src/tmux/mod.rs`, `src/tmux/operations.rs`
- **Trait:** `TmuxOperations` in `src/tmux/operations.rs` (mockable via `test-mocks` feature)
- **Server name:** `agtx` (constant `AGENT_SERVER` in `src/tmux/mod.rs`)
- **Operations used:**
  - `new-session` / `has-session` - Session lifecycle
  - `new-window` / `kill-window` / `list-windows` - Window lifecycle
  - `send-keys` - Sending commands/prompts to agents
  - `capture-pane` - Reading agent output (with `-e` for ANSI, `-J` for join)
  - `display` - Cursor position, pane height, current command
  - `resize-window` - Matching popup dimensions

### git (Required)

Worktree management, branching, diffing, and commit operations.

- **Module:** `src/git/mod.rs`, `src/git/worktree.rs`, `src/git/operations.rs`
- **Trait:** `GitOperations` in `src/git/operations.rs` (mockable via `test-mocks` feature)
- **Operations used:**
  - `worktree add/remove/prune` - Task isolation via worktrees
  - `rev-parse` - Repository detection, branch resolution
  - `branch -D` - Branch cleanup
  - `diff`, `diff --cached`, `diff --stat` - Change visualization
  - `ls-files` - File listing for fuzzy search
  - `add -A`, `commit`, `push` - PR preparation
  - `status --porcelain` - Change detection
  - `merge --no-ff` - Branch merging

### gh CLI (Optional)

GitHub pull request operations.

- **Module:** `src/git/provider.rs`
- **Trait:** `GitProviderOperations` in `src/git/provider.rs` (mockable via `test-mocks` feature)
- **Implementation:** `RealGitHubOps` struct
- **Operations used:**
  - `gh pr create --title --body --head` - Create pull requests
  - `gh pr view --json state` - Check PR state (Open/Merged/Closed)
- **Auth:** Relies on `gh auth` being configured (no env vars managed by agtx)

### Coding Agent CLIs (At Least One Required)

AI coding agents spawned in tmux sessions with auto-approve flags.

- **Module:** `src/agent/mod.rs`, `src/agent/operations.rs`
- **Traits:** `AgentOperations`, `AgentRegistry` in `src/agent/operations.rs`
- **Detection:** `which` crate checks for CLI binary availability (`src/agent/mod.rs`)
- **Supported agents:**

| Agent | Binary | Interactive Flags | Print/Generate Mode | Skill Path |
|-------|--------|-------------------|---------------------|------------|
| Claude | `claude` | `--dangerously-skip-permissions` | `--print` | `.claude/commands/agtx/` |
| Codex | `codex` | `--full-auto` | `exec --full-auto` | `.codex/skills/agtx-*/SKILL.md` |
| Copilot | `copilot` | `--allow-all-tools` | `-p` | `.github/agents/agtx/` |
| Gemini | `gemini` | `--approval-mode yolo` | `-p` | `.gemini/commands/agtx/` |
| OpenCode | `opencode` | (none) | `-p` | `.config/opencode/command/` |

- **Agent config:** defined in `known_agents()` in `src/agent/mod.rs`
- **Generate text** (non-interactive): Used for PR description generation, runs agent in print mode via `AgentOperations::generate_text()` in `src/agent/operations.rs`

## Data Storage

**Databases:**
- SQLite (bundled via `rusqlite` with `bundled` feature - no system SQLite dependency)
  - Client: `rusqlite::Connection` in `src/db/schema.rs`
  - Global index: `{data_dir}/index.db`
  - Per-project: `{data_dir}/projects/{path_hash}.db`
  - Data dir resolution: `directories::ProjectDirs::from("", "", "agtx")` data_dir
    - macOS: `~/Library/Application Support/agtx/`
    - Linux: `~/.local/share/agtx/`
  - Schema init: `CREATE TABLE IF NOT EXISTS` with `ALTER TABLE ADD COLUMN` migrations (ignores errors if column exists)
  - DateTime format: RFC3339 strings

**File Storage:**
- Git worktrees at `.agtx/worktrees/{slug}/` within project directories
- Skill files deployed to agent-native paths within worktrees
- Configuration files on local filesystem (`~/.config/agtx/`)

**Caching:**
- In-memory only: plugin instances cached in `HashMap<Option<String>, Option<WorkflowPlugin>>` per task
- Artifact file existence cached with 2-second TTL (in `src/tui/app.rs`)
- Tmux pane content cached in `ShellPopup.cached_content` (refreshed periodically)

## Authentication & Identity

**Auth Provider:**
- None. The application has no user authentication.
- GitHub operations rely on pre-configured `gh auth` (managed externally by the user).
- Coding agent CLIs handle their own authentication.

## Monitoring & Observability

**Error Tracking:**
- None. Errors are handled via `anyhow::Result` and displayed in the TUI.

**Logs:**
- No logging framework. All output goes to the TUI.
- Agent output visible via tmux pane capture.

## CI/CD & Deployment

**CI Pipeline:**
- GitHub Actions (`.github/workflows/ci.yml`)
- Triggers: push to `main`, PRs to `main`
- Matrix: `ubuntu-latest` + `macos-latest` with `stable` Rust
- Steps: build + test with `--features test-mocks`
- Note: `cargo fmt` and `cargo clippy` checks are commented out

**Release Pipeline:**
- GitHub Actions (`.github/workflows/release.yml`)
- Triggers: tags matching `v*`
- Builds 4 platform targets (macOS aarch64/x86_64, Linux x86_64/aarch64)
- Creates GitHub Release with auto-generated notes via `softprops/action-gh-release@v2`
- Produces `.tar.gz` archives per platform

**Installation:**
- `install.sh` - curl-pipe-bash installer fetching latest release from GitHub
- Source repo: `fynnfluegge/agtx` on GitHub
- Default install dir: `~/.local/bin/` (overridable via `AGTX_INSTALL_DIR`)

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None

## Workflow Plugin System

Plugins define how agtx integrates with external tools during the task lifecycle.

**Bundled plugins** (compiled into binary via `include_str!()` in `src/skills.rs`):

| Plugin | Description | External Dependencies |
|--------|-------------|----------------------|
| `agtx` | Built-in skills workflow | None |
| `gsd` | Get Shit Done framework | `npx get-shit-done-cc@latest` (npm package) |
| `spec-kit` | GitHub spec-kit | None |
| `void` | No prompting | None |

**Plugin resolution order** (in `WorkflowPlugin::load()` at `src/config/mod.rs`):
1. Project-local: `.agtx/plugins/{name}/plugin.toml`
2. User-global: `~/.config/agtx/plugins/{name}/plugin.toml`
3. Bundled: compiled-in from `plugins/{name}/plugin.toml`

**Plugin capabilities:**
- `commands` - Slash commands sent to agents per phase
- `prompts` - Task content templates with `{task}`, `{task_id}`, `{phase}` placeholders
- `artifacts` - File paths signaling phase completion (supports `*` wildcards)
- `init_script` - Shell command run in worktree before agent starts
- `copy_dirs` / `copy_files` - Files to copy into worktrees
- `copy_back` - Files to copy from worktree back to project root on phase completion
- `cyclic` - Enables Review to Planning transition for multi-phase workflows
- `supported_agents` - Agent whitelist

## Environment Configuration

**Required env vars:**
- `HOME` - Used for config path resolution (always available on Unix)

**Optional env vars:**
- `AGTX_INSTALL_DIR` - Custom install directory (used by `install.sh` only)

**Secrets location:**
- No secrets managed by agtx
- Agent CLI tools manage their own API keys/tokens
- `gh` CLI manages its own GitHub authentication

---

*Integration audit: 2026-03-03*
