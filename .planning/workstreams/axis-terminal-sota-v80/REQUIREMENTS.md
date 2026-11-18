# Requirements: Vox — Milestone v8.0 (`axis-terminal-sota-v80`)

**Defined:** 2026-11-17
**Workstream:** `.planning/workstreams/axis-terminal-sota-v80/`
**Research SSOT:** [`docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md`](../../../docs/src/architecture/terminal-surfaces-audit-and-benchmarks-research-2026.md)
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree or federated sibling repositories with zero LLM split-brain, while remaining **100% functional offline/standalone** with zero dependency on `vox-ops` and **100% transparent, disableable, strongly-typed telemetry**.

## v8.0 Requirements

Requirements for Milestone `v8.0` (*Non-Blocking Cross-Platform PTY, Stateful Nushell & Persistent VoxLang REPL, Nov 2026 Terminal UX, Spoke 9 Agentic Terminal & Competitor Benchmark Harness*). Every requirement maps to one phase in `ROADMAP.md` (`Phases 471–476`, `28/28` mapped).

### Async Non-Blocking Performance & Coalesced Streaming (`TERM-PERF`)

- [x] **TERM-PERF-01**: Replace synchronous `Condvar::wait_timeout` (`5,000 ms`) in `LocalHost::submit_with_origin` (`crates/vox-terminal-core/src/host.rs`) with an async `tokio::sync::Notify` / bounded non-blocking submission model so neither `vox-gui` nor `vox-orchestrator-d` ever stalls a Tokio worker thread.
- [x] **TERM-PERF-02**: Replace fixed `20 ms` sleep polling (`POLL_STEP_MS`) in `exec_agent_command` (`crates/vox-terminal-core/src/agent_exec.rs`) with `tokio::select!` driven by `Notify` events on `OSC 633;D` close, `feed`, and `cancel`, cutting trivial agent command overhead below `8 ms`.
- [x] **TERM-PERF-03**: Add `8 ms` (`120 Hz`) / `32 KiB` output frame coalescing in `vox-gui/src/commands/pty/warp.rs` (`spawn_term_forwarder`) and eliminate redundant fallback `termSessionAttach` round-trips on submit (`TerminalSessionPane.tsx`), holding sustained WebGL2 throughput `>= 85 MiB/s` with zero WebView frame freezes.
- [x] **TERM-PERF-04**: Eliminate speculative `vox-compiler` parsing on bare shell commands when `shellMode` is `'default'`/`'shell'` (`intentRouter.ts` + `input.rs` + `host.rs`), reserving full Vox compilation for explicit Vox syntax or locked `'vox'` mode.

### Cross-Platform Shell & Session Parity (`TERM-XPLAT`)

- [x] **TERM-XPLAT-01**: Thread live session `cwd` and exported session environment variables into any out-of-PTY subprocess capture (`run_shell_capture`, `run_nu_structured_capture` in `crates/vox-terminal-core/src/pty.rs`) so fallback captures never run in the wrong daemon directory.
- [x] **TERM-XPLAT-02**: Make `format_agent_command_for_shell` (`crates/vox-terminal-core/src/agent_exec.rs`) syntax-aware across raw `pwsh`, `powershell`, `nu`, `cmd`, and POSIX shells when `osc633_capable == false` (`$env:PAGER='cat'` on `pwsh`/`nu`, `set ...` on `cmd`, env prefix on POSIX).
- [x] **TERM-XPLAT-03**: Wire headless `vox-term` `run_plain()` (`crates/vox-term/src/app.rs`) and `input::classify` (`crates/vox-terminal-core/src/input.rs`) so piped/dumb-terminal runs dispatch commands via `SessionHost` and POSIX paths (`/usr/bin/env`) match `intentRouter.ts` `SLASH_VERB_RE` semantics.
- [x] **TERM-XPLAT-04**: Add cross-platform PTY integration tests covering macOS/Linux (`zsh`, `bash`, `fish`, `nu`) and Windows ConPTY (`pwsh`) launch shims, exit-code capture, `TIOCSWINSZ`/ConPTY resize, and non-blocking `SIGINT` cancellation.

### Stateful Nushell & Native Structured Fallback (`TERM-NU`)

- [x] **TERM-NU-01**: Wire `detect_ai_shell()` (`crates/vox-terminal-core/src/pty.rs`) behind explicit session profile/config (`prefer_nushell`) or shell discovery rather than leaving it unreferenced while `LocalHost::open_with_cwd` always calls `default_shell()`.
- [x] **TERM-NU-02**: Route `!nu <pipeline>` / `shellMode = 'nu'` (`InputIntent::StructuredShell`) through the live stateful PTY when the session hosts `nu` (or sync session `cwd`/env into `nu -c`), preserving `cd` state, env variables, and live output streaming.
- [x] **TERM-NU-03**: Implement a native Vox structured output fallback adapter (`vox shell json` / in-process structured serialization) when host `nu` is absent on `PATH`, avoiding broken `sh -c` execution of Nushell operators (`where`, `get`, `to json`) while still populating `JsonLedgerTable`.
- [x] **TERM-NU-04**: Implement or cleanly contract the `ShellKind::Nushell` backend seam (`crates/vox-terminal-core/src/pty.rs`) so no unimplemented stub comment (`"Track 6"`) drifts from shipped reality.

### Persistent Multi-Type VoxLang REPL & Unified Commands (`TERM-VOX`)

- [x] **TERM-VOX-01**: Replace the hardcoded `fn main() -> Int { ... }` wrapper (`host.rs`, `session.rs`) with smart statement/expression normalization supporting `Str`, `Bool`, `Float`, `List`, `Map`, `Unit`, `print(...)`, and multi-statement blocks.
- [x] **TERM-VOX-02**: Add a persistent per-session `VoxReplState` in `vox-terminal-core::vox_interp` preserving `let` bindings, `fn`/`type`/`import` declarations across terminal blocks, with explicit `SessionStateReset` on `/reset` or close.
- [x] **TERM-VOX-03**: Map `VoxReplState` interpreter capabilities and working directory from the session's `TermSecurityLevel` (`L1..L5`) and live `cwd`, enabling safe `std.fs`, `std.process`, `std.path`, `std.frame`, and `std.plot` usage inside Vox terminal blocks.
- [x] **TERM-VOX-04**: Unify `vox-lang` feature availability and live `/model`, `/budget`, `/skills`, `/memory`, and `/context` slash-command parity across `vox-terminal-core`, `vox-term`, `vox-orchestrator-d`, and `vox-gui` (removing static stub responses).

### Modern Nov 2026 Terminal UX & Protocol Parity (`TERM-UX26`)

- [x] **TERM-UX26-01**: Parse and render explicit `OSC 8` (`\x1b]8;;uri\x07label\x1b]8;;\x07`) hyperlinks in `sgrToSpans.ts` and `@xterm` links rather than stripping all OSC sequences (`OSC_RE`), preserving clickable file/symbol/URL targets alongside `file:line:col` regex links.
- [x] **TERM-UX26-02**: Support safe `OSC 52` clipboard integration (bounded/opt-in policy), Kitty Keyboard Protocol (`CSI u`) forwarding in `vox-term/src/app.rs` + `TerminalTab.tsx`, and bracketed paste (`?2004h`) consistency across Raw VT and `ScriptoriumPromptEditor`.
- [x] **TERM-UX26-03**: Render per-block execution telemetry chips in `BlockCard.tsx` (duration, exit badge, child `rusage` CPU/RSS summary when emitted) plus rich inline `std.frame` table and `std.plot`/SVG preview cards alongside `JsonLedgerTable`.
- [x] **TERM-UX26-04**: Bring fish/Warp-style inline ghost-text autosuggestions (`HistoryStore` + `CompletionEngine` frequency/exit ranking) and structured block navigation to `vox-term` (`InputBox`) in parity with Axis `ScriptoriumPromptEditor`.

### Spoke 9 (`AgenticTerminal`) & Dual-Tier Spoke 3 Verifier (`TERM-SPOKE`)

- [x] **TERM-SPOKE-01**: Add `OrganicSpokeKind::AgenticTerminal` (`"agentic_terminal"`) in `crates/vox-corpus/src/training/contract.rs`, mapping `to_core_spoke() -> SpokeKind::StructuredShell` to preserve the canonical 5-spoke core schema invariant.
- [x] **TERM-SPOKE-02**: Upgrade `crates/vox-corpus/src/training/spoke_roundtrip/verifiers_shell_mcp.rs` with Tier-3B `verify_terminal_session_trajectory` validating multi-step `vox_terminal_{exec,read,send_input,sessions}` traces (interactive `AwaitingInput`, `CREDENTIAL_PROMPT_USER_ONLY` safe handoff, `cwd` statefulness, and `@term:session#block:L...` citations) against `LocalHost`.
- [x] **TERM-SPOKE-03**: Wire `OrganicSpokeKind::AgenticTerminal` into MENS corpus balance/generation metrics, `vox harness eval`, and multi-turn terminal safety-floor oracle checks (`ExecPolicyV1` `L1..L5`).
- [x] **TERM-SPOKE-04**: Add synthetic + live terminal agent trajectory fixtures in `contracts/eval/` and verify zero secret/ANSI leakage through 3-stage backend scrubbing.

### Competitor Benchmark Suite & E2E Visual/Harness Verification (`TERM-BENCH`)

- [x] **TERM-BENCH-01**: Implement `vox bench terminal` (`contracts/reports/terminal-bench.v1.json`) measuring `input_to_frame_p50_ms` / `p99_ms`, `pty_scroll_throughput_mb_s`, `alt_screen_switch_ms`, `agent_exec_roundtrip_p50_ms`, `tokio_worker_stall_max_ms`, `osc633_block_accuracy_pct`, and `session_rss_kb_at_1k_blocks`.
- [x] **TERM-BENCH-02**: Document reproducible competitor comparison harnesses and baseline numbers (Ghostty 1.1+, Warp 2026, WezTerm, Kitty, Alacritty, Zed / VS Code Integrated Terminal) on matching macOS/Windows hardware.
- [x] **TERM-BENCH-03**: Add end-to-end Playwright + Rust harness tests verifying `10 MiB` SGR flood non-blocking behavior, stateful `!nu` + persistent `.vox` blocks, `OSC 8` links, `CSI u` keys, and `Autopilot -> Manual` preemption `< 50 ms`.
- [x] **TERM-BENCH-04**: Run Before/After 4-theme Playwright + CV Flywheel convergence across `Console` and `ChatSurface` Dock (including new `std.frame`/`std.plot` cards, telemetry chips, and `OSC 8` links) with `axeViolations === 0` and zero contrast regressions.
