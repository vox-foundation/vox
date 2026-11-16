---
phase: 446-roman-scriptorium-theme-and-cv-flywheel
plan: 03
subsystem: ui
tags: [cv-flywheel, terminal-audits, vlm-vision, playwright-fixtures, roman-scriptorium, wcag-contrast, xterm]

# Dependency graph
requires:
  - phase: 446-roman-scriptorium-theme-and-cv-flywheel
    provides: Style Dictionary --font-terminal-mono token, Dual-Surface --terminal-well-* CSS variables, and .xterm-helper-textarea zero-box containment (446-01)
provides:
  - Self-contained browser-serializable collectTerminalAuditMetrics & runTerminalDomAudits enforcing 4 terminal invariants (terminalStartupSpamCount, xtermHelperLeakCount, terminalFontProportionalLeakCount, terminalContrastViolationCount)
  - Universal CV Flywheel rubric deductions (>= 0.30 penalty per terminal defect class, capping at 0.60) and bootstrap remediation directives in src/lib/cvFlywheel.ts
  - Dual-Surface terminal well (.xterm / [data-terminal-well="true"] / .xterm-helper-textarea) exemptions in e2e/review/audits.ts and VLM rubric calibration across Carrara/Travertine/Slate/High-Contrast in e2e/review/vlmVisionFlywheel.ts
  - Deterministic rich terminal IPC mock (terminalRichFixtures.ts + tauriMockRich.ts) and 8 terminal CV states (7 Console + 1 Chat dock) in e2e/review/states.ts
affects:
  - 446-04-PLAN (Playwright 5x4 CV Flywheel convergence matrix execution across Console and Chat)

actuals:
  tokens: 28500
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "100% self-contained .toString() browser serialization for page.evaluate / addInitScript collectors and rich IPC mocks (verified via new Function standalone evaluation in Vitest)"
    - "Dual-Surface WCAG 2.1 contrast auditing (4.5:1 standard, 7.0:1 AAA on high-contrast) + canvas 'MMMMMMMMMM' vs 'iiiiiiiiii' fixed-width glyph verification"
    - "Smart 16-color SGR fixture pairing ANSI black (\\x1b[30;47m) with a light cell background on dark terminal wells while rendering \\x1b[31..37m and \\x1b[90..97m directly against --terminal-well-bg"

key-files:
  created:
    - crates/vox-gui/ui/e2e/review/terminalAudits.ts
    - crates/vox-gui/ui/e2e/review/terminalAudits.test.ts
    - crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts
  modified:
    - crates/vox-gui/ui/src/lib/cvFlywheel.ts
    - crates/vox-gui/ui/e2e/review/audits.ts
    - crates/vox-gui/ui/e2e/review/audits.test.ts
    - crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts
    - crates/vox-gui/ui/e2e/lib/tauriMockRich.ts
    - crates/vox-gui/ui/e2e/review/states.ts

key-decisions:
  - "Kept collectTerminalAuditMetrics, auditDomQualityInPage, buildTerminalRichFixtures, and installTerminalRichMock 100% self-contained with zero outer-scope references, adding explicit new Function('return (' + fn.toString() + ')')() tests in Vitest to prevent Chromium ReferenceError regressions"
  - "Paired ANSI color 0 black foreground with light cell background (\\x1b[30;47m) in Block 1 SGR output so all 16 SGR slots exercise real contrast without manufacturing a synthetic black-on-charcoal WCAG failure"
  - "Calibrated VLM rubric in vlmVisionFlywheel.ts so recessed dark terminal wells inside Carrara/Travertine stone chrome and Inter/flat-chrome accessibility overrides in Slate/High-Contrast are scored accurately without false daylight/serif penalties"

patterns-established:
  - "Browser-injected audit and mock functions must pass standalone new Function serialization tests with zero module-scope closures"
  - "Terminal CV states toggle deterministic window.__MOCK_TERM_* flags inside Playwright state setup hooks while preserving VIEWPORTS (['wide', 'laptop', 'compact']) untouched"

requirements-completed:
  - TERM-ROMA-CV-03

coverage:
  - id: D1
    description: "Self-contained collectTerminalAuditMetrics & runTerminalDomAudits in terminalAudits.ts enforcing startup spam, .xterm-helper-textarea zero-box, monospace fixed-width glyph invariance, and WCAG 2.1 contrast (4.5:1 / 7.0:1 AAA)"
    requirement: TERM-ROMA-CV-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/e2e/review/terminalAudits.test.ts"
        status: pass
    human_judgment: false
  - id: D2
    description: "Universal CV Flywheel rubric terminal defect penalties (>= 0.30 deduction -> visualAfterScore < 0.92) in cvFlywheel.ts, Dual-Surface exemptions in audits.ts, and VLM plate/signal calibration in vlmVisionFlywheel.ts"
    requirement: TERM-ROMA-CV-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/e2e/review/terminalAudits.test.ts"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/e2e/review/audits.test.ts"
        status: pass
    human_judgment: false
  - id: D3
    description: "Self-contained pure-ASCII terminalRichFixtures.ts wired into tauriMockRich.ts seeding 5 rich blocks (16-color SGR, failed exit 101, JSON ledger stdout, AgentTurn, /budget SlashCommand), runbooks, history, completions, apr-term-446 approval, and chat vox_terminal_exec receipt"
    requirement: TERM-ROMA-CV-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/e2e/review/terminalAudits.test.ts"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/guards/zeroEmojiAndDingbat.test.ts"
        status: pass
    human_judgment: false
  - id: D4
    description: "8 terminal CV states registered in SURFACE_STATES['console'] (7 states), SURFACE_STATES['chat'] ('chat-terminal-dock'), and TERMINAL_CV_STATES while preserving VIEWPORTS untouched"
    requirement: TERM-ROMA-CV-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/guards/reviewStates.guard.test.ts"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/e2e/review/terminalAudits.test.ts"
        status: pass
    human_judgment: false

duration: 35min
completed: 2026-11-15
status: complete
---

# Phase 446 Plan 03: Terminal CV Flywheel Audits, VLM Calibration & Rich State Fixtures Summary

**Self-contained DOM/WCAG/monospace terminal invariant detectors, CV/VLM Dual-Surface rubric calibration, pure-ASCII rich PTY/Agent/Approval/Chat IPC fixtures, and 8 terminal CV review states (`TERM-ROMA-CV-03`)**

## Performance

- **Duration:** 35 min
- **Started:** 2026-11-15T22:06:00Z
- **Completed:** 2026-11-15T22:48:00Z
- **Tasks:** 2
- **Files modified:** 9 (3 created, 6 modified; all strictly `< 440` non-blank LoC)

## Accomplishments
- Created `crates/vox-gui/ui/e2e/review/terminalAudits.ts` (`270` non-blank LoC) with 100% self-contained browser-serializable `collectTerminalAuditMetrics(doc?: Document)` and Playwright helper `runTerminalDomAudits(page)` auditing:
  1. `terminalStartupSpamCount`: raw OSC 133 (`\x1b]133;`, `]133;[ABCD]`) and shell bootstrap leakage (`__vox_(ps|preexec|precmd|trappost)`, `PROMPT_COMMAND=`, `precmd_functions`, `chpwd_functions`, `export PS1=`).
  2. `xtermHelperLeakCount`: `.xterm-helper-textarea` bounding box `> 1px`, `opacity > 0.01`, non-transparent `backgroundColor`, outline, or `boxShadow`.
  3. `terminalFontProportionalLeakCount`: proportional serif font stacks (`Cinzel`, `Palatino`, `Book Antiqua`, `Georgia`, `Times New Roman`) OR 2D canvas `'MMMMMMMMMM'` vs `'iiiiiiiiii'` glyph width delta `> 0.1px` inside terminal output wells.
  4. `terminalContrastViolationCount`: WCAG 2.1 relative luminance with ancestor background alpha compositing + 1x1 canvas CSS Color 4 (`oklch`/`color-mix`) fallback, flagging `< 4.5:1` on `arcane`/`travertine`/`glacier`/`slate`/`void` and `< 7.0:1` AAA on `high-contrast`.
- Extended `crates/vox-gui/ui/src/lib/cvFlywheel.ts` (`428` non-blank LoC) with optional terminal audit fields on `GuiDomAuditMetrics` and `afterDom` on `UniversalCvFlywheelInput`, deducting `Math.min(0.6, terminalDefectCount * 0.3)` across `technicalQuality`, `promptAdherence`, and `styleConsistency` (forcing `visualAfterScore < 0.92` on any single defect) and emitting targeted remediation bootstrap directives.
- Updated `crates/vox-gui/ui/e2e/review/audits.ts` (`360` non-blank LoC) and `crates/vox-gui/ui/e2e/review/audits.test.ts` (`150` non-blank LoC) so `auditDomQualityInPage` remains 100% self-contained while skipping `.xterm-helper-textarea` across interactive/select/glyph/backtick walks and exempting intentional recessed dark terminal wells (`.xterm, [data-terminal-well="true"]`) from `lightThemeContrastLeakCount` and `digitalColorLeakCount`.
- Calibrated `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts` (`405` non-blank LoC): mapped `console`/`terminal`/`chat` in `conceptArtPlateForSurface` to `v74-concept-bento-workbench-${variant}.jpg`, excluded `.xterm` / `[data-terminal-mono="true"]` / `[data-terminal-well="true"]` from Romanesque `fontTargets`, accounted for Dual-Surface dark terminal wells on daylight themes, and honored `Inter` + flat-chrome accessibility overrides on `slate` and `high-contrast`.
- Created pure-ASCII, self-contained `crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts` (`367` non-blank LoC) and wired it into `crates/vox-gui/ui/e2e/lib/tauriMockRich.ts` (`318` non-blank LoC), seeding sessions `s-1` (5 rich blocks including smart 16-color SGR with `\x1b[30;47m` for ANSI black on light cell bg, failed `exit_code: 101`, JSON ledger array stdout, `AgentTurn` with `VOX_PROPOSED_COMMAND`, and `/budget` `SlashCommand`) and `s-2`, plus deterministic handlers for `term_session_*`, `term_history_*`, `term_complete`, `term_runbooks_list`, `term_agent_*`, `term_slash_exec`, `term_block_feedback`, `invoke_mcp_tool` (`vox_pending_approvals` -> `apr-term-446` when `__MOCK_TERM_APPROVAL_ACTIVE__ === true`), and `chat_get_messages` (`vox_terminal_exec` receipt when `__MOCK_CHAT_TERM_DOCK__ === true`).
- Registered 7 Console states (`block-stream-ansi`, `split-panes`, `history-search-open`, `json-ledger-table`, `runbooks-open`, `approval-card-pending`, `auto-tui-alt-screen`), 1 Chat dock state (`chat-terminal-dock`), and exported `TerminalCvStateSpec` + `TERMINAL_CV_STATES` (8 states) in `crates/vox-gui/ui/e2e/review/states.ts` (`251` non-blank LoC) with `VIEWPORTS` (`['wide', 'laptop', 'compact']`) untouched.

## Task Commits

Git commits intentionally omitted in shared worktree per `00-core.md` Rule 6 and `446-03-PLAN.md` Executor Rules (parent orchestrator verifies and commits atomically).

## Files Created/Modified
- `crates/vox-gui/ui/e2e/review/terminalAudits.ts` (`270` non-blank LoC) - Self-contained terminal DOM/WCAG/monospace/helper-box audit collector + Playwright wrapper
- `crates/vox-gui/ui/e2e/review/terminalAudits.test.ts` (`207` non-blank LoC) - 7 jsdom unit tests covering standalone `.toString()` serialization, all 4 terminal invariants, `cvFlywheel` rubric deductions, and `richMockInitScript` / `TERMINAL_CV_STATES` wiring
- `crates/vox-gui/ui/src/lib/cvFlywheel.ts` (`428` non-blank LoC) - Optional terminal fields on `GuiDomAuditMetrics`, `>= 0.30` rubric penalty, and terminal bootstrap directives
- `crates/vox-gui/ui/e2e/review/audits.ts` (`360` non-blank LoC) - Self-contained `.xterm-helper-textarea` skip and `.xterm` / `[data-terminal-well="true"]` Dual-Surface exemptions
- `crates/vox-gui/ui/e2e/review/audits.test.ts` (`150` non-blank LoC) - Dual-Surface exemption + standalone `new Function` serialization unit tests
- `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts` (`405` non-blank LoC) - Concept plate mapping, terminal monospace exclusion from Romanesque font scan, and Dual-Surface / Slate / High-Contrast VLM signal calibration
- `crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts` (`367` non-blank LoC) - Pure-ASCII self-contained rich terminal sessions, 5 block archetypes, runbooks, approvals, and chat terminal receipt fixtures
- `crates/vox-gui/ui/e2e/lib/tauriMockRich.ts` (`318` non-blank LoC) - Injects `buildTerminalRichFixtures` and `installTerminalRichMock` inside `richMockInitScript`
- `crates/vox-gui/ui/e2e/review/states.ts` (`251` non-blank LoC) - 7 `SURFACE_STATES['console']` entries, `chat-terminal-dock` on `SURFACE_STATES['chat']`, and `TERMINAL_CV_STATES` export

## Verification Proof Outputs

### 1. TDD RED -> GREEN Vitest Suite
- **TDD RED** (`timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run e2e/review/terminalAudits.test.ts` before creating `terminalAudits.ts`, exit `1`):
```text
 FAIL  e2e/review/terminalAudits.test.ts [ e2e/review/terminalAudits.test.ts ]
Error: Failed to resolve import "./terminalAudits" from "e2e/review/terminalAudits.test.ts". Does the file exist?
```
- **TDD GREEN** (`timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run e2e/review/terminalAudits.test.ts e2e/review/audits.test.ts src/guards/reviewStates.guard.test.ts src/guards/zeroEmojiAndDingbat.test.ts`, exit `0`):
```text
 ✓ src/guards/reviewStates.guard.test.ts (4 tests) 5ms
 ✓ src/guards/zeroEmojiAndDingbat.test.ts (2 tests) 328ms
 ✓ e2e/review/audits.test.ts (7 tests) 285ms
 ✓ e2e/review/terminalAudits.test.ts (7 tests) 338ms

 Test Files  4 passed (4)
      Tests  20 passed (20)
   Duration  3.49s
```
- **Theme Contract Regression Check** (`timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/themeContract.test.ts`, exit `0`):
```text
 ✓ src/lib/themeContract.test.ts (8 tests)
 Test Files  1 passed (1)
      Tests  8 passed (8)
```

### 2. TypeScript Typecheck
Command: `timeout 180s pnpm --dir crates/vox-gui/ui exec tsc --noEmit` (exit `0`, 0 errors)

### 3. Non-Blank LoC Ceiling (`< 440` across all 9 files)
Command: `rg -c "\S" crates/vox-gui/ui/src/lib/cvFlywheel.ts crates/vox-gui/ui/e2e/review/terminalAudits.ts crates/vox-gui/ui/e2e/review/terminalAudits.test.ts crates/vox-gui/ui/e2e/review/audits.ts crates/vox-gui/ui/e2e/review/audits.test.ts crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts crates/vox-gui/ui/e2e/lib/tauriMockRich.ts crates/vox-gui/ui/e2e/review/states.ts`
```text
crates/vox-gui/ui/e2e/review/audits.test.ts:150
crates/vox-gui/ui/e2e/review/states.ts:251
crates/vox-gui/ui/e2e/review/audits.ts:360
crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts:405
crates/vox-gui/ui/e2e/review/terminalAudits.test.ts:207
crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts:367
crates/vox-gui/ui/src/lib/cvFlywheel.ts:428
crates/vox-gui/ui/e2e/lib/tauriMockRich.ts:318
crates/vox-gui/ui/e2e/review/terminalAudits.ts:270
```

### 4. Pure 7-Bit ASCII Fixture Check
Command: `rg -n "[^\x00-\x7F]" crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts` (exit `1`, 0 matches — zero emojis, dingbats, or Unicode middle dots)

## Decisions Made
- Guarded canvas `getContext('2d')` probing inside `collectTerminalAuditMetrics` when running under unmocked jsdom (`navigator.userAgent` matching `/jsdom/i` without a Vitest spy on `HTMLCanvasElement.prototype.getContext`) while still running the full canvas glyph width probe whenever real Chromium or a mocked 2D context is active, keeping Vitest stderr completely clean.
- Made `approval-card-pending` in `states.ts` set `__MOCK_TERM_APPROVAL_ACTIVE__ = true`, await `term-session-bar`, and gracefully wait for `term-approval-card` (`timeout: 1_500`, `.catch(() => {})`) so the state works seamlessly both before and after Phase 445's `TerminalApprovalCard` merges.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Initial edits to `cvFlywheel.ts` (`458` non-blank LoC) and `vlmVisionFlywheel.ts` (`461` non-blank LoC) exceeded the `< 440` non-blank LoC ceiling; compacted multi-line JSDoc headers and object literals down to `428` and `405` non-blank LoC respectively with zero behavioral or type-signature changes.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All terminal DOM/WCAG/monospace audit collectors, CV/VLM Dual-Surface rubrics, rich terminal IPC fixtures, and 8 terminal CV states (`TERMINAL_CV_STATES`) are verified and ready for Plan `446-04` (`terminal-cv-flywheel.spec.ts` 5x4 Playwright convergence matrix).

---
*Phase: 446-roman-scriptorium-theme-and-cv-flywheel*
*Completed: 2026-11-15*
