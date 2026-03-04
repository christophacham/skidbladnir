---
phase: 4
slug: frontend-kanban-board
status: draft
nyquist_compliant: false
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
| **Config file** | web/vitest.config.ts (Wave 0 installs) |
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
| 04-01-01 | 01 | 1 | BOARD-01 | unit | `cd web && npx vitest run` | ❌ W0 | ⬜ pending |
| 04-01-02 | 01 | 1 | BOARD-02 | unit | `cd web && npx vitest run` | ❌ W0 | ⬜ pending |
| 04-01-03 | 01 | 1 | BOARD-03 | unit | `cd web && npx vitest run` | ❌ W0 | ⬜ pending |
| 04-01-04 | 01 | 1 | BOARD-04 | unit | `cd web && npx vitest run` | ❌ W0 | ⬜ pending |
| 04-01-05 | 01 | 1 | BOARD-05 | unit | `cd web && npx vitest run` | ❌ W0 | ⬜ pending |
| 04-02-01 | 02 | 2 | BOARD-06 | unit | `cd web && npx vitest run` | ❌ W0 | ⬜ pending |
| 04-02-02 | 02 | 2 | BOARD-07 | unit | `cd web && npx vitest run` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `web/` — SvelteKit project scaffold with TypeScript, Tailwind, vitest
- [ ] `web/vitest.config.ts` — Vitest configuration
- [ ] `web/src/lib/stores/` — Shared state store stubs
- [ ] `web/src/lib/api/` — API client module stubs

*Vitest and @testing-library/svelte installed in Wave 0.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Column collapse animation | BOARD-01 | Visual transition quality | Collapse a column, verify smooth animation and icon state |
| Search dimming effect | BOARD-05 | Visual opacity rendering | Type search term, verify non-matching cards dim without layout shift |
| Command palette fuzzy search feel | BOARD-07 | Interaction quality | Open Ctrl+K, type partial action names, verify fuzzy matching and keyboard nav |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 10s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
