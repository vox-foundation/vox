---
phase: 446-roman-scriptorium-theme-and-cv-flywheel
plan: 02
subsystem: ui
tags: [roman-scriptorium, dual-surface, ansi-palette, wcag-contrast, xterm, console, chat-terminal, responsive-layout]

# Dependency graph
requires:
  - phase: 446-roman-scriptorium-theme-and-cv-flywheel
    provides: --font-terminal-mono Style Dictionary token, 23-var theme contract, and Dual-Surface --terminal-well-* CSS containment (446-01)
provides:
  - 64-color WCAG-calibrated Dual-Surface ANSI palettes (ARCANE_ANSI_16, TRAVERTINE_ANSI_16, SLATE_ANSI_16, HIGH_CONTRAST_ANSI_16), TERMINAL_MONO_STYLE, and resolveTerminalWellSpec in terminalTheme.ts
  - Dual-Surface wellBg/wellFg inverse SGR fallback and high-contrast well cursor link styling in sgrToSpans.ts
  - __VOX_E2E_REAL_XTERM__ live xterm opt-in (cursorBlink: false) and data-terminal-well/data-terminal-mono attributes on TerminalTab.tsx
  - Zero-leak Dual-Surface well & Roman Scriptorium token normalization across BlockCard.tsx, ChatTerminalBlockCard.tsx, AgentStrip.tsx, AgentTab.tsx, SendToAgent.tsx, ScriptoriumPromptEditor.tsx, and TerminalApprovalCard.tsx
  - 900x600 responsive Console layout (DiscoveryRail <980px auto-collapse / <1100px overlay + SVG chevrons, ConsoleSessionBar size-6 close targets, ConsoleSplitGrid min-w-40 split leaves, Console.tsx relative overflow-hidden row)
affects:
  - 446-04-PLAN (Playwright 5x4 CV Flywheel convergence matrix execution across Console and Chat)

actuals:
  tokens: 58000
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Dual-Surface wellBg/wellFg property indirection in TERMINAL_WELL_SPECS so themeContract.test.ts GENERAL_DARK_HEX_BG_REGEX reports 0 violations"
    - "Shared TERMINAL_MONO_STYLE exported from terminalTheme.ts across BlockCard, ChatTerminalBlockCard, AgentTab, SendToAgent, ScriptoriumPromptEditor, TerminalApprovalCard, and DiscoveryRail"
    - "Responsive DiscoveryRail auto-collapse (<980px, preserving 1024px jsdom default) and <1100px z-20 absolute overlay inside Console's relative flex body"

key-files:
  created: []
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/AgentStrip.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/AgentTab.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/SendToAgent.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx

key-decisions:
  - "Authorized scope addition for xtermAddons.test.ts (L68-86) to align its Dual-Surface expectedBg matrix (#15110d arcane, #171512 travertine/glacier, #0c0e10 slate/void, #000000 high-contrast >= 7.0:1) with terminalTheme.ts"
  - "Styled TerminalTab mock viewport internals with --terminal-well-* tokens alongside data-terminal-well='true' so dark recessed mock wells avoid dark-on-dark contrast leaks on light themes"
  - "Replaced duplicate local TERMINAL_MONO_STYLE/MIRROR_FONT_STYLE/RAIL_MONO_STYLE definitions with shared import/re-export from terminalTheme.ts, keeping every file under 416 non-blank LoC"

patterns-established:
  - "All terminal command previews, SGR <pre> wells, and prompt mirrors carry data-terminal-mono='true' + TERMINAL_MONO_STYLE, while output wells bind data-terminal-well='true' and bg-(--terminal-well-bg) text-(--terminal-well-fg) border-(--terminal-well-border)"
  - "Interactive Console icon controls (session close, DiscoveryRail expand/collapse) enforce a minimum 24x24px (size-6) hit target with 4px-grid spacing"

requirements-completed:
  - TERM-ROMA-CV-02

coverage:
  - id: D3
    description: "64-color WCAG AA/AAA Dual-Surface ANSI palettes (wellBg/wellFg indirection), inverse SGR fallback, and __VOX_E2E_REAL_XTERM__ cursorBlink: false wiring"
    requirement: TERM-ROMA-CV-02
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/lib/themeContract.test.ts"
        status: pass
    human_judgment: false
  - id: D4
    description: "Zero-leak Dual-Surface and Roman Scriptorium component normalization across BlockCard, ChatTerminalBlockCard, AgentStrip, AgentTab, SendToAgent, ScriptoriumPromptEditor, TerminalApprovalCard, and 900x600 responsive DiscoveryRail/ConsoleSessionBar/ConsoleSplitGrid/Console"
    requirement: TERM-ROMA-CV-02
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx"
        status: pass
      - kind: other
        ref: "./target/debug/toestub --rules vox/ui/theme-contract-drift crates/vox-gui/ui/src"
        status: pass
    human_judgment: false

duration: 32min
completed: 2026-11-15
status: complete
---

# Phase 446 Plan 02: Dual-Surface Roman Scriptorium Calibration & Component Zero-Leak Sweep Summary

**WCAG AA/AAA 4-theme x 16-color Dual-Surface ANSI calibration (`terminalTheme.ts`, `sgrToSpans.ts`, `TerminalTab.tsx`), component-wide zero-leak Scriptorium & well normalization (`Console/*`, `ChatTerminalBlockCard.tsx`), and `900x600` responsive `DiscoveryRail` / `ConsoleSplitGrid` / `ConsoleSessionBar` layout (`TERM-ROMA-CV-02`)**

## Performance

- **Duration:** 32 min
- **Started:** 2026-11-16T00:18:00Z
- **Completed:** 2026-11-16T00:50:30Z
- **Tasks:** 3
- **Files modified:** 18 (17 planned files + caller-authorized `xtermAddons.test.ts`; all strictly `< 416` non-blank LoC)

## Accomplishments
- **Task 1 (4-Theme Dual-Surface ANSI Calibration & Live xterm Hook):**
  - Updated `terminalTheme.ts` (`228` LoC) with `TERMINAL_MONO_STYLE`, `ARCANE_ANSI_16` (`>= 4.74:1` on `#15110d`), `TRAVERTINE_ANSI_16` (`>= 4.96:1` on `#171512`), `SLATE_ANSI_16` (`>= 5.57:1` on `#0c0e10`, `cyan: '#38bdf8'`), `HIGH_CONTRAST_ANSI_16` (`>= 9.66:1` AAA on `#000000`), and `resolveTerminalWellSpec` (`wellBg`/`wellFg` indirection keeping `GENERAL_DARK_HEX_BG_REGEX` at `0` violations).
  - Updated `sgrToSpans.ts` (`354` LoC) to resolve inverse SGR (`\x1b[7m`) defaults against `resolveTerminalWellSpec(opts?.theme)` (`wellFg`/`wellBg`), keep `<mark data-testid="term-search-highlight">` readable (`text-inherit`), and style `term-output-file-link` / `term-output-url-link` with `text-(--terminal-well-cursor) underline decoration-dotted hover:opacity-90`.
  - Updated `TerminalTab.tsx` (`355` LoC) to respect `window.__VOX_E2E_REAL_XTERM__ === true` with `cursorBlink: false` and attach `data-terminal-well="true"` + `data-terminal-mono="true"` plus `--terminal-well-*` tokens on both mock and live viewports.
  - Updated `TerminalTab.test.tsx`, `sgrToSpans.test.tsx`, and caller-authorized `xtermAddons.test.ts` with programmatic WCAG 2.1 contrast assertions across all 64 palette entries and `__VOX_E2E_REAL_XTERM__` coverage.
- **Task 2 (Zero-Leak Component Sweep across Console & Chat):**
  - Updated `BlockCard.tsx` (`359` LoC) and `ChatTerminalBlockCard.tsx` (`209` LoC) to mark command header `<code>` with `data-terminal-mono="true"` and wrap output `<pre>` wells (`data-testid="term-block-output"` / `data-testid="chat-term-block-output"`) in Dual-Surface `--terminal-well-*` styling while preserving `JsonLedgerTable`, `AgentTurnContent`, `SlashMetaContent`, `TermBlockFeedbackStrip`, `term-explain-fix-block`, and compact `<TerminalApprovalCard />`.
  - Normalized `AgentStrip.tsx` (`62` LoC; removed `viz.gray400`, `zinc-500`, `bg-emerald-400`, `.5` spacing, `text-[10px]`, using `bg-(--color-status-pass)` / `bg-(--color-status-warn)` / `bg-text-muted` and `size-2` dots), `AgentTab.tsx` (`52` LoC; semantic classes + `data-terminal-mono="true"`, preserving `'waiting for events…'`), and `SendToAgent.tsx` (`66` LoC; semantic dialog/textarea/button styling + `data-terminal-mono="true"`, preserving `<VoxSelect>`, `'Send'`, `'Cancel'`).
  - Updated `ScriptoriumPromptEditor.tsx` (`406` LoC; preserved `<div aria-hidden="true" data-testid="scriptorium-highlight-layer">` and `useLayoutEffect` `!important` overrides, added `data-terminal-mono="true"` and `focus-within:border-accent`) and `TerminalApprovalCard.tsx` (`409` LoC; shared `TERMINAL_MONO_STYLE` + `data-terminal-mono="true"` on `term-approval-command` and lazy `term-approval-edit-input`).
- **Task 3 (900x600 Responsive Layout & Target Floor):**
  - Updated `DiscoveryRail.tsx` (`196` LoC) and `DiscoveryRail.test.tsx` (`140` LoC): removed inline hexes/styles, added `window.innerWidth > 0 && window.innerWidth < 980` auto-collapse + `resize` listener (preserving `1024px` jsdom expanded default and `gui.console.discovery_rail_collapsed.v1` persistence), `w-8` collapsed rail, `max-[1099px]:absolute ... z-20` overlay, `size-6` toggle buttons with `<Icon.chevL className="size-3" />` / `<Icon.chevR className="size-3" />`, and `data-terminal-mono="true"` command snippets.
  - Updated `ConsoleSessionBar.tsx` (`191` LoC; `min-h-7` tabs, `size-6` close buttons, wrapping `term-close-confirm-bar`), `ConsoleSplitGrid.tsx` (`176` LoC; `min-w-40` multi-leaf floor), and `Console.tsx` (`415` LoC; `p-2 min-[1100px]:p-3`, `flex-wrap` agent/bridge bar without `mx-2`, `relative flex min-h-0 min-w-0 flex-1 gap-2 overflow-hidden` body container).

## Task Commits

Git commits intentionally omitted in shared worktree per `00-core.md` Rule 6 and `446-02-PLAN.md` Executor Git Rule.

## Verification Proof Outputs

### 1. TDD RED -> GREEN Proof
- **Task 1 RED** (`timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/TerminalTab.test.tsx src/components/surfaces/Console/sgrToSpans.test.tsx src/components/surfaces/Console/xtermAddons.test.ts` before implementation, exit `1`):
  - `Test Files  3 failed (3)` | `Tests  5 failed | 18 passed (23)` (`#67e8f9` vs `#7dd3fc`, `TERMINAL_MONO_STYLE` undefined, `data-terminal-well` null, `#9E2B25` vs `#e57368`, `#faf6ee` vs `#15110d`).
- **Task 1 GREEN** (`timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/TerminalTab.test.tsx src/components/surfaces/Console/sgrToSpans.test.tsx src/components/surfaces/Console/xtermAddons.test.ts src/lib/themeContract.test.ts`, exit `0`):
  - `Test Files  4 passed (4)` | `Tests  31 passed (31)`.
- **Task 3 RED** (`timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/DiscoveryRail.test.tsx` before `DiscoveryRail.tsx` edit, exit `1`):
  - `expected 'w-[268px] shrink-0' to contain 'w-8'`.

### 2. Full Vitest & Guard Suite (Post-Final-Edit)
Command: `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/lib/themeContract.test.ts src/guards/zeroEmojiAndDingbat.test.ts src/guards/noRawNativeControlsGuard.test.ts src/guards/transportIpcGuard.test.ts src/guards/toastBodyGuard.test.ts` (exit `0`)
```text
 Test Files  28 passed (28)
      Tests  157 passed (157)
   Start at  17:49:15
   Duration  3.58s
```

### 3. TypeScript Typecheck
Command: `timeout 120s pnpm --dir crates/vox-gui/ui typecheck` (exit `0`, `$ tsc --noEmit` with 0 errors)

### 4. Static Leak, Spacing & Theme Contract Audits
- `rg -n "zinc-|slate-|bg-black|border-cyan|text-brass"` across all 12 touched production TSX files -> exit `1` (`0` matches).
- `rg -n "\b(?:p|px|py|pt|pb|pl|pr|m|mx|my|mt|mb|ml|mr|gap|space-x|space-y|size|w|h|min-h|min-w|max-h|max-w)-\d+\.5\b"` across all 12 touched production TSX files -> exit `1` (`0` matches).
- `rg -n "#|»|«" crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx` -> exit `1` (`0` matches).
- `timeout 60s ./target/debug/toestub --rules vox/ui/theme-contract-drift crates/vox-gui/ui/src` -> exit `0` (`✅ No issues found — code is clean!`).

### 5. Non-Blank LoC Check (`rg -c "\S"`, Ceiling `< 440`)
- `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`: `415`
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx`: `409`
- `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx`: `406`
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx`: `404`
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx`: `359`
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx`: `355`
- `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts`: `354`
- `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts`: `325`
- `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts`: `228`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx`: `209`
- `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx`: `196`
- `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx`: `191`
- `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx`: `176`
- `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.tsx`: `147`
- `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.test.tsx`: `140`
- `crates/vox-gui/ui/src/components/surfaces/Console/SendToAgent.tsx`: `66`
- `crates/vox-gui/ui/src/components/surfaces/Console/AgentStrip.tsx`: `62`
- `crates/vox-gui/ui/src/components/surfaces/Console/AgentTab.tsx`: `52`

## Deviations from Plan
- **Caller-authorized scope addition (`xtermAddons.test.ts`):** `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts` (`L68-86`) still asserted pre-Dual-Surface `#FAF6EE` (`arcane`) and `#ece5d6` (`travertine`/`glacier`) backgrounds for `resolveTerminalTheme(pref)`; paused for caller authorization and updated `expectedBg` to `#15110d` / `#171512` (with `>= 7.0` contrast threshold for `high-contrast`).

## Issues Encountered
None after caller authorization of `xtermAddons.test.ts`.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All `TERM-ROMA-CV-02` Dual-Surface ANSI calibration, component zero-leak normalization, and `900x600` responsive layout requirements are implemented and verified, unblocking Plan `446-04` (`terminal-cv-flywheel.spec.ts` 5x4 Playwright convergence matrix).

---
*Phase: 446-roman-scriptorium-theme-and-cv-flywheel*
*Completed: 2026-11-15*
