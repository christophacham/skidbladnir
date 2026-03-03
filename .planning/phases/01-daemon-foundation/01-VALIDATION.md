---
phase: 01
slug: daemon-foundation
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-03
---

# Phase 01 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in, Rust 1.93) |
| **Config file** | Cargo.toml [dev-dependencies] |
| **Quick run command** | `cargo test --workspace` |
| **Full suite command** | `cargo test --workspace --features test-mocks` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace`
- **After every plan wave:** Run `cargo test --workspace --features test-mocks`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 01-01-01 | 01 | 1 | INFRA-01 | integration | `cargo test -p agtxd --test api_tests` | ❌ W0 | ⬜ pending |
| 01-01-02 | 01 | 1 | INFRA-04 | unit | `cargo test -p agtxd --test api_tests::health` | ❌ W0 | ⬜ pending |
| 01-01-03 | 01 | 1 | INFRA-03 | integration | `cargo test -p agtxd --test logging_tests` | ❌ W0 | ⬜ pending |
| 01-01-04 | 01 | 1 | INFRA-05 | integration | `cargo test -p agtxd --test shutdown_tests` | ❌ W0 | ⬜ pending |
| 01-01-05 | 01 | 1 | INFRA-06 | integration | `cargo test -p agtxd --test config_reload_tests` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/agtxd/tests/api_tests.rs` — stubs for INFRA-01, INFRA-04
- [ ] `crates/agtxd/tests/logging_tests.rs` — stubs for INFRA-03
- [ ] `crates/agtxd/tests/shutdown_tests.rs` — stubs for INFRA-05
- [ ] `crates/agtxd/tests/config_reload_tests.rs` — stubs for INFRA-06
- [ ] Existing tests in root `tests/` must continue passing after workspace conversion

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Log files rotate daily at midnight | INFRA-03 | Time-dependent; impractical in CI | Verify log dir contains date-stamped files after running overnight |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
