---
phase: 4
slug: frontend-kanban-board
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-03-04
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | vitest |
| **Config file** | web/vitest.config.ts (Task 1 of Plan 01 installs) |
| **Quick run command** | `cd web && npx vitest run --reporter=verbose` |
| **Full suite command** | `cd web && npx vitest run --reporter=verbose` |
| **Estimated runtime** | ~10 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cd web && npx vitest run --reporter=verbose`
- **After every plan wave:** Run `cd web && npx vitest run --reporter=verbose`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 10 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 04-01-T1 | 01 | 1 | BOARD-01 | unit | `cd web && npx vitest run --reporter=verbose` | Created by Task 1 (stubs) | pending |
| 04-01-T2 | 01 | 1 | BOARD-01 | unit | `cd web && npx vitest run --reporter=verbose` | Created by Task 1 (stubs) | pending |
| 04-01-T3 | 01 | 1 | BOARD-02 | build | `cd web && npx vitest run --reporter=verbose && npm run build` | Created by Task 1 (stubs) | pending |
| 04-02-T1 | 02 | 2 | BOARD-03, BOARD-04 | build | `cd web && npx vitest run --reporter=verbose && npm run build` | Created by 04-01-T1 (stubs) | pending |
| 04-02-T2 | 02 | 2 | BOARD-05 | build | `cd web && npx vitest run --reporter=verbose && npm run build` | Created by 04-01-T1 (stubs) | pending |
| 04-03-T1 | 03 | 2 | BOARD-06 | build | `cd web && npx vitest run --reporter=verbose && npm run build` | Created by 04-01-T1 (stubs) | pending |
| 04-03-T2 | 03 | 2 | BOARD-07 | build | `cd web && npx vitest run --reporter=verbose && npm run build` | Created by 04-01-T1 (stubs) | pending |

*Status: pending -- green -- red -- flaky*

---

## Wave 0 Requirements

- [x] `web/` -- SvelteKit project scaffold with TypeScript, Tailwind, vitest (Plan 04-01, Task 1)
- [x] `web/vitest.config.ts` -- Vitest configuration (Plan 04-01, Task 1)
- [x] `web/src/lib/stores/__tests__/tasks.test.ts` -- TaskStore test stubs (Plan 04-01, Task 1)
- [x] `web/src/lib/stores/__tests__/projects.test.ts` -- ProjectStore test stubs (Plan 04-01, Task 1)
- [x] `web/src/lib/stores/__tests__/ui.test.ts` -- UiStore test stubs (Plan 04-01, Task 1)
- [x] `web/src/lib/api/__tests__/client.test.ts` -- API client test stubs (Plan 04-01, Task 1)

*Vitest, @testing-library/svelte, and stub test files all created in Plan 04-01 Task 1.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Column collapse animation | BOARD-01 | Visual transition quality | Collapse a column, verify smooth animation and icon state |
| Search dimming effect | BOARD-05 | Visual opacity rendering | Type search term, verify non-matching cards dim without layout shift |
| Command palette fuzzy search feel | BOARD-07 | Interaction quality | Open Ctrl+K, type partial action names, verify fuzzy matching and keyboard nav |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify commands that run vitest
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (stubs created in 04-01 Task 1)
- [x] No watch-mode flags
- [x] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
