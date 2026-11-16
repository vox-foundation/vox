---
phase: 443-unified-terminal-pane-webgl-and-split-panes
milestone: v7.5
workstream: axis-warp-terminal-v75
fixed_at: 2026-11-15T13:24:00Z
review_path: .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-REVIEW.md
ui_review_path: .planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-UI-REVIEW.md
fix_scope: all_critical_warning_info_and_ui_review
iteration: 1
findings_in_scope: 15
ui_findings_in_scope: 16
fixed: 31
skipped: 0
status: all_fixed
---

# Phase 443: Code & UI Review Fix Report

**Fixed at:** `2026-11-15T13:24:00Z`
**Source reviews:**
- `.planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-REVIEW.md` (`5` Critical, `6` Warning, `4` Info = `15/15`)
- `.planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/443-UI-REVIEW.md` (`1` BLOCKER + `15` Pillar 1–6 warnings = `16/16`)
**Iteration:** `1`

**Summary:**
- Code review findings in scope: `15` (`CR-01..CR-05`, `WR-01..WR-06`, `IN-01..IN-04`) — **all `15` fixed**
- UI review findings in scope: `16` (`Top Fix #1 BLOCKER`, `Top Fix #2..#3`, and all Pillar 1–6 warnings) — **all `16` fixed**
- Skipped: `0`
- Untouched restricted files: `crates/vox-gui/ui/src/transport.ts`, `package.json`, `pnpm-lock.yaml`, `crates/vox-gui/ui/src/lib/turnEvents.ts`, `contracts/gui/turn-event-kinds.v1.json` (`0` edits)
- Non-blank LoC budget (`< 450`): verified across all `26` new/refactored Phase 443 `.ts`/`.tsx` production and test files (`72`..`437` non-blank LoC; `79`..`443` total lines)

## Fixed Issues (`443-REVIEW.md`)

### Critical (`CR-01` .. `CR-05`)

- **`CR-01` (`sgrToSpans.ts`, `xtermAddons.ts`, `sgrToSpans.test.tsx`, `xtermAddons.test.ts`):**
  - Hardened `hasUnsafeSchemePrefix` to inspect full preceding token context (guards on `:`, `/`, `\`, `,`, `-`, `://`, and `scheme:` prefixes so `file:///etc/ssh/sshd_config.conf:1` and `data:text/plain;charset=utf-8,dir/payload.sh:1` are rejected before suffix matching).
  - Added shared `sanitizeWorkspaceFileLocator` and `dispatchWorkspaceFileLocator`, rejecting leading `-` flags (`-n/foo.rs:1`), `..` path traversal (`../secret.rs:1`), and absolute/Windows-drive paths, passing sanitized `detail.path` (without `:line:col` suffix) to `voxTransport.openLocator({ kind: 'file', value: detail.path })`, and dispatching `vox://open-file-location` consistently from both `sgrToSpans` and `WebLinksTerminalAddon`.
  - Added unit tests in `sgrToSpans.test.tsx` and `xtermAddons.test.ts` covering all unsafe schemes/paths and valid `crates/vox-cli/src/main.rs:42:9`.
- **`CR-02` (`TerminalTab.tsx`, `TerminalSessionPane.tsx`, `Console.tsx`, `TerminalTab.test.tsx`, `TerminalSessionPane.test.tsx`, `Console.test.tsx`):**
  - Added `payload.seq <= paneStateRef.current.seq` frame deduplication on `listenTermOutput` in `TerminalTab.tsx` and propagated `onOutputSeq(payload.seq)` to `TerminalSessionPane.tsx` so `stateRef.current.seq` advances live.
  - Reset xterm (`termRef.current?.reset?.()`) on every non-append `replaySnapshot` (`!replaySnapshot.append`, including initial `epoch === 1`), and passed `append: !forceReplay && !snap.resync_required` in `handleReconnect` so delta `replay_utf8` bytes are written without wiping prior buffer content.
  - Removed duplicate `termSessionAttach` calls inside `SnapshotResync` handlers in both `TerminalSessionPane.tsx` and `TerminalTab.tsx` (`pty.rs` already re-attaches before emitting `SnapshotResync`).
  - Added a module-level per-`sessionId` initial-attach ref-count/deduplicator in `TerminalSessionPane.tsx` (`acquireInitialAttach` / `releaseInitialAttach`) so mounting `Console` and `ChatSurface`'s `TerminalDockPanel` against the same `sessionId` shares one `termSessionAttach` forwarder, and updated `Console.tsx` (`handleSelectSession` / `applyFocusTarget`) to focus an existing split leaf already holding `sid` rather than binding two split leaves to one session.
- **`CR-03` (`webglAddon.ts`, `TerminalTab.tsx`, `xtermAddons.test.ts`, `TerminalTab.test.tsx`):**
  - Eliminated retained detached `<canvas>` elements and idle `WebGL2RenderingContext` leaks in `WebglTerminalAddon.activate` by immediately releasing any probed context via `gl.getExtension('WEBGL_lose_context')?.loseContext()`, preserving Chromium's per-process WebGL context pool for `VoxGraph` and `Urbs` without modifying `package.json`.
  - Wired `new WebglTerminalAddon({ onRendererModeChange })` in `TerminalTab.tsx` (deduplicated per mount via `notifyFallbackOnce`) so unavailable WebGL2 (`gl === null`) or `simulateContextLoss()` truthfully triggers `onWebglFallback`.
- **`CR-04` (`ChatTerminalBlockCard.tsx`, `TurnTrace.tsx`, `ChatTranscript.tsx`, `ChatTerminalBlockCard.test.tsx`):**
  - Added `isTerminalBlockToolName(tool)` (`tool === 'vox_terminal_exec' || tool === 'vox_terminal_send_input'`) and restricted `TurnTrace.tsx` block-card promotion to execution/input tools, keeping `vox_terminal_sessions`, `vox_terminal_read`, and `vox_terminal_kill` as standard receipt chips (`ChatTurnEventRow.tsx`).
  - Removed the `eventBlockId === null` fallback to `[...bridgeState.blocks].reverse().find(...)` in `ChatTerminalBlockCard.tsx` so historical turns never mutate to display subsequent commands' stdout/status.
  - Fixed streaming `activeAgentBlock` suppression in `ChatTranscript.tsx` so non-exec receipts (`vox_terminal_sessions` / `vox_terminal_read`) or unindexed receipts do not hide live running `vox_terminal_exec` cards.
- **`CR-05` (`Console.tsx`, `TerminalSessionPane.tsx`, `TerminalTab.tsx`, `terminalBridge.ts`, `Console.test.tsx`, `TerminalSessionPane.test.tsx`, `TerminalTab.test.tsx`):**
  - Implemented immediate `Autopilot -> manual` preemption (`termSessionSetControl(sid, 'manual')` + local/bridge state sync) across all interactive user PTY inputs: `TerminalTab` `term.onData` (`onUserInput`), `BlockCard` `onSendStdin` and `onInterrupt`, `TerminalSessionPane` `submitLine` and compact `dock-term-input` typing, and `Console` `InputEditor` typing (`onChangeCapture` / `onKeyDownCapture`), submit, and `Escape`.

### Warnings (`WR-01` .. `WR-06`)

- **`WR-01` (`TerminalTab.tsx`, `terminalTheme.ts`, `sgrToSpans.ts`, `BlockCard.tsx`, `ChatTerminalBlockCard.tsx`, `TerminalTab.test.tsx`):**
  - Added `ResizeObserver` (`requestAnimationFrame` -> `fit.fit()` when `clientWidth > 0 && clientHeight > 0`) and `document.documentElement` `MutationObserver` (`['data-theme', 'class']` -> `term.options.theme = resolveTerminalTheme()`) in `TerminalTab.tsx` with full unmount cleanup (`cancelAnimationFrame`, `disconnect`).
  - Exported `LIGHT_ANSI_16_PALETTE`, `DARK_ANSI_16_PALETTE`, `resolveAnsi16PaletteArray`, and `useDocumentThemeAttr()` from `terminalTheme.ts`, importing them in `sgrToSpans.ts`, `BlockCard.tsx`, and `ChatTerminalBlockCard.tsx` so xterm and rendered ANSI spans stay single-sourced and update live on theme switch.
- **`WR-02` (`Console.tsx`, `sessionReducer.ts`, `sessionReducer.test.ts`, `Console.test.tsx`):**
  - Aligned frontend initial defaults with Rust `LocalHost` (`securityLevel: 'l3_workspace_write'`, `controlMode: 'co_pilot'`) in `sessionReducer.ts` and `Console.tsx`.
  - Tracked per-session `{ blocks, pinnedIds, securityLevel, controlMode }` in `Console.tsx` (`sessionUiById`) seeded from `TermSessionInfoDto` (`fromWireSecurityLevel` / `fromWireControlMode`) and fixed `meta.blocksCount >= 0` so zero-block session resets update tab counts accurately.
- **`WR-03` (`ChatSurface.tsx`, `ChatTerminalBlockCard.test.tsx`):**
  - Updated `TerminalDockPanel` in `ChatSurface.tsx` to query `termSessionList()` first when `termState.activeSessionId` is `null` before falling back to `termSessionOpen(80, 24)`, passing `autoOpenSession={false}` and publishing `activeSessionId`, `blocks`, `pinnedBlockIds`, and `controlMode` to `terminalBridge` while leaving Dockview's `closedPanelIds` untouched.
- **`WR-04` (`sgrToSpans.ts`, `sgrToSpans.test.tsx`):**
  - Rewrote `normalizeTerminalText` as a column-aware cell line buffer supporting partial `\r` column-0 overwrite (`"abcdef\rxy"` -> `"xycdef"`), `CSI 0K` / `CSI K` (cursor to EOL), `CSI 1K` (SOL to cursor), `CSI 2K` (entire line), and SGR reset pruning (`\x1b[0m` / `\x1b[m`) so spinner/progress loops never accumulate unbounded historical SGR tokens.
- **`WR-05` (`TerminalSessionPane.tsx`, `BlockCard.tsx`, `xtermAddons.ts`, `TerminalSessionPane.test.tsx`, `xtermAddons.test.ts`):**
  - Fixed `totalMatches` / `activeMatchIndex` in `TerminalSessionPane.tsx` to branch strictly on `showVt ? vtSearchStats.totalMatches : blockSearchMatches.length` (eliminating false block fallback when VT has `0` matches).
  - Shared `formatRenderedBlockOutput` / `formatBlockSearchCorpus` from `BlockCard.tsx` (accounting for pretty-printed JSON and normalized terminal lines) so `TerminalSessionPane` match counts align 1:1 with `BlockCard` highlights.
  - Exported `unicode11Wcwidth` / `stringCellWidth` in `xtermAddons.ts` and computed cell-column offsets in `SearchTerminalAddon` and `WebLinksTerminalAddon` so CJK/wide codepoints (`wcwidth === 2`) never shift terminal selections.
- **`WR-06` (`Console.tsx`, `ConsoleSessionBar.tsx`, `ConsoleSplitGrid.test.tsx`):**
  - Added `splittingRef` in-flight lock (closing any excess session if `MAX_SPLIT_PANES` is reached concurrently) in `Console.tsx` `handleSplit`, read latest sessions via `sessionsRef` while cleaning `runningBySession[sid]`, `sessionUiById[sid]`, and `focusedBlockId` in `handleCloseSession`, and added stale-session/finished-command auto-clear plus `Escape` key dismissal for `confirmCloseSid` in `ConsoleSessionBar.tsx`.

### Info (`IN-01` .. `IN-04`)

- **`IN-01` (`TurnTrace.tsx`, `ChatTranscript.tsx`, `ChatTerminalBlockCard.tsx`):** Removed unreachable `kind === 'tool_start'` branches and derived `ChatTranscript`'s `Thought for Xs · N tool calls` badge count from `e.kind === 'tool_receipt'`.
- **`IN-02` (`TerminalTab.tsx`, `TerminalTab.test.tsx`):** Unconditionally updated `setMockReplay(replaySnapshot.text)` on non-append snapshots even when `replaySnapshot.text === ''` so switching to an empty session clears stale mock output.
- **`IN-03` (`terminalBridge.ts`, `sessionReducer.test.ts`):** Normalized `DEFAULT_STATE` in `terminalBridge.ts` to `'l3_workspace_write'` / `'co_pilot'` and wired `triggerManualTakeover` to `termSessionSetControl(sid, 'manual')`.
- **`IN-04` (`BlockCard.tsx`):** Replaced duplicate font-family string in `BlockCard.tsx` with `TERMINAL_MONO_FONT_STACK` imported from `./terminalTheme`.

## Fixed UI Review Issues (`443-UI-REVIEW.md`)

- **Top Fix #1 (`[BLOCKER]` Global single-key shortcut hijack in `Console.tsx`):** Guarded `p`, `f`, `a`, `r` shortcuts with `!isInteractive && !e.metaKey && !e.ctrlKey && !e.altKey && insideConsole && target` and expanded `isInteractiveShortcutTarget` to exclude `input, textarea, select, button, [role="button"], [role="combobox"], [role="listbox"], [role="option"], [role="dialog"], [contenteditable="true"]`. Verified in `Console.test.tsx` that `Ctrl+A`, `Meta+A`, `Ctrl+R`, `Meta+R`, and button-focused `a`/`r` never open `SendToAgent` or re-run commands.
- **Top Fix #2 (VT zero-match search leak & focus restoration in `TerminalSessionPane.tsx`):** Branched search totals strictly on `showVt` and restored focus to the active VT helper textarea or `paneRootRef` inside `closeSearchAndRestoreFocus` on `Escape` (`443-UI-SPEC.md:220`).
- **Top Fix #3 (`JetBrains Mono` on `<pre>` in `ChatTerminalBlockCard.tsx` & `--color-status-fail` on `Confirm Close Session` in `ConsoleSessionBar.tsx`):** Applied `style={TERMINAL_CARD_MONO_STYLE}` directly on `<pre data-testid="chat-term-block-output">` (overriding UA `pre { font-family: monospace }`) and replaced `variant="danger"` (`bg-red-500`) on `Confirm Close Session` with `border-(--color-status-fail) bg-(--color-status-fail) text-bg-base hover:opacity-90` plus `Escape` dismissal.
- **Pillar 1 (Copywriting & WCAG 2.5.3 Label-in-Name):** Aligned `Interrupt Process` (`BlockCard.tsx`), `Exit Raw View` & segmented `View Blocks` / `View Raw VT` (`TerminalSessionPane.tsx`), `Prev` / `Next` / `Esc` (`TermSearchBar.tsx`), `Agent {openAgentId}` / `Return to Terminal` (`Console.tsx`), and terminal status seals (`completed` / `failed` / `exit N` instead of `verified` / `unverified` in `ChatTerminalBlockCard.tsx`).
- **Pillar 2 (Visuals):** Replaced direct `lucide-react` `Columns2`/`Rows2` import in `ConsoleSessionBar.tsx` with `Icon.splitRight` and `Icon.splitDown` (`1.5px` stroke SVGs in `Icons.tsx`), converted `term-view-mode-toggle` into a segmented `View Blocks` / `View Raw VT` group with active accent state, wrapped compact `Search Output` in `<Tooltip>`, replaced hand-rolled empty state in `BlockStreamView.tsx` with `<EmptyState>`, added `min-w-[280px] min-h-[180px]` floor on multi-pane leaves in `ConsoleSplitGrid.tsx`, and added `animate-pulse` on `TUI Active` dot in `TerminalSessionPane.tsx`.
- **Pillar 3 (Color):** Restrained 10% accent budget on repeated `BlockCard` badges/actions (`structured json`, `Focus in Chat`, `Ask Agent`) and compact `term:<sid>` to `text-text-secondary`, reserving accent for pinned cards/badges, active session tab, active view-mode pill, active search/focus rings, and `Open in Console`.
- **Pillar 4 (Typography):** Added `letterSpacing: 0` and ligature-off styles on `TerminalTab.tsx`, standardized `Console.tsx` toolbar/drawer `<Button size="xs" variant="outline">` (`11px` / `font-semibold`), and set `14px/600` on active session tabs vs `11px/600` on inactive tabs in `ConsoleSessionBar.tsx`.
- **Pillar 5 (Spacing):** Eliminated `.5` fractional spacing leaks (`py-1` on `TerminalTab.tsx` mock badge, `size="xs"` `h-6 px-2 py-1` on `Console.tsx` buttons, `gap-1 py-1` on `TurnTrace.tsx` summary), threaded `compact` `p-2` card density through `TerminalSessionPane` -> `BlockStreamView` -> `BlockCard`, and updated sticky `BlockCard` header to `py-2 backdrop-blur-xs`.

## Verification Performed

1. **Targeted Vitest Suite (`31/31` files, `253/253` tests green):**
   ```text
   $ timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/components/surfaces/Console src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/turnEventContract.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx src/components/surfaces/Chat/ChatSurface.test.tsx src/guards src/components/surfaces/__guards__/surfaceHonesty.guard.test.ts src/lib/themeContract.test.ts
   Test Files  31 passed (31)
        Tests  253 passed (253)
   ```
2. **TypeScript Typecheck (`0` errors):**
   ```text
   $ timeout 300s pnpm --dir crates/vox-gui/ui exec tsc --noEmit
   (exit 0)
   ```
