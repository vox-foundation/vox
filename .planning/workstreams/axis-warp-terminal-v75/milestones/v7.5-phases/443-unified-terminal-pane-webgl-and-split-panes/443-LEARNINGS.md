---
phase: 443
phase_name: "unified-terminal-pane-webgl-and-split-panes"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 32
  lessons: 8
  patterns: 20
  surprises: 3
missing_artifacts:
  - "UAT.md"
---

# Phase 443 Learnings: unified-terminal-pane-webgl-and-split-panes

## Decisions

### Mapped normalized 'travertine' | 'glacier' directly to THEME_CONTRACT.th...
Mapped normalized 'travertine' | 'glacier' directly to THEME_CONTRACT.themes.glacier (#ece5d6) in terminalTheme.ts so WCAG >= 4.5:1 ANSI foreground slots are verified against both Carrara (#FAF6EE) and Travertine (#ece5d6) without dark-hex literal drift

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-01-SUMMARY.md

---

### Kept started_ms
Kept started_ms: number and ended_ms: number | null non-optional on TermBlockDto while normalizing omitted wire timestamps in sessionReducer.normalizeWireBlock so parallel BlockStreamView consumers compile cleanly

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-01-SUMMARY.md

---

### Eliminated post-submit termSessionAttach and replay_utf8 re-writes in Te...
Eliminated post-submit termSessionAttach and replay_utf8 re-writes in TerminalTab.tsx, reserving re-attach strictly for initial mount or explicit SnapshotResync with resync_required === true

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-01-SUMMARY.md

---

### Extended transport.ts with 1-arg/2-arg termSessionAttach(sessionId, afte...
Extended `transport.ts` with 1-arg/2-arg `termSessionAttach(sessionId, afterSeq?)` (`afterSeq != null` guard), `TermPromptKindWire`, `TermSessionEventWire` (all 11 Rust variants), and additive `TermBlockDto` / `TermAttachmentDto` fields, plus `requestConsoleBlockFocus` / `consumePendingConsoleBlockFocus` / `onConsoleBlockFocus` in `terminalBridge.ts`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-01-SUMMARY.md

---

### Built sessionReducer.ts (233 non-blank LoC) reducing all 11 externally-t...
Built `sessionReducer.ts` (`233` non-blank LoC) reducing all 11 externally-tagged Rust `SessionEvent` variants (`BlockOpened`, `OutputAppended`, `BlockClosed`, `BlockAwaitingInput`, `BlockPinned`, `AgentMessage`, `CwdChanged`, `AltScreenChanged`, `SnapshotResync`, `SecurityLevelChanged`, `ControlModeChanged`), `<16 ms` `scanAltScreenTransition` escape detection (`\x1b[?1049h/l`, `\x1b[?1047h/l`, `\x1b[?47h/l`), and bidirectional security/control wire mappers.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-01-SUMMARY.md

---

### Created terminalTheme.ts (121 non-blank LoC), upgraded webglAddon.ts (11...
Created `terminalTheme.ts` (`121` non-blank LoC), upgraded `webglAddon.ts` (`110` non-blank LoC), and built `xtermAddons.ts` (`211` non-blank LoC) delivering WCAG >= 4.5:1 16-color ANSI palettes across all 4 profiles (`arcane`, `travertine`, `slate`, `high-contrast`), `SearchTerminalAddon`, 1-based `WebLinksTerminalAddon` (`http://` / `https://` allowlist + workspace file locators), `Unicode11TerminalAddon`, and `SerializeTerminalAddon` with zero new npm packages.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-01-SUMMARY.md

---

### Upgraded TerminalTab.tsx (295 non-blank LoC) to import @xterm/xterm/css/...
Upgraded `TerminalTab.tsx` (`295` non-blank LoC) to import `@xterm/xterm/css/xterm.css`, activate all 6 addons (`FitAddon`, `WebglTerminalAddon`, `SearchTerminalAddon`, `WebLinksTerminalAddon`, `Unicode11TerminalAddon`, `SerializeTerminalAddon`) with mock-safe guards, suppress `.xterm-helper-textarea` (`tabIndex = -1`, `aria-hidden="true"`) in `Blocks` mode, emit synchronous `onAltScreenHint` transitions, and remove post-submit `termSessionAttach` / `replay_utf8` echo storms.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-01-SUMMARY.md

---

### Preserved SGR brush sequences across bare carriage-return (\r) line over...
Preserved SGR brush sequences across bare carriage-return (\r) line overwrites in normalizeTerminalText so colored cargo/pnpm spinners retain their foreground/background attributes on the latest frame.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-02-SUMMARY.md

---

### Configured useVirtualizer observeElementRect to inspect clientHeight || ...
Configured useVirtualizer observeElementRect to inspect clientHeight || offsetHeight || getBoundingClientRect().height and gated measureElement on el.offsetHeight > 0 so real WebViews measure dynamic block heights while jsdom virtualization tests with mocked clientHeight=360 mount <35 cards cleanly.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-02-SUMMARY.md

---

### Kept Wave 1 imports 100% self-contained against existing src/lib/theme.t...
Kept Wave 1 imports 100% self-contained against existing src/lib/theme.ts, src/components/ui/*, and src/transport.ts with ExtendedTermBlockDto so Plan 443-02 compiles and passes independently alongside parallel Plan 443-01.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-02-SUMMARY.md

---

### Built sgrToSpans.ts (235 non-blank LoC) supporting WCAG >= 4.5:1 light/d...
Built `sgrToSpans.ts` (`235` non-blank LoC) supporting WCAG >= 4.5:1 light/dark 16-color palettes, xterm 256-color cube/grayscale (`38;5;n` & `38:5:n`), 24-bit TrueColor (`38;2;r;g;b` & colon subparameters), bold/dim/italic/underline/inverse/strikethrough + resets, OSC/CSI cleanup, `\r` spinner frame overwrite normalization, `<mark data-testid="term-search-highlight">` search highlighting, and scheme-allowlisted `http(s)` / `localhost:<port>` + `file:line(:col)?` buttons wired to `voxTransport.openLocator` with zero `dangerouslySetInnerHTML`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-02-SUMMARY.md

---

### Extracted BlockCard.tsx (231 non-blank LoC) replacing hardcoded dark zin...
Extracted `BlockCard.tsx` (`231` non-blank LoC) replacing hardcoded dark `zinc-*` / `bg-black/*` classes with semantic tokens (`bg-bg-surface`, `bg-bg-base`, `bg-bg-elevated`, `border-border-subtle`, `border-brass`, `--color-status-*`), `4px` spacing rhythm, `11px`/`12px`/`14px` typography, sticky command headers (`cwd` basename, `git_branch`, `Pinned` badge, `structured json`, exit seal, `Interrupt Process`, duration), output collapse/expand (auto-expanding on `Failed`), `Pin`/`Unpin` footer toggle, ANSI-stripped `Focus in Chat` line-range selection, and conditional `AwaitingInput` stdin bar (`null` when inactive, `type="password"` + immediate state wipe on `credential_prompt`, `Ctrl+B` `Promote to Background`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-02-SUMMARY.md

---

### Upgraded BlockStreamView.tsx (173 non-blank LoC) to fill its parent pane...
Upgraded `BlockStreamView.tsx` (`173` non-blank LoC) to fill its parent pane (`flex min-h-0 flex-1`), render the `443-UI-SPEC.md` empty state (`Terminal session ready`), scroll `activeSearchBlockId`/`focusedBlockId` into view, and virtualize `>24` blocks via `@tanstack/react-virtual` when `clientHeight > 0` with deterministic direct-render fallback in zero-height jsdom.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-02-SUMMARY.md

---

### Used observeElementRect fallback chain (el.clientHeight || el.offsetHeig...
Used `observeElementRect` fallback chain (`el.clientHeight || el.offsetHeight || rect.height`) inside `BlockStreamView.tsx` because `@tanstack/virtual-core`'s default observer reads `offsetHeight` (which stays `0` in jsdom when only `clientHeight` and `getBoundingClientRect` are mocked), ensuring both real browser `ResizeObserver` layouts and jsdom `clientHeight` mocks activate virtualization identically.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-02-SUMMARY.md

---

### Guarded virtualizer.measureElement(el) behind el.offsetHeight > 0 so zer...
Guarded `virtualizer.measureElement(el)` behind `el.offsetHeight > 0` so zero-layout jsdom rows retain `estimateSize: () => 136` instead of collapsing to `0px`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-02-SUMMARY.md

---

### Kept TerminalTab mounted inside an absolute inset-0 layer toggled via vi...
Kept TerminalTab mounted inside an absolute inset-0 layer toggled via visible pointer-events-auto z-10 vs invisible pointer-events-none z-0 plus suppressHelperTextarea={!isVtVisible}, achieving <16 ms ALT_SCREEN transitions with zero xterm disposal/remount and preserving a single unique textbox landmark in Blocks mode.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-03-SUMMARY.md

---

### Scoped data-testid='term-close-pane' to the active split pane leaf (data...
Scoped data-testid='term-close-pane' to the active split pane leaf (data-testid={`term-close-pane-${paneId}`} on inactive leaves) and placed split action testids (term-split-right, term-split-down) in ConsoleSessionBar so RTL single-element queries never collide when 2-4 split panes are open.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-03-SUMMARY.md

---

### Guarded all transport calls in TerminalSessionPane against null/undefine...
Guarded all transport calls in TerminalSessionPane against null/undefined returns so compact embedded usage in ChatSurface tests (where only specific invoke commands are mocked) degrades cleanly without unhandled rejections.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-03-SUMMARY.md

---

### Built TermSearchBar.tsx (72 non-blank LoC) and TerminalSessionPane.tsx (...
Built `TermSearchBar.tsx` (`72` non-blank LoC) and `TerminalSessionPane.tsx` (`299` non-blank LoC) unifying `BlockStreamView` and warm `TerminalTab` inside one viewport per session, owning `termSessionAttach(sid, afterSeq?)` + `vox://term-session-event` delta reduction, switching to the VT layer synchronously on `AltScreenChanged(true)` or `\x1b[?1049h` hints without disposing or remounting xterm, supporting manual `Blocks` / `VT` segmented overrides (`TERM-PANE-01`), and keeping `TermSearchBar` unmounted (`null`) when closed so initial `getByRole('textbox')` queries remain unique.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-03-SUMMARY.md

---

### Built ConsoleSplitGrid.tsx (170 non-blank LoC) implementing immutable Pa...
Built `ConsoleSplitGrid.tsx` (`170` non-blank LoC) implementing immutable `PaneNode` tree helpers (`splitPaneLeaf`, `closePaneLeaf`, `assignPaneSession`, `replaceSessionInTree`, `countPaneLeaves`, `MAX_SPLIT_PANES = 4`) rendered through `react-resizable-panels@4.22.1` (`orientation="horizontal" | "vertical"`, `defaultSize="50%"`, `minSize="20%"`, direct `ResizablePanel` + `ResizableHandle` children).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-03-SUMMARY.md

---

### Built ConsoleSessionBar.tsx (152 non-blank LoC) with non-nested sibling ...
Built `ConsoleSessionBar.tsx` (`152` non-blank LoC) with non-nested sibling `<button type="button">` elements inside `<div role="group">` wrappers (`axe` `nested-interactive` clean), live `cwd` basename + `git_branch` + running foreground command badges, two-step inline `Confirm Close` active-job guard, `<VoxSelect>` security policy selector, and control mode pill.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-03-SUMMARY.md

---

### Refactored Console.tsx (306 non-blank LoC, strictly < 330) onto ConsoleS...
Refactored `Console.tsx` (`306` non-blank LoC, strictly `< 330`) onto `ConsoleSplitGrid` + `TerminalSessionPane` + `ConsoleSessionBar`, eliminating the legacy duplicate `50%/50%` stacked `BlockStreamView` + `TerminalTab` layout while preserving all existing tests, keyboard shortcuts (`Escape`, `Cmd/Ctrl+F`, `Cmd/Ctrl+Shift+M`, `Cmd/Ctrl+J`, `Cmd/Ctrl+L`, `Alt+Up/Down`, `p`/`f`/`a`/`r`), `terminalBridge` pub/sub (`onConsoleBlockFocus`), and backend `termSessionSetSecurity` / `termSessionSetControl` wire synchronization.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-03-SUMMARY.md

---

### Used autoOpenSession={false} on Console.tsx's <TerminalSessionPane> leav...
Used `autoOpenSession={false}` on `Console.tsx`'s `<TerminalSessionPane>` leaves so `Console.tsx` retains single ownership of initial `termSessionList()` / fallback `termSessionOpen(80, 24)` hydration while default `autoOpenSession={true}` supports standalone `<TerminalSessionPane compact>` in Wave 3 (`443-04-PLAN.md`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-03-SUMMARY.md

---

### Detected agent terminal actions strictly via isTerminalToolName(tool) (t...
Detected agent terminal actions strictly via isTerminalToolName(tool) (tool.startsWith('vox_terminal_')) inside existing tool_start / tool_receipt events so src/lib/turnEvents.ts (TURN_EVENT_KINDS) and contracts/gui/turn-event-kinds.v1.json remain 100% unmodified and turnEventContract.test.tsx passes all 29 contract assertions.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-04-SUMMARY.md

---

### Kept data-testid='chat-term-open-console' exclusive to ChatTerminalBlock...
Kept data-testid='chat-term-open-console' exclusive to ChatTerminalBlockCard.tsx while omitting that testid on the compact Open in Console action inside ChatTurnEventRow.tsx so single-element getByTestId('chat-term-open-console') queries remain deterministic even when TurnTrace steps are expanded.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-04-SUMMARY.md

---

### Subscribed MessageBubble in ChatTranscript.tsx to subscribeTerminalBridg...
Subscribed MessageBubble in ChatTranscript.tsx to subscribeTerminalBridge only while an assistant turn is actively streaming (!isSystem && !isUser && streaming) so idle historical transcript rows incur zero bridge listener overhead.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-04-SUMMARY.md

---

### Replaced TerminalDockPanel's static termState.blocks.slice(-5) mock loop...
Replaced TerminalDockPanel's static termState.blocks.slice(-5) mock loop in ChatSurface.tsx with <TerminalSessionPane compact sessionId={termState.activeSessionId} onOpenConsole={() => props.params.onNavigate?.('console')} /> while leaving closedPanelIds, OPT_IN_PANEL_IDS, addDefaultPanel, and api.getPanel('terminal')?.update(...) completely untouched (net -22 LoC in ChatSurface.tsx).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 443-04-SUMMARY.md

---

### Built ChatTerminalBlockCard.tsx (169 total / 152 non-blank LoC) exportin...
Built `ChatTerminalBlockCard.tsx` (`169` total / `152` non-blank LoC) exporting `isTerminalToolName` (`vox_terminal_*`) and `ChatTerminalBlockCard`, rendering command header (`font-mono text-[12px]`), semantic status pill (`RUNNING` / `AWAITING INPUT` / `EXITED (N)` / `DONE`), up to 12 tail lines of ANSI-formatted output via `sgrToSpans` (zero `dangerouslySetInnerHTML`, neutralizing raw HTML/script payloads as inert React text spans), conditional `Promote to Background` action (`termSessionPromoteBg`) on `AwaitingInput`, and 1-click `Open in Console` (`data-testid="chat-term-open-console"`) dispatching `requestConsoleBlockFocus` + `onNavigate('console')` + `vox://navigate-surface`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-04-SUMMARY.md

---

### Integrated ChatTerminalBlockCard into TurnTrace.tsx (126 total / 122 non...
Integrated `ChatTerminalBlockCard` into `TurnTrace.tsx` (`126` total / `122` non-blank LoC) and added a compact `Open in Console` action to `ChatTurnEventRow.tsx` (`164` total / `154` non-blank LoC) for `vox_terminal_*` receipts while keeping `src/lib/turnEvents.ts` (`TURN_EVENT_KINDS`) and `contracts/gui/turn-event-kinds.v1.json` 100% untouched (`turnEventContract.test.tsx` 29/29 green).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-04-SUMMARY.md

---

### Wired onNavigate through ChatTranscript.tsx (826 total / 794 non-blank L...
Wired `onNavigate` through `ChatTranscript.tsx` (`826` total / `794` non-blank LoC) and added streaming-only `subscribeTerminalBridge` support on active assistant bubbles so `Running` / `AwaitingInput` agent terminal blocks render live before turn receipt events finalize.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-04-SUMMARY.md

---

### Upgraded TerminalDockPanel in ChatSurface.tsx (1845 total / 1757 non-bla...
Upgraded `TerminalDockPanel` in `ChatSurface.tsx` (`1845` total / `1757` non-blank LoC, net `-22` lines) to mount `<TerminalSessionPane compact sessionId={termState.activeSessionId} onOpenConsole={() => props.params.onNavigate?.('console')} />` inside `data-testid="chat-dock-terminal"`, keeping `closedPanelIds`, `OPT_IN_PANEL_IDS`, `addDefaultPanel`, and `api.getPanel('terminal')?.update(...)` completely untouched.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-04-SUMMARY.md

---

### Added ChatTerminalBlockCard.test.tsx (334 total / 300 non-blank LoC, 6 t...
Added `ChatTerminalBlockCard.test.tsx` (`334` total / `300` non-blank LoC, 6 tests) verifying `isTerminalToolName`, collapsed `TurnTrace` `ChatTerminalBlockCard` + `sgrToSpans` ANSI span styles + XSS `<script>` neutralization, `AwaitingInput` `Promote to Background` + `Open in Console` navigation, `ChatTurnEventRow` empty-render guard preservation, streaming `ChatTranscript` live block rendering, and `TerminalDockPanel` `<TerminalSessionPane compact>` + `closedPanelIds` non-resurrection.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 443-04-SUMMARY.md

## Lessons

### None. Scoped Vitest (15/15 test files, 72/72 tests) and pnpm --dir crate...
None. Scoped Vitest (`15/15` test files, `72/72` tests) and `pnpm --dir crates/vox-gui/ui exec tsc --noEmit` (`exit 0`) passed cleanly.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 443-01-SUMMARY.md

---

### Running 26 Vitest suites at full default worker concurrency alongside a ...
Running 26 Vitest suites at full default worker concurrency alongside a background Cargo build briefly caused two cold-start 5000ms test timeouts from CPU contention; re-running the required verification suite (`src/components/surfaces/Console src/guards src/lib/themeContract.test.ts` + `surfaceHonesty.guard.test.ts`) passed `26/26` test files (`125/125` tests) cleanly with zero code changes needed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 443-03-SUMMARY.md

---

### An initial parallel run of the 27-file Console + guards Vitest suite bri...
An initial parallel run of the 27-file Console + guards Vitest suite briefly hit two cold-start 5000ms worker contention timeouts (`BlockStreamView.test.tsx` and `Console.test.tsx`, matching the contention note in `443-03-SUMMARY.md`); re-running the exact suite passed `27/27` test files (`129/129` tests) cleanly in `22.02s` with zero code changes needed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 443-04-SUMMARY.md

---

### Top Fix #1 ([BLOCKER] Global single-key shortcut hijack in Console.tsx)
**Top Fix #1 (`[BLOCKER]` Global single-key shortcut hijack in `Console.tsx`):** Guarded `p`, `f`, `a`, `r` shortcuts with `!isInteractive && !e.metaKey && !e.ctrlKey && !e.altKey && insideConsole && target` and expanded `isInteractiveShortcutTarget` to exclude `input, textarea, select, button, [role="button"], [role="combobox"], [role="listbox"], [role="option"], [role="dialog"], [contenteditable="true"]`. Verified in `Console.test.tsx` that `Ctrl+A`, `Meta+A`, `Ctrl+R`, `Meta+R`, and button-focused `a`/`r` never open `SendToAgent` or re-run commands.

**Context:** Surfaced in structured phase code review.
**Source:** 443-REVIEW-FIX.md

---

### Top Fix #2 (VT zero-match search leak & focus restoration in TerminalSes...
**Top Fix #2 (VT zero-match search leak & focus restoration in `TerminalSessionPane.tsx`):** Branched search totals strictly on `showVt` and restored focus to the active VT helper textarea or `paneRootRef` inside `closeSearchAndRestoreFocus` on `Escape` (`443-UI-SPEC.md:220`).

**Context:** Surfaced in structured phase code review.
**Source:** 443-REVIEW-FIX.md

---

### Top Fix #3 (JetBrains Mono on <pre> in ChatTerminalBlockCard.tsx & --col...
**Top Fix #3 (`JetBrains Mono` on `<pre>` in `ChatTerminalBlockCard.tsx` & `--color-status-fail` on `Confirm Close Session` in `ConsoleSessionBar.tsx`):** Applied `style={TERMINAL_CARD_MONO_STYLE}` directly on `<pre data-testid="chat-term-block-output">` (overriding UA `pre { font-family: monospace }`) and replaced `variant="danger"` (`bg-red-500`) on `Confirm Close Session` with `border-(--color-status-fail) bg-(--color-status-fail) text-bg-base hover:opacity-90` plus `Escape` dismissal.

**Context:** Surfaced in structured phase code review.
**Source:** 443-REVIEW-FIX.md

---

### None required for Phase 443 sign-off
None required for Phase 443 sign-off. Automated multi-resolution 4-theme Playwright/CV visual gate convergence is tracked as Phase 446 (`TERM-ROMA-CV-01..04`).

**Context:** Verified in phase acceptance checks.
**Source:** 443-VERIFICATION.md

---

### No gaps found
No gaps found. All 4 ROADMAP Success Criteria, all 14 Plan Must-Have Truths (`18/18` total), all 12 Plan Prohibitions, all 31 Code + UI Review remediations (`443-REVIEW-FIX.md`), and all 8 STRIDE security threats (`443-SECURITY.md`, `threats_open: 0`) are verified in the codebase with passing automated test suites.

**Context:** Verified in phase acceptance checks.
**Source:** 443-VERIFICATION.md

## Patterns

### Mock-safe xterm addon lifecycle
Mock-safe xterm addon lifecycle: wrap Terminal construction, open, fit, and loadAddon calls in safeRun guards so minimal Vitest mocks and headless jsdom suites never throw

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-01-SUMMARY.md

---

### Synchronous <16 ms ALT_SCREEN transition hints via scanAltScreenTransiti...
Synchronous <16 ms ALT_SCREEN transition hints via scanAltScreenTransition(payload.data) on vox://term-output before xterm.write

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-01-SUMMARY.md

---

### Pure in-memory SessionEvent delta reduction (sessionReducer.ts) eliminat...
Pure in-memory SessionEvent delta reduction (sessionReducer.ts) eliminating post-submit term_session_attach + replay_utf8 storms

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-01-SUMMARY.md

---

### Self-contained ITerminalAddon adapters over @xterm/xterm@6.0.0 public bu...
Self-contained ITerminalAddon adapters over @xterm/xterm@6.0.0 public buffer/unicode/link-provider APIs with zero external addon package bloat

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-01-SUMMARY.md

---

### 1-based xterm ILinkProvider coordinate mapping over 0-based buffer.activ...
1-based xterm ILinkProvider coordinate mapping over 0-based buffer.active.getLine(bufferLineNumber - 1) with strict http/https URL scheme allowlisting

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-01-SUMMARY.md

---

### Header Pinned badge vs footer Pin/Unpin toggle button prevents duplicate...
Header Pinned badge vs footer Pin/Unpin toggle button prevents duplicate 'Pinned' text collisions in Testing Library queries.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-02-SUMMARY.md

---

### All terminal output links strictly allowlist http:// and https:// (after...
All terminal output links strictly allowlist http:// and https:// (after localhost:<port> normalization) and reject javascript:, data:, vbscript:, and file:// prefixes.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-02-SUMMARY.md

---

### Zero-dangerouslySetInnerHTML ECMA-48 SGR state machine emitting typed Re...
Zero-dangerouslySetInnerHTML ECMA-48 SGR state machine emitting typed React spans, <mark> search highlights, and scheme-allowlisted <button> locators

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-02-SUMMARY.md

---

### Custom observeElementRect reading clientHeight || offsetHeight || getBou...
Custom observeElementRect reading clientHeight || offsetHeight || getBoundingClientRect().height so @tanstack/react-virtual activates when clientHeight > 0 and falls back deterministically in zero-layout jsdom

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-02-SUMMARY.md

---

### Strict conditional null unmount of AwaitingInput stdin bar so idle Conso...
Strict conditional null unmount of AwaitingInput stdin bar so idle Console DOM exposes a single unique textbox landmark

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-02-SUMMARY.md

---

### Single per-session IPC subscription owner
Single per-session IPC subscription owner: TerminalSessionPane owns initial termSessionAttach(sid) + vox://term-session-event reduction and passes skipInitialAttach + replaySnapshot down to TerminalTab.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-03-SUMMARY.md

---

### Two-step inline active-job close guard on session tabs
Two-step inline active-job close guard on session tabs: clicking close while a foreground command is running arms Confirm Close (resetting on Escape or 4s timeout) instead of firing window.confirm or nested dialogs.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-03-SUMMARY.md

---

### Dual-layer unified terminal pane keeping xterm warm behind invisible poi...
Dual-layer unified terminal pane keeping xterm warm behind invisible pointer-events-none CSS visibility while BlockStreamView owns interactive focus in Blocks mode

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-03-SUMMARY.md

---

### Recursive immutable PaneNode split tree over react-resizable-panels@4.22...
Recursive immutable PaneNode split tree over react-resizable-panels@4.22.1 ResizablePanelGroup (orientation='horizontal' | 'vertical', defaultSize='50%', minSize='20%')

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-03-SUMMARY.md

---

### Accessible sibling <button type='button'> session tabs inside <div role=...
Accessible sibling <button type='button'> session tabs inside <div role='group'> wrappers eliminating nested-interactive axe violations

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-03-SUMMARY.md

---

### Unified TerminalSessionPane reuse across Console full-page splits and Ch...
Unified TerminalSessionPane reuse across Console full-page splits and ChatSurface compact Dockview panels without duplicating PTY IPC or xterm lifecycle state.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-04-SUMMARY.md

---

### Contract-safe specialized tool cards in TurnTrace rendered alongside col...
Contract-safe specialized tool cards in TurnTrace rendered alongside collapsed summary header while keeping per-kind ChatTurnEventRow strict field guards intact.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-04-SUMMARY.md

---

### Tool-name prefix sub-typing (tool.startsWith('vox_terminal_')) over exis...
Tool-name prefix sub-typing (tool.startsWith('vox_terminal_')) over existing tool_start / tool_receipt turn event kinds so TURN_EVENT_KINDS 1:1 contract parity remains untouched

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-04-SUMMARY.md

---

### XSS-safe ANSI terminal previews in Chat via sgrToSpans React spans (zero...
XSS-safe ANSI terminal previews in Chat via sgrToSpans React spans (zero dangerouslySetInnerHTML)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-04-SUMMARY.md

---

### Bidirectional Chat <-> Console focus routing combining requestConsoleBlo...
Bidirectional Chat <-> Console focus routing combining requestConsoleBlockFocus(blockId, sessionId), onNavigate('console'), and vox://navigate-surface CustomEvent dispatch

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 443-04-SUMMARY.md

## Surprises

### Estimate variance on 443-01
Estimate variance on 443-01: planned 45000 tokens vs 19151 actual (0.43x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 443-01-PLAN.md, 443-01-SUMMARY.md

---

### Estimate variance on 443-02
Estimate variance on 443-02: planned 38000 tokens vs 12250 actual (0.32x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 443-02-PLAN.md, 443-02-SUMMARY.md

---

### Estimate variance on 443-03
Estimate variance on 443-03: planned 48000 tokens vs 22827 actual (0.48x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 443-03-PLAN.md, 443-03-SUMMARY.md

