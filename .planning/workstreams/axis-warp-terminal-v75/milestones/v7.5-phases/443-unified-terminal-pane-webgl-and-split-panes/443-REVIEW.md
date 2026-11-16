---
phase: 443-unified-terminal-pane-webgl-and-split-panes
reviewed: 2026-11-15T12:31:00Z
depth: deep
files_reviewed: 20
files_reviewed_list:
  - crates/vox-gui/ui/src/transport.ts
  - crates/vox-gui/ui/src/lib/terminalBridge.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx
findings:
  critical: 5
  warning: 6
  info: 4
  total: 15
status: issues_found
---

# Phase 443: Code Review Report

**Reviewed:** 2026-11-15T12:31:00Z
**Depth:** deep
**Files Reviewed:** 20
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Performed a deep cross-module adversarial review of all 20 Phase 443 frontend source files (`TERM-PANE-01..04`) and traced their IPC/event contracts across `crates/vox-gui/src/commands/pty.rs`, `crates/vox-gui/src/commands/locator.rs`, `crates/vox-terminal-core/src/{host,session,block}.rs`, `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/turn_events.rs`, `crates/vox-gui/ui/src/components/ui/Resizable.tsx`, and `crates/vox-gui/ui/src/lib/{turnEvents,turnTrace,themeContract}.ts`.

**Positive architecture, guard, and budget verification:**
- `tsc --noEmit` and all 22 Phase 443 & UI guard Vitest suites (`130/130` tests, including `turnEventContract`, `zeroEmojiAndDingbat`, `noRawNativeControlsGuard`, `transportIpcGuard`, `toastBodyGuard`, and `themeContract`) pass cleanly.
- Every new or rewritten Phase 443 component/module stays well under the `< 500` non-blank LoC ceiling (largest files are `TerminalSessionPane.tsx` at 331 total lines, `Console.tsx` at 330, and `TerminalTab.tsx` at 328).
- `ConsoleSplitGrid.tsx` uses `react-resizable-panels` v4 (`4.22.1`) `orientation` and percentage string sizing (`defaultSize="50%"`, `minSize="20%"`), caps splits at `MAX_SPLIT_PANES = 4`, and `ChatSurface.tsx` preserves opt-in `closedPanelIds` discipline so closed `terminal` dock panels never resurrect on re-render.
- `BlockCard.tsx` renders `type="password"` with `autoComplete="off"` on `credential_prompt: true` stdin prompts and avoids `dangerouslySetInnerHTML` across ANSI SGR rendering.

**Key concerns requiring remediation (`5 Critical`, `6 Warning`, `4 Info`):**
1. **Scheme-filter bypass, editor CLI flag injection, and broken `:line:col` dispatch in terminal file links (`CR-01`):** `hasUnsafeSchemePrefix` in `sgrToSpans.ts` rejects `file:/` and `file://` but allows three-slash `file:///C:/...` and `file:///etc/...` URIs, long `data:` prefixes, `..` traversal, and `-flag/file.rs:1` tokens straight into `open_locator` (`quiet_command(editor).arg(value)` without `--` or `-g`).
2. **Broken `seq` tracking, duplicate `term_session_attach` forwarder clobber, and un-reset initial VT replay (`CR-02`):** `TerminalSessionPane` reads `payload.seq` from `TermEventPayload` (which has no `seq` field in Rust), leaving `stateRef.current.seq` permanently frozen; attaching two panes (or `Console` + `TerminalDockPanel`) to one `sessionId` aborts the backend forwarder in `pty.rs:289-290` and duplicates/drops VT output.
3. **Detached orphan `<canvas>` in `WebglTerminalAddon` (`CR-03`):** `WebglTerminalAddon` allocates an unattached `<canvas>` that never connects to xterm's render pipeline — rendering zero glyphs via WebGL2 while holding up to 5 idle WebGL contexts against Chromium's per-process context limit.
4. **Cross-turn block miscorrelation in `ChatTerminalBlockCard` (`CR-04`):** Rust `receipt_turn_event` never emits `block_id`, `command`, `output`, or `exit_code`; `ChatTerminalBlockCard` falls back to the latest agent block in `terminalBridge.blocks`, causing every historical `vox_terminal_*` card across the entire chat transcript to mutate to show the newest command's output.
5. **Missing `Autopilot -> Manual` preemption on interactive terminal input (`CR-05`):** Typing directly in xterm (`term.onData`), submitting via `TerminalDockPanel`, or sending `AwaitingInput` stdin leaves the session in `Autopilot`, allowing concurrent agent command injection while the user is actively typing.

## Critical Issues

### CR-01: `file:///` URI bypass, CLI option injection, and broken `:line:col` handling in clickable terminal file locators

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts:34,197-201,229-231` (also `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts:82-84,116-122,162-169`)
**Issue:** Terminal output is untrusted (compiler diagnostics, `curl` bodies, git logs, LLM/tool stdout). Three defects combine across `sgrToSpans.ts`, `xtermAddons.ts`, and `crates/vox-gui/src/commands/locator.rs:50-55`:
1. **`file:///` and `data:` scheme bypass:** `hasUnsafeSchemePrefix` (`sgrToSpans.ts:199-200`) checks `/(?:javascript:|data:|vbscript:|file:\/\/?)$/` on a 16-char slice. A standard 3-slash `file:///etc/ssh/sshd_config.conf:1` or `file:///C:/Windows/win.ini:1` has `prefix` ending in `file:///` (3 slashes) and `source[matchIndex - 1] === '/'` (not `':'`), so `hasUnsafeSchemePrefix` returns `false` and renders a clickable button invoking `openLocator({ kind: 'file', value: 'C:/Windows/win.ini:1' })`. (Existing test `sgrToSpans.test.tsx:116` only passed on `file:///etc/passwd:1` because `passwd` lacks a `.ext` suffix.) Likewise, `data:text/plain;charset=utf-8,dir/payload.sh:1` exceeds the 16-char window and has `prev === ','`, bypassing both `sgrToSpans.ts:200` and `xtermAddons.ts:166`.
2. **Editor CLI flag injection & unrestricted path traversal:** `FILE_LOCATOR_RE` (`sgrToSpans.ts:34`, `xtermAddons.ts:83`) matches segments starting with `-` (`[\w@-]+`, e.g. `--extensions-dir/x.json:1` or `-n/foo.rs:1`) as well as `../../..` and arbitrary drive paths. `locator.rs:52-54` executes `quiet_command(&editor).arg(&value).spawn()` without `--` or workspace containment, letting a crafted path token pass CLI flags to `$VOX_EDITOR` / `code`.
3. **Broken `:line:col` & `(line,col)` editor open:** Both `sgrToSpans.ts:229` and `xtermAddons.ts:116,167` pass the unsplit suffix (`crates/vox-cli/src/main.rs:42:9` or `src/app.ts(18,4)`) as `locator.value`. Without `-g`/`--goto`, `code` treats `:42:9` and `(18,4)` as literal characters in the filename instead of line/column coordinates, and `sgrToSpans.ts:229` omits the `vox://open-file-location` event dispatched by `xtermAddons.ts:121`.
**Fix:**
```ts
// In sgrToSpans.ts (and share the same helper with xtermAddons.ts):
function hasUnsafeSchemePrefix(source: string, matchIndex: number): boolean {
  if (matchIndex <= 0) return false;
  const prev = source[matchIndex - 1];
  if (prev === ':' || prev === '/' || prev === '\\' || prev === ',') return true;
  const tokenStart = source.slice(0, matchIndex).search(/\S+$/);
  const leading = tokenStart >= 0 ? source.slice(tokenStart, matchIndex).toLowerCase() : '';
  return /^(?:javascript|data|vbscript|file|ftp|mailto| custom):/.test(leading) || leading.includes('://');
}

export function sanitizeWorkspaceFileLocator(raw: string): { path: string; line?: number; col?: number } | null {
  const paren = raw.match(/^(.*)\((\d+),(\d+)\)$/);
  const colon = !paren ? raw.match(/^(.*?):(\d+)(?::(\d+))?$/) : null;
  const path = (paren ? paren[1] : colon ? colon[1] : raw)?.trim() ?? '';
  if (!path || path.startsWith('-') || /^[A-Za-z]:[\\/]/.test(path) || path.startsWith('/') || path.startsWith('\\')) {
    return null;
  }
  const segments = path.split(/[\\/]+/);
  if (segments.some((seg) => seg === '..' || seg.startsWith('-'))) return null;
  return {
    path,
    ...(paren ? { line: Number(paren[2]), col: Number(paren[3]) } : {}),
    ...(colon ? { line: Number(colon[2]), ...(colon[3] ? { col: Number(colon[3]) } : {}) } : {}),
  };
}
```
Pass `detail.path` (after rejecting unsafe/out-of-tree paths) and dispatch `vox://open-file-location` consistently from both `sgrToSpans.ts` and `WebLinksTerminalAddon`.

### CR-02: Frozen `seq` cursor, duplicate `term_session_attach` forwarder clobber, and un-reset initial VT replay corrupt terminal streams

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx:101-104,142-181,186-197` (and `TerminalTab.tsx:168-184,256-264`, `Console.tsx:182-185`, `ChatSurface.tsx:282-286`)
**Issue:** Cross-checking `TerminalSessionPane.tsx` and `TerminalTab.tsx` against `crates/vox-gui/src/commands/pty.rs:47-57,275-296,356-390` exposes three coupled stream-synchronization bugs:
1. **`stateRef.current.seq` never advances after initial attach:** `TermEventPayload` in Rust (`pty.rs:53-57`) contains only `{ session_id, event }` — `seq` is emitted exclusively on `TermOutputPayload` (`pty.rs:47-51`), which is consumed inside `TerminalTab.tsx:169-171` and never propagated to `TerminalSessionPane`. Thus `const payloadSeq = (payload as { seq?: unknown }).seq` (`TerminalSessionPane.tsx:146`) is always `undefined`, leaving `stateRef.current.seq` frozen at its initial attach value:
   - On `SnapshotResync` (`L163-168`), `pty.rs:357-388` has *already* re-attached and emitted the delta replay, yet `TerminalSessionPane` calls `termSessionAttach(sid, prev.seq)` a second time with the frozen `prev.seq` — aborting the active Rust forwarder (`pty.rs:289-290`) and replaying stale history.
   - On manual `handleReconnect` (`L190-191`), when `snap.resync_required` is `false`, `applyAttachmentSnapshot(sid, snap, false)` (`L101`) skips `setReplaySnapshot`, silently dropping `snap.replay_utf8` delta bytes missed while disconnected.
2. **Duplicate `termSessionAttach` forwarder race across split panes and `TerminalDockPanel`:** `pty.rs:289-291` stores a single forwarder task per `session_id` and aborts the previous task on every `term_session_attach`. Mounting `TerminalDockPanel` (`ChatSurface.tsx:282-286`) while `Console` displays the same `activeSessionId` — or selecting an already-open tab in a second split pane (`Console.tsx:182-185`) — triggers concurrent `termSessionAttach(sid)` calls that abort each other's forwarders, while `TerminalTab.tsx:170-175` lacks `if (payload.seq <= paneStateRef.current.seq) return;` deduplication and writes duplicate chunks to xterm.
3. **Initial `replaySnapshot` (`epoch === 1`) appends after live chunks without resetting xterm:** `TerminalTab` (`skipInitialAttach`) subscribes to `listenTermOutput` (`L182`) *before* `TerminalSessionPane`'s `await transport.termSessionAttach(sid)` (`L179`) resolves. Any live chunk arriving during attach writes to `term` first; then `replaySnapshot` (`epoch: 1`) arrives at `TerminalTab.tsx:258`, where `if (lastReplayEpochRef.current !== null)` skips `term.reset()`, duplicating the live output at the top of the terminal.
**Fix:**
```tsx
// 1. In TerminalTab.tsx: deduplicate output frames by seq, report seq up, and always reset on full replaySnapshot:
export interface TerminalTabProps {
  // ...
  replaySnapshot?: { text: string; epoch: number; seq?: number; append?: boolean } | null;
  onOutputSeq?: (seq: number) => void;
}

// Inside listenTermOutput callback (TerminalTab.tsx:169-175):
if (payload.session_id !== sid) return;
if (typeof payload.seq === 'number' && payload.seq <= paneStateRef.current.seq) return;
if (typeof payload.seq === 'number') {
  paneStateRef.current.seq = payload.seq;
  callbacksRef.current.onOutputSeq?.(payload.seq);
}
const transition = scanAltScreenTransition(payload.data);
if (transition !== null) callbacksRef.current.onAltScreenHint?.(transition);
writeOut(payload.data);

// Inside replaySnapshot effect (TerminalTab.tsx:256-264):
useEffect(() => {
  if (!replaySnapshot || replaySnapshot.epoch === lastReplayEpochRef.current) return;
  lastReplayEpochRef.current = replaySnapshot.epoch;
  if (typeof replaySnapshot.seq === 'number') paneStateRef.current.seq = Math.max(paneStateRef.current.seq, replaySnapshot.seq);
  if (!replaySnapshot.append) {
    safeRun(() => termRef.current?.reset?.());
    if (isMock) setMockReplay(replaySnapshot.text);
  } else if (isMock && replaySnapshot.text) {
    setMockReplay((prev) => prev + replaySnapshot.text);
  }
  if (replaySnapshot.text && termRef.current) termRef.current.write(replaySnapshot.text);
}, [replaySnapshot, isMock]);
```
In `TerminalSessionPane.tsx`, wire `onOutputSeq={(seq) => { stateRef.current.seq = Math.max(stateRef.current.seq, seq); }}`, pass `append: !forceReplay && !snap.resync_required` in `handleReconnect` so delta `replay_utf8` is written, and prevent assigning one `sessionId` to two `Console` split leaves simultaneously in `Console.tsx:182-185` (focus the existing leaf holding `sid` instead).

### CR-03: `WebglTerminalAddon` creates an unattached orphan `<canvas>` that never renders xterm via WebGL2 and leaks GPU contexts

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts:33-57` (and `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx:101,118-122`)
**Issue:** `WebglTerminalAddon.activate` creates an in-memory `document.createElement('canvas')` (`L39`), never appends it to the DOM, never connects it to `terminal` or xterm's render service (`@xterm/addon-webgl` is not installed in `package.json`), calls `canvas.getContext('webgl2')` on that detached element (`L47`), and sets `this.webglActive = this.gl !== null` (`L55`). As a result:
1. Zero terminal cells are ever rendered via WebGL2 — xterm continues using its built-in DOM renderer unconditionally while `isWebglActive()` and `onRendererModeChange('webgl2')` falsely report GPU acceleration (`TERM-PANE-02` non-delivery).
2. Up to 5 simultaneous `TerminalTab` instances (4 `Console` split panes + `ChatSurface`'s `TerminalDockPanel`) allocate live unused `WebGL2RenderingContext` handles against Chromium's ~8–16 per-process WebGL context cap, starving real WebGL surfaces (`VoxGraph`, `Urbs`) for zero rendering benefit.
3. `TerminalTab.tsx:101` instantiates `new WebglTerminalAddon()` without `onRendererModeChange`, so when WebGL2 is unavailable at startup (`this.gl === null`), `onWebglFallback` (`TerminalTab.tsx:121`) never fires.
**Fix:**
Either add `@xterm/addon-webgl` to `crates/vox-gui/ui/package.json` and wrap the real `WebglAddon` inside `WebglTerminalAddon`:
```ts
import { WebglAddon } from '@xterm/addon-webgl';
import type { IDisposable, ITerminalAddon, Terminal } from '@xterm/xterm';

export class WebglTerminalAddon implements ITerminalAddon {
  private inner: WebglAddon | null = null;
  private lossSub: IDisposable | null = null;
  private disposed = false;
  private webglActive = false;
  private listeners: Array<() => void> = [];
  private readonly onRendererModeChange?: (mode: 'webgl2' | 'dom') => void;

  constructor(options?: WebglTerminalAddonOptions) {
    this.onRendererModeChange = options?.onRendererModeChange;
  }

  activate(terminal: Terminal): void {
    this.disposed = false;
    try {
      const addon = new WebglAddon();
      this.lossSub = addon.onContextLoss(() => this.simulateContextLoss());
      addon.activate(terminal);
      this.inner = addon;
      this.webglActive = true;
    } catch {
      this.inner = null;
      this.webglActive = false;
    }
    this.onRendererModeChange?.(this.webglActive ? 'webgl2' : 'dom');
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.webglActive = false;
    this.lossSub?.dispose();
    this.lossSub = null;
    try { this.inner?.dispose(); } catch { /* ignore */ }
    this.inner = null;
  }
  // keep onContextLoss / simulateContextLoss / isWebglActive / isDisposed
}
```
If external npm installation is deferred, do **not** retain a live idle `WebGL2RenderingContext` on a detached `<canvas>` after capability probing — release it immediately via `WEBGL_lose_context` or wire `onRendererModeChange` in `TerminalTab.tsx:101` so DOM fallback status is truthful.

### CR-04: `ChatTerminalBlockCard` miscorrelates historical `vox_terminal_*` receipts to the latest agent block across the entire chat transcript

**File:** `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx:20-22,100-114` (and `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx:219-232`)
**Issue:** Tracing `TurnTrace.tsx:37-53,77-79` back to Rust `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/turn_events.rs:269-366` shows that `receipt_turn_event` emits **only** `{ kind: "tool_receipt", tool, receipt_id, fulfilled, verified }` — it never includes `block_id`, `session_id`, `command`, `output`, or `exit_code`. Consequently, for every persisted or completed chat turn with a `vox_terminal_*` `tool_receipt`:
1. `eventBlockId` (`parseBlockId(event?.block_id)` at `ChatTerminalBlockCard.tsx:100`) is always `null`, so line `105` executes `[...bridgeState.blocks].reverse().find((b) => isAgentTerminalBlock(b))`. Every historical `ChatTerminalBlockCard` in Turn 1, Turn 2, ..., Turn N subscribes to `terminalBridge` and dynamically overwrites its displayed `$ command`, stdout, exit code, and `AwaitingInput` controls with whichever agent command ran **most recently**!
2. `isTerminalToolName` (`L20-22`) matches all `vox_terminal_*` tools (including read-only `vox_terminal_sessions`, `vox_terminal_read`, and `vox_terminal_kill`), causing non-execution metadata calls to render as shell command cards displaying an unrelated block's command and output.
3. In `ChatTranscript.tsx:221-228`, once a streaming turn records a `tool_receipt` for e.g. `vox_terminal_sessions` (where `e.block_id === undefined`), `!(message.events ?? []).some(... (e.block_id === undefined || ...))` evaluates to `false`, suppressing the live `activeAgentBlock` card for a subsequent `vox_terminal_exec` in the same turn.
**Fix:**
1. Restrict `ChatTerminalBlockCard` rendering in `TurnTrace` to execution/input tools (`vox_terminal_exec`, `vox_terminal_send_input`), leaving `vox_terminal_sessions`/`vox_terminal_read`/`vox_terminal_kill` as standard receipt chips.
2. Never fall back from an unindexed historical `tool_receipt` (`eventBlockId === null`) to `bridgeState.blocks`'s latest block:
```tsx
export function isTerminalBlockToolName(tool: unknown): tool is string {
  return tool === 'vox_terminal_exec' || tool === 'vox_terminal_send_input';
}

// In ChatTerminalBlockCard.tsx:100-106:
const eventBlockId = parseBlockId(event?.block_id);
const correlatedBlock: TermBlockDto | undefined = liveBlock
  ? (bridgeState.blocks.find((b) => b.id === liveBlock.id) ?? liveBlock)
  : eventBlockId !== null
    ? bridgeState.blocks.find((b) => b.id === eventBlockId)
    : undefined;
```
(Pair with emitting a server-derived `terminal_block` turn event or `block_id`/`session_id` from `turn_event_for_result` for `vox_terminal_exec` in a follow-up backend pass if per-receipt historical stdout hydration is desired.)

### CR-05: Missing `Autopilot -> Manual` preemption when typing in the VT canvas, `AwaitingInput` stdin bar, or `TerminalDockPanel`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx:94-99,129-134` (and `TerminalSessionPane.tsx:199-205,313-327`, `TerminalTab.tsx:129-132`)
**Issue:** `443-CONTEXT.md` (D-09) and `443-UI-SPEC.md:239` require that **any** user submit or keystroke while in `Autopilot` immediately preempts control mode back to `Manual` locally and via `termSessionSetControl(sid, 'manual')`. Currently, `Console.tsx` only preempts on `InputEditor` Enter-submit (`L97`) and `Escape` (`L132`). None of the other interactive PTY input surfaces preempt `Autopilot`:
- Typing directly into xterm (`TerminalTab.tsx:129-132` `term.onData` -> `termSessionInput`),
- Typing in `InputEditor` before pressing Enter,
- Submitting stdin or `Ctrl+C` on a `BlockCard` (`TerminalSessionPane.tsx:313,315`),
- Submitting a shell command from `ChatSurface`'s `TerminalDockPanel` (`TerminalSessionPane.tsx:324` -> `submitLine`).

While an operator is actively typing into an interactive prompt, password prompt, or TUI in `Autopilot`, the backend `SessionHost` remains in `TermControlMode::Autopilot`, allowing an agent to inject commands into the same PTY concurrently.
**Fix:**
Preempt `Autopilot` inside `TerminalSessionPane` (and `TerminalTab` via an `onUserInput` callback) whenever `paneState.controlMode === 'autopilot'`:
```tsx
// In TerminalSessionPane.tsx:
const preemptAutopilotIfNeeded = useCallback(() => {
  const sid = activeSidRef.current;
  if (stateRef.current.controlMode !== 'autopilot') return;
  const next: SessionPaneState = { ...stateRef.current, controlMode: 'manual' };
  stateRef.current = next;
  setPaneState(next);
  cbRef.current.onControlSyncFromBackend?.('manual');
  if (sid && typeof transport.termSessionSetControl === 'function') {
    transport.termSessionSetControl(sid, 'manual').catch(() => {});
  }
}, []);
```
Invoke `preemptAutopilotIfNeeded()` in `submitLine`, `onSendStdin`, `onInterrupt`, `TerminalTab`'s `term.onData` callback, and `Console.tsx`'s `onKeyDown` when typing into `InputEditor`.

## Warnings

### WR-01: `TerminalTab` omits `ResizeObserver` and `[data-theme]` `MutationObserver`, leaving PTY geometry at `80x24` on split-pane drag and theme colors stale on theme switch

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx:96-124,248-254,323-326` (and `sgrToSpans.ts:50-54`)
**Issue:** `fitRef.current.fit()` is only called once at mount (`L123` — while `terminal-vt-layer` is hidden with `opacity-0` in Blocks mode) and when `active` toggles (`L251`). Dragging `ResizableHandle` in `ConsoleSplitGrid`, resizing `TerminalDockPanel` in `ChatSurface`, or resizing the OS window never calls `fit.fit()`, so `term.onResize` (`L133-138`) never invokes `termSessionResize(sid, cols, rows)` and TUI apps wrap against stale geometry. Additionally, `resolveTerminalTheme()` is read only once at `new Terminal(...)` (`L98`), and neither `TerminalTab` nor `BlockCard`/`ChatTerminalBlockCard` observes `document.documentElement` `data-theme` mutations, leaving xterm and rendered ANSI spans in the previous theme until remount.
**Fix:**
```tsx
// Inside TerminalTab.tsx main useEffect (when !isMock && hostRef.current && term):
let rafId: number | null = null;
const scheduleFit = () => {
  if (rafId !== null) cancelAnimationFrame(rafId);
  rafId = requestAnimationFrame(() => {
    rafId = null;
    if (hostRef.current && hostRef.current.clientWidth > 0 && hostRef.current.clientHeight > 0) {
      safeRun(() => fit?.fit());
    }
  });
};
const ro = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(scheduleFit) : null;
if (ro && hostRef.current) ro.observe(hostRef.current);

const mo =
  typeof MutationObserver !== 'undefined' && typeof document !== 'undefined'
    ? new MutationObserver(() => {
        if (termRef.current) termRef.current.options.theme = resolveTerminalTheme();
      })
    : null;
if (mo && typeof document !== 'undefined' && document.documentElement) {
  mo.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme', 'class'] });
}
// In cleanupInstance(): if (rafId !== null) cancelAnimationFrame(rafId); ro?.disconnect(); mo?.disconnect();
```

### WR-02: `Console.tsx` desynchronizes `securityLevel`, `controlMode`, `blocks`, and `pinnedBlockIds` from backend defaults and across split panes

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx:53-54,65-67,170-185,301-319`
**Issue:** Two state-synchronization bugs affect multi-pane and initial session state:
1. **Initial policy mismatch vs backend:** Rust `LocalHost` (`crates/vox-terminal-core/src/host.rs:60,88`) creates sessions with `l3_structured_default` and `co_pilot` (returned on `TermSessionInfoDto` from `term_session_list`), whereas `TermAttachmentDto` (`crates/vox-gui/src/commands/pty.rs:31-44`) omits both fields. `Console.tsx:53-54` initializes UI state to `'l2_safe_exec'` and `'manual'` and ignores `list[0].security_level` / `list[0].control_mode` in `L173-178`, so `ConsoleSessionBar` displays `L2 Safe Exec` + `Manual` while the backend enforces `L3` + `Co-Pilot`.
2. **Split-pane state bleed on pane focus switch:** `Console.tsx` holds a single `blocks`, `pinnedBlockIds`, `focusedBlockId`, `securityLevel`, and `controlMode` state (`L53-54,65-67`). Clicking from `pane-2` back to an already-mounted `pane-1` (`Console.tsx:303`) updates `activePaneId` and `activeSessionId`, but `pane-1` does not re-emit `onBlocksChange`/`onPinnedBlocksChange`, leaving header gauges, `ConsoleSessionBar` controls, keyboard shortcuts (`Alt+Up/Down`, `p`, `f`, `r`), and `terminalBridge` bound to `pane-2`'s session data.
**Fix:**
Store per-session snapshots in `Console.tsx` (`bySession: Record<string, { blocks: TermBlockDto[]; pinnedIds: number[]; securityLevel: TermSecurityLevelId; controlMode: TermControlModeId }>`) seeded from `TermSessionInfoDto` (`fromWireSecurityLevel` / `fromWireControlMode` in `sessionReducer.ts`), pass unconditional per-leaf callbacks keyed by `leaf.sessionId`, and derive the active session's `blocks`, `pinnedBlockIds`, `securityLevel`, and `controlMode` from `bySession[activeSessionId]`.

### WR-03: Opening `TerminalDockPanel` in `ChatSurface` before `Console` spawns an untracked orphan PTY session and never updates `terminalBridge`

**File:** `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx:274-289` (and `TerminalSessionPane.tsx:120-129`)
**Issue:** Prior to visiting `Console`, `terminalBridge.activeSessionId` is `null`. Mounting `TerminalDockPanel` renders `<TerminalSessionPane compact sessionId={termState.activeSessionId} />` with default `autoOpenSession={true}` and without `onSessionResolved` or `onBlocksChange`. `TerminalSessionPane` skips `termSessionList()` and calls `termSessionOpen(80, 24)` (`L123`), spawning an extra PTY session even when daemon sessions already exist, and never publishes the resolved `sessionId` or `blocks` to `terminalBridge`.
**Fix:**
```tsx
export function TerminalDockPanel(props: IDockviewPanelProps<{ onNavigate?: (viewKey: string) => void }>) {
  const [termState, setTermState] = useState<TerminalBridgeState>(() => getTerminalBridgeState());
  useEffect(() => subscribeTerminalBridge(setTermState), []);

  useEffect(() => {
    if (termState.activeSessionId) return;
    let cancelled = false;
    (async () => {
      try {
        const list = await termSessionList();
        const sid = Array.isArray(list) && list.length > 0 ? list[0]!.id : await termSessionOpen(80, 24);
        if (!cancelled && sid) publishTerminalBridgeState({ activeSessionId: sid });
      } catch { /* ignore offline/mock */ }
    })();
    return () => { cancelled = true; };
  }, [termState.activeSessionId]);

  return (
    <div data-testid="chat-dock-terminal" className="flex h-full min-h-0 flex-col overflow-hidden bg-bg-surface text-xs">
      <TerminalSessionPane
        compact
        autoOpenSession={false}
        sessionId={termState.activeSessionId}
        onSessionResolved={(sid) => publishTerminalBridgeState({ activeSessionId: sid })}
        onBlocksChange={(blocks) => publishTerminalBridgeState({ blocks })}
        onPinnedBlocksChange={(pinnedBlockIds) => publishTerminalBridgeState({ pinnedBlockIds })}
        onOpenConsole={() => props.params.onNavigate?.('console')}
      />
    </div>
  );
}
```

### WR-04: `normalizeTerminalText` wipes partial `\r` line prefixes, ignores non-`\r` `CSI 0K`/`1K` erases, and retains unbounded historical SGR tokens

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts:56-77`
**Issue:** When `pendingCr` is set and text follows (`L71`), `tokens = tokens.filter((t) => t.kind === 'sgr')` discards the entire line's visible characters instead of overwriting from column 0 (`"abcdef\rxy"` becomes `"xy"` rather than `"xycdef"`), while preserving **every** historical SGR token ever emitted across hundreds of `\r` spinner/progress-bar updates (and never pruning redundant SGR resets when `\x1b[0m` resets brush state). Additionally, `\x1b[K` / `\x1b[0K` without a preceding `\r` (`L69`: `mode === '0' && pendingCr`) and `\x1b[1K` are ignored.
**Fix:**
Collapse accumulated SGR tokens whenever a full reset (`\x1b[m` or `\x1b[0m`) is seen, handle `CSI 2K` by clearing text on the current line (keeping only the active brush SGR), and on `\r` overwrite prefix characters beyond the replacement chunk's visible length (or on `\r\x1b[0K` clear from column 0):
```ts
if (tok === '\x1b[m' || tok === '\x1b[0m') {
  // Prior SGR state is dead once a full reset arrives after a line clear:
  if (tokens.every((t) => t.kind === 'sgr')) tokens = [];
  tokens.push({ kind: 'sgr', value: tok });
}
```

### WR-05: Search match counts and highlights diverge across `TerminalSessionPane`, `BlockCard`, and `SearchTerminalAddon`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx:229-244` (and `BlockCard.tsx:114-118,186,213`, `xtermAddons.ts:24-34,56`)
**Issue:** Three search-consistency defects produce inaccurate `X of Y` counts or misaligned selections:
1. **VT-mode false positive count (`TerminalSessionPane.tsx:240,242`):** `const totalMatches = showVt && vtSearchStats.totalMatches > 0 ? vtSearchStats.totalMatches : blockSearchMatches.length;` falls back to `blockSearchMatches.length` when `showVt` is `true` and `vtSearchStats.totalMatches === 0`. Searching in Raw VT / TUI mode for a string that exists only in an older block displays `1 of N` in `TermSearchBar` while `Prev`/`Next` in VT finds nothing.
2. **Input & pretty-printed JSON count vs highlight mismatch:** `TerminalSessionPane.tsx:234` counts matches in `${stripAnsi(b.input)}\n${stripAnsi(b.output)}`, whereas `BlockCard.tsx:186` never highlights `block.input`, and `BlockCard.tsx:116,213` pretty-prints JSON (`JSON.stringify(..., null, 2)`) before passing text to `sgrToSpans`, changing whitespace/newlines relative to raw `b.output`.
3. **Wide-character column offset in `SearchTerminalAddon` (`xtermAddons.ts:28-30,56`):** `terminal.select(col, row, length)` expects terminal cell columns, but `collectMatches` passes UTF-16 code-unit indices from `translateToString(true)`, shifting the selection highlight whenever CJK/fullwidth cells (`wcwidth === 2`) precede a match on the same row.
**Fix:**
```tsx
// In TerminalSessionPane.tsx:240-243 — do not fall back to block counts when showVt is active:
const totalMatches = showVt ? vtSearchStats.totalMatches : blockSearchMatches.length;
const clampedCursor =
  blockSearchMatches.length > 0
    ? ((blockMatchCursor % blockSearchMatches.length) + blockSearchMatches.length) % blockSearchMatches.length
    : 0;
const activeMatchIndex = showVt
  ? vtSearchStats.activeMatchIndex
  : blockSearchMatches.length > 0
    ? clampedCursor + 1
    : 0;
```
Match `TerminalSessionPane`'s block search corpus to `BlockCard`'s rendered output (or highlight `block.input` in `BlockCard.tsx:186`), and compute cell-column offsets in `SearchTerminalAddon` using `unicode11Wcwidth` over preceding codepoints on the matched line.

### WR-06: Async race conditions and stale confirmation banner in `Console.tsx` / `ConsoleSessionBar.tsx` session & split lifecycle

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx:194-203,214-225` (and `ConsoleSessionBar.tsx:54,146-160`)
**Issue:**
1. `handleCloseSession` (`Console.tsx:214-225`) captures `sessions` and `activeSessionId` from render closure across `await termSessionClose(sid)` and `await termSessionOpen(80, 24)`, clobbering concurrent session additions and leaving `runningBySession[sid]` and `focusedBlockId` stale after `sid` closes.
2. `handleSplit` (`Console.tsx:194-203`) checks `countPaneLeaves(splitRoot) >= MAX_SPLIT_PANES` before `await termSessionOpen(80, 24)` with no in-flight lock; rapid clicks at 3 panes spawn extra daemon PTY sessions whose leaves are silently discarded by `splitPaneLeaf`.
3. `ConsoleSessionBar.tsx:54,146-160` retains `confirmCloseSid` even if that session finishes its foreground job or is removed from `sessions`, leaving a stale `term-close-confirm-bar` alert visible.
**Fix:**
Guard `handleSplit` with a `splittingRef` boolean (and close any excess opened session if `countPaneLeaves` hit `MAX_SPLIT_PANES` while awaiting), compute `remaining` from a `sessionsRef` (cleaning `runningBySession[sid]` and resetting `focusedBlockId`), and add in `ConsoleSessionBar.tsx`:
```tsx
React.useEffect(() => {
  if (confirmCloseSid && !sessions.some((s) => s.id === confirmCloseSid)) {
    setConfirmCloseSid(null);
  }
}, [confirmCloseSid, sessions]);
```

## Info

### IN-01: Unreachable `tool_start` branches in `TurnTrace.tsx`, `ChatTranscript.tsx`, and `ChatTerminalBlockCard.tsx`

**File:** `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx:40` (also `ChatTranscript.tsx:146-151,223`, `ChatTerminalBlockCard.tsx:84`)
**Issue:** Neither `TURN_EVENT_KINDS` (`crates/vox-gui/ui/src/lib/turnEvents.ts:7-15`) nor Rust `turn_events.rs` ever emits `kind: 'tool_start'` in `TurnEventDto[]`. Checking `ev.kind === 'tool_start'` is dead code in production (and causes the `Thought for Xs · N tool calls` badge in `ChatTranscript.tsx:146-151` to never show tool counts).
**Fix:** Remove dead `tool_start` checks or derive tool call counts from `e.kind === 'tool_receipt'` if intended.

### IN-02: Unused standalone attach branch and un-cleared `mockReplay` on empty-session switch in `TerminalTab.tsx`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx:79,186-243,260-263`
**Issue:** All production mounts (`TerminalSessionPane.tsx:319`) pass `skipInitialAttach`, leaving the 58-line standalone attach/resync effect branch (`L186-243`) unused outside unit tests. In `isMock` mode, switching from a populated session to a session with `replaySnapshot.text === ''` skips `setMockReplay('')` (`L260` guards `if (replaySnapshot.text)`), retaining the previous session's mock text.
**Fix:** Always update `setMockReplay(replaySnapshot.text)` when `isMock` is `true` regardless of whether `replaySnapshot.text` is empty, and consider consolidating session attachment exclusively in `TerminalSessionPane`.

### IN-03: Dead `triggerManualTakeover` helper and legacy default IDs in `terminalBridge.ts`

**File:** `crates/vox-gui/ui/src/lib/terminalBridge.ts:27-34,122-124`
**Issue:** `DEFAULT_STATE` uses legacy `'L3'` / `'copilot'` literals while `Console.tsx` and `ConsoleSessionBar.tsx` use `'l3_workspace_write'` / `'co_pilot'`; additionally, `triggerManualTakeover()` (`L122-124`) has no callers and mutates in-memory bridge state without calling `termSessionSetControl` on the backend.
**Fix:** Normalize `DEFAULT_STATE` to `'l3_workspace_write'` and `'co_pilot'`, and either wire `triggerManualTakeover` to backend `termSessionSetControl` or remove the unused export.

### IN-04: Duplicate `TERMINAL_MONO_STYLE` definition in `BlockCard.tsx` instead of importing `TERMINAL_MONO_FONT_STACK`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx:14-19`
**Issue:** `BlockCard.tsx` re-hardcodes `"var(--font-terminal-mono, 'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace)"` instead of importing `TERMINAL_MONO_FONT_STACK` from `./terminalTheme` (unlike `TerminalTab.tsx`, `TerminalSessionPane.tsx`, `TermSearchBar.tsx`, and `ChatTerminalBlockCard.tsx`).
**Fix:** Import `TERMINAL_MONO_FONT_STACK` from `./terminalTheme` in `BlockCard.tsx` so terminal font stack updates stay single-sourced.

---

_Reviewed: 2026-11-15T12:31:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
