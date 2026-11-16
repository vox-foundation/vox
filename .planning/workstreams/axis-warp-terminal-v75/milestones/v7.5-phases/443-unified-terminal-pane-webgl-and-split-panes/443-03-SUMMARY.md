---
phase: 443-unified-terminal-pane-webgl-and-split-panes
plan: 03
subsystem: ui
tags: [react, typescript, xterm, resizable-panels, terminal, split-panes, vitest]

requires:
  - phase: 443-unified-terminal-pane-webgl-and-split-panes
    plan: 01
    provides: "sessionReducer.ts 11-variant delta reducer, scanAltScreenTransition, TerminalTab warm VT engine with skipInitialAttach/suppressHelperTextarea/searchQuery"
  - phase: 443-unified-terminal-pane-webgl-and-split-panes
    plan: 02
    provides: "sgrToSpans.ts stripAnsi, BlockCard.tsx, and @tanstack/react-virtual BlockStreamView.tsx"
provides:
  - "Keyboard-first find bar (TermSearchBar.tsx) returning null when closed so initial Console textbox queries remain unique"
  - "Unified single-session pane (TerminalSessionPane.tsx) mounting both BlockStreamView and warm TerminalTab once with CSS visibility/pointer-events switching (<16 ms ALT_SCREEN flips, zero xterm remounts, afterSeq resync)"
  - "Recursive binary split-pane tree and react-resizable-panels@4.22.1 grid (ConsoleSplitGrid.tsx) capped at MAX_SPLIT_PANES = 4 with orientation='horizontal' | 'vertical' and percentage string sizing"
  - "Accessible sibling-button session tab strip and split/security/control toolbar (ConsoleSessionBar.tsx) with live cwd basename, git_branch, running foreground command badges, and two-step inline active-job close confirmation"
  - "Refactored Console.tsx (306 non-blank LoC) replacing duplicate stacked BlockStreamView + TerminalTab panes with ConsoleSplitGrid + TerminalSessionPane + ConsoleSessionBar"
affects:
  - 443-04-PLAN
  - 444-prompt-composer-slash-palette-and-pty-completions
  - 445-oversight-banner-session-vault-and-stall-watch

actuals:
  tokens: 22827
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Dual-layer unified terminal pane keeping xterm warm behind invisible pointer-events-none CSS visibility while BlockStreamView owns interactive focus in Blocks mode"
    - "Recursive immutable PaneNode split tree over react-resizable-panels@4.22.1 ResizablePanelGroup (orientation='horizontal' | 'vertical', defaultSize='50%', minSize='20%')"
    - "Accessible sibling <button type='button'> session tabs inside <div role='group'> wrappers eliminating nested-interactive axe violations"

key-files:
  created:
    - crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx

key-decisions:
  - "Kept TerminalTab mounted inside an absolute inset-0 layer toggled via visible pointer-events-auto z-10 vs invisible pointer-events-none z-0 plus suppressHelperTextarea={!isVtVisible}, achieving <16 ms ALT_SCREEN transitions with zero xterm disposal/remount and preserving a single unique textbox landmark in Blocks mode."
  - "Scoped data-testid='term-close-pane' to the active split pane leaf (data-testid={`term-close-pane-${paneId}`} on inactive leaves) and placed split action testids (term-split-right, term-split-down) in ConsoleSessionBar so RTL single-element queries never collide when 2-4 split panes are open."
  - "Guarded all transport calls in TerminalSessionPane against null/undefined returns so compact embedded usage in ChatSurface tests (where only specific invoke commands are mocked) degrades cleanly without unhandled rejections."

patterns-established:
  - "Single per-session IPC subscription owner: TerminalSessionPane owns initial termSessionAttach(sid) + vox://term-session-event reduction and passes skipInitialAttach + replaySnapshot down to TerminalTab."
  - "Two-step inline active-job close guard on session tabs: clicking close while a foreground command is running arms Confirm Close (resetting on Escape or 4s timeout) instead of firing window.confirm or nested dialogs."

requirements-completed:
  - TERM-PANE-01
  - TERM-PANE-02
  - TERM-PANE-03
  - TERM-PANE-04

coverage:
  - id: D1
    description: "Unified TerminalSessionPane + TermSearchBar keeping BlockStreamView and warm TerminalTab synchronized with <16 ms ALT_SCREEN switching, manual Blocks/VT override, afterSeq resync, and Ctrl/Cmd+F search"
    requirement: TERM-PANE-01
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.test.tsx"
        status: pass
    human_judgment: false
  - id: D2
    description: "Warm WebGL/xterm VT layer preserved across Blocks <-> VT mode flips without remounting Terminal or leaking .xterm-helper-textarea into Blocks-mode textbox queries"
    requirement: TERM-PANE-02
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.test.tsx#flips to VT view on AltScreenChanged true and returns to Blocks on false without remounting TerminalTab"
        status: pass
    human_judgment: false
  - id: D3
    description: "Virtualized BlockStreamView integrated into TerminalSessionPane with stdin forwarding, Ctrl+B background promotion, pin sync, and ANSI-free search across command blocks"
    requirement: TERM-PANE-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.test.tsx#opens TermSearchBar via search button, counts matches across blocks, and closes cleanly on Escape"
        status: pass
    human_judgment: false
  - id: D4
    description: "ConsoleSplitGrid (react-resizable-panels@4.22.1, MAX_SPLIT_PANES = 4) + ConsoleSessionBar sibling-button tabs + refactored Console.tsx (<330 non-blank LoC) with live cwd/git_branch labels and wire security/control sync"
    requirement: TERM-PANE-04
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx"
        status: pass
    human_judgment: false

duration: 30min
completed: 2026-11-15
status: complete
---

# Phase 443 Plan 03: Unified TerminalSessionPane, TermSearchBar, Resizable Split-Pane Grid & Console Refactor Summary

**Unified single-pane `TerminalSessionPane` (`Blocks` + warm `TerminalTab` VT layer with `<16 ms` `ALT_SCREEN` flips), keyboard-first `TermSearchBar`, recursive `react-resizable-panels@4.22.1` `ConsoleSplitGrid` (`MAX_SPLIT_PANES = 4`), sibling-button `ConsoleSessionBar`, and refactored `Console.tsx` (`306` non-blank LoC).**

## Performance

- **Duration:** 30 min
- **Started:** 2026-11-15T11:28:00Z
- **Completed:** 2026-11-15T11:58:00Z
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- Built `TermSearchBar.tsx` (`72` non-blank LoC) and `TerminalSessionPane.tsx` (`299` non-blank LoC) unifying `BlockStreamView` and warm `TerminalTab` inside one viewport per session, owning `termSessionAttach(sid, afterSeq?)` + `vox://term-session-event` delta reduction, switching to the VT layer synchronously on `AltScreenChanged(true)` or `\x1b[?1049h` hints without disposing or remounting xterm, supporting manual `Blocks` / `VT` segmented overrides (`TERM-PANE-01`), and keeping `TermSearchBar` unmounted (`null`) when closed so initial `getByRole('textbox')` queries remain unique.
- Built `ConsoleSplitGrid.tsx` (`170` non-blank LoC) implementing immutable `PaneNode` tree helpers (`splitPaneLeaf`, `closePaneLeaf`, `assignPaneSession`, `replaceSessionInTree`, `countPaneLeaves`, `MAX_SPLIT_PANES = 4`) rendered through `react-resizable-panels@4.22.1` (`orientation="horizontal" | "vertical"`, `defaultSize="50%"`, `minSize="20%"`, direct `ResizablePanel` + `ResizableHandle` children).
- Built `ConsoleSessionBar.tsx` (`152` non-blank LoC) with non-nested sibling `<button type="button">` elements inside `<div role="group">` wrappers (`axe` `nested-interactive` clean), live `cwd` basename + `git_branch` + running foreground command badges, two-step inline `Confirm Close` active-job guard, `<VoxSelect>` security policy selector, and control mode pill.
- Refactored `Console.tsx` (`306` non-blank LoC, strictly `< 330`) onto `ConsoleSplitGrid` + `TerminalSessionPane` + `ConsoleSessionBar`, eliminating the legacy duplicate `50%/50%` stacked `BlockStreamView` + `TerminalTab` layout while preserving all existing tests, keyboard shortcuts (`Escape`, `Cmd/Ctrl+F`, `Cmd/Ctrl+Shift+M`, `Cmd/Ctrl+J`, `Cmd/Ctrl+L`, `Alt+Up/Down`, `p`/`f`/`a`/`r`), `terminalBridge` pub/sub (`onConsoleBlockFocus`), and backend `termSessionSetSecurity` / `termSessionSetControl` wire synchronization.

## Task Commits

Git commits were deferred to the parent coordinator per shared-worktree non-mutation constraints (`commits: 0` in subagent session):

1. **Task 1: Build TermSearchBar.tsx and unified TerminalSessionPane.tsx with <16 ms ALT_SCREEN flip** — verified via TDD RED -> GREEN -> REFACTOR (`TerminalSessionPane.test.tsx`, `transportIpcGuard.test.ts`, `zeroEmojiAndDingbat.test.ts`).
2. **Task 2: Build ConsoleSplitGrid.tsx and ConsoleSessionBar.tsx** — verified via TDD RED -> GREEN -> REFACTOR (`ConsoleSplitGrid.test.tsx`, `noRawNativeControlsGuard.test.ts`, `zeroEmojiAndDingbat.test.ts`, `themeContract.test.ts`).
3. **Task 3: Refactor Console.tsx onto ConsoleSplitGrid + TerminalSessionPane + ConsoleSessionBar** — verified via TDD RED -> GREEN -> REFACTOR (`Console.test.tsx`, full `src/components/surfaces/Console` + `src/guards` + `surfaceHonesty.guard.test.ts` + `themeContract.test.ts`, and `tsc --noEmit`).

## Files Created/Modified

- `crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx` — Compact find bar (`aria-label="Find in terminal"`, `Enter` / `Shift+Enter` / `Escape`, `N of M` match counter, `null` when closed) (`72` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx` — Unified terminal session pane (`Blocks` + warm `TerminalTab` VT layer, `afterSeq` resync, stdin/Ctrl+B/pin IPC wiring, compact dock mode for Wave 3 `ChatSurface`) (`299` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.test.tsx` — 5 RTL unit tests covering single-pane mount + unique textbox invariant, `<16 ms` `ALT_SCREEN` flip without remount, manual `Blocks`/`VT` override, `TermSearchBar` match navigation/Escape close, and `SnapshotResync` `afterSeq` recovery (`304` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx` — Recursive binary `PaneNode` split tree + `react-resizable-panels@4.22.1` grid (`MAX_SPLIT_PANES = 4`) (`170` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.test.tsx` — 3 unit + RTL tests covering `PaneNode` tree invariants (`<= 4` cap + collapse), horizontal/vertical `ResizablePanelGroup` rendering, and `ConsoleSessionBar` sibling buttons + two-step active-job close confirmation (`152` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx` — Accessible sibling-button session strip (`cwd` basename, `git_branch`, running command badge, two-step close guard) + split/security/control toolbar (`152` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx` — Refactored Console surface orchestrating `ConsoleSplitGrid`, `TerminalSessionPane`, `ConsoleSessionBar`, `InputEditor`, `DiscoveryRail`, and `terminalBridge` (`306` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx` — 9 RTL integration tests (all 7 original assertions preserved + 2 new tests for split panes/live session tab labels and wire security/control sync) (`207` non-blank LoC).

## Decisions Made

- Used `autoOpenSession={false}` on `Console.tsx`'s `<TerminalSessionPane>` leaves so `Console.tsx` retains single ownership of initial `termSessionList()` / fallback `termSessionOpen(80, 24)` hydration while default `autoOpenSession={true}` supports standalone `<TerminalSessionPane compact>` in Wave 3 (`443-04-PLAN.md`).
- Rendered `data-testid="term-close-pane"` exclusively on the active pane leaf (`term-close-pane-${paneId}` on inactive leaves) so single-element `getByTestId('term-close-pane')` queries remain deterministic across 2-4 split panes.
- Exported `replaceSessionInTree` from `ConsoleSplitGrid.tsx` to keep all `PaneNode` tree transformations co-located and keep both `ConsoleSplitGrid.tsx` (`170 < 190`) and `Console.tsx` (`306 < 330`) comfortably within LoC budgets.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Running 26 Vitest suites at full default worker concurrency alongside a background Cargo build briefly caused two cold-start 5000ms test timeouts from CPU contention; re-running the required verification suite (`src/components/surfaces/Console src/guards src/lib/themeContract.test.ts` + `surfaceHonesty.guard.test.ts`) passed `26/26` test files (`125/125` tests) cleanly with zero code changes needed.

## Threat Flags

None - no new network endpoints, auth paths, filesystem access patterns, or trust-boundary changes introduced beyond existing `src/transport.ts` wrappers (`T-443-07`, `T-443-08`, and `T-443-09` mitigations verified: `MAX_SPLIT_PANES = 4` enforced in `splitPaneLeaf` + toolbar button `disabled`, two-step inline close confirmation armed on active foreground commands, and `toWireSecurityLevel` / `toWireControlMode` synced on policy change + `Autopilot -> manual` reset on `Escape` and user submit).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Wave 3 (`443-04-PLAN.md`) can directly import `TerminalSessionPane` (`compact` mode), `ConsoleSplitGrid`, and `terminalBridge` (`requestConsoleBlockFocus` -> handled in `Console.tsx`) to replace `ChatSurface.tsx`'s stub terminal toggle and wire `@term` block focus into Console split panes.
- All Console unit/integration tests, UI static guards (`noRawNativeControlsGuard`, `transportIpcGuard`, `toastBodyGuard`, `zeroEmojiAndDingbat`, `surfaceHonesty`, `themeContract`), and `tsc --noEmit` are 100% green.

---
*Phase: 443-unified-terminal-pane-webgl-and-split-panes*
*Completed: 2026-11-15*
