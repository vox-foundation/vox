# Milestones

## v7.5 Roman Scriptorium Warp-Class Agentic Terminal (Shipped: 2026-11-15)

**Phases completed:** 7 phases, 29 plans, 65 tasks

**Key accomplishments:**

- 5-shell (`zsh`, `bash`, `fish`, `pwsh`, `nu`) `0700` `TempDir` non-echo launch shims, real `MasterPty::resize` + `\x03` `TermSignal::Interrupt`, bounded `OSC 633`/`OSC 7` streaming parser, and persistent stateful `PtyHandle` execution in `vox-terminal-core`
- 3-byte tail `Utf8StreamDecoder` with live `633;C..D` streaming, `\r`/`CSI K` progress overwrite normalization, zero-scrollback `alacritty_terminal` `VtShadow` `ALT_SCREEN` tracking, and `2 MiB` `after_seq` delta replay across `LocalHost`, `SessionHost`, and `DaemonHost` in `vox-terminal-core`
- Loopback TCP `TermDispatch` JSON-RPC wiring for `120×32` `term.open` defaults, `LocalHost::attach_since` delta snapshots (`resync_required`), `alt_screen`/`cwd`/`git_branch` metadata and live event frames, and fail-closed origin-gated `term.signal` foreground job interruption (`TERM-PTY-04`)
- Live `SessionHost` event loop in `vox-term` (`--attach [SESSION_ID]` / `--local`, `ALT_SCREEN` raw VT pass-through, `Ctrl-C` `TermSignal::Interrupt`, clean detach), async `Cli::Term` dispatch in `vox-cli`, and `after_seq` delta attach + lossless `Utf8StreamDecoder` streaming + lag recovery in `vox-gui`
- Quote-aware POSIX + PowerShell `ExecPolicyV1` and `L1–L5` `SecurityTier` evaluator in `vox-orchestrator::oversight::exec_policy`, multi-segment wrapper-unwrapping `classify_floor`, and `check_terminal.rs` delegation (`TERM-HARNESS-03`)
- 3-stage ECMA-48/CR + `vox-redact` + Clavis `LeftmostLongest` terminal scrubber, `BlockStatus::AwaitingInput` + `pinned`/`credential_prompt` block fields, and UTF-8-safe head-40/tail-160 (`16 KiB`) block truncation (`TERM-HARNESS-04`)
- 5-shell non-subshell `__vox_pager_enter`/`__vox_pager_restore` pager guard, `ALT_SCREEN` + `1,500 ms` quiescence stall detector, AST-gated stateful `LocalHost::exec_agent_command` + `TerminalBridge` seam, and single-`Arc<LocalHost>` `vox-orchestrator-d` + `ServerState` wiring with 3-stage secret scrubbing (`TERM-HARNESS-01`, `TERM-HARNESS-02`, `TERM-HARNESS-03`)
- `commands::pty` `term_session_{set_security,set_control,pin,promote_bg}` Tauri IPC handlers, backend-resolved 3-stage scrubbed `send_terminal_block_to_agent` in `commands::console_a2a`, and canonical `safeInvoke` wrappers/DTOs in `transport.ts` (`TERM-HARNESS-03`, `TERM-HARNESS-04`)
- Boundary-safe `@term` chat citation parser (`7` selector forms), local 3-stage ECMA-48/CR + `vox-redact` + Clavis scrubber, `32 KiB` UTF-8 `<terminal_context>` budget cap, and pre-`MENTION_RE` / post-`canonicalize_prompt` `message.rs` pipeline integration (`TERM-HARNESS-04`)
- Pure 11-variant `SessionEvent` delta reducer, `afterSeq` Tauri IPC contract, 4-profile WCAG >= 4.5:1 terminal theme, 5 in-repo `@xterm/xterm@6.0.0` addons, and warm `TerminalTab` VT layer eliminating post-submit `replay_utf8` re-attach storms
- Pure-TS ECMA-48 SGR + clickable locator renderer (`sgrToSpans.ts`), semantic-token Warp `BlockCard.tsx` with conditional `AwaitingInput` stdin/background controls, and `@tanstack/react-virtual`-virtualized `BlockStreamView.tsx` (`TERM-PANE-03`).
- Unified single-pane `TerminalSessionPane` (`Blocks` + warm `TerminalTab` VT layer with `<16 ms` `ALT_SCREEN` flips), keyboard-first `TermSearchBar`, recursive `react-resizable-panels@4.22.1` `ConsoleSplitGrid` (`MAX_SPLIT_PANES = 4`), sibling-button `ConsoleSessionBar`, and refactored `Console.tsx` (`306` non-blank LoC).
- Redacted `HistoryStore`, zero-subprocess `< 50 ms` `CompletionEngine`, symlink-safe workspace Runbook scanner, base64 `OSC 633 + OSC 7` subshell `Voxify` RPC, `< 430` LoC `commands/pty.rs` Tauri endpoints, and typed `transport.ts` + `terminalBridge.ts` contracts (`TERM-WARP-02`, `TERM-WARP-03`, `TERM-WARP-04`).
- Multi-line syntax-highlighted `ScriptoriumPromptEditor` (`<pre>` React-span mirror + auto-growing `<textarea>`), hybrid IPC + `@macro`/`/slash` `terminalCompletions` engine, normalized `IntelliSensePopover`, and unmounted-when-closed `Ctrl+R` `HistorySearchPopover` with 2-step clear confirmation
- TanStack Table v9 (`@tanstack/react-table@9.3.1`) `JsonLedgerTable` + `BlockCard` `JSON`/`Table` switch, shell-quoted parameterized `RunbookPanel` + `DiscoveryRail` `Action Help | Runbooks` tabs, and `Cmd+P` `OmnibarRunbookSheet` launch flow (`TERM-WARP-04`).
- End-to-end `Console` integration of `ScriptoriumPromptEditor`, `Ctrl+R` `HistorySearchPopover`, `Cmd+J` textarea focus, `DiscoveryRail` & `Cmd+P` Runbook launch bridge, and 1-click `Voxify Subshell` (`OSC 633` + `OSC 7`) across `useConsoleWarpActions.ts`, `ConsoleSessionBar.tsx`, `TerminalSessionPane.tsx`, and `Console.tsx` (`TERM-WARP-01..04`).
- Rich terminal HITL metadata and `PendingResolution` (`Edit & Run`, compound-safe `Allow Pattern in Session`, scrubbed `Reject + Steer`, and `Autopilot -> Manual` preemption) wired across MCP and Daemon RPC
- Shipped model-agnostic terminal LLM streaming (`term_agent_prompt`, `term_agent_explain_fix`, `term_agent_cancel`), 11 live terminal slash commands (`term_slash_exec`), redacted MENS flywheel feedback persistence (`term_block_feedback`), and typed TypeScript IPC + HITL approval wrappers across `vox-gui` while preserving 100% of Phase 444.
- Self-contained DOM/WCAG/monospace terminal invariant detectors, CV/VLM Dual-Surface rubric calibration, pure-ASCII rich PTY/Agent/Approval/Chat IPC fixtures, and 8 terminal CV review states (`TERM-ROMA-CV-03`)
- Target-first `FairQueue` per canonical `CARGO_TARGET_DIR`, Windows-safe 0-byte `.lock` leader-follower flight coalescing, backward-compatible split metrics, auto-`sccache` + opt-in workstream target isolation, and post-queue `VOX_BROKER_RUN_TIMEOUT_SECS` (`exit 124`) supervision across `vox-build-queue` and `vox-cargo-shim`
- Synchronous `dev_loop_guard` enforcement for `sccache`/`.vox/agent-target`/timeout-doc parity, KB-compliant `build-broker-usage.md` + `.agents/cargo-safety.md` guidance, and dual-target `scripts/broker-ci.vox` + `scripts/broker-install.vox` updates

---
