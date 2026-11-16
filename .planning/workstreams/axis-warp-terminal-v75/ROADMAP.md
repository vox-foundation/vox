# Roadmap: Milestone v7.5 (`axis-warp-terminal-v75`) — Roman Scriptorium Warp-Class Agentic Terminal, Stateful PTY Engine, Bidirectional Chat Harness & CV Flywheel Convergence

**Workstream:** `.planning/workstreams/axis-warp-terminal-v75/`
**Scope:** Upgrade `vox-terminal-core` (`L3`), `vox-orchestrator` (`L3`), `vox-orchestrator-mcp` (`L3`), `vox-term` (`L4`), `vox-orchestrator-d` (`L5`), and `vox-gui` (`L5`: `Console`, `Chat`, `DockWorkspaceShell`, 4-profile Roman Scriptorium theme contract, and `@ngi/platform-ssot` CV Flywheel) to a zero-spam, stateful Warp-class (`95%` capability parity) agentic terminal (`Phases 441–446`, `24/24` requirements). Zero new external Rust crates in `Cargo.lock`, zero new workspace Cargo edges in `contracts/ci/crate-edges.allow.v1.json`, and 100% model-agnostic LLM routing via `vox_actor_runtime::llm`.

## Phase Checklist

- [x] **Phase 441: Zero-Spam Stateful PTY Engine, Non-Echo Shell Bootstrap & Lossless VT/OSC Streaming** (`TERM-PTY-01..04`) (completed 2026-11-14)
- [x] **Phase 442: AST Exec-Policy Gate, Daemon `term.*` Control Plane & Bidirectional MCP/Chat Terminal Harness** (`TERM-HARNESS-01..04`) (completed 2026-11-15)
- [x] **Phase 443: Unified `<TerminalSessionPane>`, Real `@xterm/addon-webgl`, Auto-TUI Handoff & Split Panes** (`TERM-PANE-01..04`) (completed 2026-11-15)
- [x] **Phase 444: Warp-Grade Universal Multi-Line Editor, Context Completions, History & Local `.vox` Runbooks** (`TERM-WARP-01..04`) (completed 2026-11-15)
- [x] **Phase 445: Terminal-Native Agentic Mode (`vox_actor_runtime::llm`), Live Slash Commands, Co-Pilot Approvals & MENS Flywheel** (`TERM-AGENT-01..04`) (completed 2026-11-15)
- [x] **Phase 446: Dual-Surface Roman Scriptorium Theme Hardening & Multi-Pass 4-Theme CV Flywheel Convergence** (`TERM-ROMA-CV-01..04`) (completed 2026-11-15)
- [x] **Phase 447: Repo-Wide Build Contention Elimination & Broker Coalescing** (`BUILD-CONTENTION-01..04`) (completed 2026-11-15)

### Phase 441: Zero-Spam Stateful PTY Engine, Non-Echo Shell Bootstrap & Lossless VT/OSC Streaming

**Directory**: `.planning/workstreams/axis-warp-terminal-v75/phases/441-stateful-pty-and-lossless-streaming/`
**Goal**: Eliminate terminal load echo spam, broken `zsh` exit codes, stateless `Command::new` per-line subprocesses, no-op `resize`, and UTF-8 multi-byte split corruption in `vox-terminal-core`, and wire `vox term --attach` for live CLI ↔ GUI session continuity.
**Depends on**: None
**Requirements**: TERM-PTY-01, TERM-PTY-02, TERM-PTY-03, TERM-PTY-04
**Success Criteria** (what must be TRUE):

  1. Opening a session in `zsh`, `bash`, `fish`, `pwsh`, or `nu` via `shell_init.rs` (`tempfile::TempDir` launch shims) produces zero echoed `OSC 633` hook source lines in `replay` or shell history, accurately reports non-zero `$?`/`$LASTEXITCODE` (including `zsh` `false` / `exit 7` without `PROMPT_SP` `%` noise), and emits live `OSC 7` / `OSC 633;P;Cwd` directory updates.
  2. Submitting shell commands (`cd`, `export`, virtualenv activation, streaming multi-line output) runs inside the persistent `PtyHandle`, `PtyHandle::resize` updates real kernel `TIOCSWINSZ`/ConPTY dimensions (`tput cols`), and `term.signal` delivers `SIGINT` (`\x03` / foreground interrupt) to running jobs.
  3. Arbitrary prime-chunked multi-byte UTF-8 streams (box-drawing, emojis, ANSI sequences) decode with zero `U+FFFD` replacement characters, `alacritty_terminal` detects `TermMode::ALT_SCREEN` (`?1049h`/`?1049l`) + normalizes `\r` progress overwrites, and `after_seq` delta streaming recovers cleanly from broadcast lag without re-writing 2 MiB `replay_utf8`.
  4. `vox term --attach <session_id>` attaches live to the same daemon-hosted session active in Axis GUI.

**Plans**: 4/4 plans executed

Plans:

- [x] 441-01-PLAN.md — Non-echo 5-shell `TempDir` bootstrap (`shell_init.rs`), persistent stateful PTY execution, real `TIOCSWINSZ`/ConPTY resize, and `TermSignal` (`TERM-PTY-01`, `TERM-PTY-02`)
- [x] 441-02-PLAN.md — Lossless `Utf8StreamDecoder`, CR overwrite normalization, `VtShadow` `ALT_SCREEN` state machine, and `2 MiB` `after_seq` delta ring across `LocalHost` + `DaemonHost` (`TERM-PTY-03`, `TERM-PTY-04`)
- [x] 441-03-PLAN.md — `vox-orchestrator-d` `TermDispatch` `120×32` defaults, `after_seq` delta snapshots, `alt_screen`/`cwd` events, and origin-gated `term.signal` (`TERM-PTY-04`)
- [x] 441-04-PLAN.md — `vox-term` `SessionHost` TUI loop + `--attach`, `vox-cli` `Cli::Term` flags, and `vox-gui` `after_seq` + `Utf8StreamDecoder` + lag recovery (`TERM-PTY-04`)

### Phase 442: AST Exec-Policy Gate, Daemon `term.*` Control Plane & Bidirectional MCP/Chat Terminal Harness

**Directory**: `.planning/workstreams/axis-warp-terminal-v75/phases/442-bidirectional-chat-harness-and-ast-security/`
**Goal**: Expose shared `SessionHost` terminal execution, streaming inspection, interactive stall detection, AST-backed `L1–L5` command policy gating, and backend-redacted `@term` citations to Chat and MCP agents with zero new workspace Cargo edges.
**Depends on**: Phase 441
**Requirements**: TERM-HARNESS-01, TERM-HARNESS-02, TERM-HARNESS-03, TERM-HARNESS-04
**Success Criteria** (what must be TRUE):

  1. `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, and `vox_terminal_sessions` are registered in `contracts/operations/catalog.v1.yaml` (synced via `operations-sync`) and execute against `vox-orchestrator::terminal_bridge` in both foreground and `background: true` (`Ctrl+B`) modes with `0` `vox-arch-check` or `crate-edges` violations.
  2. Agent-dispatched PTY commands neutralize pagers (`PAGER=cat`, `GIT_PAGER=cat`) so `git log`/`git diff` complete non-interactively, while interactive prompts (`[y/N]`, `Password:`, `?1049h`) transition the block to `AwaitingInput` within the stall window instead of hanging the agent loop.
  3. `L1`–`L5` `ExecPolicyV1` + `classify_floor` blocks subshell `$()`, backtick, redirection, and wrapper-binary (`pwsh -EncodedCommand`, `find -exec`) bypasses in `L1`/`L2`, and `Console` security/control selectors persist via `term.security` / `term.control` RPCs.
  4. `@term`, `@term:last`, `@term:failed`, `@term:pinned`, `@term:<sid>#<id>:L<a-b>`, and `SendToAgent` strip ANSI escapes and scrub secrets/paths (`vox-redact` + Clavis exact-value match) on the Rust backend before any LLM prompt assembly.

**Plans**: 6 plans

Plans:

- [x] 442-01-PLAN.md — Pure-Rust POSIX + PowerShell quote-aware `ExecPolicyV1`, `L1–L5` `SecurityTier` wrapper unwrapping, multi-segment `floor.rs` shell classification, and `check_terminal.rs` delegation (`TERM-HARNESS-03`)
- [x] 442-02-PLAN.md — 3-stage backend scrubber (`strip_ansi_and_normalize` + upgraded `vox-redact` + Clavis `scrub_resolved_secrets`), `BlockStatus::AwaitingInput`, and head-40/tail-160 `16 KiB` truncation (`TERM-HARNESS-04`)
- [x] 442-03-PLAN.md — Non-subshell 5-shell pager neutralization, `stall_detector.rs`, stateful `LocalHost`/`DaemonHost` agent execution + `promote_bg` + `pin`, `vox_orchestrator::terminal_bridge`, and `vox-orchestrator-d` `TermDispatch` RPCs (`TERM-HARNESS-01`, `TERM-HARNESS-02`, `TERM-HARNESS-03`)
- [x] 442-04-PLAN.md — `vox_terminal_{exec,read,send_input,sessions}` MCP schemas/dispatch, `catalog.v1.yaml` + `operations-sync`/`operations-verify`, and `PendingApprovals` HITL lockstep (`TERM-HARNESS-01`, `TERM-HARNESS-03`, `TERM-HARNESS-04`)
- [x] 442-05-PLAN.md — `vox-gui` `term_session_{set_security,set_control,pin,promote_bg}` Tauri commands, backend-resolved 3-stage scrubbed `send_terminal_block_to_agent`, and canonical `transport.ts` IPC wrappers (`TERM-HARNESS-03`, `TERM-HARNESS-04`)
- [x] 442-06-PLAN.md — `@term` pre-canonicalizer chat citations (`term_mentions.rs`, `mentions.rs`, `message.rs`) with 3-stage scrubbing and `32 KiB` per-turn envelope (`TERM-HARNESS-04`)

### Phase 443: Unified `<TerminalSessionPane>`, Real `@xterm/addon-webgl`, Auto-TUI Handoff & Split Panes

**Directory**: `.planning/workstreams/axis-warp-terminal-v75/phases/443-unified-terminal-pane-webgl-and-split-panes/`
**Goal**: Replace the split stacked `BlockStreamView` + `TerminalTab` layout in `Console.tsx` and the mocked `TerminalDockPanel` in `ChatSurface.tsx` with one shared `<TerminalSessionPane>` featuring real `@xterm/addon-webgl`, `<16 ms` `ALT_SCREEN` Auto-TUI takeover, virtualized ANSI blocks, split panes, and inline Chat terminal cards.
**Depends on**: Phase 442
**Requirements**: TERM-PANE-01, TERM-PANE-02, TERM-PANE-03, TERM-PANE-04
**Success Criteria** (what must be TRUE):

  1. `Console.tsx` and `DockWorkspaceShell` (`TerminalDockPanel`) mount the shared `<TerminalSessionPane>` over live `after_seq` deltas (zero duplicate stacked boxes, zero `replay_utf8` re-attach storms), and `ChatTranscript.tsx` renders live streaming `vox_terminal_*` block cards with "Open in Console".
  2. `@xterm/xterm/css/xterm.css` and `@xterm/addon-{webgl,search,web-links,unicode11,serialize}` are active (falling back cleanly to DOM rendering on `onContextLoss` or headless `no-tauri`), and entering `vim`/`lazygit`/`htop` (`?1049h`) switches seamlessly (`<16 ms`) to full-bleed interactive VT canvas and back on exit (`?1049l`).
  3. Virtualized Command Blocks (`sgrToSpans.ts`, zero `dangerouslySetInnerHTML`) render `cwd`, git branch, exit seal, and duration, with collapse/expand, sticky headers, `Cmd/Ctrl+F` search, pin/bookmark, and clickable `file:line:col` + URL handlers.
  4. Operators can split `Console` horizontally/vertically into independent panes and manage multiple session tabs showing live `cwd` and foreground-process titles.

**Plans**: 4 plans

Plans:

- [x] 443-01-PLAN.md — `afterSeq` transport contract, Console focus bridge, pure 11-variant `sessionReducer.ts` + `<16 ms` `scanAltScreenTransition`, 4-profile WCAG `terminalTheme.ts`, full 6-addon `xtermAddons.ts` + `webglAddon.ts` suite, and warm `TerminalTab.tsx` (`TERM-PANE-01`, `TERM-PANE-02`)
- [x] 443-02-PLAN.md — Pure-TS `sgrToSpans.ts` (16/256/TrueColor SGR, `\r` overwrites, clickable `file:line:col` + URLs, zero `dangerouslySetInnerHTML`), `BlockCard.tsx` (`cwd`, `git_branch`, `Pin`/`Unpin`, `AwaitingInput` stdin + `Ctrl+B`), and `@tanstack/react-virtual` `BlockStreamView.tsx` (`TERM-PANE-03`)
- [x] 443-03-PLAN.md — Unified `<TerminalSessionPane>` + `Cmd/Ctrl+F` `<TermSearchBar>`, `react-resizable-panels` v4 `<ConsoleSplitGrid>`, non-nested `<ConsoleSessionBar>` with wire policy sync, and `< 350` LoC `<Console>` refactor (`TERM-PANE-01`, `TERM-PANE-02`, `TERM-PANE-03`, `TERM-PANE-04`)
- [x] 443-04-PLAN.md — Live streaming `vox_terminal_*` `<ChatTerminalBlockCard>` in `TurnTrace.tsx` / `ChatTranscript.tsx` (preserving `TURN_EVENT_KINDS`) and shared `<TerminalSessionPane compact>` in `ChatSurface.tsx` `TerminalDockPanel` (preserving `closedPanelIds`) (`TERM-PANE-01`)

### Phase 444: Warp-Grade Universal Multi-Line Editor, Context Completions, History & Local `.vox` Runbooks

**Directory**: `.planning/workstreams/axis-warp-terminal-v75/phases/444-warp-editor-completions-and-runbooks/`
**Goal**: Deliver 95% Warp 2.0 operator ergonomics with a multi-line syntax-highlighted Universal Input editor, fuzzy persistent history (`Ctrl+R`), `$PATH`/file/git/CLI-flag completions, structured `!nu`/`.vox` table inspection, subshell "Voxify", and local `.vox` / Markdown Runbooks.
**Depends on**: Phase 443
**Requirements**: TERM-WARP-01, TERM-WARP-02, TERM-WARP-03, TERM-WARP-04
**Success Criteria** (what must be TRUE):

  1. `ScriptoriumPromptEditor` supports multi-line editing (`Shift+Enter`), syntax highlighting, and automatic/lockable routing across Shell, `?` Agent, `!nu`, `.vox`, and `/slash` modes.
  2. `Up`/`Down` arrow recall and `Ctrl+R` fuzzy search query persistent `vox-redact`-scrubbed workspace command history.
  3. Typing or pressing `Tab` completes `$PATH` binaries, `cwd`-relative paths, `git` branches/tags, and CLI subcommands/flags for any command (not only `vox *`).
  4. Operators can launch parameterized `.vox` & Markdown Runbooks from `Cmd+P` / `DiscoveryRail`, inspect `!nu`/`.vox` JSON outputs in a sortable/filterable Ledger Table, and 1-click "Voxify" nested subshells with `OSC 633` hooks.

**Plans**: 4 plans

Plans:

- [x] 444-01-PLAN.md — `vox-terminal-core` (`HistoryStore`, `CompletionEngine`, `runbooks`, `voxify`, `SessionHost::voxify_subshell`), `vox-orchestrator-d` `term.voxify` dispatch, `< 450` LoC `commands/pty.rs` + `pty/warp.rs` + `pty/tests.rs` Tauri endpoints, and `transport.ts` + `terminalBridge.ts` contracts (`TERM-WARP-02`, `TERM-WARP-03`, `TERM-WARP-04`)
- [x] 444-02-PLAN.md — Multi-line syntax-highlighted `ScriptoriumPromptEditor`, deterministic `promptHighlighter` + `intentRouter`, context-aware `terminalCompletions` + `IntelliSensePopover`, `Ctrl+R` `HistorySearchPopover`, and backward-compatible `InputEditor` wrapper (`TERM-WARP-01`, `TERM-WARP-02`, `TERM-WARP-03`)
- [x] 444-03-PLAN.md — TanStack Table v9 `JsonLedgerTable` + `BlockCard` `JSON`/`Table` switch, parameterized `.vox`/Markdown `RunbookPanel` inside `DiscoveryRail`, and `Cmd+P` `Omnibar` Runbook launch (`TERM-WARP-04`)
- [x] 444-04-PLAN.md — `ConsoleSessionBar` + `TerminalSessionPane` `Voxify Subshell` CTA, `Console` `Ctrl+R` / `Cmd+J` / Runbook launch bridge wiring, and full Phase 444 Rust + Vitest + UI static guard verification (`TERM-WARP-01`, `TERM-WARP-02`, `TERM-WARP-03`, `TERM-WARP-04`)

### Phase 445: Terminal-Native Agentic Mode (`vox_actor_runtime::llm`), Live Slash Commands, Co-Pilot Approvals & MENS Flywheel

**Directory**: `.planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/`
**Goal**: Enable full agentic control directly from the terminal via `vox_actor_runtime::llm` streaming turns, 1-click failed-block diagnosis/fix, inline Co-Pilot Command Approval Cards, live orchestrator slash commands, and redacted MENS trajectory feedback.
**Depends on**: Phase 444
**Requirements**: TERM-AGENT-01, TERM-AGENT-02, TERM-AGENT-03, TERM-AGENT-04
**Success Criteria** (what must be TRUE):

  1. Submitting `?prompt` or natural-language prose in the terminal streams a real multi-step plan and response through `vox_actor_runtime::llm` (`0` vendor SDK violations in `vox audit code`).
  2. Every failed block (`exit_code != 0`) exposes a working 1-click *Explain / Fix Failed Block* flow that proposes an executable repair command in-band.
  3. Co-Pilot gated commands render inline Roman wax-seal Command Approval Cards (`Approve`, `Edit & Run`, `Allow Pattern in Session`, `Reject + Steer`) in both `Console` and `Chat`, and any user keystroke/submit in `Autopilot` immediately switches control to `Manual`.
  4. `/budget`, `/model`, `/skills`, `/memory`, `/context`, `/clear`, and `/compact` execute against live orchestrator state, and per-block Accept/Reject/Correct actions emit redacted `vox-journal` → `CorpusWriter` records.

**Plans**: 4 plans

Plans:

- [x] 445-01-PLAN.md — `vox-terminal-core` & `vox-orchestrator-d` streaming `AgentTurn`/`SlashCommand` custom blocks, `/clear` & `/compact` `SnapshotResync` + `DaemonHost::route("snapshot")` fix, `control_epoch` preemption, 3-stage `CorpusWriter` `Accepted`/`Rejected` redaction, `DefaultPolicy` negative retention, and `JournalSink` persistence (`TERM-AGENT-01`, `TERM-AGENT-03`, `TERM-AGENT-04`)
- [x] 445-02-PLAN.md — `vox-orchestrator-mcp` `PendingApprovals` terminal metadata, dual-path `PendingResolution` (`tasks_core.rs` + `daemon_extra.rs`), compound-safe `Allow Pattern in Session` (`extract_effective_commands`), `Edit & Run` AST/tier re-validation, `Reject + Steer` note scrubbing, and `Autopilot -> Manual` preemption (`TERM-AGENT-03`)
- [x] 445-03-PLAN.md — `vox-gui` model-agnostic terminal agent streaming (`vox_actor_runtime::llm` + Clavis/local MENS), 3-stage scrubbed `Explain Failure` / `Fix Command`, live slash handlers (`pty/slash.rs`), `term_block_feedback` (`pty/approvals.rs`), and typed `mcpToolResult.ts` + `transport.ts` contracts (`TERM-AGENT-01`, `TERM-AGENT-02`, `TERM-AGENT-04`)
- [x] 445-04-PLAN.md — Console & Chat UI: `useTerminalPendingApprovals`, semantic `<Icon.waxSeal>` `TerminalApprovalCard` (`Console` + `ChatTerminalBlockCard`/`TurnTrace`), `AgentTurnContent`, `SlashMetaContent` + MENS feedback strip, `BlockCard` Explain/Fix actions, `Autopilot` preemption banner, and full Phase 445 verification (`TERM-AGENT-01`, `TERM-AGENT-02`, `TERM-AGENT-03`, `TERM-AGENT-04`)

### Phase 446: Dual-Surface Roman Scriptorium Theme Hardening & Multi-Pass 4-Theme CV Flywheel Convergence

**Directory**: `.planning/workstreams/axis-warp-terminal-v75/phases/446-roman-scriptorium-theme-and-cv-flywheel/`
**Goal**: Enforce `--font-terminal-mono` CSS containment and Dual-Surface Roman Scriptorium tokens across all 4 theme profiles (`arcane`, `slate`, `travertine`, `high-contrast`), add deterministic terminal CV metrics, and converge across multi-pass Playwright + `@ngi/platform-ssot` + VLM audits at `900×600`..`1920×1080`.
**Depends on**: Phase 445
**Requirements**: TERM-ROMA-CV-01, TERM-ROMA-CV-02, TERM-ROMA-CV-03, TERM-ROMA-CV-04
**Success Criteria** (what must be TRUE):

  1. `--font-terminal-mono` and `:not(.xterm):not(.xterm *)` containment eliminate proportional `Palatino` cell drift and `.xterm-helper-textarea` cream-box bleed while passing `vox audit code` (`vox/ui/theme-contract-drift`) and `themeContract.test.ts` with `0` errors.
  2. Dual-Surface Roman Scriptorium styling delivers Carrara marble & burnished gold chrome + warm dark-bronze/obsidian WCAG >= 4.5:1 16-color ANSI contrast in `arcane` and full token parity across `slate`, `travertine`, and `high-contrast` (`0` hardcoded `zinc-*`/`black` islands, `0` dark-on-dark `.text-brass` defects).
  3. `audits.ts`, `cvFlywheel.ts`, and `cv-flywheel.spec.ts` validate terminal ANSI contrast, fixed-width cell alignment, `<UnifiedSelect>` parity, and zero control clipping.
  4. Multi-pass `CV_STAGE=before -> after` Playwright + VLM runs achieve `visualAfterScore >= 0.92` and `0` axe/overflow/clipping defects across standalone `Console` and `Chat` + `TerminalDockPanel` at `900×600`, `1100×720`, `1280×800`, `1440×900`, and `1920×1080`, with `warp-roman-terminal-v75-research-2026.md` promoted to `status: "current"`.

**Plans**: 4 plans

Plans:

- [x] 446-01-PLAN.md — Style Dictionary `--font-terminal-mono` token pipeline, 23-var `theme-contract.v1.json` + `themeContract.ts` + `cv-flywheel.spec.ts` (`23 / 23 Verified`) sync, and `src/index.css` Dual-Surface `--terminal-well-*` + `:not(.xterm)` / `.xterm-helper-textarea` light+dark containment (`TERM-ROMA-CV-01`)
- [x] 446-02-PLAN.md — 4-theme × 16-color WCAG-calibrated Dual-Surface ANSI palettes (`terminalTheme.ts`, `wellBg`/`wellFg` indirection), `TerminalTab` `__VOX_E2E_REAL_XTERM__`, zero-leak sweep across `Console/*` & `ChatTerminalBlockCard.tsx`, and `900×600` responsive `DiscoveryRail` + `ConsoleSplitGrid` (`TERM-ROMA-CV-02`)
- [x] 446-03-PLAN.md — Deterministic terminal CV invariants (`terminalAudits.ts` + `terminalAudits.test.ts` + `cvFlywheel.ts` + `audits.ts`), daylight-aware Dual-Surface `vlmVisionFlywheel.ts`, and 8-state `terminalRichFixtures.ts` + `tauriMockRich.ts` + `states.ts` (`TERM-ROMA-CV-03`)
- [x] 446-04-PLAN.md — Multi-pass 5-viewport × 4-theme `VOX_CV_FLYWHEEL=1` `CV_STAGE=before -> after` convergence (`terminal-cv-flywheel.spec.ts`, `visualAfterScore >= 0.92`) and `warp-roman-terminal-v75-research-2026.md` promotion to `status: "current"` (`TERM-ROMA-CV-03`, `TERM-ROMA-CV-04`)

### Phase 447: Repo-Wide Build Contention Elimination & Broker Coalescing

**Directory**: `.planning/workstreams/axis-warp-terminal-v75/phases/447-repo-wide-build-contention-elimination-target-aware-broker-c/`
**Goal**: Eliminate repo-wide multi-agent `target/.cargo-lock` contention, sleeping-waiter broker slot starvation, 180k-LoC `vox-orchestrator`/`vox-compiler` cascade recompiles, feature-set ping-pong thrashing, and premature `timeout` `exit 124` kills across `vox-cargo-shim`, `vox-build-queue`, `vox-orchestrator`, `vox-terminal-core`, and agent loop guards.
**Depends on**: Independent (repo-wide build infrastructure; planned in parallel with Phase 442 and executable before or alongside Phases 443–446)
**Requirements**: BUILD-CONTENTION-01, BUILD-CONTENTION-02, BUILD-CONTENTION-03, BUILD-CONTENTION-04
**Success Criteria** (what must be TRUE):

  1. `vox-cargo-shim` acquires per-target `FairQueue` ordering before claiming a global machine slot, coalesces identical in-flight `check`/`build`/`test` invocations so followers share the leader's exit status and output without re-running Cargo, and splits `target_wait_ms` / `slot_wait_ms` / `compile_ms` truthfully in `vox-broker stats` and `broker.log`.
  2. `cargo test -p vox-orchestrator --no-default-features` compiles and passes (`victory.rs` and `dei_dispatch.rs` properly `#[cfg(feature = "toestub-gate")]`-guarded), and `vox-terminal-core` gates `vox-compiler` behind `vox-lang` while depending on `vox-orchestrator` with `default-features = false` (matching `vox-orchestrator-mcp`).
  3. `vox-cargo-shim` auto-enables `RUSTC_WRAPPER=sccache` whenever `sccache` is on `PATH` and `RUSTC_WRAPPER` is unset, supports opt-in per-workstream `CARGO_TARGET_DIR` isolation (`VOX_ISOLATED_TARGET` / `VOX_WORKSTREAM`) when `use_worktrees: false`, and updates `dev-loop-guard` / `.agents/rules/00-core.md` to batch verification per wave.
  4. `VOX_BROKER_RUN_TIMEOUT_SECS` enforces build/test timeouts strictly against active compile/run duration *after* queue and target-lock acquisition, preventing `exit 124` kills at `98%` progress.

**Plans**: 3 plans

Plans:

- [x] 447-01-PLAN.md — Target-first `FairQueue`, Windows-safe flight coalescing, split `MetricRecord` telemetry, dynamic `sccache` + workstream isolation, and post-queue `VOX_BROKER_RUN_TIMEOUT_SECS` (`BUILD-CONTENTION-01`, `BUILD-CONTENTION-03`, `BUILD-CONTENTION-04`)
- [x] 447-02-PLAN.md — `vox-orchestrator` `--no-default-features` / `--features runtime` guards + fail-closed `VOX_ROUTE_*` fallback, and `vox-terminal-core` `vox-lang` decoupling + `vox-orchestrator` fingerprint unification (`BUILD-CONTENTION-02`)
- [x] 447-03-PLAN.md — `dev_loop_guard` `sccache`/workstream/batching rules, `.agents/cargo-safety.md`, `build-broker-usage.md` KB SSOT, and `broker-install.vox` / `broker-ci.vox` (`BUILD-CONTENTION-03`, `BUILD-CONTENTION-04`)
