# Phase 443: Unified `<TerminalSessionPane>`, Real `@xterm/addon-webgl`, Auto-TUI Handoff & Split Panes - Research

**Researched:** 2026-11-15
**Domain:** React 18 + TypeScript 5.6 + `@xterm/xterm@6.0.0` + `react-resizable-panels@4.22.1` + `@tanstack/react-virtual@3.13.23` + Tauri v2 PTY/SessionHost IPC (`vox-gui` / `vox-terminal-core`)
**Confidence:** HIGH (100% verified from in-repo `.d.ts`, `.mjs`, Rust `serde` structs, Tauri command handlers, and Vitest static guards)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### 1. Unified `<TerminalSessionPane>` & Zero-Duplicate Delta Streaming (`TERM-PANE-01`)
- **Single Shared Component (`TerminalSessionPane.tsx`):** Create `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx` and mount it in both `Console.tsx` (inside each split pane) and `ChatSurface.tsx` (`TerminalDockPanel`).
- **Eliminate Duplicate Stacked Viewport:** Instead of rendering `<BlockStreamView>` (`max-h-[340px]`) above an always-visible raw `<TerminalTab>` that echoes the exact same output twice, `<TerminalSessionPane>` presents **one unified viewport** in `Blocks` mode (virtualized `BlockStreamView` + persistent mounted xterm layer kept warm for instant `<16 ms` TUI takeover or optional Raw VT toggle) and seamlessly flips to full-bleed interactive xterm VT canvas in `ALT_SCREEN` / `Raw VT` mode. Preserve `data-testid="block-stream-view"` and `aria-label="terminal"` / `data-testid="terminal-viewport"` DOM landmarks so accessibility regions and existing Vitest suites remain green.
- **Eliminate `replay_utf8` Re-Attach Storms:** Update `transport.ts::termSessionAttach(sessionId: string, afterSeq?: number | null)` so calling `termSessionAttach(sid)` with 1 argument preserves exact existing call signatures (`{ sessionId }`), while incremental refreshes on `SessionEvent` (`BlockOpened`, `BlockClosed`, `BlockAwaitingInput`, `BlockPinned`, `CwdChanged`, `AltScreenChanged`) pass `afterSeq` (`{ sessionId, afterSeq }`) and **never** re-write `replay_utf8` onto `termRef` unless `snap.resync_required === true`. Remove the post-submit `termSessionAttach` full-replay write in `TerminalTab.tsx`.
- **Dockview Non-Resurrection Rule:** `TerminalDockPanel` inside `ChatSurface.tsx` mounts `<TerminalSessionPane compact onOpenConsole={() => props.params.onNavigate?.('console')} />` without adding any auto-recreate logic that bypasses `ChatSurface.tsx`'s `closedPanelIds` guard (`DockWorkspaceShell.tsx:18-29`).
- **Live Chat Inline Terminal Cards (`ChatTerminalBlockCard.tsx`):** Render live streaming `vox_terminal_*` cards inside `ChatTranscript.tsx` / `TurnTrace.tsx` whenever a turn contains `vox_terminal_*` tool receipts or active agent-originated blocks from `terminalBridge`, complete with command header, live status/exit badge, ANSI-rendered output preview, and 1-click **"Open in Console"** (`data-testid="chat-term-open-console"`).

#### 2. `@xterm/xterm/css/xterm.css`, Self-Contained Addon Suite & `<16 ms` Auto-TUI Handoff (`TERM-PANE-02`)
- **Zero New npm Packages (`00-core.md` Rule 5):** `crates/vox-gui/ui/package.json` ships `@xterm/xterm@^6.0.0` and `@xterm/addon-fit@^0.11.0` (no `@xterm/addon-{webgl,search,web-links,unicode11,serialize}` packages in `node_modules`). Implement clean, strongly-typed `ITerminalAddon` classes in-repo (`webglAddon.ts` + `xtermAddons.ts`: `WebglTerminalAddon`, `SearchTerminalAddon`, `WebLinksTerminalAddon`, `Unicode11TerminalAddon`, `SerializeTerminalAddon`) using `@xterm/xterm` v6's native `registerLinkProvider`, `buffer.active`, and WebGL2 context management with deterministic `onContextLoss` + headless `no-tauri` / jsdom DOM fallback.
- **Import `@xterm/xterm/css/xterm.css`:** Explicitly import `@xterm/xterm/css/xterm.css` in `TerminalTab.tsx` so `.xterm-helper-textarea` is properly positioned/clipped (`opacity: 0; width: 0; height: 0`) instead of bleeding as a cream textarea box.
- **`<16 ms` Synchronous `ALT_SCREEN` Auto-TUI Takeover:** Detect `ALT_SCREEN` from three synchronized sources: (a) initial `TermAttachmentDto.alt_screen`, (b) `TERM_EVENT_EVENT` `AltScreenChanged`, and (c) zero-RTT client-side escape scanner on `TERM_OUTPUT_EVENT` chunks (`\x1b[?1049h` / `\x1b[?1047h` / `\x1b[?47h` enter; `\x1b[?1049l` / `\x1b[?1047l` / `\x1b[?47l` leave). Keep the xterm instance mounted in the pane container and toggle CSS visibility/stacking synchronously (`<16 ms`, zero xterm remount/re-open) while invoking `fit.fit()` and `term.focus()`.

#### 3. Virtualized ANSI Command Blocks, Search, Pin, `Ctrl+B` & Link Actions (`TERM-PANE-03`)
- **Pure-TS `sgrToSpans.ts` (Zero `dangerouslySetInnerHTML`):** Parse SGR 16-color (`30–37`, `90–97`, `40–47`, `100–107`), 256-color (`38;5;n`, `48;5;n`), 24-bit TrueColor (`38;2;r;g;b`, `48;2;r;g;b`), text attributes (bold, dim, italic, underline, inverse, strikethrough, reset), and embedded `file:line(:col)?` + `https?://` / `localhost` links into safe React elements (`<span>` and clickable link `<button>`/`<a>` nodes).
- **`@tanstack/react-virtual` Block Virtualization:** Virtualize `BlockStreamView.tsx` when block count exceeds viewport threshold while falling back cleanly to direct rendering when container `clientHeight === 0` (preserving 100% deterministic jsdom Vitest rendering without mock layout hacks).
- **Rich Block Chrome & Actions:** Display `cwd`, `git_branch`, origin badge (`user` vs `agent:<id>`), exit seal (`exit 0`, `exit N`, `running`, `awaiting input`, `cancelled`), duration, sticky header (`sticky top-0 z-10`), collapse/expand toggle, `AwaitingInput` inline stdin prompt bar + `Ctrl+B` (`termSessionPromoteBg`) background promotion button, backend-synced Pin (`termSessionPin`), and `Cmd/Ctrl+F` search bar (`data-testid="term-search-bar"`) with match navigation across blocks and active VT buffer.

#### 4. Split Panes & Multi-Session Tabs (`TERM-PANE-04`)
- **Horizontal & Vertical Split Panes (`ConsoleSplitGrid.tsx`):** Use installed `react-resizable-panels` (`^4.22.1`) to support recursive or 2D horizontal/vertical pane splitting (`Split Right`, `Split Down`, `Close Pane`) inside `Console.tsx`, with active-pane focus ring and independent `sessionId` assignment per pane.
- **Session Tab Bar:** Upgrade `Console.tsx` session tabs to display live `cwd` basename (from `TermSessionInfoDto.cwd` / `CwdChanged` events), `git_branch` badge, foreground command title when a block is `Running`/`AwaitingInput`, block count, and session close action, while syncing `termSessionSetSecurity` and `termSessionSetControl` to the backend (mapping UI option values cleanly to `TermSecurityLevelWire` / `TermControlModeWire` while keeping `Console.test.tsx` option names `'L2 Safe Exec'`, `'L1 Read-Only'`, etc. intact).

#### 5. Non-Negotiable Repo Guardrails
- **Zero emojis or banned glyphs** (`src/guards/zeroEmojiAndDingbat.test.ts` — use `<Icon.*>` from `components/ui/Icons.tsx` or ASCII text only; never use Unicode geometric shapes, arrows, checkmarks, or return symbols in TS/TSX source strings).
- **Zero raw `<select>` or `<input type="range">`** (`src/guards/noRawNativeControlsGuard.test.ts` — use `<VoxSelect>`).
- **Zero direct `@tauri-apps/api/*` imports outside `src/transport.ts`** (`src/guards/transportIpcGuard.test.ts`).
- **Sanitize all error toasts via `sanitizeErrorForToast`** (`src/guards/toastBodyGuard.test.ts`).
- **Every source file `< 500` non-blank LoC** (`vox-code-audit` `god_object` detector; split helpers cleanly across focused modules).

### Claude's Discretion
- Exact decomposition of internal subcomponents under `crates/vox-gui/ui/src/components/surfaces/Console/` (`sessionReducer.ts`, `sgrToSpans.ts`, `xtermAddons.ts`, `ConsoleSplitGrid.tsx`, `TerminalSessionPane.tsx`) and `Chat/` (`ChatTerminalBlockCard.tsx`) to keep every file well below `< 500` non-blank LoC.
- Virtualization threshold in `BlockStreamView.tsx` (e.g., virtualize when `blocks.length > 25` AND `scrollContainer.clientHeight > 0`).
- Exact keyboard shortcut bindings for split right/down and search (`Cmd/Ctrl+F` scoped to focused terminal pane so browser/global search is not hijacked outside Console/TerminalDock).

### Deferred Ideas (OUT OF SCOPE)
- Multi-line `ScriptoriumPromptEditor`, `Ctrl+R` persistent history, `$PATH` completions, and `.vox` Runbooks -> Phase 444 (`TERM-WARP-01..04`).
- `?prompt` LLM streaming, 1-click *Explain / Fix Failed Block*, Roman wax-seal Co-Pilot Approval Cards, and live `/slash` commands -> Phase 445 (`TERM-AGENT-01..04`).
- Global `index.css` `--font-terminal-mono` `:not(.xterm)` rule changes and 4-theme Playwright CV Flywheel convergence -> Phase 446 (`TERM-ROMA-CV-01..04`).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| `TERM-PANE-01` | **Unified `<TerminalSessionPane>`:** One shared `<TerminalSessionPane>` powers both the full `Console` surface (`Console.tsx`) and the docked Chat panel (`ChatSurface.tsx::TerminalDockPanel`), eliminating duplicate stacked `BlockStreamView` + `TerminalTab` rendering, replacing full `replay_utf8` re-attaches with incremental `after_seq` delta streaming, and streaming live `vox_terminal_*` cards directly inside `ChatTranscript.tsx` with 1-click **"Open in Console"**. | §Deep Verification 3 (`SessionEvent` reducer + `termSessionAttach(sid, afterSeq)` + forwarder-churn avoidance) and §Deep Verification 5 (`TerminalDockPanel` `closedPanelIds` invariant + `ChatTerminalBlockCard` + `turn-event-kinds.v1.json` contract safety). |
| `TERM-PANE-02` | **Real `@xterm/xterm` Addon Suite & Auto-TUI Handoff:** Load `@xterm/xterm/css/xterm.css` plus typed `ITerminalAddon` implementations (`WebglTerminalAddon`, `FitAddon`, `WebLinksTerminalAddon`, `SearchTerminalAddon`, `Unicode11TerminalAddon`, `SerializeTerminalAddon`) with graceful `onContextLoss` DOM fallback, and switch synchronously (`< 16 ms`) between Command Block stream mode and full-bleed VT canvas mode on `ALT_SCREEN` (`?1049h`/`?1049l`). | §Deep Verification 2 (`xterm.d.ts` `ITerminalAddon`, `ILinkProvider` 1-based coords, `allowProposedApi: true` runtime requirement in `xterm.mjs:49` for `unicode`, `.xterm-helper-textarea` CSS clipping, 3-source `ALT_SCREEN` detector, mock-safe optional chaining). |
| `TERM-PANE-03` | **Virtualized ANSI Command Blocks:** Render Command Blocks via `@tanstack/react-virtual` and a pure-TS SGR ANSI-to-React-span parser (`sgrToSpans.ts`, 16/256/TrueColor, zero `dangerouslySetInnerHTML`), displaying `cwd`, `git` branch, origin badge (`user` vs `agent:<id>`), exit seal, duration, sticky header, collapse/expand, `Cmd/Ctrl+F` search, Pin, `Ctrl+B` background promotion, and clickable `file:line:col` / URL links (`openLocator`). | §Deep Verification 4 (ECMA-48 SGR state machine, `\r` carriage-return handling, single-pass link tokenizer calling `voxTransport.openLocator`, `useVirtualizer` `clientHeight === 0` jsdom fallback). |
| `TERM-PANE-04` | **Split Panes & Multi-Session Tabs:** Support horizontal and vertical split panes inside `Console.tsx` via `react-resizable-panels@^4.22.1`, plus multi-session tabs showing live `cwd` basename, git branch, and active foreground command title. | §Deep Verification 1 (`react-resizable-panels@4.22.1` v4 breaking API: `Group` / `Panel` / `Separator`, `orientation` prop, `"50%"` string percentage vs numeric pixel sizing trap, direct DOM children requirement). |
</phase_requirements>

---

## Summary

Phase 443 is a pure frontend (`crates/vox-gui/ui/`) unification and hardening phase — all Rust PTY/daemon/MCP/IPC primitives (`term_session_{open,list,attach,input,submit,resize,close,set_security,set_control,pin,promote_bg}`, `send_terminal_block_to_agent`, `vox://term-output`, `vox://term-event`, and `vox_terminal_{exec,read,send_input,sessions}`) are **already shipped and verified** in Phases 441–442 `[VERIFIED: crates/vox-gui/src/commands/pty.rs:241-471, crates/vox-gui/ui/src/transport.ts:882-995]`.

Five high-impact API & wire-protocol gotchas were uncovered during deep inspection and **must** govern planning:
1. **`react-resizable-panels@4.22.1` v4 Breaking API:** `PanelGroup` and `PanelResizeHandle` **do not exist** in v4; the package exports `Group`, `Panel`, and `Separator`, uses `orientation="horizontal" | "vertical"` (not `direction`), and **interprets bare numbers (`defaultSize={50}`) as PIXELS (`50px`), requiring string percentages (`defaultSize="50%"`, `minSize="15%"`)** `[VERIFIED: crates/vox-gui/ui/node_modules/react-resizable-panels/dist/react-resizable-panels.d.ts:638-1528]`.
2. **`SessionEvent` Wire Tag & `term_session_attach` Forwarder Side Effect:** On `vox://term-event`, Rust's `SessionEvent` uses default externally-tagged PascalCase serde serialization where output chunks arrive under **`OutputAppended`** (NOT `BlockOutput`!), `BlockId` is a bare `number`, `BlockOrigin` is internally tagged by `kind` (`snake_case`), and `Block` has **no** `started_ms`/`ended_ms` fields on the wire `[VERIFIED: crates/vox-terminal-core/src/session.rs:16-80, crates/vox-terminal-core/src/block.rs:3-75]`. Furthermore, calling `term_session_attach` in `pty.rs:289-296` **aborts and respawns** the Tauri event forwarder task on every invocation — so live block updates MUST be reduced directly in TypeScript from `SessionEvent` payloads, calling `termSessionAttach(sid)` once on mount (1-arg signature required by `Console.test.tsx:88` / `TerminalTab.test.tsx:97`) and `termSessionAttach(sid, afterSeq)` **only** when `SnapshotResync.resync_required` or gap recovery occurs `[VERIFIED: crates/vox-gui/src/commands/pty.rs:275-332]`.
3. **`@xterm/xterm@6.0.0` `allowProposedApi` Gate & 1-Based Link Coordinates:** Accessing `terminal.unicode` in `@xterm/xterm@6.0.0` throws `"You must set the allowProposedApi option to true to use proposed API"` unless `new Terminal({ allowProposedApi: true, ... })` is set `[VERIFIED: crates/vox-gui/ui/node_modules/@xterm/xterm/lib/xterm.mjs:49]`, while `ILinkProvider.provideLinks(bufferLineNumber, cb)` uses **1-based** `bufferLineNumber` and **1-based inclusive** `IBufferCellPosition` (`x`, `y`) `[VERIFIED: crates/vox-gui/ui/node_modules/@xterm/xterm/typings/xterm.d.ts:1393-1498]`. Mocked `Terminal` classes in `Console.test.tsx` and `TerminalTab.test.tsx` omit `registerLinkProvider`, `unicode`, `focus`, and `buffer.active.getLine`, so every addon/caller must guard optional methods `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx:41-61]`.
4. **Vitest Transport Mock Parity:** `Console.test.tsx:11-35` mocks `../../../transport` with a closed object literal omitting `termSessionSetSecurity`, `termSessionSetControl`, `termSessionPin`, `termSessionPromoteBg`, and `voxTransport.openLocator`. Importing those symbols in `Console.tsx` or child modules causes Vitest 3 to throw `"No export is defined on the mock"` unless `Console.test.tsx` (and `TerminalTab.test.tsx`) mock exports are updated in lockstep `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx:11-35]`.
5. **Closed Turn-Event Contract (`turnEventContract.test.tsx`):** `TURN_EVENT_KINDS` in `src/lib/turnEvents.ts:7-15` is asserted to equal `contracts/gui/turn-event-kinds.v1.json` 1-for-1 `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx:43-45]`. Do **not** add new event kinds to `TURN_EVENT_KINDS`; `vox_terminal_*` tool invocations already arrive as `kind: "tool_receipt"` (`tool: "vox_terminal_exec" | "vox_terminal_read" | "vox_terminal_send_input" | "vox_terminal_sessions"`) and live streaming output comes from `terminalBridge.ts`.

**Primary recommendation:** Build a pure `sessionReducer.ts` + `sgrToSpans.ts` + `xtermAddons.ts` foundation first, compose `<TerminalSessionPane>` over virtualized `<BlockStreamView>` and persistent warm `<TerminalTab>` (with `@xterm/xterm/css/xterm.css` and `<16 ms` CSS layer flip on `ALT_SCREEN`), wrap `Console.tsx` panes in `<ConsoleSplitGrid>` using `react-resizable-panels` (`Group`/`Panel`/`Separator` with `"%"` string sizes), and wire `<TerminalDockPanel>` + `<ChatTerminalBlockCard>` in `Chat/`.

---

## Project Constraints (from AGENTS.md, GEMINI.md, 00-core.md & Static UI Guards)

| Rule Source | Constraint | Enforcement Gate |
|-------------|------------|------------------|
| `00-core.md` Rule 5 | **Zero new npm or Cargo dependencies** (`package.json` / `Cargo.lock` untouched). All 5 xterm addons (`Webgl`, `Search`, `WebLinks`, `Unicode11`, `Serialize`) and `sgrToSpans` must be authored in-repo against installed `@xterm/xterm@6.0.0`. | `git diff -- package.json pnpm-lock.yaml Cargo.lock` |
| `00-core.md` Rule 9 | **Foreground commands with `timeout` prefix only** (`timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run ...`). Never run watch mode. | Task runner discipline |
| `vox-code-audit` | **Every source file `< 500` non-blank LoC.** `Console.tsx` is currently `524` total lines (~`475` non-blank) — extract split grid & session helpers so `Console.tsx` drops well under `380` non-blank LoC. | `vox audit code` (`god_object`) |
| `zeroEmojiAndDingbat.test.ts` | **Zero emojis, dingbats, geometric shapes (`▲▼◆●`), checkmarks (`✓✗`), or Unicode arrows (`→←↵`)** in non-test `.ts`/`.tsx` files (`•`, `·`, `—`, `–`, `…`, `×` allowed). Use `<Icon.*>` from `components/ui/Icons.tsx` or ASCII (`->`, `[x]`, `Enter`). | `src/guards/zeroEmojiAndDingbat.test.ts` `[VERIFIED: crates/vox-gui/ui/src/guards/zeroEmojiAndDingbat.test.ts:19-68]` |
| `noRawNativeControlsGuard.test.ts` | **Zero raw `<select>` or `<input type="range">` JSX tags** anywhere under `src/`. Always use `<VoxSelect>` from `components/ui/Select`. | `src/guards/noRawNativeControlsGuard.test.ts` `[VERIFIED: crates/vox-gui/ui/src/guards/noRawNativeControlsGuard.test.ts:13-15,152-200]` |
| `transportIpcGuard.test.ts` | **Zero `@tauri-apps/api/*` imports outside `src/transport.ts`** (except existing allowlisted files — do NOT add new direct `@tauri-apps/api` imports; route `TERM_OUTPUT_EVENT` / `TERM_EVENT_EVENT` listeners or helpers cleanly or keep existing `TerminalTab.tsx` allowlist intact — verify `transportIpcGuard.test.ts`!). | `src/guards/transportIpcGuard.test.ts` & `src/guards/ipcBoundaries.test.ts` |
| `toastBodyGuard.test.ts` | **All error toasts MUST sanitize messages via `sanitizeErrorForToast(err)`**. | `src/guards/toastBodyGuard.test.ts` |
| `surfaceHonesty.guard.test.ts` | **Zero placeholder strings (`TODO`, `coming soon`, `mock`), dead empty `onClick={() => {}}` handlers, or fake statuses** in `src/components/surfaces/**`. `HIDDEN_ALLOWLIST` is empty (`[]`) and must stay empty. | `src/components/surfaces/__guards__/surfaceHonesty.guard.test.ts` `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/__guards__/honestyScan.allowlist.ts:1-3]` |
| `turnEventContract.test.tsx` | **`TURN_EVENT_KINDS` in `src/lib/turnEvents.ts` MUST match `contracts/gui/turn-event-kinds.v1.json` 1:1.** | `src/components/surfaces/Chat/turnEventContract.test.tsx` `[VERIFIED: crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx:43-58]` |

> [!IMPORTANT]
> Let's note the `TerminalTab.tsx:3` `@tauri-apps/api/event` allowlist rule: `TerminalTab.tsx` currently imports `listen` from `@tauri-apps/api/event` (one of the 7 allowlisted files in `backendGuard.test.ts:67` / `transportIpcGuard.test.ts`). Do **not** import `@tauri-apps/api/event` in new files like `TerminalSessionPane.tsx`! Either expose a typed subscription helper `subscribeTermEvents` / `subscribeTermOutput` from `src/transport.ts` OR keep the Tauri `listen(...)` calls inside `src/transport.ts` / `TerminalTab.tsx`! Exporting `listenTermOutput` and `listenTermEvent` from `src/transport.ts` is 100% compliant with `transportIpcGuard.test.ts` and makes unit-testing `<TerminalSessionPane>` trivial without mocking `@tauri-apps/api/event`.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Canonical File / Module | Secondary Consumer(s) |
|------------|-------------|-------------------------|-----------------------|
| Tauri PTY & `SessionHost` IPC wrappers | GUI Transport Boundary | `crates/vox-gui/ui/src/transport.ts` | `TerminalSessionPane.tsx`, `TerminalTab.tsx`, `Console.tsx` |
| Cross-surface live terminal pub/sub store | GUI Shared State | `crates/vox-gui/ui/src/lib/terminalBridge.ts` | `TerminalSessionPane.tsx` (publisher), `ChatTranscript.tsx` / `ChatTerminalBlockCard.tsx` (subscriber) |
| Pure `SessionEvent` delta state machine & `ALT_SCREEN` scanner | Console Domain Logic | `crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.ts` (new) | `TerminalSessionPane.tsx`, `Console.tsx` |
| Pure ECMA-48 SGR + URL/`file:line:col` React span parser | Console Domain Logic | `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts` (new) | `BlockStreamView.tsx`, `ChatTerminalBlockCard.tsx` |
| `@xterm/xterm` v6 `ITerminalAddon` suite (`Webgl`, `Search`, `WebLinks`, `Unicode11`, `Serialize`) | Console VT Engine | `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts` + `xtermAddons.ts` (new) | `TerminalTab.tsx` |
| Warm VT canvas + `@xterm/xterm/css/xterm.css` + resize/fit lifecycle | Console VT Engine | `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx` | `TerminalSessionPane.tsx` |
| Virtualized Command Block list + inline prompt/pin/`Ctrl+B` chrome | Console Block View | `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx` | `TerminalSessionPane.tsx` |
| Unified single-session terminal pane (`Blocks` <-> `<16 ms` `ALT_SCREEN` VT + `Cmd/Ctrl+F` search bar) | Shared Terminal Component | `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx` (new) | `ConsoleSplitGrid.tsx`, `ChatSurface.tsx` (`TerminalDockPanel`) |
| Recursive horizontal/vertical resizable split tree (`Group`/`Panel`/`Separator`) | Console Layout | `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx` (new) | `Console.tsx` |
| Full Console surface (multi-session tabs, split toolbar, security/control `<VoxSelect>`, status footer) | Console Surface | `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx` | `App.tsx` (`console` surface) |
| Inline streaming `vox_terminal_*` block card in Chat turns | Chat Transcript | `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx` (new) | `ChatTranscript.tsx`, `TurnTrace.tsx`, `ChatTurnEventRow.tsx` |

---

## Standard Stack

### Installed Packages (Verified Directly from `package.json` & `node_modules`)

| Library | Declared (`package.json`) | Installed (`node_modules`) | Purpose in Phase 443 | Verification Citation |
|---------|---------------------------|----------------------------|----------------------|-----------------------|
| `@xterm/xterm` | `^6.0.0` | `6.0.0` | VT100/xterm terminal engine, `ITerminalAddon`, `ILinkProvider`, `IBuffer`, `unicode`, `xterm.css` | `[VERIFIED: crates/vox-gui/ui/package.json:35, node_modules/@xterm/xterm/package.json]` |
| `@xterm/addon-fit` | `^0.11.0` | `0.11.0` | `FitAddon` viewport cell dimension measurement & `fit()` | `[VERIFIED: crates/vox-gui/ui/package.json:34, node_modules/@xterm/addon-fit/typings/addon-fit.d.ts]` |
| `react-resizable-panels` | `^4.22.1` | `4.22.1` | Horizontal & vertical split panes (`Group`, `Panel`, `Separator`) | `[VERIFIED: crates/vox-gui/ui/package.json:45, node_modules/react-resizable-panels/package.json:3]` |
| `@tanstack/react-virtual` | `^3.13.23` | `3.13.23` | `useVirtualizer` (and repo wrapper `src/hooks/useVirtualList.ts`) for virtualized Command Blocks | `[VERIFIED: crates/vox-gui/ui/package.json:31, node_modules/@tanstack/react-virtual/package.json:3]` |
| `dockview` | `^6.6.1` | `6.6.1` | Hosts `TerminalDockPanel` inside `ChatSurface.tsx` via `DockWorkspaceShell` | `[VERIFIED: crates/vox-gui/ui/package.json:38, src/components/dock/DockWorkspaceShell.tsx:3-8]` |
| `vitest` + `@testing-library/react` | `^3.2.4` / `^16.0.1` | `3.2.4` / `16.0.1` | Fast headless jsdom component & unit testing (`< 5s` per test file) | `[VERIFIED: crates/vox-gui/ui/package.json:51-65]` |

### Packages Explicitly NOT Installed (Must Be Authored In-Repo per `00-core.md` Rule 5)
- `@xterm/addon-webgl`, `@xterm/addon-search`, `@xterm/addon-web-links`, `@xterm/addon-unicode11`, `@xterm/addon-serialize`, `ansi-to-react`, `anser` — **none** exist in `crates/vox-gui/ui/package.json`. Zero new npm dependencies are permitted; implement clean, zero-dependency TypeScript modules (`webglAddon.ts`, `xtermAddons.ts`, `sgrToSpans.ts`).

---

## Deep Technical Verification 1: `react-resizable-panels@^4.22.1` Exact Exports & Sizing Semantics

> [!CAUTION]
> **`react-resizable-panels` v4 (`4.22.1`) BROKE v2/v3 component names, prop names, AND numeric size units!** Writing `<PanelGroup direction="horizontal">` or `<Panel defaultSize={50}>` will **fail TypeScript compilation or collapse panels to `50px` wide!**

### Verified Exports (`node_modules/react-resizable-panels/dist/react-resizable-panels.d.ts`)

1. **Top-Level Exports (`dist/react-resizable-panels.d.ts:3062-3075`):**
   ```ts
   // [VERIFIED: crates/vox-gui/ui/node_modules/react-resizable-panels/dist/react-resizable-panels.d.ts:3062-3075]
   export {
     Cell, type CellImperativeHandle, type CellProps,
     Grid, type GridImperativeHandle, type GridLayout, type GridProps,
     Gridline, type GridlineProps,
     Group, type GroupImperativeHandle, type GroupProps,
     type Layout, type LayoutChangedMeta, type LayoutStorage, type On GroupLayoutChange,
     Panel, type PanelImperativeHandle, type PanelProps, type PanelSize,
     Separator, type SeparatorProps,
     Workspace, ...
     useDefaultLayout, useGroupCallbackRef, useGroupRef, usePanelCallbackRef, usePanelRef
   };
   ```
   - `PanelGroup` **DOES NOT EXIST** -> use **`Group`**.
   - `PanelResizeHandle` **DOES NOT EXIST** -> use **`Separator`**.
   - `Panel` exists (`Panel`).

2. **`GroupProps` (`dist/react-resizable-panels.d.ts:638-776`):**
   - **Axis prop is `orientation`, NOT `direction`:**
     ```ts
     // [VERIFIED: crates/vox-gui/ui/node_modules/react-resizable-panels/dist/react-resizable-panels.d.ts:742-746]
     /**
      * Specifies the resizable orientation ("horizontal" or "vertical"); defaults to "horizontal"
      */
     orientation?: Orientation | undefined; // type Orientation = "horizontal" | "vertical"
     ```
   - **DOM attributes rendered automatically:** `<div data-group data-testid="group-id-prop" id="group-id-prop">` `[VERIFIED: dist/react-resizable-panels.d.ts:764-775]`.
   - **Layout callbacks:** `onLayoutChange?: (layout: Layout) => void` and `onLayoutChanged?: (layout: Layout, meta: LayoutChangedMeta) => void` where `type Layout = { [id: string]: number }` (a **record mapping panel `id` to percentage `0..100`**, NOT `number[]`!) `[VERIFIED: dist/react-resizable-panels.d.ts:624-631, 725-740]`.
   - **Imperative ref prop:** `groupRef?: Ref<GroupImperativeHandle | null>` (not React `ref`).

3. **`PanelProps` & The Critical String-vs-Number Sizing Trap (`dist/react-resizable-panels.d.ts:890-1091`):**
   ```ts
   // [VERIFIED: crates/vox-gui/ui/node_modules/react-resizable-panels/dist/react-resizable-panels.d.ts:901-907, 1004-1007]
   /**
    * Initial size of Panel within its parent group; default unit is pixels.
    * ...
    * Supported formats:
    * - Numeric values are interpreted as pixels (e.g. 100 -> 100px)
    * - Strings without explicit units are percentages (e.g. "50" -> 50%)
    * - Explicit percent/px/rem/em/vh/vw strings (e.g. "50%", "200px")
    */
   defaultSize?: number | `${number}` | `${number}%` | `${number}px` | ... | undefined;
   minSize?: number | `${number}%` | `${number}px` | ...;
   maxSize?: number | `${number}%` | `${number}px` | ...;
   ```
   - **ALWAYS pass percentage strings** (`defaultSize="50%"`, `minSize="15%"`), **NEVER** bare numbers (`defaultSize={50}` means `50px`!).
   - **Direct DOM child requirement (`dist/react-resizable-panels.d.ts:888, 1450`):** `Panel` and `Separator` components **MUST be direct DOM children** of their parent `Group` element (do not wrap `<Panel>` or `<Separator>` in an intermediate `<div>`!). When rendering a recursive split tree (`SplitNode = { kind: 'leaf'; paneId: string; sessionId: string } | { kind: 'split'; id: string; orientation: 'horizontal' | 'vertical'; first: SplitNode; second: SplitNode }`), a child `'split'` node sits **inside** a `<Panel id={...} defaultSize="50%" minSize="15%">` which itself renders `<Group orientation={child.orientation}>...</Group>`.
   - **DOM attributes rendered automatically:** `<div data-panel data-testid="panel-id-prop" id="panel-id-prop">` for `Panel` (`L1079-1090`), and `<div data-separator data-testid="separator-id-prop" id="separator-id-prop" role="separator">` for `Separator` (`L1515-1527`). Note that `Panel` applies `className` to an inner nested `<div>` (`L970`), so give `className="flex h-full min-h-0 min-w-0 flex-col"` to `<Panel>` so child flex layouts stretch properly!

---

## Deep Technical Verification 2: `@xterm/xterm@^6.0.0`, Addons, `xterm.css` & Vitest Mock Safety

### 1. `@xterm/xterm/css/xterm.css` Clipping Fix (`node_modules/@xterm/xterm/css/xterm.css:40-87`)
Currently, neither `TerminalTab.tsx`, `Console.tsx`, nor `index.css` imports `@xterm/xterm/css/xterm.css`! Without it, xterm's hidden input element `<textarea class="xterm-helper-textarea">` renders as a visible default unstyled browser `<textarea>` box above/beside the canvas.
```css
/* [VERIFIED: crates/vox-gui/ui/node_modules/@xterm/xterm/css/xterm.css:67-86] */
.xterm .xterm-helper-textarea {
    padding: 0;
    border: 0;
    margin: 0;
    /* Move textarea out of the screen to the far left, so that the cursor is not visible */
    position: absolute;
    opacity: 0;
    left: -9999em;
    top: 0;
    width: 0;
    height: 0;
    z-index: -5;
    white-space: nowrap;
    overflow: hidden;
    resize: none;
}
```
Importing `import '@xterm/xterm/css/xterm.css';` at the top of `TerminalTab.tsx` immediately fixes `.xterm-helper-textarea`, `.xterm-viewport`, `.xterm-screen`, and `.xterm-accessibility` layout in both Vite dev/prod builds and Vitest (`vitest` handles CSS imports as no-ops by default).

### 2. Exact `@xterm/xterm@6.0.0` Public Addon & Buffer Types (`node_modules/@xterm/xterm/typings/xterm.d.ts`)

- **`ITerminalAddon` (`xterm.d.ts:1308-1313`):**
  ```ts
  // [VERIFIED: crates/vox-gui/ui/node_modules/@xterm/xterm/typings/xterm.d.ts:1308-1313]
  export interface ITerminalAddon extends IDisposable {
    activate(terminal: Terminal): void;
  }
  ```
- **`Terminal.registerLinkProvider` & 1-Based `ILinkProvider` Coordinates (`xterm.d.ts:1102, 1393-1498`):**
  ```ts
  // [VERIFIED: crates/vox-gui/ui/node_modules/@xterm/xterm/typings/xterm.d.ts:1393-1404, 1480-1498]
  export interface ILinkProvider {
    provideLinks(bufferLineNumber: number, callback: (links: ILink[] | undefined) => void): void;
  }
  export interface ILink {
    range: IBufferRange;
    text: string;
    decorations?: ILinkDecorations;
    activate(event: MouseEvent, text: string): void;
    hover?(event: MouseEvent, text: string): void;
    leave?(event: MouseEvent, text: string): void;
    dispose?(): void;
  }
  export interface IBufferRange {
    start: IBufferCellPosition;
    end: IBufferCellPosition;
  }
  export interface IBufferCellPosition {
    /** The x position of the cell. This is a 1-based value that refers to the缓冲区 column. */
    x: number;
    /** The y position of the cell. This is a 1-based value that refers to the buffer line. */
    y: number;
  }
  ```
  **1-Based Indexing Gotcha:** `bufferLineNumber` passed to `provideLinks` is **1-based**! To read the line from `terminal.buffer.active`, call `terminal.buffer.active.getLine(bufferLineNumber - 1)?.translateToString(true)`. For a regex match starting at 0-based string index `matchStart` with length `matchLen` on that single line:
  - `start: { x: matchStart + 1, y: bufferLineNumber }`
  - `end: { x: matchStart + matchLen, y: bufferLineNumber }` (1-based **inclusive** — verified in `xterm.mjs:49` `_selectWordAtCursor` where `selectionStart = [r.start.x - 1, r.start.y - 1]`).
- **`IBufferNamespace` / `IBuffer` / `IBufferLine` (`xterm.d.ts:1749-1848`):**
  - `terminal.buffer.active` returns `IBuffer` (`type: 'normal' | 'alternate'`, `cursorY`, `cursorX`, `viewportY`, `baseY`, `length`, `getLine(y: number): IBufferLine | undefined`).
  - `IBufferLine.translateToString(trimRight?: boolean, startColumn?: number, endColumn?: number): string` (`xterm.d.ts:1819`).
  - Powers both **`SearchTerminalAddon`** (`findNext(term, opts)` / `findPrevious(term, opts)` scanning `buffer.active` lines `0 .. length - 1` and calling `terminal.select(col, row, match.length)` + `terminal.scrollToLine(row)`) and **`SerializeTerminalAddon`** (`serialize(opts?)` iterating `buffer.active` lines `0 .. length - 1` with `isWrapped` continuation handling).
- **`Terminal.unicode` Proposed API Runtime Throw (`lib/xterm.mjs:49`, `xterm.d.ts:858, 1871-1902`):**
  ```js
  // [VERIFIED: crates/vox-gui/ui/node_modules/@xterm/xterm/lib/xterm.mjs:49]
  _checkProposedApi(){if(!this._core.optionsService.rawOptions.allowProposedApi)throw new Error("You must set the allowProposedApi option to true to use proposed API")}
  get unicode(){return this._checkProposedApi(),new Ln(this._core)}
  ```
  - **MUST pass `allowProposedApi: true`** into `new Terminal({ allowProposedApi: true, cursorBlink: true, convertEol: true, ... })` in `TerminalTab.tsx`!
  - In `Unicode11TerminalAddon.activate(terminal)`, wrap `terminal.unicode` access in a `try { ... } catch {}` block and register a compact Unicode 11 wide-character table (`IUnicodeVersionProvider`: `{ version: '11', wcwidth(codepoint: number): 0 | 1 | 2, charProperties(codepoint: number, preceding: number): number }`) if `'11'` is not already in `terminal.unicode.versions`, then set `terminal.unicode.activeVersion = '11'`.

### 3. Existing Vitest Mock Shapes (`Console.test.tsx` & `TerminalTab.test.tsx`)

Inspect the exact mock `Terminal` classes in existing tests:
```ts
// [VERIFIED: crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx:41-61]
class Terminal {
  cols = 80;
  rows = 24;
  open = openSpy;
  write = writeSpy;
  dispose = disposeSpy;
  loadAddon(addon: { activate?: (t: unknown) => void }) {
    addon.activate?.(this);
  }
  onData(cb: (d: string) => void) { ... }
  attachCustomKeyEventHandler(fn: (ev: KeyboardEvent) => boolean) { ... }
}
```
Notice what `this` in `TerminalTab.test.tsx` and `Console.test.tsx:36-58` **lacks**:
- `this.registerLinkProvider` is `undefined`!
- `this.unicode` is `undefined`!
- `this.buffer` is `undefined`!
- `this.focus`, `this.select`, `this.clearSelection`, `this.scrollToLine`, `this.onResize` are `undefined`!

**Mandatory Mock-Safety Invariant:** Every `ITerminalAddon.activate(terminal)` and `dispose()` method, plus every `TerminalTab.tsx` method call, **MUST** use optional chaining (`terminal.registerLinkProvider?.(...)`, `term.focus?.()`, `term.buffer?.active?.getLine?.(...)`) so `TerminalTab.test.tsx` and `Console.test.tsx` continue to pass with zero mock breakage!

### 4. `<16 ms` Synchronous `ALT_SCREEN` Auto-TUI Handoff
- **Why `<16 ms` requires keeping xterm mounted:** Calling `new Terminal()` + `term.open(el)` + WebGL2 context init takes `35–90 ms` and drops in-flight VT state (`vim`, `htop`, `lazygit` initial frame). Keeping `<TerminalTab>` mounted inside `<TerminalSessionPane>`'s relative stack and flipping CSS classes (`invisible pointer-events-none opacity-0 z-0` <-> `visible pointer-events-auto opacity-100 z-20` while hiding/showing the `Blocks` layer) completes in `< 1 ms` (1 React synchronous commit / next `requestAnimationFrame`), preserving every byte already written to `term` via `TERM_OUTPUT_EVENT`!
- **3-Source `ALT_SCREEN` State Synchronization:**
  1. `TermAttachmentDto.alt_screen === true` on initial `termSessionAttach(sid)`.
  2. `SessionEvent` `{ AltScreenChanged: { active: boolean } }` on `vox://term-event` (`session.rs:48`).
  3. Zero-RTT client-side VT sequence scanner `scanAltScreenTransition(chunk: string): boolean | null` inspecting raw chunks on `vox://term-output`:
     - Enter sequences: `\x1b[?1049h`, `\x1b[?1047h`, `\x1b[?47h` -> returns `true` (scan last occurrence in chunk in case enter+exit arrive in one chunk!).
     - Exit sequences: `\x1b[?1049l`, `\x1b[?1047l`, `\x1b[?47l` -> returns `false`.
     - Neither present -> returns `null` (keep current state).
  - On transition to `altScreen === true` (or manual `Raw VT` view toggle): synchronously flip active layer to VT canvas, schedule `fitRef.current?.fit?.()` + `termRef.current?.focus?.()`.
  - Also keep `aria-label="terminal"` / `data-testid="terminal-viewport"` and `data-testid="block-stream-view"` in the DOM (using `sr-only` / CSS visibility rather than conditional unmount when appropriate, or clear mode toggle) — verify `Console.test.tsx:89` (`expect(screen.getByLabelText('terminal')).toBeTruthy()`) and `TerminalTab.test.tsx:96`! Keeping `<TerminalTab>` always mounted in the DOM guarantees `screen.getByLabelText('terminal')` is **always** found in both `Blocks` mode and `ALT_SCREEN` / `Raw VT` mode!

---

## Deep Technical Verification 3: Rust `SessionEvent` Wire Shapes & `TermAttachmentDto` Delta Streaming

### 1. Exact `vox://term-event` Envelope (`crates/vox-gui/src/commands/pty.rs:44-56`)
```rust
// [VERIFIED: crates/vox-gui/src/commands/pty.rs:44-56]
pub const TERM_OUTPUT_EVENT: &str = "vox://term-output";
pub const TERM_EVENT_EVENT: &str = "vox://term-event";

#[derive(Clone, Serialize)]
pub struct TermOutputPayload {
    pub session_id: String,
    pub chunk: String,
}

#[derive(Clone, Serialize)]
pub struct TermEventPayload {
    pub session_id: String,
    pub event: vox_terminal_core::SessionEvent,
}
```

### 2. Exact `SessionEvent` Serde Wire Shapes (`crates/vox-terminal-core/src/session.rs:16-80`)

`SessionEvent` derives `serde::Serialize` with **no** `#[serde(tag = ...)]` or `#[serde(rename_all = ...)]` `[VERIFIED: crates/vox-terminal-core/src/session.rs:16-17]`. Serde therefore serializes unit-less struct variants as **externally-tagged single-key JSON objects** with PascalCase variant keys!

> [!WARNING]
> **CRITICAL WIRE GOTCHA:** The incremental block output variant in `session.rs:31-34` is **`OutputAppended`**, **NOT** `BlockOutput`! Any listener checking `'BlockOutput' in event` will silently drop 100% of streaming block output!

| # | Rust Variant (`session.rs:17-80`) | Exact JSON Wire Shape on `payload.event` | Nested Serde Rules Verified |
|---|-----------------------------------|------------------------------------------|-----------------------------|
| 1 | `BlockOpened` (`L20-29`) | `{ "BlockOpened": { "id": number, "kind": TermBlockKind \| null, "input": string, "origin": TermBlockOrigin \| null, "cwd"?: string, "git_branch"?: string } }` | `BlockId(pub u64)` (`block.rs:4`) is a tuple newtype -> bare JSON `number`. `BlockOrigin` (`block.rs:47-55`) has `#[serde(tag = "kind", rename_all = "snake_case")]` -> `{ "kind": "user" } \| { "kind": "agent", "agent_id": string } \| { "kind": "system" }`. `cwd` & `git_branch` have `skip_serializing_if = "Option::is_none"`. |
| 2 | **`OutputAppended`** (`L31-34`) | `{ "OutputAppended": { "id": number, "chunk": { "stream": "Stdout" \| "Stderr" \| "Agent", "text": string } } }` | `Stream` (`block.rs:25-29`) is `"Stdout" \| "Stderr" \| "Agent"` (update `transport.ts:884` `'Meta'` union to `'Stdout' \| 'Stderr' \| 'Agent' \| 'Meta'` for backwards-compat!). `OutputChunk` (`block.rs:32-35`) has `{ stream, text }` — `ts_ms` is optional (`ts_ms?: number`)! |
| 3 | `BlockClosed` (`L36-40`) | `{ "BlockClosed": { "id": number, "exit_code": number \| null, "status": "Running" \| "AwaitingInput" \| "Ok" \| "Failed" \| "Cancelled" \| null } }` | `BlockStatus` (`block.rs:16-22`) is PascalCase unit enum (`"Running" \| "AwaitingInput" \| "Ok" \| "Failed" \| "Cancelled"`). |
| 4 | `AgentMessage` (`L42`) | `{ "AgentMessage": { "text": string } }` | Plain string payload. |
| 5 | `CwdChanged` (`L44-47`) | `{ "CwdChanged": { "cwd": string, "git_branch": string \| null } }` | Updates live tab & prompt header `cwd` + `git_branch`. |
| 6 | `AltScreenChanged` (`L48`) | `{ "AltScreenChanged": { "active": boolean } }` | Triggers `<16 ms` Auto-TUI layer switch. |
| 7 | `SnapshotResync` (`L50-58`) | `{ "SnapshotResync": { "blocks": TermBlockDto[], "resync_required": boolean, "alt_screen": boolean, "cwd"?: string, "git_branch"?: string } }` | Full state replacement emitted by `DaemonHost` / `SessionHost` on lagged receiver recovery. |
| 8 | `BlockAwaitingInput` (`L60-66`) | `{ "BlockAwaitingInput": { "id": number, "prompt_tail": string, "prompt_kind": "yes_no" \| "credential" \| "selection" \| "press_enter" \| "interactive_tui", "credential_prompt": boolean } }` | `PromptKind` (`stall_detector.rs:18-25`) has `#[serde(rename_all = "snake_case")]`. When `credential_prompt: true`, render `<input type="password">` (never echo secret input in state/logs; route directly to `termSessionInput`). |
| 9 | `BlockPinned` (`L68-71`) | `{ "BlockPinned": { "id": number, "pinned": boolean } }` | Syncs `block.pinned` across Console & docked Chat panes. |
| 10 | `SecurityLevelChanged` (`L73-75`) | `{ "SecurityLevelChanged": { "security_level": "l1_read_only" \| "l2_workspace_build" \| "l3_structured_default" \| "l4_host_gated" \| "l5_autopilot_floor" } }` | `TermSecurityLevel` (`host.rs:54-67`) has `#[serde(rename_all = "snake_case")]`. |
| 11 | `ControlModeChanged` (`L77-79`) | `{ "ControlModeChanged": { "control_mode": "manual" \| "co_pilot" \| "autopilot" } }` | `TermControlMode` (`host.rs:84-90`) has `#[serde(rename_all = "snake_case")]`. |

### 3. `Block` Wire Shape (`crates/vox-terminal-core/src/block.rs:58-75`) vs `TermBlockDto` (`transport.ts:893-907`)
In Rust (`block.rs:58-75`), `Block` has:
`id`, `kind`, `status`, `origin` (`Option<BlockOrigin>`), `input`, `output` (`Vec<OutputChunk>`), `exit_code`, `cwd`, `git_branch`, `pinned` (`bool`), `credential_prompt` (`bool`).
Notice `Block` in Rust does **not** emit `started_ms` or `ended_ms`, whereas existing UI fixtures (`BlockStreamView.test.tsx:16-17`) supply `started_ms` and `ended_ms`!
- **Solution in `TermBlockDto` + `sessionReducer.ts`:** Keep `started_ms?: number` and `ended_ms?: number` optional on `TermBlockDto` (plus add `pinned?: boolean`, `credential_prompt?: boolean`, `prompt_tail?: string`, `prompt_kind?: TermPromptKindWire`), and stamp client-side `started_ms: Date.now()` on `BlockOpened` and `ended_ms: Date.now()` on `BlockClosed` whenever absent on the wire! That makes live durations (`ended_ms - started_ms`) work in production AND keeps `BlockStreamView.test.tsx:52,55` (`250ms`, `500ms`) 100% green!

### 4. `term_session_attach` Wire Args & Forwarder-Churn Pitfall (`pty.rs:275-332`)
```rust
// [VERIFIED: crates/vox-gui/src/commands/pty.rs:275-296]
#[tauri::command]
pub async fn term_session_attach(
    app: AppHandle,
    manager: State<'_, Arc<PtyManager>>,
    daemon: State<'_, DaemonClient>,
    session_id: String,
    after_seq: Option<u64>,
) -> Result<TermAttachmentDto, String> {
    let host = manager.host_for(daemon.inner()).await;
    let mut att = host.attach(&session_id, after_seq).await?;
    ...
    // Replace any prior forwarder for this session so re-attaching does not duplicate events.
    if let Some(prev) = manager.attached_forwarders.lock().insert(session_id.clone(), { ... }) {
        prev.abort();
    }
```
**Two critical consequences:**
1. **Tauri v2 camelCase IPC argument mapping + 1-arg test spy compatibility:**
   - Rust parameter `after_seq: Option<u64>` maps to Tauri JS invoke payload key **`afterSeq`**.
   - `Console.test.tsx:88,111` (`expect(t.termSessionAttach).toHaveBeenCalledWith('s-1')`) and `TerminalTab.test.tsx:97,117` (`expect(attachMock).toHaveBeenCalledWith('s-1')`) and `transport.console.test.ts:54-56` (`expect(invokeMock).toHaveBeenCalledWith('term_session_attach', { sessionId: 's-1' })`) **all assert 1-arg initial attach**!
   - Therefore `transport.ts` MUST implement:
     ```ts
     export function termSessionAttach(
       sessionId: string,
       afterSeq?: number | null,
     ): Promise<TermAttachmentDto> {
       return safeInvoke<TermAttachmentDto>(
         'term_session_attach',
         afterSeq != null ? { sessionId, afterSeq } : { sessionId },
       );
     }
     ```
     And initial mount MUST call `termSessionAttach(sid)` (1 argument), while delta recovery calls `termSessionAttach(sid, lastSeqRef.current)`.
2. **Avoid Forwarder Churn by Reducing `SessionEvent` In-Place:**
   - Previously, `TerminalTab.tsx:157-170` called `termSessionAttach(sid)` on every `SessionEvent`, which aborted and respawned the Rust `attached_forwarders` tokio task on every single event AND in `TerminalTab.tsx:206-210` re-wrote `snap.replay_utf8` onto xterm after every `termSessionSubmit`!
   - With `sessionReducer.ts` handling all 11 `SessionEvent` variants (`BlockOpened`, `OutputAppended`, `BlockClosed`, `BlockAwaitingInput`, `BlockPinned`, `CwdChanged`, `AltScreenChanged`, `SnapshotResync`, `SecurityLevelChanged`, `ControlModeChanged`, `AgentMessage`) directly in memory:
     - Initial mount calls `termSessionAttach(sid)` **once**, seeds `blocks`, `seq`, `alt_screen`, `cwd`, `git_branch`, and writes `snap.replay_utf8` once to the fresh xterm instance.
     - Subsequent `SessionEvent` messages update `blocks` / `alt_screen` / `cwd` / `security_level` / `control_mode` **purely in `sessionReducer.ts` with zero IPC round-trips**!
     - `termSessionAttach(sid, lastSeqRef.current)` is invoked **only** on explicit reconnect / resync recovery, and `snap.replay_utf8` is written to `term` **only** if `snap.resync_required === true` (`term.reset()` + `term.write(snap.replay_utf8)`).
     - `TerminalTab.tsx`'s `submit` handler simply calls `await termSessionSubmit(sid, line)` — **never** calling `termSessionAttach` or writing `replay_utf8` post-submit!

---

## Deep Technical Verification 4: `sgrToSpans.ts`, Link Tokenization & `@tanstack/react-virtual`

### 1. Pure-TS ECMA-48 SGR Parser (`sgrToSpans.ts`) — Zero `dangerouslySetInnerHTML`
Command block chunks (`OutputChunk.text`) contain raw ANSI/VT sequences from `cargo`, `git`, `pytest`, `vitest`, `pwsh`, and CLI tools. `BlockStreamView.tsx` currently renders `{block.output.map((c) => c.text).join('')}` inside `<pre>`, exposing literal `\x1b[32m` garbage to the user.

**Required Normalization & SGR State Machine:**
1. **Pre-SGR Terminal Line Normalization (`normalizeCarriageReturns`):**
   - Progress bars (`cargo build`, `pnpm`) emit `foo\rbar` (CR without LF) to overwrite the current line, plus CSI erase-in-line `\x1b[K` / `\x1b[0K` / `\x1b[2K` and OSC sequences (`\x1b]...\x07` or `\x1b]...\x1b\\`, including OSC 8 hyperlinks `\x1b]8;;url\x07label\x1b]8;;\x07` and OSC 633 shell integration markers).
   - Strip non-SGR CSI sequences (`\x1b\[[0-9;?]*[A-LN-Za-ln-z]`, preserving `m` for SGR) and strip OSC 633/title sequences cleanly (optionally extracting OSC 8 hyperlink targets if present). Handle `\r` not followed by `\n` as line-start overwrite so `cargo` progress lines don't stack 200 duplicate lines in a block card!
2. **SGR Parameter Interpreter (`\x1b\[([0-9;:]*)m`):**
   - Empty parameter or `0`: reset all style attributes.
   - `1`: `bold = true`; `2`: `dim = true`; `3`: `italic = true`; `4`: `underline = true`; `7`: `inverse = true`; `9`: `strikethrough = true`.
   - `22`: `bold = false; dim = false`; `23`: `italic = false`; `24`: `underline = false`; `27`: `inverse = false`; `29`: `strikethrough = false`.
   - `30..37`: standard 16 ANSI foreground palette (theme-harmonized CSS variables / accessible high-contrast hex values); `39`: default fg.
   - `40..47`: standard 16 ANSI background palette; `49`: default bg.
   - `90..97`: bright foreground palette; `100..107`: bright background palette.
   - `38;5;n` / `48;5;n` (`0 <= n <= 255`): xterm 256-color lookup (`0..15` system, `16..231` `6x6x6` RGB cube `[0, 95, 135, 175, 215, 255]`, `232..255` 24-step grayscale `8 + 10 * (n - 232)`). Also support colon subparameters `38:5:n` / `38:2::r:g:b` emitted by modern tools.
   - `38;2;r;g;b` / `48;2;r;g;b` (`0 <= r,g,b <= 255` clamped): 24-bit TrueColor `rgb(r, g, b)`.
   - `inverse === true`: swap effective foreground & background (defaulting fg/bg to theme text/bg tokens when unset).
3. **Single-Pass URL & `file:line(:col)?` Link Tokenization (Inside Each Styled Run):**
   - Match URLs and workspace/compiler file locators in a single combined regex with **zero HTML injection**:
     - Web URLs: `\bhttps?:\/\/[^\s<>"')\]]+` and `\blocalhost:\d+(?:\/[^\s<>"')\]]*)?` -> `onOpenLocator({ kind: 'web', value: normalizedUrl })` (trim trailing punctuation `.,;:!?` and unbalanced `)`).
     - File locators (Rust `--> crates/foo/src/lib.rs:42:9`, TS `src/App.tsx(12,5)` or `src/App.tsx:12:5`, Python/Unix `./path/file.ext:12`): `(?:[A-Za-z]:[\\/]|(?:\.{1,2}[\\/])|(?:[\w@-]+[\\/])+ )[\w.-]+\.[A-Za-z0-9]+(?::\d+(?::\d+)?|\(\d+,\d+\))` (avoid matching bare version numbers like `v1.2.3:4` or URLs already matched by branch 1!).
   - Click action calls `voxTransport.openLocator({ kind: 'web' | 'file', value })` `[VERIFIED: crates/vox-gui/ui/src/transport.ts:629, crates/vox-gui/ui/src/transport.test.ts:53-65]`.
   - Render plain runs as `<span>` with inline `React.CSSProperties` (`color`, `backgroundColor`, `fontWeight`, `opacity`, `fontStyle`, `textDecorationLine`) and link runs as `<button type="button" onClick={...} className="underline decoration-brass/60 hover:text-brass cursor-pointer ...">` so keyboard activation and screen readers work natively without `href="#"` navigation traps.

### 2. `@tanstack/react-virtual` in `BlockStreamView.tsx` & jsdom Compatibility
We inspected `crates/vox-gui/ui/src/hooks/useVirtualList.ts` and `useVirtualList.test.ts`:
```ts
// [VERIFIED: crates/vox-gui/ui/src/hooks/useVirtualList.test.ts:6-8]
// useVirtualizer needs a container element with a measurable height.
// jsdom doesn't do layout, so we supply a fixed height via the
// scrollElement ref and override getBoundingClientRect.
```
**CRITICAL JSDOM GOTCHA:** In `BlockStreamView.test.tsx:36-46`, `<BlockStreamView blocks={sampleBlocks} ... />` is rendered in jsdom **without** mocking `getBoundingClientRect` or `clientHeight`, so `scrollRef.current.clientHeight === 0`, which causes `useVirtualizer.getVirtualItems()` to return **`[]` (0 items)** unless guarded!
- **Deterministic Solution:** Call `useVirtualizer({ count: blocks.length, getScrollElement: () => scrollRef.current, estimateSize: () => 132, overscan: 6, measureElement: ... })` unconditionally (hooks rules), AND check:
  ```ts
  const virtualItems = virtualizer.getVirtualItems();
  const useVirtual =
    blocks.length > VIRTUALIZE_THRESHOLD &&
    (scrollRef.current?.clientHeight ?? 0) > 0 &&
    virtualItems.length > 0;
  ```
  - When `useVirtual === true` (real browser / Tauri WebView with `> 25` blocks and measurable viewport height, or an explicit virtualization unit test with mocked `clientHeight`/`getBoundingClientRect` like `useVirtualList.test.ts:9-16`), render the virtualized container (`height: virtualizer.getTotalSize()`, absolute items with `ref={virtualizer.measureElement}` and `data-index={vItem.index}`).
  - When `useVirtual === false` (small block lists `<= 25` OR jsdom tests where `clientHeight === 0`), render `blocks.map(...)` directly!
  - Result: 500+ block streams stay 60fps in WebView AND 100% of existing `BlockStreamView.test.tsx` tests pass untouched!

---

## Deep Technical Verification 5: `ChatSurface.tsx` (`TerminalDockPanel` + `closedPanelIds`) & `ChatTranscript` / Turn-Event Contract Integration

### 1. `ChatSurface.tsx` (`TerminalDockPanel` & `closedPanelIds` Non-Resurrection Rule)
Verified directly in `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`:
- `TerminalDockPanel` is defined at `ChatSurface.tsx:273-312` and registered in `CHAT_DOCK_COMPONENTS` under key `terminal` (`L326`).
- `panelDefs.terminal` is at `ChatSurface.tsx:1156`:
  `terminal: { title: 'Terminal', params: { onNavigate }, referenceChain: ['todos', 'executionRail', 'transcript'] }`.
- In the refresh `useEffect` (`ChatSurface.tsx:1297-1311`), opt-in panels have an **update-only** rule:
  ```ts
  // [VERIFIED: crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx:1297-1308]
  // Opt-in panels: update-only, no create branch. They can only be
  // (re)created via the Panels menu's Add section (Step 4 below) — this is
  // what makes the "resurrects after close" bug structurally impossible.
  api.getPanel('terminal')?.update({ params: panelDefs.terminal.params });
  ```
- **Replacement Contract for `TerminalDockPanel` (`ChatSurface.tsx:273-312`):**
  - Replace the static `termState.blocks.slice(-5)` readout inside `TerminalDockPanel` with `<TerminalSessionPane compact onOpenConsole={() => props.params.onNavigate?.('console')} />` wrapped inside `<div data-testid="chat-dock-terminal" className="flex h-full min-h-0 flex-col overflow-hidden">`.
  - **Preserve `data-testid="chat-dock-terminal"`** and **preserve the `"Open Console"` button** (`onClick={() => props.params.onNavigate?.('console')}`).
  - **DO NOT** touch `closedPanelIds`, `addDefaultPanel`, or the update-only line `api.getPanel('terminal')?.update(...)` (`L1308`)! Never auto-open the `terminal` dockview panel when a command runs unless the user clicks an explicit button.
  - **Check `ChatSurface.tsx` mocks in `ChatSurface.test.tsx`:** Mounting `<TerminalSessionPane>` inside `TerminalDockPanel` only executes when the `terminal` opt-in panel is actually added to dockview, AND lazy/mock-safe imports ensure `ChatSurface.test.tsx` never crashes if `transport` methods aren't mocked there (guard `typeof termSessionList === 'function'` or fallback gracefully when transport mock returns `undefined`).

### 2. `ChatTranscript.tsx` / `TurnTrace.tsx` / `ChatTurnEventRow.tsx` & `turnEventContract.test.tsx` Invariants
Verified directly in `contracts/gui/turn-event-kinds.v1.json`, `src/lib/turnEvents.ts`, and `src/components/surfaces/Chat/turnEventContract.test.tsx`:
- `turnEventContract.test.tsx:43-58` enforces three strict invariants:
  1. `expect(CONTRACT.kinds.map((k) => k.kind).sort()).toEqual([...TURN_EVENT_KINDS].sort())` — **you CANNOT add a new kind to `TURN_EVENT_KINDS` without editing `contracts/gui/turn-event-kinds.v1.json` AND Rust `turn_events.rs`!** (And `00-core.md` prohibits touching Rust/contract files outside Phase 443 UI scope.)
  2. Every shipped `entry.example` in `turn-event-kinds.v1.json` rendered via `<ChatTurnEventRow event={entry.example} />` must produce a non-empty DOM element (`L47-50`).
  3. Every `entry.example` with **any** required field deleted must render an **empty** DOM element (`expect(container).toBeEmptyDOMElement()`, `L52-58`).
- **How `vox_terminal_*` Integrates 100% Cleanly Within the Existing Contract:**
  - Phase 442 registered four MCP terminal tools: `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, and `vox_terminal_sessions` `[VERIFIED: crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs:34]`.
  - When an agent turn invokes any `vox_terminal_*` tool, Rust emits a standard `tool_receipt` turn event:
    `{ kind: 'tool_receipt', tool: 'vox_terminal_exec', receipt_id: '...', fulfilled: true, verified: true }`!
  - Meanwhile, while the agent's command is actively streaming (even **before** the turn reply finishes!) and after it completes, `terminalBridge.ts` (`getTerminalBridgeState()` / `subscribeTerminalBridge()`) holds the live `TermBlockDto[]` with `origin: { kind: 'agent', agent_id }` and `activeSessionId`!
  - **Integration Design:**
    1. **`ChatTurnEventRow.tsx` (`tool_receipt` branch, `L68-86`):** Keep `isKnownTurnEvent(event)` guard untouched at line 19 (preserving `turnEventContract.test.tsx` 100% green!). When `event.kind === 'tool_receipt'` and `String(event.tool).startsWith('vox_terminal_')`, render a terminal badge + optional `"Open in Console"` action (`onOpenConsole` callback or custom window event `vox:open-console-session` handled by `App.tsx` / `ChatSurface` `onNavigate?.('console')`) while keeping all standard `chat-turn-receipt-row` testids and required-field checks intact!
    2. **`ChatTerminalBlockCard.tsx` (new, mounted in `ChatTranscript.tsx`):** Subscribe to `subscribeTerminalBridge`:
       - During an active streaming turn (`agentStreamItems` / pending assistant turn) OR on assistant messages whose `events` include a `tool_receipt` with `tool.startsWith('vox_terminal_')` (matched to agent-originated blocks `block.origin?.kind === 'agent'`), render `<ChatTerminalBlockCard>` displaying:
         - Command header (`$ {block.input}`), `cwd` / `git_branch` chip, live status seal (`Running`, `AwaitingInput`, `exit 0`, `exit N`),
         - ANSI-styled output preview rendered through `sgrToSpans.ts` (zero `dangerouslySetInnerHTML`),
         - 1-click **"Open in Console"** button (`data-testid="chat-term-open-console"`) that focuses that block in `terminalBridge` (`focusTerminalBlockInChat` / `setActiveConsoleTarget`) and navigates to `'console'` (via `onNavigate?.('console')` threaded from `ChatSurface.tsx:899` to `ChatTranscript` or custom event fallback).

---

## Architecture Patterns & Module Budget (`< 500` Non-Blank LoC)

Every file in `crates/vox-gui/ui/src/` must stay strictly below **500 non-blank LoC** (`vox-code-audit`). `Console.tsx` is currently `524` total lines (`~475` non-blank) — adding split panes and rich tab metadata directly into `Console.tsx` would breach the 500-line ceiling. Decompose cleanly as follows:

```
crates/vox-gui/ui/src/
├── transport.ts                          # Update TermBlockDto/TermSessionEventWire types, termSessionAttach(sid, afterSeq?), + listenTermOutput/listenTermEvent helpers
├── lib/
│   └── terminalBridge.ts                 # Add focusConsoleBlockRequest (sessionId + blockId) pub/sub for 1-click "Open in Console" jump
└── components/surfaces/
    ├── Console/
    │   ├── sessionReducer.ts             # NEW (~160 LoC): Pure SessionEvent reducer (all 11 variants) + scanAltScreenTransition() + security/control wire mappers
    │   ├── sgrToSpans.ts                 # NEW (~220 LoC): ECMA-48 SGR (16/256/TrueColor + attrs) + \r/OSC normalizer + single-pass URL & file:line:col React span tokenizer
    │   ├── webglAddon.ts                 # UPDATE (~130 LoC): Real WebGL2 cell/canvas context loss listener + deterministic DOM fallback
    │   ├── xtermAddons.ts                # NEW (~240 LoC): SearchTerminalAddon, WebLinksTerminalAddon (1-based ILinkProvider), Unicode11TerminalAddon, SerializeTerminalAddon
    │   ├── TerminalTab.tsx               # UPDATE (~230 LoC): import '@xterm/xterm/css/xterm.css', allowProposedApi: true, load 6 addons, pure VT stream (no post-submit attach storm)
    │   ├── BlockStreamView.tsx           # UPDATE (~320 LoC): @tanstack/react-virtual (with clientHeight===0 jsdom fallback), sgrToSpans, sticky header, collapse, AwaitingInput stdin bar + Ctrl+B + Pin
    │   ├── TerminalSessionPane.tsx       # NEW (~290 LoC): Unified pane mounting warm TerminalTab + BlockStreamView, <16ms ALT_SCREEN layer flip, Cmd/Ctrl+F search bar, afterSeq resync
    │   ├── ConsoleSplitGrid.tsx          # NEW (~200 LoC): Recursive SplitTree state + react-resizable-panels <Group>/<Panel>/<Separator> ("%" string sizes) + Split Right/Down/Close
    │   └── Console.tsx                   # UPDATE (~340 LoC, reduced from 524!): Multi-session tabs (cwd basename, git_branch, running cmd title), <VoxSelect> security/control backend sync, <ConsoleSplitGrid>
    └── Chat/
        ├── ChatTerminalBlockCard.tsx     # NEW (~140 LoC): Live streaming vox_terminal_* block card with sgrToSpans preview + "Open in Console" button
        ├── ChatTurnEventRow.tsx          # UPDATE (~165 LoC): Enrich vox_terminal_* tool_receipt rows with "Open in Console" action (contract-neutral)
        ├── ChatTranscript.tsx            # UPDATE: Render <ChatTerminalBlockCard> for vox_terminal_* turns / active agent terminal blocks; accept optional onNavigate
        └── ChatSurface.tsx               # UPDATE: Mount <TerminalSessionPane compact ... /> inside TerminalDockPanel (preserving closedPanelIds invariant) + pass onNavigate to ChatTranscript
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Split-pane pointer drag, keyboard separator resize (`ArrowLeft`/`Right`/`Up`/`Down`), ARIA `role="separator"`, min-size clamping | Custom `mousedown`/`mousemove` flex splitter | `react-resizable-panels@4.22.1` (`Group`, `Panel`, `Separator` with `"%"` string sizes) | Handles pointer capture, nested horizontal/vertical groups, WAI-ARIA window-splitter keyboard spec, and deterministic layout ratios out of the box `[VERIFIED: dist/react-resizable-panels.d.ts:638-1528]`. |
| Viewport windowing for 500+ Command Blocks | Manual `scrollTop` math & slice offsets | `@tanstack/react-virtual@3.13.23` (`useVirtualizer` with dynamic `measureElement` + `clientHeight === 0` fallback) | Variable-height collapsible ANSI blocks require dynamic `ResizeObserver` item measurement built into `virtualizer.measureElement` `[VERIFIED: node_modules/@tanstack/react-virtual/dist/esm/index.d.ts:48]`. |
| VT100 / alt-screen TUI cursor addressing (`vim`, `htop`, `lazygit`) | Custom VT grid emulator | `@xterm/xterm@6.0.0` + `@xterm/addon-fit@0.11.0` + `@xterm/xterm/css/xterm.css` | Full DEC private mode (`?1049h`/`?1049l`), mouse tracking, bracketed paste, and accessibility tree already implemented in `@xterm/xterm`. |
| Workspace file / external URL opening | Custom `window.open` or raw Tauri shell invoke | `voxTransport.openLocator({ kind: 'file' \| 'web', value })` | Canonical workspace-validated, OS-safe opener already tested and wired in `transport.ts:629` `[VERIFIED: crates/vox-gui/ui/src/transport.ts:629]`. |
| Select dropdowns for Security Level & Control Mode | Native `<select>` | `<VoxSelect>` (`components/ui/Select`) | Native `<select>` is strictly banned by `noRawNativeControlsGuard.test.ts` `[VERIFIED: crates/vox-gui/ui/src/guards/noRawNativeControlsGuard.test.ts:153-176]`. |

---

## Common Pitfalls

### Pitfall 1: `react-resizable-panels` v4 Numeric `px` Sizing & v2/v3 Exports
- **What goes wrong:** Importing `{ PanelGroup, Panel, PanelResizeHandle }` fails `tsc --noEmit`; passing `defaultSize={50}` makes each split pane `50px` wide instead of `50%`.
- **Why it happens:** `react-resizable-panels` v4 renamed exports to `Group`/`Panel`/`Separator`, renamed `direction` to `orientation`, and changed numeric `PanelSize` units from percentages to pixels (`dist/react-resizable-panels.d.ts:904`).
- **How to avoid:** Import `{ Group, Panel, Separator } from 'react-resizable-panels'`, pass `orientation="horizontal" | "vertical"`, and pass string percentages `defaultSize="50%"` and `minSize="15%"`. Ensure `<Panel>` and `<Separator>` are direct DOM children of `<Group>`.

### Pitfall 2: Listening for `BlockOutput` Instead of `OutputAppended` on `vox://term-event`
- **What goes wrong:** Command blocks open (`BlockOpened`) and close (`BlockClosed`), but their output stays blank during execution unless re-fetched via `termSessionAttach`.
- **Why it happens:** Rust `SessionEvent` (`crates/vox-terminal-core/src/session.rs:31-34`) names the variant `OutputAppended { id: BlockId, chunk: OutputChunk }`, serialized externally-tagged as `{ "OutputAppended": { "id": 1, "chunk": { "stream": "Stdout", "text": "..." } } }`.
- **How to avoid:** Match `'OutputAppended' in event` in `sessionReducer.ts` (you may accept `'BlockOutput'` as a defensive test alias if desired, but `'OutputAppended'` is the canonical Rust wire key).

### Pitfall 3: Calling `termSessionAttach` on Every `SessionEvent` or Post-Submit
- **What goes wrong:** Duplicate prompts/lines echo in xterm (`$ ls\r\n$ ls\r\n`) and Tauri aborts/respawns the session's `attached_forwarders` tokio task on every output chunk (`pty.rs:289-296`), dropping in-flight events.
- **Why it happens:** `TerminalTab.tsx:166,206` previously re-ran `termSessionAttach(sid)` and called `term.write(snap.replay_utf8)` after every submit and event.
- **How to avoid:** Call `termSessionAttach(sid)` (1-arg!) once per session mount; apply all `SessionEvent` deltas in `sessionReducer.ts`; only call `termSessionAttach(sid, afterSeq)` on explicit resync and only write `replay_utf8` if `snap.resync_required === true`.

### Pitfall 4: `terminal.unicode` Throwing in Production OR `registerLinkProvider` Throwing in Vitest
- **What goes wrong:** In production, `Unicode11TerminalAddon` throws `Error: You must set the allowProposedApi option to true to use proposed API` (`xterm.mjs:49`). In Vitest (`Console.test.tsx`, `TerminalTab.test.tsx`), `terminal.registerLinkProvider(...)` throws `TypeError: terminal.registerLinkProvider is not a function` because the mocked `Terminal` class only defines `open`, `write`, `dispose`, `loadAddon`, `onData`, `attachCustomKeyEventHandler`.
- **How to avoid:** Pass `allowProposedApi: true` in `new Terminal({ allowProposedApi: true, ... })` AND guard every addon method with optional chaining + `try/catch` (`if (typeof terminal.registerLinkProvider === 'function') ...`, `try { const u = terminal.unicode; ... } catch {}`).

### Pitfall 5: Breaking `Console.test.tsx` Label Maps or Transport Mock Exports
- **What goes wrong:**
  1. `Console.test.tsx:100-102` asserts security options `'L1 Read-Only'`, `'L2 Safe Exec'`, `'L3 Workspace Write'`, `'L4 System Admin'` and control options `'Manual'`, `'Co-Pilot'`, `'Autopilot'`. Changing those visible labels or breaking `Console.test.tsx`'s `vi.mock('../../../transport', ...)` factory causes immediate test failures.
  2. Mapping UI security values `'L1' | 'L2' | 'L3' | 'L4'` directly to `termSessionSetSecurity` without converting to Rust's snake_case `TermSecurityLevelWire` (`'l1_read_only' | 'l2_workspace_build' | 'l3_structured_default' | 'l4_host_gated' | 'l5_autopilot_floor'`) causes serde deserialization errors in Tauri (`host.rs:54-67`).
- **How to avoid:**
  1. Preserve the exact `<VoxSelect>` `aria-label`s (`"Session security level"`, `"Session control mode"`) and option labels in `Console.tsx`, and map `'L1' -> 'l1_read_only'`, `'L2' -> 'l2_workspace_build'`, `'L3' -> 'l3_structured_default'`, `'L4' -> 'l4_host_gated'`, `'manual' -> 'manual'`, `'co-pilot' -> 'co_pilot'`, `'autopilot' -> 'autopilot'` when calling `termSessionSetSecurity` / `termSessionSetControl`!
  2. Update `Console.test.tsx`'s `vi.mock('../../../transport', ...)` to export `termSessionSetSecurity`, `termSessionSetControl`, `termSessionPin`, `termSessionPromoteBg`, `listenTermOutput`, `listenTermEvent`, and `voxTransport: { openLocator: vi.fn() }` (AND still guard optional mock functions with `?.()` at callsites for defense-in-depth).

### Pitfall 6: Banned Unicode Symbols Tripping `zeroEmojiAndDingbat.test.ts`
- **What goes wrong:** Using `↵`, `→`, `✓`, `✗`, `▲`, `▼`, `●`, or `⌘` inside search buttons, collapse chevrons, or status badges fails `src/guards/zeroEmojiAndDingbat.test.ts`.
- **How to avoid:** Use `<Icon.chevronDown>`, `<Icon.chevronRight>`, `<Icon.search>`, `<Icon.close>`, `<Icon.copy>`, `<Icon.terminal>`, `<Icon.pin>` (from `components/ui/Icons.tsx`) or plain ASCII (`Enter`, `Prev`, `Next`, `x`, `·`, `—`, `×`).

---

## Code Examples (Verified Against Installed `.d.ts` & Rust Wire Contracts)

### 1. Recursive Split Grid with `react-resizable-panels@4.22.1`
```tsx
// Verified against node_modules/react-resizable-panels/dist/react-resizable-panels.d.ts:638-1528
import React from 'react';
import { Group, Panel, Separator } from 'react-resizable-panels';

export type SplitNode =
  | { kind: 'leaf'; paneId: string; sessionId: string }
  | {
      kind: 'split';
      id: string;
      orientation: 'horizontal' | 'vertical';
      first: SplitNode;
      second: SplitNode;
    };

export function renderSplitNode(
  node: SplitNode,
  renderLeaf: (paneId: string, sessionId: string) => React.ReactNode,
): React.ReactNode {
  if (node.kind === 'leaf') {
    return renderLeaf(node.paneId, node.sessionId);
  }
  return (
    <Group id={node.id} orientation={node.orientation} className="h-full w-full">
      <Panel id={`${node.id}-a`} defaultSize="50%" minSize="15%" className="flex h-full min-h-0 min-w-0 flex-col">
        {renderSplitNode(node.first, renderLeaf)}
      </Panel>
      <Separator
        id={`${node.id}-sep`}
        className={
          node.orientation === 'horizontal'
            ? 'w-1 bg-border-subtle transition hover:bg-brass/50 focus-visible:bg-brass'
            : 'h-1 bg-border-subtle transition hover:bg-brass/50 focus-visible:bg-brass'
        }
      />
      <Panel id={`${node.id}-b`} defaultSize="50%" minSize="15%" className="flex h-full min-h-0 min-w-0 flex-col">
        {renderSplitNode(node.second, renderLeaf)}
      </Panel>
    </Group>
  );
}
```

### 2. Mock-Safe `@xterm/xterm@6.0.0` `WebLinksTerminalAddon` & `Unicode11TerminalAddon`
```ts
// Verified against node_modules/@xterm/xterm/typings/xterm.d.ts:1308-1498, 1871-1902 & lib/xterm.mjs:49
import type { IDisposable, ILink, ITerminalAddon, Terminal } from '@xterm/xterm';

export class WebLinksTerminalAddon implements ITerminalAddon {
  private linkDisposable: IDisposable | null = null;
  constructor(private readonly onActivateUrl: (url: string) => void) {}

  activate(terminal: Terminal): void {
    if (typeof terminal.registerLinkProvider !== 'function') return; // Vitest mock guard
    this.linkDisposable = terminal.registerLinkProvider({
      provideLinks: (bufferLineNumber, callback) => {
        const line = terminal.buffer?.active?.getLine?.(bufferLineNumber - 1);
        if (!line) {
          callback(undefined);
          return;
        }
        const text = line.translateToString(true);
        const links: ILink[] = [];
        const re = /\b(?:https?:\/\/[^\s<>"')\]]+|localhost:\d+(?:\/[^\s<>"')\]]*)?)/g;
        let m: RegExpExecArray | null;
        while ((m = re.exec(text)) !== null) {
          const raw = m[0].replace(/[.,;:!?]+$/, '');
          if (!raw) continue;
          const startCol1 = m.index + 1; // 1-based column
          const endCol1 = m.index + raw.length; // 1-based inclusive column
          const target = raw.startsWith('localhost:') ? `http://${raw}` : raw;
          links.push({
            range: {
              start: { x: startCol1, y: bufferLineNumber },
              end: { x: endCol1, y: bufferLineNumber },
            },
            text: raw,
            activate: (_ev, _txt) => this.onActivateUrl(target),
          });
        }
        callback(links.length > 0 ? links : undefined);
      },
    });
  }

  dispose(): void {
    this.linkDisposable?.dispose();
    this.linkDisposable = null;
  }
}

export class Unicode11TerminalAddon implements ITerminalAddon {
  activate(terminal: Terminal): void {
    try {
      const unicodeApi = terminal.unicode;
      if (!unicodeApi || typeof unicodeApi.register !== 'function') return;
      if (!unicodeApi.versions?.includes('11')) {
        unicodeApi.register({
          version: '11',
          wcwidth: (cp: number) => (isWideUnicode11(cp) ? 2 : cp < 32 || (cp >= 0x7f && cp < 0xa0) ? 0 : 1),
        });
      }
      unicodeApi.activeVersion = '11';
    } catch {
      // Safe fallback if allowProposedApi was omitted on an external/mocked Terminal instance
    }
  }
  dispose(): void {}
}
```

### 3. `termSessionAttach` Optional `afterSeq` Signature (`transport.ts`)
```ts
// Preserves exact 1-arg invoke shape `{ sessionId }` for existing tests while adding `{ sessionId, afterSeq }`
export function termSessionAttach(
  sessionId: string,
  afterSeq?: number | null,
): Promise<TermAttachmentDto> {
  return safeInvoke<TermAttachmentDto>(
    'term_session_attach',
    afterSeq != null ? { sessionId, afterSeq } : { sessionId },
  );
}
```

---

## State of the Art

| Legacy Pattern in `Console/` & `Chat/` | Phase 443 Target Pattern | Impact |
|----------------------------------------|--------------------------|--------|
| Stacked `<BlockStreamView>` (`max-h-[340px]`) above visible `<TerminalTab>` showing identical output twice (`Console.tsx:457-492`) | Single `<TerminalSessionPane>` with unified `Blocks` viewport + warm hidden VT layer that flips `< 16 ms` on `ALT_SCREEN` | Zero duplicate output; instant full-bleed `vim`/`htop`/`lazygit` TUI takeover |
| Full `termSessionAttach(sid)` + `term.write(replay_utf8)` on every submit/event (`TerminalTab.tsx:166,206`) | 1-time attach + pure `SessionEvent` delta reducer + `afterSeq` resync-only replay | Zero duplicate VT echo; zero Tauri `attached_forwarders` task churn |
| Unstyled `.xterm-helper-textarea` bleeding into Console DOM (missing `xterm.css`) | Explicit `import '@xterm/xterm/css/xterm.css'` in `TerminalTab.tsx` | Proper `left: -9999em; opacity: 0; width: 0; height: 0` textarea clipping |
| Raw ANSI escape garbage in `<pre>` (`BlockStreamView.tsx:165`) & unvirtualized `.map()` | Pure-TS `sgrToSpans.ts` (16/256/TrueColor + URL/`file:line:col` links) + `@tanstack/react-virtual` | Crisp colored compiler/test output, 1-click file/URL open, 60fps at 500+ blocks, zero XSS |
| Single-session pane in `Console.tsx` & static `slice(-5)` mock readout in `ChatSurface.tsx:300` | `ConsoleSplitGrid` (`Group`/`Panel`/`Separator`) + live `<TerminalSessionPane compact>` in `TerminalDockPanel` + `<ChatTerminalBlockCard>` in `ChatTranscript` | True split-pane IDE terminal + unified live Chat <-> Console bidirectional experience |

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| **Framework** | Vitest `3.2.4` + `@testing-library/react` `16.0.1` (`jsdom`) + TypeScript `tsc --noEmit` (`5.6.2`) |
| **Config file** | `crates/vox-gui/ui/vite.config.ts` / `crates/vox-gui/ui/tsconfig.json` |
| **Quick run command** | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/` |
| **Full phase gate command** | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/turnEventContract.test.tsx src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/guards/ src/components/surfaces/__guards__/surfaceHonesty.guard.test.ts` + `timeout 300s pnpm --dir crates/vox-gui/ui run typecheck` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| `TERM-PANE-01` | `termSessionAttach(sid, afterSeq?)` 1-arg & 2-arg IPC payloads; pure `SessionEvent` (`OutputAppended`, `BlockOpened`, `BlockClosed`, `BlockAwaitingInput`, `BlockPinned`, `CwdChanged`, `AltScreenChanged`, `SnapshotResync`) delta reducer; unified `<TerminalSessionPane>` in `Console` & `TerminalDockPanel` (`closedPanelIds` preserved); `<ChatTerminalBlockCard>` with `"Open in Console"` (`turnEventContract.test.tsx` 100% green) | unit + component + contract | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/components/surfaces/Console/TerminalSessionPane.test.tsx src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/turnEventContract.test.tsx` | ❌ Wave 1/3 TDD (`transport.console.test.ts` & `turnEventContract.test.tsx` exist ✅) |
| `TERM-PANE-02` | `@xterm/xterm/css/xterm.css` import; `WebglTerminalAddon` (`onContextLoss` DOM fallback), `SearchTerminalAddon`, `WebLinksTerminalAddon` (1-based `ILinkProvider`), `Unicode11TerminalAddon` (`allowProposedApi: true`), `SerializeTerminalAddon`; `< 16 ms` synchronous `ALT_SCREEN` (`?1049h`/`?1049l` + `AltScreenChanged`) layer flip without xterm remount | unit + component | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/xtermAddons.test.ts src/components/surfaces/Console/TerminalTab.test.tsx src/components/surfaces/Console/TerminalSessionPane.test.tsx` | ❌ Wave 1/2 TDD (`TerminalTab.test.tsx` exists ✅) |
| `TERM-PANE-03` | `sgrToSpans.ts` (16/256/TrueColor, bold/dim/italic/underline/inverse/strike, `\r` overwrite, zero `dangerouslySetInnerHTML`, XSS literal escaping, URL & `file:line:col` `openLocator` links); `@tanstack/react-virtual` windowing at `500` blocks + `clientHeight === 0` jsdom fallback; sticky header, collapse, `AwaitingInput` stdin bar, `Ctrl+B` (`termSessionPromoteBg`), Pin (`termSessionPin`), `Cmd/Ctrl+F` search | unit + component | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/sgrToSpans.test.ts src/components/surfaces/Console/BlockStreamView.test.tsx` | ❌ Wave 1/2 TDD (`BlockStreamView.test.tsx` exists ✅) |
| `TERM-PANE-04` | `ConsoleSplitGrid.tsx` (`Group`/`Panel`/`Separator` with `"%"` string sizes, horizontal/vertical split, focus pane, close pane); multi-session tabs (`cwd` basename, `git_branch`, active running command title); `<VoxSelect>` security/control backend wire sync | component + guard | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/ConsoleSplitGrid.test.tsx src/components/surfaces/Console/Console.test.tsx src/guards/zeroEmojiAndDingbat.test.ts src/guards/noRawNativeControlsGuard.test.ts src/guards/transportIpcGuard.test.ts` | ❌ Wave 3 TDD (`Console.test.tsx` & guards exist ✅) |

### Wave 0 Gaps
None — Vitest `3.2.4`, `@testing-library/react`, `jsdom`, and static guard suites are already configured and fast (`< 4s` per suite); each task writes its failing unit/component test first (RED -> GREEN).

---

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V5 Input Validation & Output Encoding (DOM XSS) | **YES (CRITICAL)** | Terminal output (`Stdout`/`Stderr`/`replay_utf8`) frequently contains untrusted external payloads (`curl`, git commit messages, log files with `<script>alert(1)</script>` or `\x1b]8;;javascript:alert(1)\x07`). `sgrToSpans.ts` MUST emit React text child nodes (`<span>{text}</span>`), **NEVER** `dangerouslySetInnerHTML` or `innerHTML`. |
| V5 URL / File Protocol Validation (`openLocator`) | **YES** | Both `sgrToSpans.ts` and `WebLinksTerminalAddon` MUST strictly allowlist web URL schemes to `http://` and `https://` (reject `javascript:`, `data:`, `file://`, `vbscript:` URLs!) before passing `{ kind: 'web', value }` to `voxTransport.openLocator`. |
| V2 / Secret Input Handling (`credential_prompt`) | **YES** | When `BlockAwaitingInput` arrives with `credential_prompt: true` (`session.rs:60-66`), the inline stdin bar MUST render `<input type="password" autoComplete="off">`, clear its React state immediately on submit, never copy the secret into `terminalBridge` or toast messages, and forward directly via `termSessionInput(sessionId, `${secret}\n`)` (never `termSessionSubmit`, which would record the secret in `Block.input`!). |
| V1 / IPC Boundary Enforcement | **YES** | All Tauri commands pass through `src/transport.ts` (`safeInvoke`), enforced by `transportIpcGuard.test.ts` and Rust `remote::gate` (`pty.rs` commands classified in Phase 442). |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| — | *Zero unverified assumptions.* Every package export, `.d.ts` signature, runtime `.mjs` check, Rust `serde` enum representation, Tauri parameter convention, and UI static guard was directly read and verified in the workspace. | — | None |

## Open Questions

None — all technical contracts, package APIs, wire shapes, and test invariants are 100% resolved.

---

## Sources

### Primary (HIGH Confidence — Verified Directly In-Repo)
- `crates/vox-gui/ui/package.json` (`L31-66`) — exact frontend dependencies (`@xterm/xterm@^6.0.0`, `@xterm/addon-fit@^0.11.0`, `react-resizable-panels@^4.22.1`, `@tanstack/react-virtual@^3.13.23`, `dockview@^6.6.1`).
- `crates/vox-gui/ui/node_modules/react-resizable-panels/dist/react-resizable-panels.d.ts` (`L624-1528`, `L3062-3075`) — `Group`, `Panel`, `Separator`, `orientation`, string `%` vs numeric `px` `PanelSize` rules.
- `crates/vox-gui/ui/node_modules/@xterm/xterm/typings/xterm.d.ts` (`L858`, `L1102`, `L1308-1498`, `L1749-1902`), `lib/xterm.mjs` (`L49`), and `css/xterm.css` (`L67-86`) — `ITerminalAddon`, 1-based `ILinkProvider`, `IBuffer`, `_checkProposedApi()` on `terminal.unicode`, and `.xterm-helper-textarea` CSS clipping.
- `crates/vox-gui/ui/node_modules/@tanstack/react-virtual/dist/esm/index.d.ts` (`L1-49`) & `crates/vox-gui/ui/src/hooks/useVirtualList.test.ts` (`L6-16`) — `useVirtualizer` signature and jsdom `clientHeight === 0` behavior.
- `crates/vox-terminal-core/src/session.rs` (`L16-80`), `block.rs` (`L3-75`), `host.rs` (`L42-106`), `stall_detector.rs` (`L17-25`) — exact `SessionEvent` (`OutputAppended`), `BlockId`, `BlockOrigin`, `PromptKind`, `TermSecurityLevel`, and `TermControlMode` serde wire representations.
- `crates/vox-gui/src/commands/pty.rs` (`L44-56`, `L241-471`) — `TERM_OUTPUT_EVENT`, `TERM_EVENT_EVENT`, `term_session_attach` (`after_seq`), and `attached_forwarders` lifecycle.
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` (`L273-329`, `L899-904`, `L1156`, `L1297-1311`), `DockWorkspaceShell.tsx` (`L18-29`), `contracts/gui/turn-event-kinds.v1.json` (`L1-131`), `src/lib/turnEvents.ts` (`L7-68`), `turnEventContract.test.tsx` (`L42-87`) — Dockview `closedPanelIds` non-resurrection invariant and closed turn-event contract rules.
- `crates/vox-gui/ui/src/guards/{zeroEmojiAndDingbat,noRawNativeControlsGuard,transportIpcGuard,toastBodyGuard}.test.ts` & `src/components/surfaces/__guards__/surfaceHonesty.guard.test.ts` — mandatory static UI guardrails.

---

## Metadata

**Confidence breakdown:**
- Standard stack & `.d.ts` APIs: **HIGH** (inspected verbatim in `node_modules`)
- Rust <-> TypeScript IPC wire contracts: **HIGH** (inspected verbatim in `vox-terminal-core` & `vox-gui/src/commands/pty.rs`)
- Vitest mock & static guard invariants: **HIGH** (inspected verbatim across all existing Console, Chat, and guard test files)

**Research date:** 2026-11-15
**Valid until:** 2026-12-15 (pinned to workspace `package.json` & Phase 442 Rust wire contracts)
