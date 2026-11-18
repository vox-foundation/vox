# Roadmap: Milestone v8.0 (`axis-terminal-sota-v80`) — Non-Blocking Cross-Platform PTY, Stateful Nushell & Persistent VoxLang REPL, Nov 2026 UX, Spoke 9 Agentic Terminal & Competitor Benchmarks

**Workstream:** `.planning/workstreams/axis-terminal-sota-v80/`
**Research SSOT:** [`docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md`](../../../docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md)
**Scope:** Eliminate `LocalHost` Tokio blocking (`5,000 ms` `Condvar`) and WebView IPC burst saturation across all five Vox terminal surfaces, achieve stateful `cwd`/env parity for Nushell (`!nu`) and cross-platform subprocess fallbacks, replace single-shot `fn main() -> Int` evaluation with a stateful capability-scoped `VoxLang` REPL + live slash commands, ship Nov 2026 SOTA protocol/UX parity (`OSC 8`, `OSC 52`, Kitty `CSI u`, per-block `rusage`, inline `std.frame`/`std.plot`, `vox-term` autosuggestions), add `OrganicSpokeKind::AgenticTerminal` (Spoke 9) + Tier-3B stateful PTY trajectory verification, and prove performance via `vox bench terminal` + 4-theme Playwright/CV Flywheel across `Phases 471–476` (`28/28` requirements). Zero `layers.toml` / `crate-edges` inversions and 100% model-agnostic LLM routing via `vox_actor_runtime::llm`.

## Phase Checklist

- [x] **Phase 471: Async `Notify` Non-Blocking PTY Host, `120 Hz` IPC Frame Coalescing & Cross-Platform Shell/CWD Parity** (`TERM-PERF-01..04`, `TERM-XPLAT-01..04`) — completed 2026-11-17
- [x] **Phase 472: Stateful PTY Nushell Execution, `detect_ai_shell` Profile Routing & Native Structured Pipeline Fallback** (`TERM-NU-01..04`) — completed 2026-11-17
- [x] **Phase 473: Persistent Multi-Type `VoxLang` REPL Session State, Tier-Mapped Capabilities & Unified Slash Commands** (`TERM-VOX-01..04`) — completed 2026-11-17
- [x] **Phase 474: Nov 2026 SOTA Terminal UX: `OSC 8` / `OSC 52` / Kitty `CSI u`, Per-Block Telemetry, `std.frame`/`std.plot` & `vox-term` Ghost Text** (`TERM-UX26-01..04`) — completed 2026-11-17
- [x] **Phase 475: `OrganicSpokeKind::AgenticTerminal` (Spoke 9) & Tier-3B Stateful PTY Trajectory Round-Trip Verifier** (`TERM-SPOKE-01..04`) — completed 2026-11-17
- [x] **Phase 476: `vox bench terminal` Competitor Benchmark Suite (`terminal-bench.v1.json`) & 4-Theme Playwright/CV Terminal Verification** (`TERM-BENCH-01..04`) — completed 2026-11-17

---

### Phase 471: Async `Notify` Non-Blocking PTY Host, `120 Hz` IPC Frame Coalescing & Cross-Platform Shell/CWD Parity

**Directory**: `.planning/workstreams/axis-terminal-sota-v80/phases/471-async-pty-host-coalescing-and-xplat/`
**Goal**: Eliminate synchronous `Condvar` stalling on Tokio worker threads, replace fixed `20 ms` agent polling with `tokio::sync::Notify`, batch PTY output frames at `8 ms` / `32 KiB`, fix speculative Vox compilation on bare shell inputs, and restore cross-platform `cwd`/env/pager/POSIX-path parity.
**Depends on**: None
**Requirements**: TERM-PERF-01, TERM-PERF-02, TERM-PERF-03, TERM-PERF-04, TERM-XPLAT-01, TERM-XPLAT-02, TERM-XPLAT-03, TERM-XPLAT-04
**Success Criteria** (what must be TRUE):

  1. `LocalHost::submit_with_origin` and `exec_agent_command` await `tokio::sync::Notify` cleanly without calling blocking `std::sync::Condvar::wait_timeout` on Tokio worker threads (`tokio_worker_stall_max_ms < 2.0 ms` during a `sleep 2` submission).
  2. `spawn_term_forwarder` (`vox-gui/src/commands/pty/warp.rs`) coalesces high-rate PTY reads (`8 ms` / `32 KiB` windows) without stalling `10 MiB` SGR floods, and `ScriptoriumPromptEditor` (`intentRouter.ts`) + `input::classify` avoid speculative `vox-compiler` parsing on bare shell commands or POSIX paths (`/usr/bin/env`).
  3. `run_shell_capture`, `run_nu_structured_capture`, and `format_agent_command_for_shell` preserve session `cwd`/env and emit native Windows `pwsh`/`nu` vs. `cmd` vs. POSIX pager syntax when `osc633_capable == false`.
  4. Headless `vox-term` `run_plain()` dispatches piped intents through `SessionHost`, and cross-shell tests pass for Unix PTYs and Windows ConPTY.

**Plans**: 1/1 complete

### Phase 472: Stateful PTY Nushell Execution, `detect_ai_shell` Profile Routing & Native Structured Pipeline Fallback

**Directory**: `.planning/workstreams/axis-terminal-sota-v80/phases/472-stateful-nushell-and-structured-fallback/`
**Goal**: Make Nushell a first-class stateful citizen across sessions by activating `detect_ai_shell` behind shell configuration, executing `!nu` pipelines with persistent `cwd`/env in live sessions, and providing a clean native structured fallback when `nu` is absent on `PATH`.
**Depends on**: Phase 471
**Requirements**: TERM-NU-01, TERM-NU-02, TERM-NU-03, TERM-NU-04
**Success Criteria** (what must be TRUE):

  1. `LocalHost::open_with_cwd` honors explicit/configured AI shell preferences via `detect_ai_shell()` (`nu` when available on `PATH`, falling back cleanly to platform default).
  2. Submitting `!nu <pipeline>` / `/nu <pipeline>` or locked `nu` mode runs stateful against the session PTY when `nu` is the active shell, or inherits session `cwd` and env vars when run via daemon subprocess capture, without blocking Tokio workers.
  3. On hosts without `nu` installed, structured pipelines route through an explicit native structured adapter or fail loudly with actionable diagnostics instead of silently passing Nushell syntax (`where`, `to json`) into `/bin/sh`.
  4. `ShellKind::Nushell` (`pty.rs`) is cleanly resolved (either implemented or explicitly contracted without stale `"Track 6"` stub drift).

**Plans**: 1/1 complete

### Phase 473: Persistent Multi-Type `VoxLang` REPL Session State, Tier-Mapped Capabilities & Unified Slash Commands

**Directory**: `.planning/workstreams/axis-terminal-sota-v80/phases/473-persistent-voxlang-repl-and-slash-parity/`
**Goal**: Upgrade terminal `.vox` execution from a stateless `fn main() -> Int` one-liner into a true persistent multi-type Vox REPL scoped to session `TermSecurityLevel` + `cwd`, with `vox-lang` enabled and live slash-command parity across TUI, daemon, and GUI.
**Depends on**: Phase 471
**Requirements**: TERM-VOX-01, TERM-VOX-02, TERM-VOX-03, TERM-VOX-04
**Success Criteria** (what must be TRUE):

  1. Terminal `.vox` blocks evaluate `Str`, `Bool`, `Float`, `List`, `Map`, `Unit`, `print(...)` output, and multi-statement snippets cleanly without requiring `fn main() -> Int`.
  2. Variable bindings (`let x = ...`), functions, and imports persist across sequential `VoxNative` blocks within the same terminal session (`VoxReplState`), isolated per session ID.
  3. Interpreter `CapabilitySet` and workspace root derive cleanly from the session's `TermSecurityLevel` (`L1..L5`) and tracked `cwd`, enabling safe `std.fs`/`std.process`/`std.path`/`std.frame`/`std.plot` execution.
  4. `vox-term`, `vox-orchestrator-d`, and `vox-gui` share `vox-lang` parity and route `/model`, `/budget`, `/skills`, `/memory`, and `/context` to live daemon/workspace state instead of static stubs.

**Plans**: 1/1 complete

### Phase 474: Nov 2026 SOTA Terminal UX: `OSC 8` / `OSC 52` / Kitty `CSI u`, Per-Block Telemetry, `std.frame`/`std.plot` & `vox-term` Ghost Text

**Directory**: `.planning/workstreams/axis-terminal-sota-v80/phases/474-nov2026-ux-protocols-and-rich-blocks/`
**Goal**: Close every Nov 2026 modern terminal UX/protocol gap across both Axis `Console` and standalone `vox-term`: clickable `OSC 8` hyperlinks, policy-gated `OSC 52` clipboard, Kitty `CSI u` key encoding, per-block `rusage`/duration chips, inline `std.frame`/`std.plot` cards, and TUI ghost-text autosuggestions.
**Depends on**: Phase 473
**Requirements**: TERM-UX26-01, TERM-UX26-02, TERM-UX26-03, TERM-UX26-04
**Success Criteria** (what must be TRUE):

  1. `sgrToSpans.ts` and `@xterm` parse explicit `OSC 8` (`\x1b]8;;...`) hyperlinks with custom labels and workspace locator dispatch alongside `file:line:col` links.
  2. Kitty Keyboard Protocol (`CSI u`) dispatches cleanly in `vox-term` (`key_event_to_vt_bytes`) and WebView raw VT mode, alongside safe `OSC 52` clipboard handling and bracketed paste (`?2004h`).
  3. `BlockCard.tsx` displays child telemetry badges (`duration_ms`, exit seal, optional `rusage` CPU/RSS) and renders inline `std.frame` tables + `std.plot`/SVG cards cleanly across all 4 themes.
  4. `vox-term` `InputBox` surfaces inline dimmed ghost-text autosuggestions (`Right`/`Tab` accept) backed by `HistoryStore` + `CompletionEngine`.

**Plans**: 1/1 complete

### Phase 475: `OrganicSpokeKind::AgenticTerminal` (Spoke 9) & Tier-3B Stateful PTY Trajectory Round-Trip Verifier

**Directory**: `.planning/workstreams/axis-terminal-sota-v80/phases/475-agentic-terminal-spoke-and-verifier/`
**Goal**: Preserve the canonical 5-spoke core while adding `OrganicSpokeKind::AgenticTerminal` (Spoke 9) and a deterministic Tier-3B multi-turn PTY trajectory verifier for training, evaluating, and hardening agent terminal driving.
**Depends on**: Phase 471, Phase 472, Phase 473
**Requirements**: TERM-SPOKE-01, TERM-SPOKE-02, TERM-SPOKE-03, TERM-SPOKE-04
**Success Criteria** (what must be TRUE):

  1. `OrganicSpokeKind::AgenticTerminal` is registered in `crates/vox-corpus/src/training/contract.rs` with `to_core_spoke() -> SpokeKind::StructuredShell`, breaking zero 5-spoke schema invariants.
  2. `verify_terminal_session_trajectory` (`verifiers_shell_mcp.rs`) executes and scores multi-turn `vox_terminal_{exec,read,send_input,sessions}` sequences over `LocalHost`, verifying `AwaitingInput` recovery, `CREDENTIAL_PROMPT_USER_ONLY` yielding, persistent `cwd` state, `.vox` REPL blocks, and `ExecPolicyV1` tier safety.
  3. `vox corpus` metrics and `vox harness eval` include Spoke 9 (`agentic_terminal`) balance and golden trajectory assertions in `contracts/eval/`.
  4. End-to-end 3-stage scrubbing (`strip_ansi_and_normalize` -> Clavis -> `vox-redact`) passes zero-leakage tests across every Tier-3B trace.

**Plans**: 1/1 complete

### Phase 476: `vox bench terminal` Competitor Benchmark Suite (`terminal-bench.v1.json`) & 4-Theme Playwright/CV Terminal Verification

**Directory**: `.planning/workstreams/axis-terminal-sota-v80/phases/476-competitor-benchmarks-and-cv-flywheel/`
**Goal**: Quantify terminal latency, throughput, `OSC 633` accuracy, memory footprint, and Tokio responsiveness against Nov 2026 competitors via `vox bench terminal`, and prove zero UI/accessibility regressions across 4 themes via Playwright + CV Flywheel.
**Depends on**: Phase 474, Phase 475
**Requirements**: TERM-BENCH-01, TERM-BENCH-02, TERM-BENCH-03, TERM-BENCH-04
**Success Criteria** (what must be TRUE):

  1. `vox bench terminal` generates `contracts/reports/terminal-bench.v1.json` covering all 7 benchmark metrics (`input_to_frame_p50_ms`, `pty_scroll_throughput_mb_s`, `alt_screen_switch_ms`, `agent_exec_roundtrip_p50_ms`, `tokio_worker_stall_max_ms`, `osc633_block_accuracy_pct`, `session_rss_kb_at_1k_blocks`) with reproducible competitor comparison tables (**Ghostty 1.1+**, **Warp 2026**, **WezTerm**, **Kitty**, **Alacritty**, **Zed / VS Code**).
  2. All performance thresholds meet Milestone `v8.0` SLAs (`tokio_worker_stall_max_ms < 2.0 ms`, `alt_screen_switch_ms < 12.0 ms`, `agent_exec_roundtrip_p50_ms < 8.0 ms`, `pty_scroll_throughput_mb_s >= 85 MiB/s` VT).
  3. Playwright + Rust E2E suite verifies `10 MiB` SGR non-blocking bursts, stateful `!nu` + `.vox` REPL persistence, `OSC 8` links, `CSI u`, and `Autopilot -> Manual` preemption `< 50 ms`.
  4. 4-theme Playwright + VLM CV Flywheel passes across all affected `Console` and `Chat` Dock surfaces with `axeViolations === 0` and zero contrast leaks.

**Plans**: 1/1 complete
