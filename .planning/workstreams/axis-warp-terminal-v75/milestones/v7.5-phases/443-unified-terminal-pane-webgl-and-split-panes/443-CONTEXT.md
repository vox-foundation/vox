# Phase 443: Unified `<TerminalSessionPane>`, Real `@xterm/addon-webgl`, Auto-TUI Handoff & Split Panes - Context

**Gathered:** 2026-11-15
**Status:** Ready for UI spec & planning
**Mode:** Autonomous Smart Discuss (`--ws axis-warp-terminal-v75`)

<domain>
## Phase Boundary

Replace the duplicate stacked `BlockStreamView` + `TerminalTab` boxes in `Console.tsx` and the static `slice(-5)` mock readout in `ChatSurface.tsx` (`TerminalDockPanel`) with a single shared `<TerminalSessionPane>` powered by `after_seq` delta streaming, `@xterm/xterm/css/xterm.css` + self-contained WebGL2/Search/WebLinks/Unicode11/Serialize xterm addons, `<16 ms` `ALT_SCREEN` (`?1049h`/`?1049l`) Auto-TUI takeover, `@tanstack/react-virtual` + `sgrToSpans.ts` ANSI Command Blocks (zero `dangerouslySetInnerHTML`), horizontal/vertical split panes (`react-resizable-panels`), live `cwd`/branch/foreground-process session tabs, and inline streaming `vox_terminal_*` cards in `ChatTranscript.tsx`.

Covers requirements: `TERM-PANE-01`, `TERM-PANE-02`, `TERM-PANE-03`, `TERM-PANE-04`.

</domain>

<decisions>
## Implementation Decisions

### 1. Unified `<TerminalSessionPane>` & Zero-Duplicate Delta Streaming (`TERM-PANE-01`)
- **Single Shared Component (`TerminalSessionPane.tsx`):** Create `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx` and mount it in both `Console.tsx` (inside each split pane) and `ChatSurface.tsx` (`TerminalDockPanel`).
- **Eliminate Duplicate Stacked Viewport:** Instead of rendering `<BlockStreamView>` (`max-h-[340px]`) above an always-visible raw `<TerminalTab>` that echoes the exact same output twice, `<TerminalSessionPane>` presents **one unified viewport** in `Blocks` mode (virtualized `BlockStreamView` + persistent mounted xterm layer kept warm for instant `<16 ms` TUI takeover or optional Raw VT toggle) and seamlessly flips to full-bleed interactive xterm VT canvas in `ALT_SCREEN` / `Raw VT` mode. Preserve `data-testid="block-stream-view"` and `aria-label="terminal"` / `data-testid="terminal-viewport"` DOM landmarks so accessibility regions and existing Vitest suites remain green.
- **Eliminate `replay_utf8` Re-Attach Storms:** Update `transport.ts::termSessionAttach(sessionId: string, afterSeq?: number | null)` so calling `termSessionAttach(sid)` with 1 argument preserves exact existing call signatures (`{ sessionId }`), while incremental refreshes on `SessionEvent` (`BlockOpened`, `BlockClosed`, `BlockAwaitingInput`, `BlockPinned`, `CwdChanged`, `AltScreenChanged`) pass `afterSeq` (`{ sessionId, afterSeq }`) and **never** re-write `replay_utf8` onto `termRef` unless `snap.resync_required === true`. Remove the post-submit `termSessionAttach` full-replay write in `TerminalTab.tsx`.
- **Dockview Non-Resurrection Rule:** `TerminalDockPanel` inside `ChatSurface.tsx` mounts `<TerminalSessionPane compact onOpenConsole={() => props.params.onNavigate?.('console')} />` without adding any auto-recreate logic that bypasses `ChatSurface.tsx`'s `closedPanelIds` guard (`DockWorkspaceShell.tsx:18-29`).
- **Live Chat Inline Terminal Cards (`ChatTerminalBlockCard.tsx`):** Render live streaming `vox_terminal_*` cards inside `ChatTranscript.tsx` / `TurnTrace.tsx` whenever a turn contains `vox_terminal_*` tool receipts or active agent-originated blocks from `terminalBridge`, complete with command header, live status/exit badge, ANSI-rendered output preview, and 1-click **"Open in Console"** (`data-testid="chat-term-open-console"`).

### 2. `@xterm/xterm/css/xterm.css`, Self-Contained Addon Suite & `<16 ms` Auto-TUI Handoff (`TERM-PANE-02`)
- **Zero New npm Packages (`00-core.md` Rule 5):** `crates/vox-gui/ui/package.json` ships `@xterm/xterm@^6.0.0` and `@xterm/addon-fit@^0.11.0` (no `@xterm/addon-{webgl,search,web-links,unicode11,serialize}` packages in `node_modules`). Implement clean, strongly-typed `ITerminalAddon` classes in-repo (`webglAddon.ts` + `xtermAddons.ts`: `WebglTerminalAddon`, `SearchTerminalAddon`, `WebLinksTerminalAddon`, `Unicode11TerminalAddon`, `SerializeTerminalAddon`) using `@xterm/xterm` v6's native `registerLinkProvider`, `buffer.active`, and WebGL2 context management with deterministic `onContextLoss` + headless `no-tauri` / jsdom DOM fallback.
- **Import `@xterm/xterm/css/xterm.css`:** Explicitly import `@xterm/xterm/css/xterm.css` in `TerminalTab.tsx` so `.xterm-helper-textarea` is properly positioned/clipped (`opacity: 0; width: 0; height: 0`) instead of bleeding as a cream textarea box.
- **`<16 ms` Synchronous `ALT_SCREEN` Auto-TUI Takeover:** Detect `ALT_SCREEN` from three synchronized sources: (a) initial `TermAttachmentDto.alt_screen`, (b) `TERM_EVENT_EVENT` `AltScreenChanged`, and (c) zero-RTT client-side escape scanner on `TERM_OUTPUT_EVENT` chunks (`\x1b[?1049h` / `\x1b[?1047h` / `\x1b[?47h` enter; `\x1b[?1049l` / `\x1b[?1047l` / `\x1b[?47l` leave). Keep the xterm instance mounted in the pane container and toggle CSS visibility/stacking synchronously (`<16 ms`, zero xterm remount/re-open) while invoking `fit.fit()` and `term.focus()`.

### 3. Virtualized ANSI Command Blocks, Search, Pin, `Ctrl+B` & Link Actions (`TERM-PANE-03`)
- **Pure-TS `sgrToSpans.ts` (Zero `dangerouslySetInnerHTML`):** Parse SGR 16-color (`30–37`, `90–97`, `40–47`, `100–107`), 256-color (`38;5;n`, `48;5;n`), 24-bit TrueColor (`38;2;r;g;b`, `48;2;r;g;b`), text attributes (bold, dim, italic, underline, inverse, strikethrough, reset), and embedded `file:line(:col)?` + `https?://` / `localhost` links into safe React elements (`<span>` and clickable link `<button>`/`<a>` nodes).
- **`@tanstack/react-virtual` Block Virtualization:** Virtualize `BlockStreamView.tsx` when block count exceeds viewport threshold while falling back cleanly to direct rendering when container `clientHeight === 0` (preserving 100% deterministic jsdom Vitest rendering without mock layout hacks).
- **Rich Block Chrome & Actions:** Display `cwd`, `git_branch`, origin badge (`user` vs `agent:<id>`), exit seal (`exit 0`, `exit N`, `running`, `awaiting input`, `cancelled`), duration, sticky header (`sticky top-0 z-10`), collapse/expand toggle, `AwaitingInput` inline stdin prompt bar + `Ctrl+B` (`termSessionPromoteBg`) background promotion button, backend-synced Pin (`termSessionPin`), and `Cmd/Ctrl+F` search bar (`data-testid="term-search-bar"`) with match navigation across blocks and active VT buffer.

### 4. Split Panes & Multi-Session Tabs (`TERM-PANE-04`)
- **Horizontal & Vertical Split Panes (`ConsoleSplitGrid.tsx`):** Use installed `react-resizable-panels` (`^4.22.1`) to support recursive or 2D horizontal/vertical pane splitting (`Split Right`, `Split Down`, `Close Pane`) inside `Console.tsx`, with active-pane focus ring and independent `sessionId` assignment per pane.
- **Session Tab Bar:** Upgrade `Console.tsx` session tabs to display live `cwd` basename (from `TermSessionInfoDto.cwd` / `CwdChanged` events), `git_branch` badge, foreground command title when a block is `Running`/`AwaitingInput`, block count, and session close action, while syncing `termSessionSetSecurity` and `termSessionSetControl` to the backend (mapping UI option values cleanly to `TermSecurityLevelWire` / `TermControlModeWire` while keeping `Console.test.tsx` option names `'L2 Safe Exec'`, `'L1 Read-Only'`, etc. intact).

### 5. Non-Negotiable Repo Guardrails
- **Zero emojis or banned glyphs** (`src/guards/zeroEmojiAndDingbat.test.ts` — use `<Icon.*>` from `components/ui/Icons.tsx` or ASCII text only; never use Unicode geometric shapes, arrows, checkmarks, or return symbols in TS/TSX source strings).
- **Zero raw `<select>` or `<input type="range">`** (`src/guards/noRawNativeControlsGuard.test.ts` — use `<VoxSelect>`).
- **Zero direct `@tauri-apps/api/*` imports outside `src/transport.ts`** (`src/guards/transportIpcGuard.test.ts`).
- **Sanitize all error toasts via `sanitizeErrorForToast`** (`src/guards/toastBodyGuard.test.ts`).
- **Every source file `< 500` non-blank LoC** (`vox-code-audit` `god_object` detector; split helpers cleanly across focused modules).

</decisions>

<canonical_refs>
## Canonical References

- `.planning/workstreams/axis-warp-terminal-v75/ROADMAP.md` — Phase 443 goal & 4 success criteria
- `.planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md` — `TERM-PANE-01..04`
- `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` — v7.5 terminal architecture & Dual-Surface Roman Scriptorium spec
- `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx` — Console surface shell (`524` total lines; extract split grid/helpers so it stays well under `< 500` non-blank LoC!)
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx` — xterm + `SessionHost` bridge
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx` — Command block cards
- `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts` — `WebglTerminalAddon` + `renderTermBlockForAgent`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` — `TerminalDockPanel` & `closedPanelIds` invariant
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx` & `TurnTrace.tsx` & `ChatTurnEventRow.tsx` — Chat transcript & tool trace rendering
- `crates/vox-gui/ui/src/transport.ts` — Canonical Tauri IPC boundary (`termSession*`)
- `crates/vox-gui/ui/src/guards/zeroEmojiAndDingbat.test.ts` & `noRawNativeControlsGuard.test.ts` & `transportIpcGuard.test.ts` — Static UI regression guards

</canonical_refs>

<deferred>
## Deferred Ideas

- Multi-line `ScriptoriumPromptEditor`, `Ctrl+R` persistent history, `$PATH` completions, and `.vox` Runbooks -> Phase 444 (`TERM-WARP-01..04`).
- `?prompt` LLM streaming, 1-click *Explain / Fix Failed Block*, Roman wax-seal Co-Pilot Approval Cards, and live `/slash` commands -> Phase 445 (`TERM-AGENT-01..04`).
- Global `index.css` `--font-terminal-mono` `:not(.xterm)` rule changes and 4-theme Playwright CV Flywheel convergence -> Phase 446 (`TERM-ROMA-CV-01..04`).

</deferred>
