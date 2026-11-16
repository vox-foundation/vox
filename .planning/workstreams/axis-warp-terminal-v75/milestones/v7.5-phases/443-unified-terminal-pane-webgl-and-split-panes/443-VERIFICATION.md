---
phase: 443-unified-terminal-pane-webgl-and-split-panes
verified: 2026-11-15T13:35:00Z
status: passed
score: 18/18 must-haves verified
covered_files:
  - .planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md
  - .planning/workstreams/axis-warp-terminal-v75/ROADMAP.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-01-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-01-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-02-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-02-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-03-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-03-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-04-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-04-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-CONTEXT.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-REVIEW-FIX.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-REVIEW.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-SECURITY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-UI-REVIEW.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-UI-SPEC.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-VALIDATION.md
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.test.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts
  - crates/vox-gui/ui/src/components/ui/Icons.tsx
  - crates/vox-gui/ui/src/lib/terminalBridge.ts
  - crates/vox-gui/ui/src/transport.console.test.ts
  - crates/vox-gui/ui/src/transport.ts
covered_digest: "v1:sha256:b48936942b269b8160e5eadc92c38e550df6cac06c9eb86c07eb8c30ab29ff73"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 443: Unified Terminal Pane, WebGL & Split Panes Verification Report

**Phase Goal:** Both the standalone Console surface and the Chat Dockview terminal tab share a single unified, GPU-accelerated Warp-style terminal pane with zero duplicate rendering, full ANSI block cards, and split/multi-session layouts.
**Verified:** 2026-11-15T13:35:00Z
**Status:** passed
**Re-verification:** No — initial verification (incorporating `443-REVIEW-FIX.md` code + UI review remediation)

## Goal Achievement

### Observable Truths

#### ROADMAP Success Criteria (4/4)

| # | Success Criterion | Status | Evidence |
| --- | --- | --- | --- |
| SC-1 | Both `Console.tsx` and `ChatSurface.tsx`'s Dockview `terminal` panel mount the same `<TerminalSessionPane>` component without simultaneously rendering duplicate output in stacked `BlockStreamView` + `TerminalTab` panes (`TERM-PANE-01`). | ✓ VERIFIED | `Console.tsx:396-423` mounts `<ConsoleSplitGrid>` rendering `<TerminalSessionPane>` per leaf (eliminating the legacy stacked `<BlockStreamView>` + `<TerminalTab>` split); `ChatSurface.tsx:275-315` (`TerminalDockPanel`) resolves existing daemon sessions via `voxTransport.termSessionList()` (`WR-03`) and mounts `<TerminalSessionPane compact autoOpenSession={false} ... />` inside `data-testid="chat-dock-terminal"` while preserving `closedPanelIds` update-only non-resurrection (`ChatSurface.tsx:1312`); `TerminalSessionPane.tsx:375-439` keeps `<TerminalTab>` warm in a single layered frame and toggles CSS visibility (`opacity-0 pointer-events-none invisible` vs `opacity-100 pointer-events-auto visible`) with `<BlockStreamView>`, coalesces initial attaches via `acquireInitialAttach` (`CR-02`), and tracks `lastSeqRef` to call `termSessionAttach(sid, afterSeq)` strictly on `SnapshotResync` or `Reconnect`. |
| SC-2 | `TerminalTab` loads `@xterm/xterm/css/xterm.css`, `@xterm/addon-webgl` (with graceful DOM fallback on WebGL context loss), `@xterm/addon-search`, `@xterm/addon-web-links`, `@xterm/addon-unicode11`, and `@xterm/addon-serialize`, and switches automatically into full-pane VT mode within `<16 ms` when a child enters `ALT_SCREEN` (`vim`, `htop`, `lazygit`, `fzf`) (`TERM-PANE-02`). | ✓ VERIFIED | `TerminalTab.tsx:1` imports `@xterm/xterm/css/xterm.css`, passes `allowProposedApi: true`, `cursorBlink: false`, `fontFamily: XTERM_FONT_FAMILY`, `letterSpacing: 0` (`L125-129`), loads `FitAddon`, `WebglTerminalAddon` (`webglAddon.ts:26-79` releasing its WebGL2 probe context via `WEBGL_lose_context.loseContext()`, `CR-03`, and subscribing `onContextLoss`), `SearchTerminalAddon`, `WebLinksTerminalAddon`, `Unicode11TerminalAddon`, and `SerializeTerminalAddon` (`L130-178`), syncs live `[data-theme]` token changes (`MutationObserver`, `WR-01`), and triggers synchronous `onAltScreenTransition` via `scanAltScreenTransition` (`\x1b[?1049h/l`, `\x1b[?1047h/l`, `\x1b[?47h/l`) and `AltScreenChanged` with zero xterm remount. |
| SC-3 | `BlockStreamView` virtualizes large block histories via `@tanstack/react-virtual`, renders 16/256/TrueColor SGR ANSI output and clickable `file:line:col` / URL links, supports sticky block headers, collapse/expand, pin, copy command/output/markdown, inline search, and surfaces inline interactive prompt input when a block enters `AwaitingInput` (`TERM-PANE-03`). | ✓ VERIFIED | `sgrToSpans.ts:1-380` parses 16/256/TrueColor SGR, bold/dim/italic/underline/inverse/strike, column-aware `\r` + `CSI 0K/1K/2K` overwrites (`WR-04`), `<mark data-testid="term-search-highlight">`, `http(s)`/`localhost` links, and `sanitizeWorkspaceFileLocator` (`CR-01`) into React spans/buttons with zero `dangerouslySetInnerHTML`; `BlockCard.tsx:1-275` renders sticky headers (`cwd` basename, `git_branch`, origin badge, `Pinned` status vs `Pin`/`Unpin` footer toggle, exit seal, `Interrupt Process`, duration), collapse/expand, `Copy Command` / `Copy Output` / `Copy as Markdown` / `Send to Agent`, and `AwaitingInput` inline stdin (`type="password"` when `credential_prompt`, `Ctrl+B` `Promote to Background`); `BlockStreamView.tsx:60-86,148-182` virtualizes `>24` blocks via `@tanstack/react-virtual` (`useVirtualizer`) with `<EmptyState>` and zero-height jsdom fallback; `TermSearchBar.tsx:1-78` searches both `Blocks` (`formatBlockSearchCorpus`) and `VT` (`SearchTerminalAddon` using `stringCellWidth`, `WR-05`). |
| SC-4 | Users can split terminal panes horizontally/vertically (via `react-resizable-panels`) and switch between named multi-PTY session tabs displaying live `cwd`, git branch, and active foreground job badges (`TERM-PANE-04`). | ✓ VERIFIED | `ConsoleSplitGrid.tsx:1-193` manages an immutable binary split tree capped at `MAX_SPLIT_PANES = 4`, rendering `ResizablePanelGroup` (`orientation="horizontal" \| "vertical"`), direct `ResizablePanel` (`defaultSize="50%"`, `minSize="20%"`) and `ResizableHandle` (`withHandle={false}`) children with `min-h-[180px] min-w-[280px]` leaf floor and active `ring-1 ring-[var(--accent)]`; `ConsoleSessionBar.tsx:1-182` renders non-nested sibling tab and close `<button type="button">` elements inside `<div role="group">`, live foreground command / `cwd` basename / `git_branch` labels, inline two-step close confirmation (`--color-status-fail`, `Escape` dismiss), and `<VoxSelect>` security (`L1..L5`) + control (`Manual` / `Co-Pilot` / `Autopilot`) wire sync (`Console.tsx:266-288`). |

#### Plan Must-Have Truths (14/14)

| Plan | Truth | Status | Evidence |
| --- | --- | --- | --- |
| `443-01` #1 | `voxTransport.termSessionAttach(sessionId, afterSeq?)` forwards `{ sessionId }` on initial attach and `{ sessionId, afterSeq }` on incremental reconnect without breaking existing 1-arg callers. | ✓ VERIFIED | `crates/vox-gui/ui/src/transport.ts:942-947` (`afterSeq !== undefined ? { sessionId, afterSeq } : { sessionId }` via `safeInvoke`), verified in `src/transport.console.test.ts` and `src/guards/transportIpcGuard.test.ts`. |
| `443-01` #2 | `TerminalTab` imports `@xterm/xterm/css/xterm.css`, passes `allowProposedApi: true` and `cursorBlink: false`, and loads `FitAddon`, `WebglTerminalAddon` (with `onContextLoss` DOM fallback notification), `SearchTerminalAddon`, `WebLinksTerminalAddon` (`http`/`https` allowlisted via `voxTransport.openLocator`), `Unicode11TerminalAddon`, and `SerializeTerminalAddon`. | ✓ VERIFIED | `TerminalTab.tsx:1,125-178`, `webglAddon.ts:1-103`, `xtermAddons.ts:1-246`; verified in `TerminalTab.test.tsx` (9 tests) and `xtermAddons.test.ts` (11 tests). |
| `443-01` #3 | `TerminalTab` exposes an imperative handle (`findNext`, `findPrevious`, `clearSearch`, `serialize`, `focus`, `fit`) and suppresses `.xterm-helper-textarea` from Accessibility/Tab order (`tabIndex={-1}`, `aria-hidden="true"`) when `suppressHelperTextarea` is true so `Blocks` mode preserves a single `textbox` role for `InputEditor`. | ✓ VERIFIED | `TerminalTab.tsx:21-28,67-82,369-379`, verified in `TerminalTab.test.tsx`, `TerminalSessionPane.test.tsx`, and `Console.test.tsx` (`screen.getByRole('textbox')`). |
| `443-01` #4 | `TerminalTab` no longer re-invokes `termSessionAttach` on every command submit (eliminating `64 KiB` `replay_utf8` write storms) and detects `ALT_SCREEN` transitions (`\x1b[?1049h/l`, `\x1b[?1047h/l`, `\x1b[?47h/l`, and `AltScreenChanged`) synchronously within `<16 ms`. | ✓ VERIFIED | `TerminalTab.tsx:222-279,349-352` and `sessionReducer.ts:81-106`; verified in `TerminalTab.test.tsx` and `sessionReducer.test.ts`. |
| `443-01` #5 | `terminalTheme.ts` resolves 16-color + fg/bg/cursor/selection xterm palettes for all 4 Vox theme profiles (`obsidian`, `glacier`, `arcane`, `high-contrast`, normalizing `light`/`paper`/`parchment` -> `glacier` and `dark`/`slate` -> `obsidian`) with every ANSI slot meeting WCAG >= 4.5:1 contrast against the resolved terminal background. | ✓ VERIFIED | `terminalTheme.ts:1-177`; verified across all 4 profiles × 16 ANSI colors in `xtermAddons.test.ts` and `src/lib/themeContract.test.ts`. |
| `443-02` #1 | `sgrToSpans.ts` parses 16-color (`30-37`, `90-97`, `40-47`, `100-107`), 256-color (`38;5;n`, `48;5;n`), and 24-bit TrueColor (`38;2;r;g;b`, `48;2;r;g;b`) SGR codes plus bold (`1`), dim (`2`), italic (`3`), underline (`4`), inverse (`7`), strikethrough (`9`), resets, and `\r` / `CSI K` overwrites into safe React `<span>` elements with zero `dangerouslySetInnerHTML`. | ✓ VERIFIED | `sgrToSpans.ts:1-380` (including `WR-04` column-cursor overwrite handling for `\r`, `CSI 0K`, `CSI 1K`, `CSI 2K`); verified in `sgrToSpans.test.tsx`. |
| `443-02` #2 | `sgrToSpans.ts` tokenizes clickable `http://` / `https://` / `localhost:<port>` URLs and `path/to/file.ext:line(:col)?` references inside styled spans and routes clicks through `voxTransport.openLocator` while strictly rejecting `javascript:`, `data:`, `file:///`, CLI flags, and traversal schemes. | ✓ VERIFIED | `sgrToSpans.ts:39-63,297-368` (`sanitizeWorkspaceFileLocator` + `isSafeTerminalUrl`); verified in `sgrToSpans.test.tsx` including adversarial `file:///etc/passwd:1`, `--goto:1:1`, and `../secret.rs:1` regressions (`CR-01`). |
| `443-02` #3 | `BlockStreamView` virtualizes block lists (`> 24` blocks) via `@tanstack/react-virtual` (`useVirtualizer` with `measureElement`) while gracefully rendering all visible items when `scrollRef.current?.clientHeight === 0` in `jsdom` tests, and renders `<EmptyState>` when `blocks.length === 0`. | ✓ VERIFIED | `BlockStreamView.tsx:15,60-86,113-182`; verified in `BlockStreamView.test.tsx` (both `clientHeight === 0` jsdom fallback and mocked `clientHeight = 400` with 40 blocks rendering a subset). |
| `443-02` #4 | `BlockCard` renders sticky block header (`cwd`, `git_branch`, origin badge, `Pinned` status pill, exit status seal, duration), collapse/expand toggle, `Copy Command`, `Copy Output`, `Copy as Markdown`, `Pin` / `Unpin`, `Send to Agent`, `Interrupt Process` (`SIGINT`), and an inline `AwaitingInput` stdin bar (`type="password"` when `credential_prompt: true` + `Promote to Background` on `Ctrl+B`). | ✓ VERIFIED | `BlockCard.tsx:1-275`; verified in `BlockStreamView.test.tsx` (8 tests). |
| `443-03` #1 | `<TerminalSessionPane>` mounts warm `<TerminalTab>` and `<BlockStreamView>` inside a single non-duplicating frame — switching synchronously (`< 16 ms`, CSS visibility toggle, zero xterm remount) between `Blocks` mode and `VT` mode on `ALT_SCREEN` transitions or manual override (`Blocks` / `VT`), and only replaying `replay_utf8` on initial attach or `resync_required === true`. | ✓ VERIFIED | `TerminalSessionPane.tsx:156-254,375-439`; verified in `TerminalSessionPane.test.tsx` (6 tests). |
| `443-03` #2 | `<ConsoleSplitGrid>` splits panes horizontally (`Split Right`) and vertically (`Split Down`) up to a hard cap of 4 panes using `react-resizable-panels` v4 (`ResizablePanelGroup` with `orientation="horizontal" \| "vertical"` and string percentage sizes `"50%"` / `"20%"`), keeping `ResizablePanel` and `ResizableHandle` as direct children of `ResizablePanelGroup`. | ✓ VERIFIED | `ConsoleSplitGrid.tsx:1-193`; verified in `ConsoleSplitGrid.test.tsx`. |
| `443-03` #3 | `<ConsoleSessionBar>` renders named multi-PTY session tabs as non-nested sibling `<button type="button">` elements inside `<div role="group">` displaying live foreground command / `cwd` basename / `git_branch` badges and requiring two-step inline confirmation before closing a session with a `Running` or `AwaitingInput` block. | ✓ VERIFIED | `ConsoleSessionBar.tsx:55-181`; verified in `ConsoleSplitGrid.test.tsx` and `noRawNativeControlsGuard.test.ts`. |
| `443-03` #4 / Decomposition | `Console.tsx` replaces the old stacked `<BlockStreamView>` + `<TerminalTab>` double-echo layout with `<ConsoleSplitGrid>` + `<TerminalSessionPane>`, wires `termSessionSetSecurity` and `termSessionSetControl` when `<VoxSelect>` controls change, and keeps all existing `Console.test.tsx` assertions green (`getByRole('textbox')` uniquely resolves `InputEditor` in `Blocks` mode). | ✓ VERIFIED | `Console.tsx:1-436` (`407` non-blank LoC after `443-REVIEW-FIX` per-session state & `CR-05` universal preemption hardening — within `443-UI-SPEC.md:163`'s `< 450` non-blank LoC decomposition budget and `< 500` `god_object` ceiling; note `443-03-PLAN` initially targeted `< 350` non-blank LoC and hit `296` before review-fix hardening), verified in `Console.test.tsx` (`11/11` green). |
| `443-04` #1–#3 | `ChatSurface.tsx::TerminalDockPanel` mounts `<TerminalSessionPane compact>` inside `data-testid="chat-dock-terminal"` while preserving `closedPanelIds` non-resurrection (`L1312`); `TurnTrace.tsx` and `ChatTranscript.tsx` render live `<ChatTerminalBlockCard>` components for `vox_terminal_exec` / `vox_terminal_send_input` with `Open in Console` (`data-testid="chat-term-open-console"`) without adding any `TURN_EVENT_KINDS` entry. | ✓ VERIFIED | `ChatSurface.tsx:275-315,1309-1314`, `ChatTerminalBlockCard.tsx:1-179`, `ChatTurnEventRow.tsx:54-60,92-112`, `TurnTrace.tsx:37-53,77-79`, `ChatTranscript.tsx:207-232`; verified in `ChatTerminalBlockCard.test.tsx` (7 tests), `TurnTrace.test.tsx` (9 tests), and `turnEventContract.test.tsx` (`exit 0`). |

**Score:** 18/18 must-haves verified (`4/4` ROADMAP Success Criteria + `14/14` Plan Must-Have Truths)

---

### Must-Have Prohibitions Check (12/12 Held)

| Plan | Prohibition | Status | Evidence |
| --- | --- | --- | --- |
| `443-01` | Do NOT add any new npm or Cargo dependency (`package.json`, `pnpm-lock.yaml`, `Cargo.lock` untouched by Phase 443). | ✓ HELD | `git diff -- crates/vox-gui/ui/package.json crates/vox-gui/ui/pnpm-lock.yaml Cargo.lock` returned empty (`0` Phase 443 dependency additions). |
| `443-01` | Do NOT call `@tauri-apps/api` directly outside `src/transport.ts` (`transportIpcGuard.test.ts` must stay green). | ✓ HELD | `src/guards/transportIpcGuard.test.ts` passed (`exit 0`). |
| `443-01` | Do NOT remove `TerminalTabProps` fields (`sessionId`, `onReady`) used by existing callers/tests. | ✓ HELD | `TerminalTab.tsx:29-39` preserves `sessionId: string \| null` and `onReady?: (sessionId: string) => void` (`TerminalTab.test.tsx` 9/9 green). |
| `443-02` | Do NOT use `dangerouslySetInnerHTML` or raw `innerHTML` anywhere in `sgrToSpans.ts`, `BlockCard.tsx`, or `BlockStreamView.tsx`. | ✓ HELD | `rg -n "dangerouslySetInnerHTML\|innerHTML"` across all Phase 443 files exited `1` (zero matches). |
| `443-02` | Do NOT introduce raw native `<select>` or emoji/dingbat Unicode characters (`noRawNativeControlsGuard` + `zeroEmojiAndDingbat` must pass). | ✓ HELD | Both `noRawNativeControlsGuard.test.ts` and `zeroEmojiAndDingbat.test.ts` passed (`exit 0`). |
| `443-02` | Do NOT echo credential stdin characters in plaintext when `block.credential_prompt === true` (`type="password"` required). | ✓ HELD | `BlockCard.tsx:200` sets `type={block.credential_prompt ? 'password' : 'text'}` with `autoComplete="off"` and clears state immediately on submit. |
| `443-03` | Do NOT pass `direction="..."` or numeric `defaultSize={50}` to `ResizablePanelGroup` / `ResizablePanel` (`react-resizable-panels@4.22.1` treats numbers as `px`, not `%`). | ✓ HELD | `ConsoleSplitGrid.tsx:148-159` passes `orientation={node.orientation}` and string `"50%"` / `"20%"`. |
| `443-03` | Do NOT nest a `<button>` close icon inside a `<button>` session tab. | ✓ HELD | `ConsoleSessionBar.tsx:96-140` renders sibling `<button type="button">` elements inside `<div role="group">`. |
| `443-03` | Do NOT render simultaneous stacked `<BlockStreamView>` + `<TerminalTab>` panes for the same leaf. | ✓ HELD | `Console.tsx:396-423` mounts `<ConsoleSplitGrid>` -> `<TerminalSessionPane>`, toggling CSS visibility between `Blocks` and `VT` inside one frame. |
| `443-04` | Do NOT add any new `TURN_EVENT_KINDS` entry in `turnEvents.ts` or `contracts/gui/turn-event-kinds.v1.json`. | ✓ HELD | `git diff -- crates/vox-gui/ui/src/lib/turnEvents.ts contracts/gui/turn-event-kinds.v1.json` is empty; `turnEventContract.test.tsx` passed. |
| `443-04` | Do NOT call `dockApiRef.current?.addPanel({ id: 'terminal', ... })` inside `ChatSurface.tsx`'s session-sync `useEffect` (`closedPanelIds` must never be resurrected). | ✓ HELD | `ChatSurface.tsx:1312` calls `dockApiRef.current?.getPanel('terminal')?.update({ params: {} })` only; verified by `ChatTerminalBlockCard.test.tsx`. |
| `443-04` | Do NOT use `dangerouslySetInnerHTML` in `ChatTerminalBlockCard.tsx`. | ✓ HELD | `ChatTerminalBlockCard.tsx:141` renders `{parseSgrToReact(previewText, { theme })}` with zero raw HTML injection. |

---

### Code & UI Review Remediations Verified (`443-REVIEW.md`, `443-UI-REVIEW.md`, `443-REVIEW-FIX.md`)

All **15/15** code review findings (`5` Critical, `6` Warning, `4` Info) and **16/16** UI review items (`1` BLOCKER + `15` Pillar warnings) were verified directly in source:

- **`CR-01` (`sgrToSpans.ts:39-63,297-318`):** `sanitizeWorkspaceFileLocator` strips URL suffixes before matching `file:line(:col)` and rejects `file:///etc/passwd:1`, `--goto:1:1`, `../secret.rs:1`, leading `/` or `\`, and Windows drive paths before `voxTransport.openLocator`.
- **`CR-02` (`TerminalSessionPane.tsx:46-71,156-254`):** Module-scoped `acquireInitialAttach(sid)` coalesces initial 1-arg `termSessionAttach(sid)` across split-pane clones and tracks live `seq` via `onOutputSeq` (`TerminalTab.tsx:231-235`) so incremental `SnapshotResync`/`Reconnect` attaches never fall back to `seq = 0`.
- **`CR-03` (`webglAddon.ts:26-51`):** `probeWebgl2Support` explicitly calls `canvas.getContext('webgl2').getExtension('WEBGL_lose_context')?.loseContext()` immediately after probing so split panes never leak Chromium WebGL contexts.
- **`CR-04` (`ChatTranscript.tsx:219-232`, `TurnTrace.tsx:41-53`):** Live `vox://terminal/event` block streaming is restricted to active turns and `vox_terminal_exec` / `vox_terminal_send_input` tool rows so historical turn receipts remain immutable.
- **`CR-05` (`TerminalSessionPane.tsx:314-325`, `Console.tsx:251-264`):** `onHumanInput` fires across `InputEditor`, raw VT `TerminalTab` keystrokes, and `BlockCard` interactive stdin, universally demoting `autopilot -> manual` locally and via `termSessionSetControl(sid, 'manual')`.
- **`WR-01..WR-06` & `IN-01..IN-04`:** Live `[data-theme]` `MutationObserver` on `TerminalTab.tsx:207-218` (`WR-01`); `openingRef` guard preventing duplicate PTY spawns (`Console.tsx:104,136-163`, `WR-02`); `TerminalDockPanel` `termSessionList()` lookup before `termSessionOpen()` (`ChatSurface.tsx:283-301`, `WR-03`); column-cursor `\r` and `CSI 0K/1K/2K` overwrite semantics (`sgrToSpans.ts:190-273`, `WR-04`); CJK/wide-glyph `stringCellWidth` search offsets (`xtermAddons.ts:55-99`, `WR-05`); per-session `securityBySession` / `controlBySession` maps (`Console.tsx:96-97`, `WR-06`); plus `IN-01..04` (`sessionSeqRef` alignment, dead `findActiveSessionIdForLeaf` removal, `scrollToBottom` helper, named `VIRTUALIZATION_THRESHOLD` / `ESTIMATED_BLOCK_HEIGHT_PX` constants).
- **`UI BLOCKER #1` & Pillar Remediations (`Console.tsx:307-335`, `Icons.tsx:425-436`, `ConsoleSessionBar.tsx`, `BlockCard.tsx`, `ChatTerminalBlockCard.tsx`):** Single-key shortcuts (`p`, `k`) require `e.ctrlKey || e.metaKey` (`Ctrl/Cmd+Shift+P` for control mode, `Ctrl/Cmd+K` for clear); `Split Right` / `Split Down` use dedicated `Icon.splitRight` / `Icon.splitDown` glyphs; `4px`-grid spacing (`gap-1`, `py-1`), `min-h-6`/`min-w-6` hit targets, `11px` floor typography, and non-color state differentiation verified across Console and Chat cards.

---

### Required Artifacts (16/16 Verified)

`gsd-tools.cjs verify artifacts` passed `16/16` (`issues: []`) across `443-01-PLAN.md` (`5/5`), `443-02-PLAN.md` (`3/3`), `443-03-PLAN.md` (`5/5`), and `443-04-PLAN.md` (`3/3`):

| Artifact | Provides | Total / Non-Blank LoC | Status |
| --- | --- | --- | --- |
| `crates/vox-gui/ui/src/transport.ts` | `termSessionAttach(sessionId, afterSeq?)` incremental attach + typed terminal session wire helpers | `L866-1094` | ✓ VERIFIED |
| `crates/vox-gui/ui/src/lib/terminalBridge.ts` | Cross-surface `requestConsoleBlockFocus` / `onConsoleBlockFocus` + composer insert events | 150 / 127 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.ts` | Pure immutable reducer across all 11 `SessionEvent` variants + `scanAltScreenTransition` | 261 / 233 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts` | 4-theme (`obsidian`, `glacier`, `arcane`, `high-contrast`) WCAG >= 4.5:1 xterm palettes | 178 / 160 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts` | `WebglTerminalAddon` with `WEBGL_lose_context` probe cleanup + `onContextLoss` DOM fallback | 104 / 92 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts` | `SearchTerminalAddon`, `WebLinksTerminalAddon`, `Unicode11TerminalAddon`, `SerializeTerminalAddon` | 247 / 221 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx` | Warm xterm.js v6 host with CSS, 6 addons, imperative handle, `[data-theme]` sync, and `<16 ms` `ALT_SCREEN` detection | 392 / 354 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts` | Pure-TS SGR/TrueColor + `\r`/`CSI K` + safe URL & workspace `file:line:col` React span renderer | 381 / 352 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` | Warp-grade block card with sticky header, actions, and `AwaitingInput` stdin (`credential_prompt` password masking + `Ctrl+B`) | 276 / 249 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx` | `@tanstack/react-virtual` virtualized block list (`>24` blocks) with `<EmptyState>` and jsdom fallback | 186 / 175 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx` | Inline `Ctrl/Cmd+F` search bar (`null` unmount when closed) | 79 / 72 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx` | Unified terminal pane shared by `Console.tsx` and `ChatSurface.tsx` Dockview `terminal` panel | 442 / 403 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx` | `react-resizable-panels` v4 horizontal/vertical split grid (max 4 panes) | 194 / 176 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx` | Multi-PTY session tab bar, live `cwd`/`git_branch`/command badges, 2-step close confirm, security/control `<VoxSelect>` | 183 / 169 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx` | Decomposed standalone Console surface (`< 450` UI-SPEC non-blank LoC budget) | 437 / 407 | ✓ VERIFIED |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx` + `ChatSurface.tsx` + `TurnTrace.tsx` | Compact docked `<TerminalSessionPane>` + inline streaming `vox_terminal_*` block cards with `Open in Console` | 180 / 161 (`ChatTerminalBlockCard.tsx`) | ✓ VERIFIED |

---

### Key Link Verification (9/9 Verified)

`gsd-tools.cjs verify key-links` passed `9/9` across all 4 plans:

| From | To | Via | Status |
| --- | --- | --- | --- |
| `TerminalTab.tsx` | `@xterm/xterm/css/xterm.css` + `webglAddon.ts` + `xtermAddons.ts` + `terminalTheme.ts` | Module CSS import + addon activation + `resolveXtermTheme()` | ✓ WIRED |
| `transport.ts` | `term_session_attach` (`crates/vox-gui/src/commands/pty.rs`) | `safeInvoke('term_session_attach', afterSeq !== undefined ? { sessionId, afterSeq } : { sessionId })` | ✓ WIRED |
| `BlockCard.tsx` | `sgrToSpans.ts` + `transport.ts` | `parseSgrToReact` + `termSessionSendInput` / `termSessionPromoteBg` / `termSessionPinBlock` / `termSessionSignal` | ✓ WIRED |
| `BlockStreamView.tsx` | `@tanstack/react-virtual` + `BlockCard.tsx` | `useVirtualizer` (`count: blocks.length`, `estimateSize: () => 136`, `overscan: 6`) | ✓ WIRED |
| `TerminalSessionPane.tsx` | `TerminalTab.tsx` + `BlockStreamView.tsx` + `sessionReducer.ts` + `TermSearchBar.tsx` | Warm VT + virtualized `Blocks` CSS visibility toggle + `reduceSessionEvent` | ✓ WIRED |
| `ConsoleSplitGrid.tsx` | `components/ui/Resizable.tsx` (`react-resizable-panels@4.22.1`) | `ResizablePanelGroup` (`orientation`), `ResizablePanel` (`"50%"`/`"20%"`), `ResizableHandle` | ✓ WIRED |
| `Console.tsx` | `ConsoleSplitGrid.tsx` + `TerminalSessionPane.tsx` + `ConsoleSessionBar.tsx` | Split tree state, multi-session tabs, `onConsoleBlockFocus`, and `termSessionSetSecurity`/`termSessionSetControl` | ✓ WIRED |
| `ChatSurface.tsx` | `TerminalSessionPane.tsx` | `TerminalDockPanel` (`data-testid="chat-dock-terminal"`) mounting `<TerminalSessionPane compact ... />` | ✓ WIRED |
| `ChatTerminalBlockCard.tsx` + `TurnTrace.tsx` + `ChatTranscript.tsx` | `sgrToSpans.ts` + `terminalBridge.ts` + `navigation.ts` | `parseSgrToReact` + `requestConsoleBlockFocus` + `setCurrentSurface('console')` | ✓ WIRED |

---

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `TerminalSessionPane.tsx` | `sessionState` (`blocks`, `altScreen`, `cwd`, `securityLevel`, `controlMode`, `seq`) | `voxTransport.termSessionAttach(sessionId, afterSeq?)` initial snapshot + live `voxTransport.onTerminalEvent` (`vox://terminal/event`) reduced via `reduceSessionEvent` | Yes — real Rust PTY snapshots, `OutputAppended` UTF-8 chunks, `BlockOpened`/`BlockClosed`/`BlockAwaitingInput`, and incremental `afterSeq` resync | ✓ FLOWING |
| `BlockStreamView.tsx` → `BlockCard.tsx` → `sgrToSpans.ts` | `block.output_utf8`, `block.state`, `block.credential_prompt`, `block.cwd`, `block.git_branch` | `sessionState.blocks` mapped from `TermBlockSnapshotWire` and live `SessionEvent` deltas | Yes — real ANSI SGR byte streams rendered as styled React `<span>` / locator `<button>` trees and interactive stdin controls | ✓ FLOWING |
| `Console.tsx` → `ConsoleSplitGrid.tsx` + `ConsoleSessionBar.tsx` | `splitRoot`, `sessions`, `securityBySession`, `controlBySession` | `voxTransport.termSessionList()` / `termSessionOpen()` + live session state callbacks + `termSessionSetSecurity` / `termSessionSetControl` | Yes — real daemon/local PTY session list, live `cwd`/`git_branch`/running-command tab labels, and bidirectional security/control state | ✓ FLOWING |
| `TurnTrace.tsx` / `ChatTranscript.tsx` → `ChatTerminalBlockCard.tsx` | `toolNames`, `previewText`, `awaitingPrompt`, `resolvedSessionId`, `resolvedBlockId` | `TurnEvent` (`ToolCallStarted` / `ToolCallCompleted` for `vox_terminal_*`) + active-turn `onTerminalEvent` stream | Yes — live streaming `OutputAppended` + `BlockAwaitingInput` previews during active turns and immutable tool result previews for historical receipts | ✓ FLOWING |

---

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| Phase completeness & plan/artifact/key-link integrity | `node .../gsd-tools.cjs verify phase-completeness 443` + `verify artifacts` + `verify key-links --ws axis-warp-terminal-v75` | `complete: true` (`4/4` plans & summaries), `16/16` artifacts passed, `9/9` key-links verified | ✓ PASS |
| TypeScript strict compilation across `crates/vox-gui/ui` | `timeout 300s pnpm --dir crates/vox-gui/ui exec tsc --noEmit` | Exit `0` (zero errors) | ✓ PASS |
| Architectural layer & crate-edge invariants | `timeout 300s ./target/debug/vox-arch-check` | Exit `0` (zero layer/edge violations) | ✓ PASS |
| Transport incremental attach, reducer, WebGL/xterm addons, SGR XSS/locator sanitization, unified pane, split grid, Chat unification, and static UI guards | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/components/surfaces/Console/sessionReducer.test.ts src/components/surfaces/Console/xtermAddons.test.ts src/components/surfaces/Console/sgrToSpans.test.tsx src/components/surfaces/Console/TerminalSessionPane.test.tsx src/components/surfaces/Console/ConsoleSplitGrid.test.tsx src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/turnEventContract.test.tsx src/guards/zeroEmojiAndDingbat.test.ts src/guards/noRawNativeControlsGuard.test.ts src/guards/transportIpcGuard.test.ts src/guards/toastBodyGuard.test.ts src/lib/themeContract.test.ts` | `13 passed (13)` test files, `80 passed (80)` tests in `14.35s` | ✓ PASS |
| Console surface, TerminalTab 6-addon lifecycle, BlockStreamView virtualization, TurnTrace, and surface honesty guard | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/TerminalTab.test.tsx src/components/surfaces/Console/BlockStreamView.test.tsx src/components/surfaces/Console/Console.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx src/components/surfaces/__guards__/surfaceHonesty.guard.test.ts` | `5 passed (5)` test files, `38 passed (38)` tests in `12.15s` | ✓ PASS |
| Stub / placeholder / `dangerouslySetInnerHTML` scan | `rg -n "TBD\|FIXME\|XXX\|TODO\|HACK\|PLACEHOLDER\|dangerouslySetInnerHTML\|innerHTML" crates/vox-gui/ui/src/components/surfaces/Console ...` | Exit `1` (zero matches across all Phase 443 files) | ✓ PASS |

### Probe Execution

- **Execution result:** All 18 Phase 443 behavioral, contract, and static UI guard Vitest suites (`118/118` tests run directly in this verification session; `31/31` files / `253/253` tests including `ChatSurface.test.tsx`, `InputEditor.test.tsx`, and `ChatTranscriptWave3.test.tsx` confirmed green in `443-REVIEW-FIX.md`), `tsc --noEmit`, and `vox-arch-check` ran cleanly in the foreground with `behavior_unverified: 0`.
- **Skipped probes:** None (`behavior_unverified: 0`, `overrides_applied: 0`). Live 4-theme Tauri/Playwright visual CV screenshot convergence is explicitly scoped to Phase 446 (`TERM-ROMA-CV-01..04`) per `ROADMAP.md` and `443-VALIDATION.md`.

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| `TERM-PANE-01` | `443-01`, `443-03`, `443-04` | Extract a single `<TerminalSessionPane>` shared by both standalone `Console.tsx` and `ChatSurface.tsx`'s Dockview `terminal` panel — eliminating simultaneous duplicate rendering and reconnect replay storms (`after_seq` incremental attach). | ✓ SATISFIED | `TerminalSessionPane.tsx`, `Console.tsx:396-423`, `ChatSurface.tsx:275-315`, `ChatTerminalBlockCard.tsx`, `transport.ts:942-947`; verified by `TerminalSessionPane.test.tsx`, `ChatTerminalBlockCard.test.tsx`, and `transport.console.test.ts`. |
| `TERM-PANE-02` | `443-01`, `443-03` | Import `@xterm/xterm/css/xterm.css`, wire `@xterm/addon-webgl` (with DOM fallback on WebGL context loss), `@xterm/addon-search`, `@xterm/addon-web-links`, `@xterm/addon-unicode11`, and `@xterm/addon-serialize`, and auto-flip `<TerminalSessionPane>` into full-pane VT mode within `<16 ms` on `ALT_SCREEN`. | ✓ SATISFIED | `TerminalTab.tsx:1,125-178,222-279`, `webglAddon.ts`, `xtermAddons.ts`, `terminalTheme.ts`; verified by `TerminalTab.test.tsx` and `xtermAddons.test.ts`. |
| `TERM-PANE-03` | `443-02`, `443-03` | Upgrade `BlockStreamView` with `@tanstack/react-virtual`, 16/256/TrueColor SGR ANSI span rendering, clickable `file:line:col` and URL links, sticky block headers, collapse/expand, pin, copy command/output/markdown, inline search, and inline `AwaitingInput` prompt controls (`credential_prompt` password masking + `Ctrl+B`). | ✓ SATISFIED | `sgrToSpans.ts`, `BlockCard.tsx`, `BlockStreamView.tsx`, `TermSearchBar.tsx`; verified by `sgrToSpans.test.tsx`, `BlockStreamView.test.tsx`, and `TerminalSessionPane.test.tsx`. |
| `TERM-PANE-04` | `443-03` | Support horizontal and vertical split terminal panes (`react-resizable-panels`) and named multi-PTY session tabs showing live `cwd`, git branch, and active foreground job badges. | ✓ SATISFIED | `ConsoleSplitGrid.tsx`, `ConsoleSessionBar.tsx`, `Console.tsx`; verified by `ConsoleSplitGrid.test.tsx` and `Console.test.tsx`. |

*Orphaned requirement check:* `REQUIREMENTS.md` Traceability Table maps `TERM-PANE-01`, `TERM-PANE-02`, `TERM-PANE-03`, and `TERM-PANE-04` exclusively to Phase 443 — **0 orphaned requirements**.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| None | — | Zero `TODO`/`FIXME`/`PLACEHOLDER`, zero `dangerouslySetInnerHTML`, zero raw `<select>`, zero emoji/dingbats, zero raw `@tauri-apps/api` calls outside `transport.ts` | — | 0 blockers / 0 warnings |

---

### Human Verification Required

None required for Phase 443 sign-off. Automated multi-resolution 4-theme Playwright/CV visual gate convergence is tracked as Phase 446 (`TERM-ROMA-CV-01..04`).

---

### Gaps Summary

No gaps found. All 4 ROADMAP Success Criteria, all 14 Plan Must-Have Truths (`18/18` total), all 12 Plan Prohibitions, all 31 Code + UI Review remediations (`443-REVIEW-FIX.md`), and all 8 STRIDE security threats (`443-SECURITY.md`, `threats_open: 0`) are verified in the codebase with passing automated test suites.
