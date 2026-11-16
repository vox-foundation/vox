---
phase: 444-warp-editor-completions-and-runbooks
plan: 04
subsystem: terminal-ui
tags: [react, typescript, vitest, terminal, console, warp-editor, history-search, runbooks, voxify, osc633]

requires:
  - phase: 444-01
    provides: "Typed termSessionVoxify / termRunbooksList / termHistory* / termComplete IPC wrappers and terminalBridge RunbookLaunchRequest pub/sub"
  - phase: 444-02
    provides: "Multi-line ScriptoriumPromptEditor, InputEditor wrapper, and unmounted-when-closed HistorySearchPopover (role=region, role=searchbox)"
  - phase: 444-03
    provides: "JsonLedgerTable, BlockCard Table/JSON toggle, RunbookPanel, DiscoveryRail Action Help | Runbooks tabs, and OmnibarRunbookSheet"
provides:
  - "useConsoleWarpActions.ts (< 90 non-blank LoC) encapsulating Ctrl+R history sequence, DiscoveryRail tab & Runbook bridge pub/sub, interactive nested subshell shell-hint detection, and partial-mock-safe termSessionVoxify execution with sanitizeErrorForToast error handling"
  - "ConsoleSessionBar.tsx 1-click Voxify Subshell button (data-testid=term-voxify-subshell) transitioning to disabled Subshell Voxified state with zero nested buttons and normalized 4px spacing"
  - "TerminalSessionPane.tsx (!compact) contextual nested-subshell detection strip (data-testid=term-subshell-voxify-banner) with unambiguous data-testid=term-voxify-subshell-inline secondary trigger"
  - "Console.tsx (< 430 non-blank LoC) end-to-end orchestration of 6-mode PromptShellMode, Ctrl+R HistorySearchPopover shortcut, Cmd+J [aria-label=\"console input\"] textarea focus, Window-safe keydown Node check, active sessionId/cwd forwarding, DiscoveryRail Runbooks Insert/Run, and Voxify Subshell"
  - "Expanded Console.test.tsx integration suite verifying unique role=textbox invariant, Ctrl+R region popover (zero role=dialog), Cmd+J textarea focus, Shift+Enter multi-line submit, Runbook Insert/Run + requestOpenRunbook bridge, and Voxify Subshell success/reset/toast-sanitization paths"
affects:
  - 445-terminal-agentic-mode-and-mens-flywheel
  - 446-roman-scriptorium-theme-and-cv-flywheel

actuals:
  tokens: 16925
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Dedicated Console Warp orchestration hook (useConsoleWarpActions.ts, 89 non-blank LoC) keeping Console.tsx at 404 non-blank LoC well below the 450 god_object ceiling"
    - "Shared pure detectSubshellShellHint(command) parser mirroring vox-terminal-core::voxify::detect_subshell_command across bash/zsh/fish/pwsh/nu, sudo -i/-s, ssh, and interactive docker/podman/kubectl exec"
    - "Distinct test landmarks for primary toolbar CTA (term-voxify-subshell in ConsoleSessionBar) vs contextual pane strip CTA (term-voxify-subshell-inline in TerminalSessionPane)"

key-files:
  created:
    - crates/vox-gui/ui/src/components/surfaces/Console/useConsoleWarpActions.ts
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx

key-decisions:
  - "Extracted HistorySearch sequence, DiscoveryRail tab state, onOpenRunbook bridge subscription, subshell shell-hint detection, and Voxify session tracking into useConsoleWarpActions.ts (89 non-blank LoC) so Console.tsx remains at 404 non-blank LoC."
  - "Widened Console.tsx shellMode state to PromptShellMode ('default' | 'shell' | 'agent' | 'nu' | 'vox' | 'slash') while projecting bridgeShellMode ('default' | 'nu' | 'vox') into publishTerminalBridgeState so all 6 editor modes work end-to-end without touching terminalBridge.ts."
  - "Guarded Console.tsx window keydown listener with e.target instanceof Node before consoleRootRef.current?.contains(e.target) so synthetic or browser window-targeted keydown events never throw jsdom Node.contains TypeErrors."
  - "Skipped Cargo test invocations during Wave 3 per orchestrator Cargo-contention avoidance rule (zero .rs files modified in 444-04; parallel Phase 445 backend executor active) while running pre-built ./target/debug/vox-arch-check, full LoC audits across all 19 Phase 444 files, tsc --noEmit, and all 31 Vitest Console/Omnibar/transport/guard/theme suites."

patterns-established:
  - "Operator-explicit subshell hook injection: Voxify Subshell is triggered exclusively by human click, passes normalized shell hint ('bash'|'zsh'|'fish'|'pwsh'|'nu'), disables as Subshell Voxified once active, and re-arms automatically when a subsequent interactive subshell command is submitted."
  - "Sanitized toast error boundary: every Console IPC catch path routes errors through sanitizeErrorForToast(err) with cause: 'backend-error'."

requirements-completed:
  - TERM-WARP-01
  - TERM-WARP-02
  - TERM-WARP-03
  - TERM-WARP-04

coverage:
  - id: D1
    description: "End-to-end Console integration of multi-line ScriptoriumPromptEditor, 6-mode PromptShellMode, Cmd+J textarea focus, and unique role=textbox landmark on mount"
    requirement: TERM-WARP-01
    verification:
      - kind: integration
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx"
        status: pass
    human_judgment: false
  - id: D2
    description: "Console Ctrl+R global shortcut opening unmounted-when-closed HistorySearchPopover (role=region, role=searchbox) without mounting role=dialog or colliding with block shortcuts"
    requirement: TERM-WARP-02
    verification:
      - kind: integration
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx"
        status: pass
    human_judgment: false
  - id: D3
    description: "Active sessionId and live session cwd wired from Console into InputEditor / terminalCompletions and DiscoveryRail"
    requirement: TERM-WARP-03
    verification:
      - kind: integration
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx"
        status: pass
    human_judgment: false
  - id: D4
    description: "DiscoveryRail Runbook Insert/Run, requestOpenRunbook / consumePendingOpenRunbook bridge, ConsoleSessionBar + TerminalSessionPane (!compact) Voxify Subshell CTA, and sanitizeErrorForToast error handling"
    requirement: TERM-WARP-04
    verification:
      - kind: integration
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/guards/toastBodyGuard.test.ts"
        status: pass
    human_judgment: false

duration: 18min
completed: 2026-11-15
status: complete
---

# Phase 444 Plan 04: Console Warp Integration & End-to-End Gate Verification Summary

**End-to-end `Console` integration of `ScriptoriumPromptEditor`, `Ctrl+R` `HistorySearchPopover`, `Cmd+J` textarea focus, `DiscoveryRail` & `Cmd+P` Runbook launch bridge, and 1-click `Voxify Subshell` (`OSC 633` + `OSC 7`) across `useConsoleWarpActions.ts`, `ConsoleSessionBar.tsx`, `TerminalSessionPane.tsx`, and `Console.tsx` (`TERM-WARP-01..04`).**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-11-15T15:38:00Z
- **Completed:** 2026-11-15T15:56:00Z
- **Tasks:** 2 / 2 completed (Task 1 TDD RED -> GREEN verified; Task 2 architecture, LoC, TypeScript, and 31-suite Vitest gates verified)
- **Files modified:** 5 (1 created, 4 modified; all strictly `< 430` non-blank LoC)
- **Commits:** `0` (left uncommitted in shared worktree per orchestrator git-safety instructions)

## Accomplishments

- **Console Warp Orchestration Hook (`useConsoleWarpActions.ts`, `89` non-blank LoC)**: Extracted `historyOpenRequestSeq`, `railTab` (`'help' | 'runbooks'`), `selectedRunbookId`, `consumePendingOpenRunbook` + `onOpenRunbook` subscription (`'run'`, `'insert'`, `'open_rail'`), pure `detectSubshellShellHint` (`bash`/`sh`, `zsh`, `fish`, `pwsh`/`powershell`, `nu`/`nushell`, `sudo -i/-s`, `ssh`, interactive `docker`/`podman`/`kubectl exec`), and partial-mock-safe `handleVoxifySubshell` + `resetSessionVoxifiedIfSubshell` with `sanitizeErrorForToast` error reporting.
- **1-Click `Voxify Subshell` Toolbar & Contextual Subshell Strip (`ConsoleSessionBar.tsx`, `TerminalSessionPane.tsx`)**: Added non-nested `data-testid="term-voxify-subshell"` (`Voxify Subshell` -> disabled `Subshell Voxified`) in `ConsoleSessionBar.tsx`, added `!compact` nested-subshell detection strip (`data-testid="term-subshell-voxify-banner"` with `data-testid="term-voxify-subshell-inline"`) in `TerminalSessionPane.tsx`, and normalized legacy `size-3.5` / `h-7` classes onto strict `4px` multiples (`size-4`, `h-8`).
- **Unified `Console.tsx` Integration (`404` non-blank LoC)**: Widened `shellMode` to `PromptShellMode` (all 6 modes) with `TermShellModeId` projection for `publishTerminalBridgeState`, wired `activeSessionId` and `activeCwd` into `InputEditor` and `DiscoveryRail`, updated `Cmd+J` to focus `[aria-label="console input"]`, wired `Ctrl+R` to `openHistorySearch()` without opening any `role="dialog"`, and guarded `window` `keydown` `Node.contains` checks.
- **Integration Test Suite (`Console.test.tsx`, `385` non-blank LoC)**: Added end-to-end regression tests covering unique `screen.getByRole('textbox')` on mount, `Cmd+J` textarea focus, `Ctrl+R` `HistorySearchPopover` (`role="region"`, zero `role="dialog"`), `Shift+Enter` multi-line editing + `Enter` submit, `DiscoveryRail` Runbooks `Insert into Prompt` / `Run Runbook`, `requestOpenRunbook` bridge insertion, and `Voxify Subshell` success, subshell re-arm, and `sanitizeErrorForToast` failure handling.

## Non-Blank LoC Audit (`rg -c "\S"`, `< 450` Phase Gate / `< 430` Plan 04 Cap)

- `useConsoleWarpActions.ts`: 89
- `ConsoleSessionBar.tsx`: 185
- `Console.test.tsx`: 385
- `Console.tsx`: 404
- `TerminalSessionPane.tsx`: 416
- Prior Wave 1 & Wave 2 files verified `< 450`: `DiscoveryRail.tsx` (171), `RunbookPanel.tsx` (189), `OmnibarRunbookSheet.tsx` (192), `JsonLedgerTable.tsx` (269), `BlockCard.tsx` (297), `HistorySearchPopover.tsx` (319), `pty/warp.rs` (361), `ScriptoriumPromptEditor.tsx` (379), `voxify.rs` (380), `history.rs` (399), `pty.rs` (402), `completions_specs.rs` (422), `runbooks.rs` (441), `completions.rs` (449).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Guarded `e.target instanceof Node` in `Console.tsx` `window` `keydown` listener**
- **Found during:** Task 1 TDD RED verification (`Console.test.tsx`)
- **Issue:** `Console.tsx` previously evaluated `consoleRootRef.current?.contains(e.target as Node)` on `window` `keydown` events. When `keydown` is dispatched directly on `window` (`e.target === window`), jsdom and browsers throw `TypeError: Failed to execute 'contains' on 'Node': parameter 1 is not of type 'Node'`.
- **Fix:** Guarded with `e.target instanceof Node && Boolean(consoleRootRef.current?.contains(e.target))`.
- **Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`
- **Verification:** `Console.test.tsx` (`14/14` tests) passes with zero unhandled errors.

**2. [Rule 2 - UI Spec Compliance] Normalized pre-existing fractional `size-3.5` / `h-7` classes in `ConsoleSessionBar.tsx` and `TerminalSessionPane.tsx`**
- **Found during:** Task 1 implementation (`444-UI-SPEC.md` Zero Fractional `.5` Spacing Rule)
- **Issue:** `ConsoleSessionBar.tsx` (`Icon.splitRight`, `Icon.splitDown`) and `TerminalSessionPane.tsx` (`Icon.search`, `term-tui-active-banner`) contained legacy `size-3.5` and `h-7` classes.
- **Fix:** Normalized `size-3.5` to `size-4` and `h-7` to `h-8` so all touched Phase 444 files pass the strict `4px` spacing check with zero matches.
- **Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`
- **Verification:** Static spacing/typography `rg` check exited `1` (`0` matches); all Console & UI guard tests pass.

---

**Total deviations:** 2 auto-fixed (1 DOM event target bug, 1 `444-UI-SPEC.md` 4px spacing normalization)
**Impact on plan:** Zero scope creep; Rust `cargo test` commands intentionally skipped per orchestrator Cargo-contention avoidance directive (zero `.rs` files modified in `444-04`).

## Threat Flags

None — `termSessionVoxify` is gated strictly on explicit human button clicks with closed-allowlist shell hints (`T-444-04b`), and all toast error paths route through `sanitizeErrorForToast` (`T-444-07`, verified by `toastBodyGuard.test.ts` and `Console.test.tsx`).

## Self-Check: PASSED

- All 5 plan target files + `444-04-SUMMARY.md` exist on disk and are strictly `< 430` non-blank LoC (`max = 416`).
- `./target/debug/vox-arch-check`: **exit code 0**.
- `pnpm --dir crates/vox-gui/ui exec tsc --noEmit`: **exit code 0**.
- Full Vitest verification (`src/transport.console.test.ts`, `src/components/surfaces/Console/`, `src/components/layout/Omnibar.test.tsx`, `src/guards/`, `src/styles/themeContract.test.ts`): **31 passed (31) files, 193 passed (193) tests**.

---
*Phase: 444-warp-editor-completions-and-runbooks*
*Completed: 2026-11-15*
