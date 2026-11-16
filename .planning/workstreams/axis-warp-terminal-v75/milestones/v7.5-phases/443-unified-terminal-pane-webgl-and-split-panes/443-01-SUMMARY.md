---
phase: 443-unified-terminal-pane-webgl-and-split-panes
plan: 01
subsystem: ui
tags: [react, typescript, xterm, webgl2, tauri-ipc, wcag, terminal]

requires:
  - phase: 442-stateful-pty-sessionhost-utf8-alt-screen-and-shell-init
    provides: "SessionHost after_seq attachment delta contract, 11-variant SessionEvent enum, and streaming UTF-8 / ALT_SCREEN PTY transport"
provides:
  - "1-arg and 2-arg compatible termSessionAttach(sessionId, afterSeq?) IPC helper plus TermPromptKindWire and 11-variant TermSessionEventWire contracts in transport.ts"
  - "Cross-surface Console block/session focus pub/sub (requestConsoleBlockFocus, consumePendingConsoleBlockFocus, onConsoleBlockFocus) in terminalBridge.ts"
  - "Pure 11-variant SessionEvent reducer (sessionReducer.ts) with <16 ms scanAltScreenTransition escape scanner and bidirectional security/control wire mappers"
  - "4-profile WCAG >= 4.5:1 terminal ANSI 16-color + semantic CSS-variable xterm ITheme resolver (terminalTheme.ts)"
  - "Self-contained @xterm/xterm@6.0.0 addon suite (WebglTerminalAddon, SearchTerminalAddon, WebLinksTerminalAddon with http/https allowlist, Unicode11TerminalAddon, SerializeTerminalAddon)"
  - "Warm VT TerminalTab with @xterm/xterm/css/xterm.css, 6-addon lifecycle, .xterm-helper-textarea focus suppression in Blocks mode, and zero post-submit replay_utf8 re-attach storms"
affects:
  - 443-03-PLAN
  - 444-prompt-composer-slash-palette-and-pty-completions
  - 445-oversight-banner-session-vault-and-stall-watch

actuals:
  tokens: 19151
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Pure in-memory SessionEvent delta reduction (sessionReducer.ts) eliminating post-submit term_session_attach + replay_utf8 storms"
    - "Self-contained ITerminalAddon adapters over @xterm/xterm@6.0.0 public buffer/unicode/link-provider APIs with zero external addon package bloat"
    - "1-based xterm ILinkProvider coordinate mapping over 0-based buffer.active.getLine(bufferLineNumber - 1) with strict http/https URL scheme allowlisting"

key-files:
  created:
    - crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.test.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts
  modified:
    - crates/vox-gui/ui/src/transport.ts
    - crates/vox-gui/ui/src/transport.console.test.ts
    - crates/vox-gui/ui/src/lib/terminalBridge.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx

key-decisions:
  - "Mapped normalized 'travertine' | 'glacier' directly to THEME_CONTRACT.themes.glacier (#ece5d6) in terminalTheme.ts so WCAG >= 4.5:1 ANSI foreground slots are verified against both Carrara (#FAF6EE) and Travertine (#ece5d6) without dark-hex literal drift"
  - "Kept started_ms: number and ended_ms: number | null non-optional on TermBlockDto while normalizing omitted wire timestamps in sessionReducer.normalizeWireBlock so parallel BlockStreamView consumers compile cleanly"
  - "Eliminated post-submit termSessionAttach and replay_utf8 re-writes in TerminalTab.tsx, reserving re-attach strictly for initial mount or explicit SnapshotResync with resync_required === true"

patterns-established:
  - "Mock-safe xterm addon lifecycle: wrap Terminal construction, open, fit, and loadAddon calls in safeRun guards so minimal Vitest mocks and headless jsdom suites never throw"
  - "Synchronous <16 ms ALT_SCREEN transition hints via scanAltScreenTransition(payload.data) on vox://term-output before xterm.write"

requirements-completed:
  - TERM-PANE-01
  - TERM-PANE-02

coverage:
  - id: D1
    description: "1-arg/2-arg termSessionAttach(sessionId, afterSeq?) IPC helper, cross-surface Console focus pub/sub, and pure 11-variant SessionEvent reducer + scanAltScreenTransition + wire mappers"
    requirement: TERM-PANE-01
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/transport.console.test.ts#termSessionAttach sends { sessionId } on 1-arg call, { sessionId, afterSeq } on numeric afterSeq, and omits null"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.test.ts#reduces all 11 externally-tagged Rust SessionEvent variants including OutputAppended"
        status: pass
    human_judgment: false
  - id: D2
    description: "4-profile WCAG >= 4.5:1 terminalTheme.ts and self-contained WebglTerminalAddon, SearchTerminalAddon, WebLinksTerminalAddon (http/https allowlisted), Unicode11TerminalAddon, and SerializeTerminalAddon"
    requirement: TERM-PANE-02
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts#guarantees WCAG >= 4.5:1 across all 16 ANSI foreground slots on all 4 profile backgrounds"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts#provides 1-based inclusive link ranges for https, localhost, and file locators while rejecting javascript: and data: URLs"
        status: pass
    human_judgment: false
  - id: D3
    description: "TerminalTab warm VT engine with @xterm/xterm/css/xterm.css, 6-addon lifecycle, .xterm-helper-textarea focus suppression in Blocks mode, and zero post-submit replay_utf8 re-attach storms"
    requirement: TERM-PANE-02
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx#submits pendingLine once without post-submit termSessionAttach or replay_utf8 re-write storms"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx#suppresses .xterm-helper-textarea focus in Blocks mode and restores it when unsuppressed"
        status: pass
    human_judgment: false

duration: 18min
completed: 2026-11-15
status: complete
---

# Phase 443 Plan 01: Transport afterSeq, Pure SessionEvent Reducer, 4-Profile WCAG Theme, xterm v6 Addon Suite, and Warm TerminalTab Summary

**Pure 11-variant `SessionEvent` delta reducer, `afterSeq` Tauri IPC contract, 4-profile WCAG >= 4.5:1 terminal theme, 5 in-repo `@xterm/xterm@6.0.0` addons, and warm `TerminalTab` VT layer eliminating post-submit `replay_utf8` re-attach storms**

## Performance

- **Duration:** 18 min
- **Started:** 2026-11-15T11:07:13Z
- **Completed:** 2026-11-15T11:25:56Z
- **Tasks:** 3
- **Files modified:** 11

## Accomplishments
- Extended `transport.ts` with 1-arg/2-arg `termSessionAttach(sessionId, afterSeq?)` (`afterSeq != null` guard), `TermPromptKindWire`, `TermSessionEventWire` (all 11 Rust variants), and additive `TermBlockDto` / `TermAttachmentDto` fields, plus `requestConsoleBlockFocus` / `consumePendingConsoleBlockFocus` / `onConsoleBlockFocus` in `terminalBridge.ts`.
- Built `sessionReducer.ts` (`233` non-blank LoC) reducing all 11 externally-tagged Rust `SessionEvent` variants (`BlockOpened`, `OutputAppended`, `BlockClosed`, `BlockAwaitingInput`, `BlockPinned`, `AgentMessage`, `CwdChanged`, `AltScreenChanged`, `SnapshotResync`, `SecurityLevelChanged`, `ControlModeChanged`), `<16 ms` `scanAltScreenTransition` escape detection (`\x1b[?1049h/l`, `\x1b[?1047h/l`, `\x1b[?47h/l`), and bidirectional security/control wire mappers.
- Created `terminalTheme.ts` (`121` non-blank LoC), upgraded `webglAddon.ts` (`110` non-blank LoC), and built `xtermAddons.ts` (`211` non-blank LoC) delivering WCAG >= 4.5:1 16-color ANSI palettes across all 4 profiles (`arcane`, `travertine`, `slate`, `high-contrast`), `SearchTerminalAddon`, 1-based `WebLinksTerminalAddon` (`http://` / `https://` allowlist + workspace file locators), `Unicode11TerminalAddon`, and `SerializeTerminalAddon` with zero new npm packages.
- Upgraded `TerminalTab.tsx` (`295` non-blank LoC) to import `@xterm/xterm/css/xterm.css`, activate all 6 addons (`FitAddon`, `WebglTerminalAddon`, `SearchTerminalAddon`, `WebLinksTerminalAddon`, `Unicode11TerminalAddon`, `SerializeTerminalAddon`) with mock-safe guards, suppress `.xterm-helper-textarea` (`tabIndex = -1`, `aria-hidden="true"`) in `Blocks` mode, emit synchronous `onAltScreenHint` transitions, and remove post-submit `termSessionAttach` / `replay_utf8` echo storms.

## Task Commits

Git commits were deferred to the parent coordinator per Wave 1 parallel workspace execution policy (`commits: 0` in subagent session):

1. **Task 1: Extend transport.ts afterSeq IPC, terminalBridge Console focus pub/sub, and pure 11-variant sessionReducer.ts** — verified via TDD RED → GREEN (`transport.console.test.ts`, `sessionReducer.test.ts`, `transportIpcGuard.test.ts`).
2. **Task 2: Create 4-profile WCAG terminalTheme.ts and self-contained xtermAddons.ts + webglAddon.ts suite** — verified via TDD RED → GREEN (`xtermAddons.test.ts`, `themeContract.test.ts`, `zeroEmojiAndDingbat.test.ts`).
3. **Task 3: Upgrade TerminalTab.tsx with xterm.css, 6-addon lifecycle, helper-textarea suppression, and zero post-submit replay storm** — verified via TDD RED → GREEN (`TerminalTab.test.tsx`, full `src/guards`, and `tsc --noEmit`).

## Files Created/Modified
- `crates/vox-gui/ui/src/transport.ts` — 1-arg/2-arg `termSessionAttach(sessionId, afterSeq?)`, `TermPromptKindWire`, `TermSessionEventWire`, and additive block/attachment wire fields (`1545` non-blank LoC).
- `crates/vox-gui/ui/src/transport.console.test.ts` — Unit tests for 1-arg, numeric `afterSeq`, `null` omission, and `ConsoleFocusTarget` pub/sub (`88` non-blank LoC).
- `crates/vox-gui/ui/src/lib/terminalBridge.ts` — Added `ConsoleFocusTarget`, `requestConsoleBlockFocus`, `consumePendingConsoleBlockFocus`, and `onConsoleBlockFocus` (`125` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.ts` — Pure 11-variant `SessionEvent` reducer, `hydrateSessionFromAttachment`, `scanAltScreenTransition`, and security/control wire mappers (`233` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.test.ts` — Unit tests covering all 11 `SessionEvent` variants, `OutputAppended` preservation, `scanAltScreenTransition`, and wire mappers (`280` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts` — `TERMINAL_MONO_FONT_STACK`, `XTERM_FONT_FAMILY`, 4-profile WCAG >= 4.5:1 `resolveAnsi16Palette`, and CSS-variable `resolveTerminalTheme` (`121` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts` — Added `isWebglActive()`, `onRendererModeChange`, and ANSI CSI/OSC stripping in `renderTermBlockForAgent` (`110` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts` — `SearchTerminalAddon`, `WebLinksTerminalAddon` (1-based `ILinkProvider` + `http/https` allowlist), `Unicode11TerminalAddon`, and `SerializeTerminalAddon` (`211` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts` — Unit + WCAG contrast + `auditSourceTextAgainstThemeContract` tests (`282` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx` — `@xterm/xterm/css/xterm.css` import, 6-addon lifecycle, semantic CSS-var mock branch, `.xterm-helper-textarea` suppression, `skipInitialAttach`/`replaySnapshot`/search props, and pure event reduction (`295` non-blank LoC).
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx` — 7 unit tests verifying addon activation, zero post-submit `replay_utf8` storm, `<16 ms` `ALT_SCREEN` detection, and `.xterm-helper-textarea` focus suppression (`251` non-blank LoC).

## Decisions Made
- Mapped normalized `'travertine' | 'glacier'` directly to `THEME_CONTRACT.themes.glacier` (`#ece5d6`) in `terminalTheme.ts` because `resolveThemeProfileSpec('travertine')` in `themeContract.ts` falls back to `arcane` (`#FAF6EE`), ensuring contrast tests validate against `#ece5d6` directly and avoiding any literal dark hex background assignment (`GENERAL_DARK_HEX_BG_REGEX`).
- Preserved non-optional `started_ms: number` and `ended_ms: number | null` on `TermBlockDto` while stamping fallback timestamps in `normalizeWireBlock` so `BlockStreamView.tsx` (owned in parallel by Plan `443-02`) compiles without friction.
- Kept `transport.ts` comments free of raw `invoke(` / `listen(` tokens so `src/guards/transportIpcGuard.test.ts` remains 100% green.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- None. Scoped Vitest (`15/15` test files, `72/72` tests) and `pnpm --dir crates/vox-gui/ui exec tsc --noEmit` (`exit 0`) passed cleanly.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Wave 2 (`443-03-PLAN.md`) has all required Wave 1 foundations ready: `sessionReducer.ts`, `terminalTheme.ts`, `xtermAddons.ts`, `webglAddon.ts`, `TerminalTab.tsx` (`skipInitialAttach`, `suppressHelperTextarea`, `replaySnapshot`, `searchQuery`, `onAltScreenHint`), `termSessionAttach(sid, afterSeq)`, and `terminalBridge.ts` Console focus pub/sub.
- No blockers or concerns.

---
*Phase: 443-unified-terminal-pane-webgl-and-split-panes*
*Completed: 2026-11-15*
