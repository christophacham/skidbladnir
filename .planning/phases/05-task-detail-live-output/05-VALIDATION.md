---
phase: 5
slug: task-detail-live-output
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-04
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Vitest 3.x |
| **Config file** | `web/vitest.config.ts` |
| **Quick run command** | `cd web && npx vitest run --reporter=verbose` |
| **Full suite command** | `cd web && npx vitest run` |
| **Estimated runtime** | ~5 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd web && npx vitest run --reporter=verbose`
- **After every plan wave:** Run `cd web && npx vitest run`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 5 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 1 | OUTPUT-01 | unit | `cd web && npx vitest run src/lib/stores/__tests__/ui.test.ts -x` | ✅ (extend) | ⬜ pending |
| 05-01-02 | 01 | 1 | OUTPUT-02 | unit | `cd web && npx vitest run src/lib/stores/__tests__/websocket.test.ts -x` | ❌ W0 | ⬜ pending |
| 05-01-03 | 01 | 1 | OUTPUT-03 | unit | `cd web && npx vitest run src/lib/stores/__tests__/websocket.test.ts -x` | ❌ W0 | ⬜ pending |
| 05-01-04 | 01 | 1 | OUTPUT-04 | unit | `cd web && npx vitest run src/lib/stores/__tests__/websocket.test.ts -x` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `web/src/lib/stores/__tests__/websocket.test.ts` — stubs for OUTPUT-02, OUTPUT-03, OUTPUT-04 (WebSocket store connect/disconnect, message handling, output classification, phase status)
- [ ] Extend `web/src/lib/stores/__tests__/ui.test.ts` — covers OUTPUT-01 (selectedTask, selectTask, closeDetail methods)
- [ ] `web/src/lib/api/__tests__/sessions.test.ts` — covers REST session output fetching

*If none: "Existing infrastructure covers all phase requirements."*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Split-view visual layout (board 40% / detail 60%) | OUTPUT-01 | CSS layout verification requires visual inspection | Open board, click task, verify panel slides in from right with correct proportions |
| Live output auto-scroll + pause on scroll-up | OUTPUT-02 | Scroll behavior requires real DOM interaction | Stream output, verify auto-scroll; scroll up manually, verify pause + "Jump to bottom" button |
| Output type color coding (left borders) | OUTPUT-03 | Visual styling verification | Send mixed output types, verify blue border on tool calls, red on errors |
| Pulsing status dot animation | OUTPUT-04 | CSS animation requires visual check | Verify green dot pulses for Working, yellow steady for Idle, checkmark for Ready, gray for Exited |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 5s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
