---
phase: 2
slug: pty-process-management
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-04
---

# Phase 2 -- Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) + tokio::test for async |
| **Config file** | Cargo.toml [dev-dependencies] |
| **Quick run command** | `cargo test -p agtxd -- --lib` |
| **Full suite command** | `cargo test -p agtxd` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p agtxd -- --lib`
- **After every plan wave:** Run `cargo test -p agtxd`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | PTY-01 | integration | `cargo test -p agtxd test_spawn_session -x` | No -- Wave 0 | pending |
| 02-01-02 | 01 | 1 | PTY-02 | integration | `cargo test -p agtxd test_read_pty_output -x` | No -- Wave 0 | pending |
| 02-01-03 | 01 | 1 | PTY-03 | integration | `cargo test -p agtxd test_write_to_session -x` | No -- Wave 0 | pending |
| 02-01-04 | 01 | 1 | PTY-04 | integration | `cargo test -p agtxd test_resize_session -x` | No -- Wave 0 | pending |
| 02-01-05 | 01 | 1 | PTY-06 | unit | `cargo test -p agtxd test_session_tracks_pid -x` | No -- Wave 0 | pending |
| 02-02-01 | 02 | 2 | PTY-05 | integration | `cargo test -p agtxd test_shutdown_all -x` | No -- Wave 0 | pending |
| 02-03-01 | 03 | 3 | PTY-07 | unit | `cargo test -p agtxd test_process_metrics -x` | No -- Wave 0 | pending |

---

## Wave 0 Requirements

- [ ] `crates/agtxd/tests/session_tests.rs` -- stubs for PTY-01 through PTY-04, PTY-06
- [ ] `crates/agtxd/tests/session_tests.rs` -- stub for `test_shutdown_all_kills_all_sessions` (PTY-05)
- [ ] `crates/agtxd/tests/metrics_tests.rs` -- stubs for PTY-07
- [ ] `crates/agtxd/tests/session_api_tests.rs` -- HTTP endpoint tests for session CRUD
- [ ] Dependencies: `pty-process` (features = ["async"]), `procfs`, `libc` added to Cargo.toml

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| No zombie processes after daemon exit | PTY-05 | Requires observing process table after daemon kill | 1. Start daemon, spawn session 2. Kill daemon with SIGTERM 3. Run `ps aux | grep defunct` -- expect no zombies |
| PTY resize reflows agent output | PTY-04 | Visual reflow depends on agent terminal behavior | 1. Start session with `stty size` running 2. Send resize 3. Verify output reflects new dimensions |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
