---
phase: 443-unified-terminal-pane-webgl-and-split-panes
plan: 04
subsystem: ui
tags: [react, typescript, terminal, chat, dockview, ansi, sgr, vitest]

requires:
  - phase: 443-unified-terminal-pane-webgl-and-split-panes
    plan: 01
    provides: "sessionReducer.ts 11-variant delta reducer and warm TerminalTab VT engine"
  - phase: 443-unified-terminal-pane-webgl-and-split-panes
    plan: 02
    provides: "sgrToSpans.ts React span ANSI renderer (zero innerHTML) and BlockStreamView.tsx"
  - phase: 443-unified-terminal-pane-webgl-and-split-panes
    plan: 03
    provides: "TerminalSessionPane.tsx with compact embedded mode and Console.tsx onConsoleBlockFocus split-pane handler"
provides:
  - "Live streaming inline terminal block card (ChatTerminalBlockCard.tsx) for vox_terminal_* agent tool executions with sgrToSpans ANSI preview, AwaitingInput Promote to Background action, and 1-click Open in Console navigation"
  - "Contract-preserving TurnTrace.tsx and ChatTurnEventRow.tsx vox_terminal_* card/row integration without touching TURN_EVENT_KINDS or contracts/gui/turn-event-kinds.v1.json"
  - "Live streaming assistant bubble vox_terminal_* fallback card in ChatTranscript.tsx before turn events settle"
  - "Unified ChatSurface.tsx TerminalDockPanel mounting <TerminalSessionPane compact> inside data-testid='chat-dock-terminal' while preserving Dockview closedPanelIds update-only non-resurrection"
affects:
  - 444-prompt-composer-slash-palette-and-pty-completions
  - 445-oversight-banner-session-vault-and-stall-watch

actuals:
  tokens: 37862
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Tool-name prefix sub-typing (tool.startsWith('vox_terminal_')) over existing tool_start / tool_receipt turn event kinds so TURN_EVENT_KINDS 1:1 contract parity remains untouched"
    - "XSS-safe ANSI terminal previews in Chat via sgrToSpans React spans (zero dangerouslySetInnerHTML)"
    - "Bidirectional Chat <-> Console focus routing combining requestConsoleBlockFocus(blockId, sessionId), onNavigate('console'), and vox://navigate-surface CustomEvent dispatch"

key-files:
  created:
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx

key-decisions:
  - "Detected agent terminal actions strictly via isTerminalToolName(tool) (tool.startsWith('vox_terminal_')) inside existing tool_start / tool_receipt events so src/lib/turnEvents.ts (TURN_EVENT_KINDS) and contracts/gui/turn-event-kinds.v1.json remain 100% unmodified and turnEventContract.test.tsx passes all 29 contract assertions."
  - "Kept data-testid='chat-term-open-console' exclusive to ChatTerminalBlockCard.tsx while omitting that testid on the compact Open in Console action inside ChatTurnEventRow.tsx so single-element getByTestId('chat-term-open-console') queries remain deterministic even when TurnTrace steps are expanded."
  - "Subscribed MessageBubble in ChatTranscript.tsx to subscribeTerminalBridge only while an assistant turn is actively streaming (!isSystem && !isUser && streaming) so idle historical transcript rows incur zero bridge listener overhead."
  - "Replaced TerminalDockPanel's static termState.blocks.slice(-5) mock loop in ChatSurface.tsx with <TerminalSessionPane compact sessionId={termState.activeSessionId} onOpenConsole={() => props.params.onNavigate?.('console')} /> while leaving closedPanelIds, OPT_IN_PANEL_IDS, addDefaultPanel, and api.getPanel('terminal')?.update(...) completely untouched (net -22 LoC in ChatSurface.tsx)."

patterns-established:
  - "Unified TerminalSessionPane reuse across Console full-page splits and ChatSurface compact Dockview panels without duplicating PTY IPC or xterm lifecycle state."
  - "Contract-safe specialized tool cards in TurnTrace rendered alongside collapsed summary header while keeping per-kind ChatTurnEventRow strict field guards intact."

requirements-completed:
  - TERM-PANE-01

coverage:
  - id: D1
    description: "ChatTerminalBlockCard.tsx + TurnTrace.tsx + ChatTurnEventRow.tsx + ChatTranscript.tsx surface live streaming vox_terminal_* cards with sgrToSpans ANSI formatting, AwaitingInput Promote to Background action, and 1-click Open in Console without modifying TURN_EVENT_KINDS"
    requirement: TERM-PANE-01
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx"
        status: pass
    human_judgment: false
  - id: D2
    description: "ChatSurface.tsx TerminalDockPanel mounts <TerminalSessionPane compact> inside data-testid='chat-dock-terminal' and preserves Dockview closedPanelIds update-only non-resurrection"
    requirement: TERM-PANE-01
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx#mounts TerminalSessionPane compact inside TerminalDockPanel and never resurrects a closed terminal panel in ChatSurface"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx"
        status: pass
    human_judgment: false

duration: 19min
completed: 2026-11-15
status: complete
---

# Phase 443 Plan 04: Bidirectional Console <-> Chat Terminal Unification Summary

**Inline `sgrToSpans`-rendered `ChatTerminalBlockCard` for `vox_terminal_*` agent actions with 1-click `Open in Console` focus routing, plus unified `<TerminalSessionPane compact>` inside `ChatSurface.tsx` `TerminalDockPanel` preserving `closedPanelIds` Dockview non-resurrection (`TERM-PANE-01`).**

## Performance

- **Duration:** 19 min
- **Started:** 2026-11-15T12:01:00Z
- **Completed:** 2026-11-15T12:20:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Built `ChatTerminalBlockCard.tsx` (`169` total / `152` non-blank LoC) exporting `isTerminalToolName` (`vox_terminal_*`) and `ChatTerminalBlockCard`, rendering command header (`font-mono text-[12px]`), semantic status pill (`RUNNING` / `AWAITING INPUT` / `EXITED (N)` / `DONE`), up to 12 tail lines of ANSI-formatted output via `sgrToSpans` (zero `dangerouslySetInnerHTML`, neutralizing raw HTML/script payloads as inert React text spans), conditional `Promote to Background` action (`termSessionPromoteBg`) on `AwaitingInput`, and 1-click `Open in Console` (`data-testid="chat-term-open-console"`) dispatching `requestConsoleBlockFocus` + `onNavigate('console')` + `vox://navigate-surface`.
- Integrated `ChatTerminalBlockCard` into `TurnTrace.tsx` (`126` total / `122` non-blank LoC) and added a compact `Open in Console` action to `ChatTurnEventRow.tsx` (`164` total / `154` non-blank LoC) for `vox_terminal_*` receipts while keeping `src/lib/turnEvents.ts` (`TURN_EVENT_KINDS`) and `contracts/gui/turn-event-kinds.v1.json` 100% untouched (`turnEventContract.test.tsx` 29/29 green).
- Wired `onNavigate` through `ChatTranscript.tsx` (`826` total / `794` non-blank LoC) and added streaming-only `subscribeTerminalBridge` support on active assistant bubbles so `Running` / `AwaitingInput` agent terminal blocks render live before turn receipt events finalize.
- Upgraded `TerminalDockPanel` in `ChatSurface.tsx` (`1845` total / `1757` non-blank LoC, net `-22` lines) to mount `<TerminalSessionPane compact sessionId={termState.activeSessionId} onOpenConsole={() => props.params.onNavigate?.('console')} />` inside `data-testid="chat-dock-terminal"`, keeping `closedPanelIds`, `OPT_IN_PANEL_IDS`, `addDefaultPanel`, and `api.getPanel('terminal')?.update(...)` completely untouched.
- Added `ChatTerminalBlockCard.test.tsx` (`334` total / `300` non-blank LoC, 6 tests) verifying `isTerminalToolName`, collapsed `TurnTrace` `ChatTerminalBlockCard` + `sgrToSpans` ANSI span styles + XSS `<script>` neutralization, `AwaitingInput` `Promote to Background` + `Open in Console` navigation, `ChatTurnEventRow` empty-render guard preservation, streaming `ChatTranscript` live block rendering, and `TerminalDockPanel` `<TerminalSessionPane compact>` + `closedPanelIds` non-resurrection.

## Task Commits

Git commits were deferred to the parent coordinator per shared-worktree non-mutation constraints (`commits: 0` in subagent session):

1. **Task 1: Build ChatTerminalBlockCard.tsx and wire into TurnTrace.tsx, ChatTurnEventRow.tsx & ChatTranscript.tsx** — verified via TDD RED -> GREEN (`ChatTerminalBlockCard.test.tsx`, `turnEventContract.test.tsx`, `TurnTrace.test.tsx`).
2. **Task 2: Upgrade ChatSurface.tsx TerminalDockPanel to mount <TerminalSessionPane compact> and verify closedPanelIds non-resurrection** — verified via TDD RED -> GREEN (`ChatTerminalBlockCard.test.tsx`, `ChatSurface.test.tsx`, Console + transport + static UI guards, and `tsc --noEmit`).

## Files Created/Modified

- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx` — Live streaming `vox_terminal_*` terminal block card with `sgrToSpans` preview, `Promote to Background` (`AwaitingInput`), and `Open in Console` navigation (`169` total / `152` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx` — 6 RTL unit/integration tests covering `ChatTerminalBlockCard`, `TurnTrace`, `ChatTurnEventRow`, `ChatTranscript`, `TerminalDockPanel`, and `ChatSurface` `closedPanelIds` non-resurrection (`334` total / `300` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx` — Added optional `onNavigate` prop and compact `Open in Console` button on `vox_terminal_*` `tool_receipt` rows while preserving strict required-field empty-render guards (`164` total / `154` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx` — Threaded `onNavigate` and surfaced deduplicated `vox_terminal_*` cards inside `data-testid="chat-trace"` even while trace steps remain collapsed (`126` total / `122` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx` — Threaded `onNavigate` to `MessageBubble` -> `TurnTrace` and rendered streaming live `vox_terminal_*` blocks from `terminalBridge` (`826` total / `794` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` — Replaced static `slice(-5)` loop in exported `TerminalDockPanel` with `<TerminalSessionPane compact>` inside `data-testid="chat-dock-terminal"` and passed `onNavigate` to `<ChatTranscript>` (`1845` total / `1757` non-blank LoC).

## Decisions Made

- Used `tool.startsWith('vox_terminal_')` (`isTerminalToolName`) on existing `tool_start` / `tool_receipt` DTOs rather than adding a new turn event kind, preserving 100% compatibility with `contracts/gui/turn-event-kinds.v1.json` and `turnEventContract.test.tsx`.
- Overrode `<Button size="xs">` padding/font classes with `h-6 px-2 py-1 text-[11px] font-semibold` in `ChatTerminalBlockCard.tsx` so all new controls obey the strict `4px` spacing grid and `11px/12px/14px/18px` typography scale (`443-UI-SPEC.md`).
- Exported `TerminalDockPanel` from `ChatSurface.tsx` for direct unit testing alongside full `<ChatSurface>` Dockview panel lifecycle regression coverage.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- An initial parallel run of the 27-file Console + guards Vitest suite briefly hit two cold-start 5000ms worker contention timeouts (`BlockStreamView.test.tsx` and `Console.test.tsx`, matching the contention note in `443-03-SUMMARY.md`); re-running the exact suite passed `27/27` test files (`129/129` tests) cleanly in `22.02s` with zero code changes needed.

## Threat Flags

None - no new network endpoints, auth paths, filesystem access patterns, or trust-boundary changes introduced (`T-443-02` mitigated by exclusive `sgrToSpans` React span rendering with zero `dangerouslySetInnerHTML`, verified by `<script>` payload neutralization test; `T-443-07` mitigated by keeping `api.getPanel('terminal')?.update(...)` strictly update-only in `ChatSurface.tsx`, verified by `closedPanelIds` non-resurrection test).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 443 (`443-01` through `443-04`) UI unification is complete across `Console` (`ConsoleSplitGrid`, `TerminalSessionPane`, `ConsoleSessionBar`, `TermSearchBar`, `BlockStreamView`, `TerminalTab`) and `Chat` (`TerminalDockPanel`, `ChatTerminalBlockCard`, `TurnTrace`, `ChatTurnEventRow`, `ChatTranscript`).
- Ready for Phase 443 verification / review gates and Phase 444 (`Prompt Composer, Slash Palette, and PTY Completions`).

---
*Phase: 443-unified-terminal-pane-webgl-and-split-panes*
*Completed: 2026-11-15*
