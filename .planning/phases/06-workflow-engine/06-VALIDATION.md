---
phase: 6
slug: workflow-engine
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-04
---

# Phase 6 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | vitest 3.x (frontend), cargo test (daemon/core) |
| **Config file** | `web/vitest.config.ts`, `crates/agtxd/Cargo.toml`, `crates/agtx-core/Cargo.toml` |
| **Quick run command** | `cd web && npx vitest run --reporter=verbose` / `cargo test -p agtxd` |
| **Full suite command** | `cd web && npx vitest run && cd .. && cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p agtxd` + `cd web && npx vitest run --reporter=verbose`
- **After every plan wave:** Run `cd web && npx vitest run && cd .. && cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 06-01-01 | 01 | 1 | FLOW-02 | unit | `cargo test -p agtx-core plugin_resolution` | Partial | ⬜ pending |
| 06-01-02 | 01 | 1 | FLOW-03 | unit | `cargo test -p agtx-core skill_deployment` | ❌ W0 | ⬜ pending |
| 06-01-03 | 01 | 1 | FLOW-04 | unit | `cargo test -p agtx-core command_resolution` | Partial | ⬜ pending |
| 06-02-01 | 02 | 2 | FLOW-01 | integration | `cargo test -p agtxd advance_task` | ❌ W0 | ⬜ pending |
| 06-02-02 | 02 | 2 | FLOW-05 | unit | `cargo test -p agtxd artifact_detection` | ❌ W0 | ⬜ pending |
| 06-02-03 | 02 | 2 | FLOW-06 | unit | `cargo test -p agtxd cyclic_advance` | ❌ W0 | ⬜ pending |
| 06-03-01 | 03 | 3 | FLOW-07 | integration | `cd web && npx vitest run pr` | ❌ W0 | ⬜ pending |
| 06-03-02 | 03 | 3 | FLOW-08 | unit | `cd web && npx vitest run diff` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/agtxd/tests/workflow_tests.rs` — stubs for FLOW-01, FLOW-05, FLOW-06
- [ ] `crates/agtx-core/tests/workflow_core_tests.rs` — stubs for FLOW-02, FLOW-03, FLOW-04
- [ ] `web/src/lib/stores/__tests__/workflow.test.ts` — stubs for frontend advance/PR actions
- [ ] `web/src/lib/components/__tests__/DiffView.test.ts` — stubs for FLOW-08 diff rendering

*Existing test infrastructure covers framework installation — no new framework needed.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| PR created on GitHub | FLOW-07 | Requires authenticated gh CLI and real GitHub repo | 1. Move task to Review, 2. Click Create PR, 3. Verify PR appears on GitHub |
| Syntax highlighting renders correctly | FLOW-08 | Visual correctness requires human eye | 1. Open diff tab, 2. Verify Rust/TS/Python files have correct highlighting |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
