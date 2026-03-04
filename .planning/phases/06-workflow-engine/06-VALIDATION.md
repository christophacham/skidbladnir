---
phase: 6
slug: workflow-engine
status: draft
nyquist_compliant: true
wave_0_complete: true
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
| 06-01-00 | 01 | 1 | FLOW-01..06 | stub | `cargo test -p agtx-core workflow_core_tests` | Created by Task 0 | W0 |
| 06-01-01 | 01 | 1 | FLOW-02 | unit | `cargo test -p agtx-core plugin_resolution` | Partial | pending |
| 06-01-02 | 01 | 1 | FLOW-03 | unit | `cargo test -p agtx-core skill_deployment` | Created by 06-01 Task 0 | pending |
| 06-01-03 | 01 | 1 | FLOW-04 | unit | `cargo test -p agtx-core command_resolution` | Partial | pending |
| 06-02-00 | 02 | 2 | FLOW-01,06 | stub | `cd web && npx vitest run workflow` | Created by Task 0 | W0 |
| 06-02-01 | 02 | 2 | FLOW-01 | integration | `cargo test -p agtxd advance_task` | Created by 06-01 Task 0 | pending |
| 06-02-02 | 02 | 2 | FLOW-05 | unit | `cargo test -p agtxd artifact_detection` | Created by 06-01 Task 0 | pending |
| 06-02-03 | 02 | 2 | FLOW-06 | unit | `cargo test -p agtxd cyclic_advance` | Created by 06-01 Task 0 | pending |
| 06-03-00 | 03 | 3 | FLOW-08 | stub | `cd web && npx vitest run DiffView` | Created by Task 0 | W0 |
| 06-03-01 | 03 | 3 | FLOW-07 | integration | `cd web && npx vitest run pr` | Created by 06-02 Task 0 | pending |
| 06-03-02 | 03 | 3 | FLOW-08 | unit | `cd web && npx vitest run diff` | Created by 06-03 Task 0 | pending |

*Status: W0 = wave 0 stub, pending = not started, green = passing, red = failing, flaky = intermittent*

---

## Wave 0 Requirements

- [x] `crates/agtxd/tests/workflow_tests.rs` — stubs for FLOW-01, FLOW-05, FLOW-06 (created by 06-01 Task 0)
- [x] `crates/agtx-core/tests/workflow_core_tests.rs` — real tests for FLOW-02, FLOW-03, FLOW-04 (created by 06-01 Task 0)
- [x] `web/src/lib/stores/__tests__/workflow.test.ts` — stubs for frontend advance/PR actions (created by 06-02 Task 0)
- [x] `web/src/lib/components/__tests__/DiffView.test.ts` — stubs for FLOW-08 diff rendering (created by 06-03 Task 0)

*Wave 0 stubs are created as Task 0 within each plan, running before implementation tasks.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| PR created on GitHub | FLOW-07 | Requires authenticated gh CLI and real GitHub repo | 1. Move task to Review, 2. Click Create PR, 3. Verify PR appears on GitHub |
| Syntax highlighting renders correctly | FLOW-08 | Visual correctness requires human eye | 1. Open diff tab, 2. Verify Rust/TS/Python files have correct highlighting |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved
