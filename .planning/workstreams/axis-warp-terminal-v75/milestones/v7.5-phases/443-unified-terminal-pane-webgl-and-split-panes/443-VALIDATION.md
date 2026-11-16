---
phase: "443"
slug: "unified-terminal-pane-webgl-and-split-panes"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-15"
validated: "2026-11-15"
---

# Phase 443 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Vitest `3.2.4` + `@testing-library/react` `16.0.1` (`jsdom`) + TypeScript `tsc --noEmit` (`5.6.2`) |
| **Config file** | `crates/vox-gui/ui/vite.config.ts` & `crates/vox-gui/ui/tsconfig.json` |
| **Quick run command** | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/` |
| **Full suite command** | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/turnEventContract.test.tsx src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/guards/zeroEmojiAndDingbat.test.ts src/guards/noRawNativeControlsGuard.test.ts src/guards/transportIpcGuard.test.ts src/guards/toastBodyGuard.test.ts src/components/surfaces/__guards__/surfaceHonesty.guard.test.ts && timeout 300s pnpm --dir crates/vox-gui/ui run typecheck` (run as two separate foreground commands per `GEMINI.md` command-shape rules) |
| **Estimated runtime** | ~4–8 seconds per Vitest file; ~15 seconds for `typecheck` |

---

## Sampling Rate

- **After every task:** Run the task's scoped `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run <spec>` command + `timeout 300s pnpm --dir crates/vox-gui/ui run typecheck`
- **After every plan wave:** Run all wave-touched Console/Chat specs + static UI guard tests (`zeroEmojiAndDingbat`, `noRawNativeControlsGuard`, `transportIpcGuard`, `toastBodyGuard`, `surfaceHonesty.guard`)
- **Before `/gsd-verify-work`:** Full Console + Chat + Transport + Static Guards Vitest matrix green AND `pnpm --dir crates/vox-gui/ui run typecheck` exits `0` with zero errors
- **Max feedback latency:** 20 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 443-01-01 | 01 | 1 | TERM-PANE-01, TERM-PANE-02 | T-443-01 | `termSessionAttach(sid, afterSeq?)` preserves 1-arg `{ sessionId }` and forwards `{ sessionId, afterSeq }` when provided; `terminalBridge.ts` exposes `requestConsoleBlockFocus` / `onConsoleBlockFocus`; pure `sessionReducer.ts` handles all 11 externally-tagged `SessionEvent` variants (`OutputAppended`, `BlockOpened`, `BlockClosed`, `BlockAwaitingInput`, `BlockPinned`, `CwdChanged`, `AltScreenChanged`, `SnapshotResync`, `SecurityLevelChanged`, `ControlModeChanged`, `AgentMessage`) + `scanAltScreenTransition` + wire mappers | unit | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/components/surfaces/Console/sessionReducer.test.ts src/guards/transportIpcGuard.test.ts` | ✅ | ✅ green |
| 443-01-02 | 01 | 1 | TERM-PANE-02 | T-443-03, T-443-SC | `terminalTheme.ts` guarantees WCAG >= 4.5:1 across all 4 normalized theme profiles; `WebglTerminalAddon` (`WEBGL_lose_context` probe cleanup + `onContextLoss` DOM fallback), `SearchTerminalAddon`, `WebLinksTerminalAddon` (1-based `ILinkProvider`, `http`/`https` allowlist), `Unicode11TerminalAddon` (`allowProposedApi: true` + mock-safe try/catch), and `SerializeTerminalAddon` activate cleanly without new npm dependencies | unit + security | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/xtermAddons.test.ts src/lib/themeContract.test.ts src/guards/zeroEmojiAndDingbat.test.ts` | ✅ | ✅ green |
| 443-01-03 | 01 | 1 | TERM-PANE-01, TERM-PANE-02 | T-443-01 | `TerminalTab.tsx` imports `@xterm/xterm/css/xterm.css`, loads all 6 addons safely, tracks live `[data-theme]` changes (`MutationObserver`), suppresses `.xterm-helper-textarea` (`tabIndex={-1}`, `aria-hidden="true"`) in `Blocks` mode, scans `ALT_SCREEN` transitions (`< 16 ms`), and eliminates post-submit `replay_utf8` re-attach storms | component + typecheck | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/TerminalTab.test.tsx src/components/surfaces/Console/xtermAddons.test.ts src/transport.console.test.ts` | ✅ | ✅ green |
| 443-02-01 | 02 | 1 | TERM-PANE-03 | T-443-02, T-443-03 | Pure-TS `sgrToSpans.ts` parses 16/256/TrueColor SGR + bold/dim/italic/underline/inverse/strike + column-aware `\r` / `CSI 0K/1K/2K` overwrite + search highlights into safe React `<span>` / link `<button>` elements with **zero** `dangerouslySetInnerHTML`, escapes `<script>` payloads literally, allowlists `http:`/`https:` URLs, and validates workspace-relative `file:line(:col)?` via `sanitizeWorkspaceFileLocator` before dispatching `voxTransport.openLocator` | unit + security | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/sgrToSpans.test.tsx src/lib/themeContract.test.ts src/guards/zeroEmojiAndDingbat.test.ts` | ✅ | ✅ green |
| 443-02-02 | 02 | 1 | TERM-PANE-03 | T-443-02, T-443-04 | `BlockCard.tsx` and `BlockStreamView.tsx` virtualize >24 blocks via `@tanstack/react-virtual` (`useVirtualizer`) with `clientHeight === 0` jsdom fallback, render ANSI spans via `sgrToSpans.ts`, display `cwd`/`git_branch`/origin/exit/duration/sticky header/collapse/`Pin`-`Unpin`, unmount `term-block-stdin-bar` to `null` when not `AwaitingInput`, mask `credential_prompt: true` stdin as `type="password"`, and wire `Ctrl+B` (`Promote to Background`) | component + security + typecheck | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/BlockStreamView.test.tsx src/components/surfaces/Console/sgrToSpans.test.tsx src/lib/themeContract.test.ts src/guards/zeroEmojiAndDingbat.test.ts src/guards/noRawNativeControlsGuard.test.ts` | ✅ | ✅ green |
| 443-03-01 | 03 | 2 | TERM-PANE-01, TERM-PANE-02, TERM-PANE-03 | T-443-01 | `TermSearchBar.tsx` (strict `null` unmount when closed) and unified `<TerminalSessionPane>` mount warm `<TerminalTab>` + `<BlockStreamView>` without duplicate stacked output, coalesce split-clone initial attaches (`acquireInitialAttach`), flip synchronously (`< 16 ms`, zero xterm remount) between `Blocks` and `ALT_SCREEN` (`?1049h`/`?1049l` + `AltScreenChanged`), and write `replay_utf8` only on initial attach or `resync_required === true` | component | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/TerminalSessionPane.test.tsx src/guards/transportIpcGuard.test.ts src/guards/zeroEmojiAndDingbat.test.ts` | ✅ | ✅ green |
| 443-03-02 | 03 | 2 | TERM-PANE-04 | T-443-05 | `ConsoleSplitGrid.tsx` uses `react-resizable-panels@4.22.1` (`ResizablePanelGroup` `orientation="horizontal" \| "vertical"`, `"50%"`/`"20%"` string sizes, direct children, max-4 cap) and `ConsoleSessionBar.tsx` renders non-nested sibling tab/close buttons with `cwd`/`git_branch`/foreground command labels and active-job close confirmation | component | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/ConsoleSplitGrid.test.tsx src/guards/noRawNativeControlsGuard.test.ts src/guards/zeroEmojiAndDingbat.test.ts src/lib/themeContract.test.ts` | ✅ | ✅ green |
| 443-03-03 | 03 | 2 | TERM-PANE-01, TERM-PANE-04 | T-443-05, T-443-06 | `Console.tsx` (`407` non-blank LoC, within `< 450` UI-SPEC decomposition budget) mounts `<ConsoleSplitGrid>` + `<TerminalSessionPane>` + `<ConsoleSessionBar>`, preserves unique `screen.getByRole('textbox')` and all existing `Console.test.tsx` tests, enforces chord-safe keyboard shortcuts, and syncs `<VoxSelect>` security + control mode changes (plus universal `Autopilot -> manual` preemption) via `termSessionSetSecurity` / `termSessionSetControl` | component + guard + typecheck | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/ src/guards/zeroEmojiAndDingbat.test.ts src/guards/noRawNativeControlsGuard.test.ts src/guards/transportIpcGuard.test.ts src/guards/toastBodyGuard.test.ts src/components/surfaces/__guards__/surfaceHonesty.guard.test.ts src/lib/themeContract.test.ts` | ✅ | ✅ green |
| 443-04-01 | 04 | 3 | TERM-PANE-01 | T-443-02 | `ChatTerminalBlockCard.tsx`, `ChatTurnEventRow.tsx`, and `TurnTrace.tsx` render live streaming `vox_terminal_exec` / `vox_terminal_send_input` block cards (`sgrToSpans` ANSI preview, historical receipt isolation, `AwaitingInput` `Promote to Background`, 1-click `Open in Console` `data-testid="chat-term-open-console"`) without touching `TURN_EVENT_KINDS` (`turnEventContract.test.tsx` 100% green) | component + contract | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/turnEventContract.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx src/guards/zeroEmojiAndDingbat.test.ts` | ✅ | ✅ green |
| 443-04-02 | 04 | 3 | TERM-PANE-01 | T-443-02, T-443-07 | `ChatSurface.tsx::TerminalDockPanel` resolves existing daemon sessions via `termSessionList()` and mounts `<TerminalSessionPane compact>` inside `data-testid="chat-dock-terminal"` without violating `closedPanelIds` update-only non-resurrection (`L1312`), `ChatTranscript.tsx` streams active agent `ChatTerminalBlockCard`s, and full Console + Chat + Guard Vitest suites and `tsc --noEmit` exit `0` | component + guard + typecheck | `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/turnEventContract.test.tsx src/components/surfaces/Chat/ChatSurface.test.tsx` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Vitest `3.2.4`, `@testing-library/react` `16.0.1`, `jsdom`, `tsc --noEmit`, and all static UI regression guards (`zeroEmojiAndDingbat`, `noRawNativeControlsGuard`, `transportIpcGuard`, `toastBodyGuard`, `surfaceHonesty.guard`, `turnEventContract`) are already installed and passing in `crates/vox-gui/ui` — each task embeds its TDD RED -> GREEN unit/component test inline with zero separate Wave 0 scaffold plan required.

---

## Manual-Only Verifications

*All phase behaviors have automated Vitest + `tsc --noEmit` verification (4-theme live WebView Playwright CV visual convergence follows in Phase 446 `TERM-ROMA-CV-01..04`).*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags (`vitest run` only, prefixed with `timeout 300s`)
- [x] Feedback latency < 20s per task
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-11-15
