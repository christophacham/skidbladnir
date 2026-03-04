---
phase: 03
slug: websocket-streaming
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-04
---

# Phase 03 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (built-in) |
| **Config file** | Cargo.toml `[dev-dependencies]` |
| **Quick run command** | `cargo test -p agtxd --test ws_tests` |
| **Full suite command** | `cargo test -p agtxd` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p agtxd --test ws_tests`
- **After every plan wave:** Run `cargo test -p agtxd`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 15 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | INFRA-02 | integration | `cargo test -p agtxd --test ws_tests::test_ws_upgrade_succeeds` | ❌ W0 | ⬜ pending |
| 03-01-01 | 01 | 1 | INFRA-02 | integration | `cargo test -p agtxd --test ws_tests::test_ws_upgrade_404` | ❌ W0 | ⬜ pending |
| 03-01-01 | 01 | 1 | WS-01 | integration | `cargo test -p agtxd --test ws_tests::test_ws_receives_live_output` | ❌ W0 | ⬜ pending |
| 03-01-01 | 01 | 1 | WS-01 | integration | `cargo test -p agtxd --test ws_tests::test_ws_multiple_clients` | ❌ W0 | ⬜ pending |
| 03-01-01 | 01 | 1 | WS-02 | unit | `cargo test -p agtxd --test session_tests` | ✅ | ⬜ pending |
| 03-02-01 | 02 | 2 | WS-03 | integration | `cargo test -p agtxd --test ws_tests::test_ws_cursor_reconnection` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 2 | WS-03 | integration | `cargo test -p agtxd --test ws_tests::test_output_offset_limit` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 2 | WS-04 | integration | `cargo test -p agtxd --test ws_tests::test_ws_connected_message` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 2 | WS-04 | integration | `cargo test -p agtxd --test ws_tests::test_ws_state_change_on_exit` | ❌ W0 | ⬜ pending |
| 03-02-01 | 02 | 2 | WS-05 | integration | Covered by WS-01 test | N/A | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/agtxd/tests/ws_tests.rs` — WebSocket integration test stubs (INFRA-02, WS-01, WS-03, WS-04)
- [ ] `Cargo.toml` dev-dependencies: `tokio-tungstenite = "0.26"`, `futures-util = "0.3"`
- [ ] axum `"ws"` feature flag added to workspace `Cargo.toml`

*Existing infrastructure covers WS-02 (output persistence already tested in session_tests).*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Auto-scroll pauses on scroll-up | WS-05 | Frontend behavior (Phase 4/5) | Verify in browser when frontend exists |
| Connection status indicator | WS-04 | Frontend UI element | Verify visually in browser |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 15s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
