---
phase: 446-roman-scriptorium-theme-and-cv-flywheel
plan: 04
title: "5x4 Terminal CV Flywheel Verification, Remediation, & Research Promotion"
status: complete
requirements_completed:
  - TERM-ROMA-CV-03
  - TERM-ROMA-CV-04
---

# Phase 446 Plan 04 Summary (`TERM-ROMA-CV-03`, `TERM-ROMA-CV-04`)

## 1. Delivered Scope

- **Created `crates/vox-gui/ui/e2e/review/terminal-cv-flywheel.spec.ts` (`326` non-blank LoC):**
  - Runs all 8 `TERMINAL_CV_STATES` across 5 viewports (`900x600`, `1100x720`, `1280x800`, `1440x900`, `1920x1080`) × 4 themes (`arcane`, `slate`, `travertine`, `high-contrast`) = `112` cells when `VOX_CV_FLYWHEEL=1` is set on Chromium (`test-results/cv-flywheel-terminal/ledger-after.json` + screenshots).
  - Enforces hard zero tolerances (`outerOverflowPx === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0`, `unstyledNativeSelectCount === 0`, `axeViolations === 0`, `terminalStartupSpamCount === 0`, `xtermHelperLeakCount === 0`, `terminalFontProportionalLeakCount === 0`, `terminalContrastViolationCount === 0`) alongside `verdict.visualAfterScore >= 0.92` (`@ngi/platform-ssot` `2026.10.19-v3.5`) and `vlmCritique.vlmOverallScore >= 0.90`.
- **Remediated all defects surfaced by `CV_STAGE=before` and verified full E2E parity (authorized allowlist):**
  - `crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts` & `crates/vox-gui/ui/e2e/review/terminalAudits.test.ts`: switched `ANSI0-BLK` fixture from `\x1b[30;47m` to `\x1b[30m` so ANSI 0 renders against dark `--terminal-well-bg` and updated the unit test expectation in lockstep.
  - `crates/vox-gui/ui/src/components/surfaces/Console/{BlockStreamView,BlockCard,TerminalTab,TerminalSessionPane,AgentTurnContent,HistorySearchPopover,TerminalApprovalCard,ConsoleSessionBar}.tsx`: fixed Axe `scrollable-region-focusable`, hidden inactive VT/Block layers (`hidden` / `aria-hidden`), fixed `travertine` `term-agent-model-pill` contrast (`bg-bg-elevated`), removed nested `<button>` inside `role="option"` in `HistorySearchPopover`, bounded `TerminalPaneApprovalBanner` at `< 1100px`, and prevented session tab label truncation in `ConsoleSessionBar`.
  - `crates/vox-gui/ui/src/lib/cvFlywheel.ts`: tightened flat-region Laplacian grain mask (`mag < 0.008` and 3×3 neighborhood luminance `span < 0.04`) so crisp 1px `#ffffff`-on-`#000000` hairlines in `high-contrast` do not inflate `noiseFloor`.
  - `crates/vox-gui/ui/e2e/review/states.ts`: scrolled `[data-testid="block-stream-view"]` to `0` on `block-stream-ansi` and `runbooks-open`, and awaited `term-runbook-card-*` attachment in `runbooks-open`.
  - `crates/vox-gui/ui/e2e/review/audits.ts` & `crates/vox-gui/ui/e2e/review/audits.test.ts`: intersected `main button[aria-expanded]` / `open-chat-cta` bounding rects with viewport bounds and clipping `overflow-x`/`overflow-y` (`auto|scroll|hidden|clip`) ancestor rects so off-screen scroll-clipped buttons do not trigger false overlaps; added unit test coverage.
  - `crates/vox-gui/ui/e2e/lib/tauriMock.ts` & `crates/vox-gui/ui/e2e/console-workbench.spec.ts`: emitted `'vox://term-event'` (`BlockOpened`, `OutputAppended`, `BlockClosed`, `BlockPinned`, `SecurityLevelChanged`, `ControlModeChanged`) on `term_session_submit`/`term_agent_prompt`/`term_slash_exec`/`term_session_pin`/`term_session_set_*`, seeded `s-1` with `control_mode: 'manual'`, and updated `console-workbench.spec.ts:71-72` to interact with the Radix `<VoxSelect>` combobox.
- **Promoted `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` (`status: "current"`):**
  - Updated baseline/target phrasing across Sections 1–3 to reflect shipped `v7.5` behavior and appended Section 4 documenting `--terminal-well-*` tokens, `--font-terminal-mono` / `:not(.xterm)` containment, all `64` WCAG ANSI-16 colors, `terminalAudits.ts` thresholds, and measured `ledger-after.json` `5×4` CV convergence metrics.

## 2. Final Verification Results (Post-Final-Edit Runs)

### Vitest (`crates/vox-gui/ui`)
- `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run --maxWorkers=4 e2e/review/audits.test.ts e2e/review/terminalAudits.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/lib/themeContract.test.ts src/guards/` -> **exit `0`**, `36` test files passed, `211` tests passed (`8.84s`).
- `timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run --maxWorkers=4 src/lib/cvFlywheel.test.ts` -> **exit `0`**, `1` test file passed, `4` tests passed (`593ms`, run after final `cvFlywheel.ts` edit).

### UI Build & Typecheck
- `timeout 180s pnpm --dir crates/vox-gui/ui build` (`tokens:build`, `tsc --noEmit`, `vite build`) -> **exit `0`** (`3352 modules transformed`, built in `770ms`).

### Playwright E2E (`--project=chromium`)
- `timeout 180s pnpm --dir crates/vox-gui/ui exec playwright test e2e/console-workbench.spec.ts --project=chromium` -> **exit `0`**, `2 passed (3.5s)`.
- `timeout 180s env VOX_CV_FLYWHEEL=1 pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/cv-flywheel.spec.ts -g "Phase 386" --project=chromium` -> **exit `0`**, `1 passed (5.3s)` (`23 / 23 Verified`, `ENFORCED · ZERO DRIFT`; note `-g "Settings"` finds 0 tests because the test title at `cv-flywheel.spec.ts:1229` is `"Phase 386: Enforceable Theme Contract Observability Ledger & Runtime CSS Token Parity (arcane + slate)"`).
- `timeout 300s env VOX_CV_FLYWHEEL=1 CV_STAGE=after pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/terminal-cv-flywheel.spec.ts --project=chromium --workers=3` -> **exit `0`**, `8 passed (1.4m)`.

### Measured `ledger-after.json` Convergence (`crates/vox-gui/ui/test-results/cv-flywheel-terminal/ledger-after.json`)
- **Summary:** `ssotVersion: "2026.10.19-v3.5"`, `totalCells: 112`, `passedCells: 112`, `failedCells: 0`, `minVisualAfterScore: 0.9282`, `minVlmOverallScore: 0.9506`.
- **By Theme (`n = 28` each):**
  - `arcane`: min `0.9302`, mean `0.9428`, max `0.9665`, minVlm `0.9664`
  - `slate`: min `0.9282`, mean `0.9361`, max `0.9402`, minVlm `0.9506`
  - `travertine`: min `0.9286`, mean `0.9423`, max `0.9665`, minVlm `0.9666`
  - `high-contrast`: min `0.9394`, mean `0.9458`, max `0.9513`, minVlm `0.9674`
- **By Viewport:**
  - `900x600` (`32`): min `0.9331`, mean `0.9400`, max `0.9513`, minVlm `0.9528`
  - `1100x720` (`16`): min `0.9323`, mean `0.9423`, max `0.9581`, minVlm `0.9508`
  - `1280x800` (`16`): min `0.9310`, mean `0.9434`, max `0.9665`, minVlm `0.9512`
  - `1440x900` (`32`): min `0.9298`, mean `0.9418`, max `0.9665`, minVlm `0.9517`
  - `1920x1080` (`16`): min `0.9282`, mean `0.9428`, max `0.9665`, minVlm `0.9506`
- **By State:**
  - `block-stream-ansi` (`20`): min `0.9310`, mean `0.9450`, max `0.9665`, minVlm `0.9539`
  - `split-panes` (`20`): min `0.9286`, mean `0.9363`, max `0.9492`, minVlm `0.9537`
  - `history-search-open` (`8`): min `0.9309`, mean `0.9372`, max `0.9491`, minVlm `0.9525`
  - `json-ledger-table` (`8`): min `0.9309`, mean `0.9372`, max `0.9496`, minVlm `0.9532`
  - `runbooks-open` (`8`): min `0.9338`, mean `0.9450`, max `0.9608`, minVlm `0.9558`
  - `approval-card-pending` (`8`): min `0.9307`, mean `0.9374`, max `0.9496`, minVlm `0.9523`
  - `auto-tui-alt-screen` (`20`): min `0.9282`, mean `0.9497`, max `0.9665`, minVlm `0.9536`
  - `chat-terminal-dock` (`20`): min `0.9308`, mean `0.9401`, max `0.9493`, minVlm `0.9506`

### Static Gates
- `timeout 60s ./target/debug/vox-doc-pipeline --lint-only --paths architecture/warp-roman-terminal-v75-research-2026.md` -> **exit `0`** (`vox-doc-pipeline lint complete — no hard errors.`)
- `timeout 60s ./target/debug/toestub --rules vox/ui crates/vox-gui/ui/src` -> **exit `0`** (`0 critical, 0 error, 12 warning, 0 info`; pre-existing `ResizeObserver` warnings only)
- `timeout 60s ./target/debug/toestub --rules vox/llm-provider-call crates/vox-gui/src` -> **exit `0`** (`No issues found — code is clean!`)
- `timeout 180s ./target/debug/vox-arch-check` -> **exit `0`**

## 3. Non-Blank LoC Counts (`rg -c "\S"`)

| File | Non-Blank LoC | Budget Status |
| --- | --- | --- |
| `crates/vox-gui/ui/e2e/review/terminal-cv-flywheel.spec.ts` | `326` | `< 360` |
| `crates/vox-gui/ui/e2e/review/audits.ts` | `390` | `< 435` |
| `crates/vox-gui/ui/e2e/review/audits.test.ts` | `181` | `< 435` |
| `crates/vox-gui/ui/e2e/review/terminalAudits.test.ts` | `207` | `< 435` |
| `crates/vox-gui/ui/e2e/review/states.ts` | `260` | `< 435` |
| `crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts` | `367` | `< 435` |
| `crates/vox-gui/ui/e2e/console-workbench.spec.ts` | `121` | `< 435` |
| `crates/vox-gui/ui/src/lib/cvFlywheel.ts` | `431` | `< 435` |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx` | `356` | `< 435` |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx` | `398` | `< 435` |
| `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx` | `219` | `< 435` |
| `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` | `363` | `< 435` |
| `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx` | `412` | `< 435` |
| `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx` | `367` | `< 435` |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx` | `428` | `< 435` |
| `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx` | `191` | `< 435` |
| `crates/vox-gui/ui/e2e/lib/tauriMock.ts` | `2296` | pre-existing mock file (`+32` net lines) |
| `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` | `110` | Architecture SSOT |

## 4. Handoff Notes

- `.planning/workstreams/axis-warp-terminal-v75/ROADMAP.md` was left untouched per coordinator instructions (`[x]` Phase 446 / Plan `446-04` update owned by parent coordinator).
- No `git` mutating commands or `cargo` commands were run; all changes remain uncommitted in the working tree for coordinator review/commit.
