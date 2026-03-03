# Technology Stack

**Analysis Date:** 2026-03-03

## Languages

**Primary:**
- Rust (Edition 2021) - All application code (`src/`)

**Secondary:**
- Bash - Installer script (`install.sh`), init scripts in plugins
- TOML - Configuration files (`Cargo.toml`, `plugins/*/plugin.toml`, config files)
- Markdown - Skill files (`plugins/*/skills/*.md`)
- YAML - CI/CD workflows (`.github/workflows/`)

## Runtime

**Environment:**
- Native binary (compiled Rust) - no interpreter or VM required
- Minimum Rust version: 1.70+ (stated in `CLAUDE.md`; CI uses `stable` toolchain)
- Current dev toolchain: rustc 1.93.1

**Package Manager:**
- Cargo (1.93.1 on dev machine)
- Lockfile: `Cargo.lock` present and committed

## Frameworks

**Core:**
- `ratatui` 0.30 - Terminal UI framework (widget-based rendering)
- `crossterm` 0.29 - Terminal backend (raw mode, events, alternate screen)
- `tokio` 1.44 (full features) - Async runtime for the main event loop

**Testing:**
- Built-in `#[test]` / `cargo test` - Test runner
- `mockall` 0.13 - Mock generation for trait-based testing (optional via `test-mocks` feature)
- `tempfile` 3 - Temporary directories for test isolation

**Build/Dev:**
- Cargo (standard Rust build system)
- `include_str!()` macros - Compile-time embedding of plugin configs and skill files

## Key Dependencies

**Critical:**
- `ratatui` 0.30 - Entire UI rendering layer (`src/tui/app.rs`, `src/tui/shell_popup.rs`)
- `crossterm` 0.29 - Terminal I/O, raw mode, key events (`src/main.rs`, `src/tui/app.rs`)
- `rusqlite` 0.34 (bundled feature) - All data persistence via SQLite (`src/db/schema.rs`)
- `tokio` 1.44 - Async main, background threads for PR operations (`src/main.rs`, `src/tui/app.rs`)

**Infrastructure:**
- `anyhow` 1.0 - Error handling throughout all modules
- `thiserror` 2.0 - Custom error type definitions
- `serde` 1.0 (derive feature) - Serialization for config, models, and plugins
- `serde_json` 1.0 - JSON parsing (GitHub CLI output)
- `toml` 0.8 - TOML parsing for config files and plugins (`src/config/mod.rs`, `src/skills.rs`)
- `chrono` 0.4 (serde feature) - DateTime handling for tasks and projects (`src/db/models.rs`)
- `uuid` 1.16 (v4 feature) - Unique ID generation for tasks and projects (`src/db/models.rs`)
- `which` 7.0 - Agent CLI binary detection (`src/agent/mod.rs`)
- `directories` 6.0 - Platform-appropriate config/data directory resolution (`src/config/mod.rs`, `src/db/schema.rs`)

## Configuration

**Environment:**
- No environment variables required for basic operation
- `HOME` env var used for config path resolution (`src/config/mod.rs`)
- `AGTX_INSTALL_DIR` optional env var in `install.sh` for custom install path
- No `.env` files used; app is self-contained

**Global Config:**
- `~/.config/agtx/config.toml` - Global settings (default agent, theme, worktree config)
- Managed by `GlobalConfig` in `src/config/mod.rs`

**Project Config:**
- `.agtx/config.toml` - Per-project overrides (agent, base branch, GitHub URL, plugin)
- Managed by `ProjectConfig` in `src/config/mod.rs`

**Plugin Config:**
- `plugins/*/plugin.toml` - Workflow plugin definitions (bundled at compile time)
- `.agtx/plugins/*/plugin.toml` - Project-local plugin overrides
- `~/.config/agtx/plugins/*/plugin.toml` - User-global plugin overrides
- Resolution order: project-local > user-global > bundled

**Build:**
- `Cargo.toml` - Single workspace, single binary crate
- Feature flags: `default = []`, `test-mocks` (enables `mockall` for testing)

## Platform Requirements

**Development:**
- Rust 1.70+ with Cargo
- SQLite bundled via `rusqlite` (no system SQLite required)
- Any Unix-like OS (Linux, macOS)

**Runtime Dependencies (external CLIs):**
- `tmux` - Required; manages agent sessions (`src/tmux/mod.rs`, `src/tmux/operations.rs`)
- `git` - Required; worktree management, branching, diffing (`src/git/`)
- `gh` - Optional; GitHub PR creation and status checks (`src/git/provider.rs`)
- At least one coding agent CLI (claude, codex, copilot, gemini, opencode) - Optional but needed for agent features

**Production/Release:**
- Compiled binary, no runtime interpreter needed
- Release targets (from `.github/workflows/release.yml`):
  - `aarch64-apple-darwin` (macOS ARM)
  - `x86_64-apple-darwin` (macOS Intel)
  - `x86_64-unknown-linux-gnu` (Linux x86)
  - `aarch64-unknown-linux-gnu` (Linux ARM)

**Data Storage:**
- macOS: `~/Library/Application Support/agtx/` (via `directories` crate)
- Linux: `~/.local/share/agtx/` (via `directories` crate)
- Database files: `index.db` (global), `projects/{hash}.db` (per-project)

---

*Stack analysis: 2026-03-03*
