---
phase: 446-roman-scriptorium-theme-and-cv-flywheel
plan: 01
subsystem: ui
tags: [style-dictionary, tailwind-v4, xterm, theme-contract, roman-scriptorium, css-containment]

# Dependency graph
requires:
  - phase: 386-enforceable-theme-contract
    provides: contracts/gui/theme-contract.v1.json, src/lib/themeContract.ts, and vox/ui/theme-contract-drift static enforcement
provides:
  - Style Dictionary --font-terminal-mono token ('JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace) across all 4 theme CSS bundles and tokens.generated.ts
  - Synchronized 23-variable required_semantic_css_vars SSOT in contracts/gui/theme-contract.v1.json, src/lib/themeContract.ts, src/lib/themeContract.test.ts, and e2e/review/cv-flywheel.spec.ts ('23 / 23 Verified')
  - Dual-Surface --terminal-well-{bg,fg,muted,border,cursor,selection} variables across arcane, glacier/travertine, void/slate, and high-contrast in src/index.css
  - Full :not(.xterm) / :not([data-terminal-mono="true"]) / :not([data-terminal-well="true"]) typography, button, table, input, and light/dark normalizer containment plus .xterm-helper-textarea zero-box lock
affects:
  - 446-02-PLAN (Console & Chat Dual-Surface component remediation)
  - 446-03-PLAN (Terminal CV Flywheel audits & 5x4 convergence matrix)

actuals:
  tokens: 14800
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Dual-Surface terminal CSS containment via --font-terminal-mono, --terminal-well-*, and :not(.xterm):not(.xterm *):not([data-terminal-mono='true']):not([data-terminal-mono='true'] *):not([data-terminal-well='true']):not([data-terminal-well='true'] *) exclusions"
    - "Zero-box .xterm-helper-textarea isolation preventing focus-ring and light-theme form background bleed"

key-files:
  created: []
  modified:
    - crates/vox-gui/ui/tokens/primitive.json
    - crates/vox-gui/ui/src/styles/tokens.generated.css
    - crates/vox-gui/ui/src/styles/tokens.slate.generated.css
    - crates/vox-gui/ui/src/styles/tokens.contrast.generated.css
    - crates/vox-gui/ui/src/styles/tokens.travertine.generated.css
    - crates/vox-gui/ui/src/styles/tokens.generated.ts
    - contracts/gui/theme-contract.v1.json
    - crates/vox-gui/ui/src/lib/themeContract.ts
    - crates/vox-gui/ui/src/lib/themeContract.test.ts
    - crates/vox-gui/ui/src/index.css
    - crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts

key-decisions:
  - "Used exact allowlisted monospace stack 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace with zero banned Romanesque font literals"
  - "Excluded :not(.xterm), :not([data-terminal-mono='true']), and :not([data-terminal-well='true']) across @layer base display/mono/button rules AND both light-theme and dark-theme normalizers in src/index.css so terminal spans and inline path/URL buttons remain fixed-width and unboxed"

patterns-established:
  - "Terminal output wells and monospace blocks consume var(--font-terminal-mono) and var(--terminal-well-*) while outer workbench chrome retains Roman Scriptorium Cinzel/Palatino stone styling"

requirements-completed:
  - TERM-ROMA-CV-01

coverage:
  - id: D1
    description: "Style Dictionary emits --font-terminal-mono across all 4 generated theme CSS files and tokens.generated.ts, and 23-variable theme contract parity holds across theme-contract.v1.json, themeContract.ts, and cv-flywheel.spec.ts ('23 / 23 Verified')"
    requirement: TERM-ROMA-CV-01
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/lib/themeContract.test.ts#defines all 23 required semantic CSS variables individually in every generated Style Dictionary theme file (arcane, slate, high-contrast, travertine)"
        status: pass
      - kind: other
        ref: "./target/debug/toestub --rules vox/ui/theme-contract-drift crates/vox-gui/ui/src"
        status: pass
    human_judgment: false
  - id: D2
    description: "src/index.css defines Dual-Surface --terminal-well-{bg,fg,muted,border,cursor,selection} variables across all 4 themes, isolates .xterm / [data-terminal-mono='true'] / [data-terminal-well='true'] from proportional typography and light/dark normalizers, and locks .xterm-helper-textarea to an invisible 0x0 box"
    requirement: TERM-ROMA-CV-01
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/lib/themeContract.test.ts#enforces Dual-Surface --terminal-well-* variables, terminal monospace isolation, and .xterm-helper-textarea zero-box containment in index.css"
        status: pass
    human_judgment: false

duration: 18min
completed: 2026-11-15
status: complete
---

# Phase 446 Plan 01: Roman Scriptorium Typography & CSS Containment Summary

**End-to-end `--font-terminal-mono` Style Dictionary token pipeline, 23-variable GUI theme contract synchronization, and Dual-Surface `--terminal-well-*` / `.xterm-helper-textarea` CSS containment in `src/index.css`**

## Performance

- **Duration:** 18 min
- **Started:** 2026-11-15T21:45:55Z
- **Completed:** 2026-11-15T22:03:30Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments
- Added `font.terminal-mono` and `font.family.terminal-mono` (`'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace`) to `crates/vox-gui/ui/tokens/primitive.json` and regenerated `tokens.generated.css`, `tokens.slate.generated.css`, `tokens.contrast.generated.css`, `tokens.travertine.generated.css`, and `tokens.generated.ts` via `pnpm tokens:build` with zero banned font literals.
- Synchronized `contracts/gui/theme-contract.v1.json` (surgical `+10/-5` diff), `crates/vox-gui/ui/src/lib/themeContract.ts`, `crates/vox-gui/ui/src/lib/themeContract.test.ts`, and `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` (`'23 / 23 Verified'`) around the expanded 23-variable `REQUIRED_SEMANTIC_CSS_VARS` contract.
- Declared calibrated Dual-Surface `--terminal-well-{bg,fg,muted,border,cursor,selection}` custom properties across `arcane`, `glacier`/`travertine`, `void`/`slate`, and `high-contrast` in `crates/vox-gui/ui/src/index.css`.
- Guarded `@layer base` proportional `Cinzel`/`Palatino` typography, 3D marble button chrome, `*:focus-visible`, and both light-theme (`arcane`/`travertine`) and dark-theme (`slate`/`void`/`high-contrast`) normalizer rules with `:not(.xterm):not(.xterm *):not([data-terminal-mono="true"]):not([data-terminal-mono="true"] *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)` and `:not(.xterm-helper-textarea)`, plus an explicit zero-box `.xterm-helper-textarea` containment lock.

## Task Commits

Git commits intentionally omitted in shared worktree per `00-core.md` Rule 6 and `446-01-PLAN.md` Executor Git Rule (parent orchestrator verifies and commits atomically).

## Files Created/Modified
- `crates/vox-gui/ui/tokens/primitive.json` - Added `font.terminal-mono` and `font.family.terminal-mono`
- `crates/vox-gui/ui/src/styles/tokens.generated.css` - Regenerated `:root, [data-theme="arcane"]` tokens including `--font-terminal-mono`
- `crates/vox-gui/ui/src/styles/tokens.slate.generated.css` - Regenerated `slate`/`void` tokens including `--font-terminal-mono`
- `crates/vox-gui/ui/src/styles/tokens.contrast.generated.css` - Regenerated `high-contrast` tokens including `--font-terminal-mono`
- `crates/vox-gui/ui/src/styles/tokens.travertine.generated.css` - Regenerated `glacier`/`travertine` tokens including `--font-terminal-mono`
- `crates/vox-gui/ui/src/styles/tokens.generated.ts` - Regenerated nested TypeScript token object
- `contracts/gui/theme-contract.v1.json` - Appended `--font-terminal-mono` to `required_semantic_css_vars` (23 total) and all 4 theme `tokens` maps
- `crates/vox-gui/ui/src/lib/themeContract.ts` - Appended `--font-terminal-mono` to `REQUIRED_SEMANTIC_CSS_VARS` and all 4 `THEME_CONTRACT.themes` profiles
- `crates/vox-gui/ui/src/lib/themeContract.test.ts` - Added TDD assertions for 23 semantic CSS variables, generated bundle parity, and `index.css` Dual-Surface / `.xterm-helper-textarea` containment
- `crates/vox-gui/ui/src/index.css` - Added `--font-terminal-mono`, 4-theme `--terminal-well-*` variables, terminal monospace rule, `.xterm-helper-textarea` zero-box lock, and `:not(...)` exclusions across `@layer base`, light, and dark normalizers
- `crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts` - Updated Settings semantic token ledger assertion from `'22 / 22 Verified'` to `'23 / 23 Verified'`

## Verification Proof Outputs

### 1. Style Dictionary Token Build
Command: `timeout 120s pnpm --dir crates/vox-gui/ui run tokens:build` (exit `0`)
```text
$ node style-dictionary.config.mjs

css
✔︎ src/styles/tokens.generated.css

ts
✔︎ src/styles/tokens.generated.ts

css
✔︎ src/styles/tokens.slate.generated.css

css
✔︎ src/styles/tokens.contrast.generated.css

css
✔︎ src/styles/tokens.travertine.generated.css
tokens built: tokens.generated.css, tokens.slate.generated.css, tokens.contrast.generated.css, tokens.travertine.generated.css, tokens.generated.ts
```

### 2. Vitest Theme Contract & Guard Suite (TDD RED -> GREEN)
- Task 1 RED (`vitest run src/lib/themeContract.test.ts`): failed as expected (`AssertionError: expected [ '--brass', '--color-bg-base', …(20) ] to include '--font-terminal-mono'`) before token/contract updates.
- Task 2 RED (`vitest run src/lib/themeContract.test.ts`): failed as expected (`AssertionError: Expected --terminal-well-bg across all 4 theme blocks in src/index.css: expected 0 to be greater than or equal to 4`) before `src/index.css` updates.
- Final GREEN (`timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/lib/themeContract.test.ts src/guards/zeroEmojiAndDingbat.test.ts`, exit `0`):
```text
 ✓ src/guards/zeroEmojiAndDingbat.test.ts (2 tests) 224ms
 ✓ src/lib/themeContract.test.ts (8 tests) 468ms

 Test Files  2 passed (2)
      Tests  10 passed (10)
```

### 3. TypeScript Typecheck
Command: `timeout 180s pnpm --dir crates/vox-gui/ui exec tsc --noEmit` (exit `0`, 0 errors)

### 4. Static Code Audit (`vox/ui/theme-contract-drift`)
- Unscoped `./target/debug/vox audit code` across the full Rust+TS monorepo hit the 300s timeout (`exit 124`), so per Rule 9 it was not retried unscoped; instead the prebuilt standalone `toestub` binary (`vox-code-audit` CLI) was invoked directly on `crates/vox-gui/ui/src`:
Command: `timeout 60s ./target/debug/toestub --rules vox/ui/theme-contract-drift crates/vox-gui/ui/src` (exit `0`)
```text
✅ No issues found — code is clean!
```
Command: `timeout 60s ./target/debug/toestub --rules vox/ui crates/vox-gui/ui/src` (exit `0`, `0 critical, 0 error`)

### 5. Surgical Contract Diff Check
Command: `git diff --numstat -- contracts/gui/theme-contract.v1.json`
```text
10	5	contracts/gui/theme-contract.v1.json
```

## Decisions Made
- Included `[data-terminal-well="true"], [data-terminal-well="true"] *` alongside `.xterm, .xterm *, [data-terminal-mono="true"], [data-terminal-mono="true"] *` in the `@layer base` terminal monospace rule and guarded `@layer base` light-theme 3D button rules (`L378-401`) so any `<pre>` or inline clickable path/URL `<button>` inside a terminal well stays fixed-width and unboxed across all 4 themes.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Unscoped `./target/debug/vox audit code` timed out at 300s (`exit 124`) scanning the entire workspace Rust AST; verified `vox/ui/theme-contract-drift` directly via `./target/debug/toestub --rules vox/ui/theme-contract-drift crates/vox-gui/ui/src` (`exit 0`, `✅ No issues found — code is clean!`) plus Vitest's `auditSourceTextAgainstThemeContract` full-`src/` sweep (`8/8` tests passing).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `--font-terminal-mono`, 23-var theme contract parity, and `src/index.css` Dual-Surface `--terminal-well-*` + `:not(.xterm)` + `.xterm-helper-textarea` containment are live and ready for Plan `446-02` (`Console/*` & `ChatTerminalBlockCard` Dual-Surface component remediation) and Plan `446-03` (5x4 CV Flywheel verification).

---
*Phase: 446-roman-scriptorium-theme-and-cv-flywheel*
*Completed: 2026-11-15*
