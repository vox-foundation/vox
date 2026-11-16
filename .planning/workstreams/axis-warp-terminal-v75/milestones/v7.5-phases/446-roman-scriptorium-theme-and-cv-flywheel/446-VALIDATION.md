---
phase: 446
slug: roman-scriptorium-theme-and-cv-flywheel
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-11-15
validated_at: 2026-11-16
---

# Phase 446 — Validation Strategy

> Per-phase validation contract verified after `446-REVIEW-FIX.md`.

---

## Test Infrastructure

| Property | Value |
|---|---|
| **Framework** | Vitest `3.2.6` (jsdom), Playwright Test (`--project=chromium`, `VOX_CV_FLYWHEEL=1`), Style Dictionary `tokens:build`, `tsc --noEmit`, `toestub`, `vox-doc-pipeline`, `vox-arch-check` |
| **Config file** | `crates/vox-gui/ui/package.json` & `crates/vox-gui/ui/playwright.config.ts` |
| **Quick run command** | `pnpm --dir crates/vox-gui/ui exec vitest run --maxWorkers=4 e2e/review/audits.test.ts e2e/review/terminalAudits.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/lib/themeContract.test.ts src/lib/cvFlywheel.test.ts src/guards/ src/__tests__/noVersionedModelIds.test.ts src/__tests__/chatTypeScale.test.ts src/components/surfaces/Loquela/Loquela.test.tsx` |
| **Full suite command** | `env VOX_CV_FLYWHEEL=1 CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts --project=chromium` |
| **Estimated runtime** | Vitest `40` files (`276` tests): ~`19s`; Playwright `5x4` (`112` cells): ~`1.7m` |

---

## Sampling Rate

- **After every task commit:** Ran Vitest + `tsc --noEmit` + `toestub` static guards.
- **After every plan wave:** Ran `pnpm --dir crates/vox-gui/ui run build` + `e2e/console-workbench.spec.ts` + `e2e/review/cv-flywheel.spec.ts -g "Phase 386"`.
- **Before `/gsd-verify-work`:** Full `40`-file (`276`-test) Vitest suite, `tsc --noEmit`, `CV_STAGE=before` (`8/8`), `CV_STAGE=after` (`8/8`, `112/112` matrix cells), `toestub`, `vox-arch-check`, and `vox-doc-pipeline --lint-only` all observed green.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---|---|---|---|---|---|---|---|
| 446-01-01 | 446-01 | 1 | TERM-ROMA-CV-01 | Token build + Contract Vitest | `pnpm --dir crates/vox-gui/ui run tokens:build` + `themeContract.test.ts` + `toestub --rules vox/ui/theme-contract-drift` | Yes | ✅ green |
| 446-01-02 | 446-01 | 1 | TERM-ROMA-CV-01 | CSS Containment & E2E parity | `e2e/review/cv-flywheel.spec.ts -g "Phase 386"` (`23 / 23 Verified`) | Yes | ✅ green |
| 446-02-01 | 446-02 | 1 | TERM-ROMA-CV-02 | Unit (WCAG Dual-Surface ANSI + xterm addons) | `xtermAddons.test.ts` + `sgrToSpans.test.tsx` + `TerminalTab.test.tsx` | Yes | ✅ green |
| 446-02-02 | 446-02 | 1 | TERM-ROMA-CV-02 | Component & Guards | `Console/*` (`DiscoveryRail`, `Console`, `BlockStreamView`, `TerminalSessionPane`, etc.) + `ChatTerminalBlockCard.test.tsx` + `src/guards/` | Yes | ✅ green |
| 446-02-03 | 446-02 | 1 | TERM-ROMA-CV-02 | Responsive Viewport Check | `split-panes` / `history-search-open` / `approval-card-pending` at `900x600` & `1100x720` | Yes | ✅ green |
| 446-03-01 | 446-03 | 2 | TERM-ROMA-CV-03 | TDD + Browser Serialization | `terminalAudits.test.ts` + `audits.test.ts` + `cvFlywheel.test.ts` (`new Function` standalone check) | Yes | ✅ green |
| 446-03-02 | 446-03 | 2 | TERM-ROMA-CV-03 | Fixtures & 8 State Machine Check | `states.ts` + `terminalRichFixtures.ts` + `reviewStates.guard.test.ts` | Yes | ✅ green |
| 446-04-01 | 446-04 | 3 | TERM-ROMA-CV-03 | Playwright 5x4 CV Flywheel | `CV_STAGE=before` (`8/8`) + `CV_STAGE=after` (`8/8`, `112/112` cells, `minVisualAfterScore: 0.9284`, `minVlmOverallScore: 0.9506`) + `console-workbench.spec.ts` (`2/2`) | Yes | ✅ green |
| 446-04-02 | 446-04 | 3 | TERM-ROMA-CV-04 | Docs SSOT Promotion | `vox-doc-pipeline --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md` (`0` errors) | Yes | ✅ green |

---

## Wave 0 Requirements

- [x] `terminalAudits.test.ts` TDD assertions proving positive catch on proportional fonts, `.xterm-helper-textarea` cream-box leak, blank `.xterm-rows`, startup echo spam, and `< 4.5:1` / `< 7.0:1` ANSI contrast violations, plus clean pass on valid Dual-Surface DOM.
- [x] `audits.test.ts` coverage for viewport + `overflow-x`/`overflow-y` ancestor clipping and opacity calculation.
- [x] `noVersionedModelIds.test.ts`, `chatTypeScale.test.ts`, and `Loquela.test.tsx` repo-wide parity green (`40/40` files, `276/276` tests).

---

## Manual-Only Verifications

Visual screenshot inspection completed on `6` regenerated `--after.png` cells in `crates/vox-gui/ui/test-results/cv-flywheel-terminal/` (`auto-tui-alt-screen--arcane--1440x900--after.png`, `split-panes--arcane--1100x720--after.png`, `split-panes--arcane--900x600--after.png`, `history-search-open--arcane--900x600--after.png`, `runbooks-open--arcane--1440x900--after.png`, `approval-card-pending--arcane--900x600--after.png`) confirming crisp glyphs, readable Dual-Surface ANSI contrast, unclipped responsive layouts, and clean Roman typography.

---

## Validation Sign-Off

- [x] All tasks have automated verify runs green after final edits
- [x] Sampling continuity: Vitest + `tsc` + `toestub` + Playwright before/after verified
- [x] Zero skipped assertions (`276/276` Vitest, `112/112` CV matrix cells)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated
