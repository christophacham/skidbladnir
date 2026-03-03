# Technology Stack

**Analysis Date:** 2026-03-03

## Languages

**Primary:**
- Rust 2021 edition - Entire application (TUI, database, git, agent integration, testing)

## Runtime

**Environment:**
- Rust 1.70+ (Cargo-managed)

**Package Manager:**
- Cargo
- Lockfile: `Cargo.lock` present

## Frameworks

**Core:**
- ratatui 0.30 - Terminal UI rendering with crossterm backend
- crossterm 0.29 - Cross-platform terminal manipulation (input, colors, cursor, events)

**Async:**
- tokio 1.44 (full features) - Async runtime for concurrent operations

**Serialization:**
- serde 1.0 (derive feature) - Serialization/deserialization framework
- serde_json 1.0 - JSON serialization
- toml 0.8 - TOML parsing for configuration and plugin files

**Database:**
- rusqlite 0.34 (bundled feature) - SQLite bindings with bundled SQLite library

**Testing:**
- mockall 0.13 (optional feature: test-mocks) - Trait mocking for testing

**Utilities:**
- anyhow 1.0 - Error handling and context propagation
- thiserror 2.0 - Custom error types
- chrono 0.4 (serde feature) - Date/time handling with serialization support
- uuid 1.16 (v4 feature) - UUID v4 generation for task IDs
- which 7.0 - Find executables in PATH (detect agent availability)
- directories 6.0 - Cross-platform directory resolution

## Key Dependencies

**Critical:**
- tokio - Enables async operations for background tasks (PR generation, background polling)
- rusqlite - Bundles SQLite, no external database server required
- anyhow - Standard error handling pattern throughout codebase

**Build/Dev:**
- crossterm - Provides terminal abstraction for ratatui
- ratatui - TUI rendering engine, largest visual surface of application

**Testing:**
- mockall - Conditional feature for testing; enables mock implementations of Git, Tmux, and Agent trait operations
- tempfile - Temporary filesystem operations for tests

## Configuration

**Environment:**
- Configuration stored in:
  - Global: `~/.config/agtx/config.toml` (GlobalConfig - agent defaults, theme colors, worktree settings)
  - Per-project: `.agtx/config.toml` (ProjectConfig - project-specific overrides)
  - Plugins: `.agtx/plugins/{name}/plugin.toml` (WorkflowPlugin configuration)
- Uses standard XDG base directory paths via `directories` crate

**Build:**
- `Cargo.toml` - Primary build manifest
- `Cargo.lock` - Dependency lock file for reproducible builds
- Conditional feature: `test-mocks` enables mockall for integration tests

**Agent Configuration Directories:**
- `.claude/` - Claude Code CLI configuration
- `.gemini/` - Google Gemini CLI configuration
- `.codex/` - OpenAI Codex CLI configuration
- `.github/agents/` - GitHub Copilot CLI configuration
- `.config/opencode/` - OpenCode CLI configuration
- All copied to worktrees automatically during initialization

## Platform Requirements

**Development:**
- Rust 1.70+ (for compilation)
- Cargo (package manager, included with Rust)

**Runtime:**
- Linux, macOS, or Windows (via crossterm abstraction)
- tmux (required for agent session management; separate binary)
- git (required for worktree operations; separate binary)
- gh (GitHub CLI; required for PR operations via `gh pr create` and `gh pr view`)
- One or more AI agent CLIs available on PATH:
  - claude (Anthropic's Claude Code CLI)
  - codex (OpenAI Codex CLI)
  - copilot (GitHub Copilot CLI)
  - gemini (Google Gemini CLI)
  - opencode (OpenCode CLI)

**Production:**
- Single binary deployment (fully self-contained except for external CLIs)
- Database stored centrally in user config directory (no global setup required)
- No external services required (all operations via local CLI tools)

## Dependency Tree Summary

```
agtx (binary)
├── ratatui (UI rendering)
│   └── crossterm (terminal control)
├── tokio (async runtime)
├── rusqlite (bundled SQLite)
├── serde/toml/serde_json (config serialization)
├── chrono (timestamps)
├── uuid (ID generation)
├── anyhow (error handling)
├── thiserror (error types)
├── which (executable detection)
└── directories (config paths)

[dev-only]
├── mockall (testing mocks)
└── tempfile (test fixtures)
```

## Notable Compilation Features

**Default:** None

**Available:**
- `test-mocks` - Includes mockall crate; enables mock trait implementations for `GitOperations`, `TmuxOperations`, `AgentOperations` (used in integration tests)

---

*Stack analysis: 2026-03-03*
