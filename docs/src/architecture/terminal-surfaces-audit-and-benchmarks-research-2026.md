---
title: "Vox Terminal Surfaces Audit, Spoke Architecture & Nov 2026 Competitor Benchmarks"
description: "Comprehensive codebase-grounded audit of all five Vox terminal surfaces covering cross-platform PTY fidelity, Nushell and VoxLang integration gaps, Nov 2026 terminal UX parity, Tokio/IPC performance bottlenecks, MENS Spoke 9 architecture, and competitor benchmarks."
category: "Architecture SSOTs"
status: "research"
topic: "terminal-ui"
domain: "gui"
authorship: "llm-generated"
authorship_model: "gemini-3-1-pro"
training_eligible: false
---

# Vox Terminal Surfaces Audit, Spoke Architecture & Nov 2026 Competitor Benchmarks

<!-- kb:summary -->
This research SSOT audits all five Vox terminal surfaces (`vox-terminal-core`, `vox-term`, `vox-orchestrator-d`, Axis GUI `Console` + Chat Dock, and `vox shell` / `vox repl` CLI) following Milestone `v7.5`. It evaluates cross-platform PTY behavior, real Nushell and VoxLang integration fidelity, Nov 2026 SOTA UX gaps, Tokio/IPC bottlenecks in `LocalHost`, whether terminal driving warrants expanding `SpokeKind::StructuredShell` plus `OrganicSpokeKind::AgenticTerminal`, and defines an empirical `vox bench terminal` harness against Ghostty, Warp, WezTerm, Kitty, Alacritty, and VS Code/Cursor.
<!-- /kb:summary -->

## 1. Surface Inventory (5 Surfaces)

| Surface | Key Crates & Files | Current Role | Runtime Host Seam |
|---|---|---|---|
| Core PTY & Session Engine | `crates/vox-terminal-core/src/{host,agent_exec,session,pty,shell_init,vox_interp,commands,stall_detector,vt_shadow}.rs` | Stateful `LocalHost` / `DaemonHost`, `OSC 633`/`OSC 7` framing, replay ring (`2 MiB`), `ExecPolicyV1` gating, inline `.vox` eval | `Arc<dyn SessionHost>` |
| Standalone TUI (`vox term`) | `crates/vox-term/src/app.rs`, `crates/vox-term/Cargo.toml` | `ratatui` + `crossterm` TUI supporting `--attach [latest\|id]` and `--local` | `resolve_session_host()` |
| Daemon Control Plane & MCP Bridge | `crates/vox-orchestrator-d/src/{term_dispatch,term_bridge_impl}.rs`, `crates/vox-orchestrator-d/Cargo.toml` | JSON-RPC `term.*` methods + `LocalHostTerminalBridge` for `vox_terminal_{exec,read,send_input,sessions}` with 3-stage scrubbing | Shared `Arc<LocalHost>` |
| Axis Desktop GUI (`Console` & Chat Dock) | `crates/vox-gui/src/commands/pty/{mod,warp,slash,approvals}.rs`, `crates/vox-gui/ui/src/components/surfaces/Console/{Console,TerminalSessionPane,TerminalTab,BlockCard,BlockStreamView,ScriptoriumPromptEditor,intentRouter,sgrToSpans,xtermAddons}.*` | WebGL2 `@xterm/xterm` + virtualized block stream, split panes, Co-Pilot wax-seal approvals, Chat bottom dock & `ChatTerminalBlockCard` | `PtyManager` -> `DaemonHost` / `LocalHost` fallback |
| CLI Shell Policy & REPL | `crates/vox-cli/src/commands/runtime/shell/{mod,check_terminal}.rs`, `crates/vox-cli/src/commands/repl.rs` | `vox shell check` AST/grammar validator over `contracts/terminal/exec-policy.v1.yaml`; dev micro-REPL | Subprocess / local CLI |

---

## 2. Cross-Platform Fidelity & Remaining Gaps

**What works today:**
- `portable-pty` (`native_pty_system()`) in `crates/vox-terminal-core/src/pty.rs` spawns real Unix PTYs (`TIOCSWINSZ` resize) and Windows `ConPTY`.
- `crates/vox-terminal-core/src/shell_init.rs` creates owner-only (`0700` on Unix) `tempfile::TempDir` non-echo launch shims for **five shells** (`zsh`, `bash`, `fish`, `pwsh`/`powershell`, `nu`), sourcing user dotfiles first without dumping hook text into replay or shell history.

**Verified cross-platform gaps:**
1. **One-shot captures ignore session `cwd` and environment (`crates/vox-terminal-core/src/pty.rs:197-221,251-298`):** `run_shell_capture` and `run_nu_structured_capture` call `std::process::Command::new(...)` directly inside the host process's working directory with no `.current_dir(...)` and no exported session env vars (`VIRTUAL_ENV`, `PATH`, etc.).
2. **POSIX pager prefix leaks onto raw Windows shells (`crates/vox-terminal-core/src/agent_exec.rs:178-189`):** `format_agent_command_for_shell` checks `if osc633_capable` else `if shell_name == "cmd"` else prepends POSIX `PAGER=cat GIT_PAGER=cat ... {trimmed}`. If a `pwsh` or `nu` session lacks `osc633_capable`, POSIX inline env assignment syntax fails on Windows `pwsh`/`nu`.
3. **Headless `vox-term` drops input (`crates/vox-term/src/app.rs:276-291`):** Under `TERM=dumb` or piped stdin/stdout, `run_plain()` reads lines and discards them (`let _line = line?;`) instead of dispatching through `SessionHost`.
4. **Input classification ambiguity on POSIX paths (`crates/vox-terminal-core/src/input.rs:27-51`):** `classify("/usr/bin/env")` strips `/` and treats `"usr/bin/env"` as `InputIntent::Command` instead of `Shell`, whereas frontend `intentRouter.ts:12,45` uses `SLASH_VERB_RE = /^\/[a-zA-Z][\w-]*(?:\s|$)/` to exclude POSIX paths.

---

## 3. Nushell Integration Audit

**What works today:**
- `shell_init.rs:225-253` (`build_nu_shim`) and `pty.rs:104-128` (`NU_OSC633`) hook `env.config.hooks.pre_prompt` and `pre_execution` to emit `OSC 633;D/A/E/C/B` + `OSC 7` while preserving user `env.nu` / `config.nu`.
- `completions.rs` supports Nushell subcommands/flags, `diagnose_nu_posix_isms` (`pty.rs:224-247`) flags POSIX `&&`/`||`/`2>&1`/`$HOME` mistakes, and `BlockCard.tsx` + `JsonLedgerTable.tsx` render JSON output as sortable/filterable tables.

**Critical gaps (Nushell is mostly subprocess-wrapped, not native or stateful):**
1. **`detect_ai_shell()` is dead in production (`pty.rs:53-62` vs. `agent_exec.rs:193-196`):** `detect_ai_shell()` checks `nu --version` and returns `"nu"`, yet `LocalHost::open_with_cwd` unconditionally calls `default_shell()` (`pwsh` on Windows, `$SHELL` or `bash` on Unix). Opening a session with `shell: None` never auto-selects Nushell even when `nu` is installed.
2. **Unimplemented `ShellKind::Nushell` seam (`pty.rs:7-9,302-317`):** The `nushell` Cargo feature and `NuShell` in-process `ShellBackend` implementation referenced as `"Track 6"` do not exist in `crates/vox-terminal-core/Cargo.toml`.
3. **`!nu` / `shellMode = 'nu'` bypasses the live PTY (`crates/vox-terminal-core/src/host.rs:731-776` & `session.rs:347-363`):** Inside `LocalHost::submit_with_origin`, `InputIntent::StructuredShell(pipeline)` does **not** run inside the live session PTY (even when the PTY *is* running `nu`!). It falls through to the wildcard arm calling `Session::submit_with_origin`, which runs a **blocking, stateless** `nu --no-config-file -c "do { <pipeline> } | to json --raw"` subprocess in the daemon's CWD — losing `cd` directory state, shell variables, streaming output, and non-blocking execution.
4. **Silent fallback degradation when `nu` is absent (`pty.rs:293-297`):** Verified on this macOS host (`which nu` exits `1`): `run_nu_structured_capture` emits `[nu not found on PATH; executed via platform shell fallback]` and runs the Nushell pipeline through `bash`/`zsh`/`pwsh`, which rejects Nushell operators (`where`, `get`, `to json`) and loses structured table output unless Vox supplies an internal structured adapter (`vox shell json` / in-process fallback or auto-bootstrap).

---

## 4. Native Vox (`VoxLang`) Integration Audit

**What works today:**
- `vox_interp::eval_line` (`crates/vox-terminal-core/src/vox_interp.rs:13-40`) runs `vox_compiler::pipeline::run_frontend_str_with_options` with `script_mode: true` and `Interpreter::new(100_000)` when the `vox-lang` Cargo feature is active.
- `vox-gui` (`crates/vox-gui/Cargo.toml:62`) and `vox-orchestrator-d` (`crates/vox-orchestrator-d/Cargo.toml:19`) both compile `vox-terminal-core` with `features = ["vox-lang"]`.

**Critical gaps (not a true stateful Vox REPL or capability-integrated runtime):**
1. **Hardcoded `fn main() -> Int` wrapper (`host.rs:740-744`, `session.rs:391-395`):** Any bare expression without `"fn main"` is wrapped verbatim in `fn main() -> Int { <trimmed> }`. Any expression evaluating to `Str`, `Bool`, `Float`, `List`, `Map`, `Unit`, `print(...)`, or multi-statement bindings fails typechecking.
2. **Stateless per-line interpreter & zero capabilities (`vox_interp.rs:30-38`):** Every submission constructs a brand-new `Interpreter::new(100_000)` with empty capabilities (`CapabilitySet::parse("")`). Variable bindings (`let x = ...`), function definitions, session `cwd`, `std.fs`, `std.process`, `std.frame`, `std.plot`, and `import` declarations cannot persist across blocks or map to the session's `TermSecurityLevel` (`L1..L5`).
3. **Speculative compiler penalty on every bare shell command (`intentRouter.ts:130-136` + `input.rs:53` + `host.rs:733-770`):** In Axis `ScriptoriumPromptEditor`, `'default'` and `'shell'` modes send bare commands (`ls -la`, `cargo check`) **without** a leading `!`. `input::classify` classifies all non-prefixed inputs as `InputIntent::VoxNative`. `LocalHost::submit_with_origin` then wraps every shell command in `fn main() -> Int { <cmd> }`, runs the full `vox-compiler` lexer/parser/typechecker, fails, and only *then* falls back to the PTY — adding compiler latency on every shell command and accidentally intercepting valid Vox expressions (`42`, `1 + 2`) even when `shellMode === 'shell'`.
4. **Feature-gate split-brain (`crates/vox-terminal-core/Cargo.toml:9`, `crates/vox-term/Cargo.toml:9-10`):** Standalone `vox-term` compiled with default features has `vox-lang` disabled, so `VoxNative` falls back to running `.vox` lines in the host POSIX shell unless invoked via daemon `vox-orchestrator-d` or compiled with `--features vox-lang`.
5. **Core slash-command stubs (`crates/vox-terminal-core/src/commands.rs:70-195`):** `/budget` (`"Token budget: unlimited"`), `/skills`, `/memory`, and `/context` in `CommandRegistry::default()` return static instruction strings in `vox-term` / direct `LocalHost` calls unless intercepted by `vox-gui/src/commands/pty/slash.rs`.

---

## 5. Modern Nov 2026 Terminal Niceties Audit

Benchmarking Axis Console + `vox-term` against Nov 2026 peers (**Ghostty 1.1+**, **Warp 2026**, **WezTerm**, **Kitty**, **Alacritty**, **Zed Terminal**):

| Capability | Status in Vox (Nov 2026) | Evidence & Delta vs. Competitors |
|---|---|---|
| `OSC 633` + `OSC 7` Block Framing | **Shipped (Best-in-Class)** | `shell_init.rs` 5-shell `0700` non-echo shims + virtualized `BlockStreamView.tsx` + sticky headers; rivals Warp 2026 |
| WebGL2 + DOM Fallback Renderer | **Shipped** | `@xterm/addon-webgl` (`webglAddon.ts`) + `Fit`, `Search`, `Unicode11`, `WebLinks`, `Serialize` in `TerminalTab.tsx` |
| `<16 ms` Auto-TUI `ALT_SCREEN` Flip | **Shipped** | `vt_shadow.rs` + `scanAltScreenTransition` (`sessionReducer.ts`) auto-switches Blocks <-> Raw VT cleanly |
| Inline Ghost-Text Autosuggestions | **Partial (GUI only)** | `ScriptoriumPromptEditor.tsx:373-375` renders `ghostSuffix` (`Right`/`Tab` accept); missing in `vox-term` TUI (`InputBox`) & lacks live frequency/exit-code ranking |
| Multi-Line Paste Guard & Highlighter | **Shipped (GUI)** | `ScriptoriumPromptEditor.tsx:262-319` detects multi-line paste with banner + `tokenizePromptBuffer` syntax coloring |
| Clickable `file:line:col` & URLs | **Partial (`OSC 8` Stripped)** | `sgrToSpans.ts:30-31` auto-links regex URLs & `file:line:col`, **but line 26 `OSC_RE` strips explicit `\x1b]8;;...` (`OSC 8`) hyperlinks** instead of parsing labels/URIs |
| Per-Block Telemetry Badges | **Partial** | `BlockCard.tsx:66-72` shows duration (`ms`/`s`) & exit code; lacks child CPU/RSS `rusage` badges (`utime`, `stime`, `max_rss_kb`) & failure delta hints |
| Kitty Keyboard Protocol (`CSI u`) | **Missing** | `vox-term/src/app.rs:294-325` (`key_event_to_vt_bytes`) emits legacy VT escapes only; misses unambiguous `Ctrl+Enter`, `Shift+Enter`, `Cmd/Super` chord forwarding (`CSI u`) |
| `OSC 52` Clipboard & Inline Rich Output | **Missing** | `OSC_RE` strips `OSC 52`; `BlockCard.tsx` renders JSON (`JsonLedgerTable`) + plain ANSI only — no `std.frame` Arrow/table schema viewer, `std.plot`/SVG inline card, or Kitty/iTerm2 image protocol |
| Split Panes, Search, Runbooks & Voxify | **Shipped** | `ConsoleSplitGrid.tsx`, `TermSearchBar.tsx`, `runbooks.rs`, and 1-click subshell `Voxify` (`TerminalSessionPane.tsx:391-399`) |

---

## 6. Chat & Harness Integration and Performance Audit

**What works well:**
- Unified `TerminalBridge` (`crates/vox-orchestrator-d/src/term_bridge_impl.rs`) exposes `vox_terminal_{exec,read,send_input,sessions}` to agents with defense-in-depth AST (`ExecPolicyV1`) + `classify_floor` gates, `CREDENTIAL_PROMPT_USER_ONLY` hard guard (`stall_detector.rs`), 3-stage output scrubbing (`scrub_three_stage`), `Autopilot -> Manual` instant preemption (`host.rs:841-856`), and `@term:session#block:Lstart-Lend` chat citations with `ChatTerminalBlockCard`.

**Critical performance bottlenecks found in code inspection:**
1. **Synchronous `Condvar` blocking on Tokio async threads (`crates/vox-terminal-core/src/host.rs:703,725,791`):** `LocalHost::submit_with_origin` blocks synchronously on `entry.settled_cv.wait_timeout(sess, rem)` for up to **5,000 ms** inside `submit_with_origin`, and `SessionHost::submit` (`host.rs`) awaits it without `spawn_blocking` or an async `tokio::sync::Notify`. Long commands (`cargo build`, `sleep 3`) stall a Tokio runtime worker thread in both `vox-gui` and `vox-orchestrator-d`!
2. **Fixed `20 ms` sleep polling in `exec_agent_command` (`crates/vox-terminal-core/src/agent_exec.rs:412`):** The agent execution wait loop polls every `20 ms` (`POLL_STEP_MS`) instead of `tokio::select!` on an async `tokio::sync::Notify` triggered by `OSC 633;D` / `feed`, adding `20–80 ms` avoidable tail latency to every short agent command.
3. **Unbatched per-chunk Tauri IPC emissions (`crates/vox-gui/src/commands/pty/warp.rs:167-177`):** `spawn_term_forwarder` calls `app_handle.emit(TERM_OUTPUT_EVENT, ...)` synchronously for every single read buffer frame without `8 ms` (`120 Hz`) or `32 KiB` frame coalescing, saturating the WebView IPC bridge during `cat`/compiler bursts.
4. **Sequential JSON-RPC `term.attach` polling on fallback (`TerminalSessionPane.tsx:270-275`):** If sequence notification lags slightly on submit, the GUI triggers an extra full `termSessionAttach` round-trip.

---

## 7. Spoke Architecture Decision: Should Terminal Driving Have a Separate Spoke?

In `crates/vox-corpus/src/training/contract.rs:61-149`, Vox enforces a **5-spoke canonical core** (`SpokeKind::{VoxLang, RustWorkspace, StructuredShell, McpToolChain, AgentHarness}`) plus an **8-spoke extended organic taxonomy** (`OrganicSpokeKind`, which adds `SpeechToCode`, `PopuliMesh`, and `SkillCreation` mapping onto the 5 core admission lanes via `to_core_spoke()`).

Today, Spoke 3 (`SpokeKind::StructuredShell`) is verified in `crates/vox-corpus/src/training/spoke_roundtrip/verifiers_shell_mcp.rs:36-159` exclusively via `verify_shell_roundtrip` — a **single-line stateless syntax + regex check** (`!has_top_level_shell_chaining` + `classify_shell_safety_floor` + substring presence of `Get-ChildItem`/`!nu`/`cargo `).

**Architectural Recommendation:**
- **Do NOT break the 5-spoke `SpokeKind` core invariant.** Keep `SpokeKind::StructuredShell` as canonical Spoke 3 for storage/SQL schemas and Rust contracts.
- **Upgrade `SpokeKind::StructuredShell` + Add 9th Organic Spoke `OrganicSpokeKind::AgenticTerminal`:**
  1. Split `SpokeKind::StructuredShell` verification into **Tier 3A (Stateless Structured Pipeline)** and **Tier 3B (Stateful Agentic PTY Session Trajectory)** covering multi-turn `vox_terminal_exec` -> `AwaitingInput` (`vox_terminal_send_input` or `CREDENTIAL_PROMPT_USER_ONLY` safe handoff) -> `vox_terminal_read` -> `@term:session#block:L...` grounding and `.vox` REPL interactions over a live/synthetic `LocalHost`.
  2. Add `OrganicSpokeKind::AgenticTerminal` (`"agentic_terminal"`) mapping `to_core_spoke() -> SpokeKind::StructuredShell`, giving `vox corpus` balancing and `vox harness eval` dedicated difficulty-tiered coverage for driving terminals safely and interactively without splitting the 5-spoke core.

---

## 8. Harness Testing & Nov 2026 Competitor Benchmark Design

### 8.1 Automated Harness Verification Matrix
Run deterministically via `vox harness eval --suite terminal-sota` + Playwright WebView harness + headless `LocalHost`:
1. **Cross-Shell `OSC 633` Boundary & CWD Matrix:** Spawn sessions across `zsh`, `bash`, `fish` (when present), `pwsh`, and `nu` -> verify non-zero exit codes, `cwd` preservation across `!nu` & `.vox`, multi-byte UTF-8 box-drawing, and `0` leaked hook echoes.
2. **Blocking / Throughput Stress:** Flood `10 MiB` SGR + UTF-8 output (`seq` / log dump) while measuring Tokio task responsiveness (concurrent `health`/`status` RPC `< 5 ms` p99) and WebView frame drops.
3. **Interactive Stall & Safety-Floor Matrix:** Drive `read`/`confirm [y/N]` (`AwaitingInput` + `vox_terminal_send_input`), `Password:` (`CREDENTIAL_PROMPT_USER_ONLY` agent refusal), `$()`/`-EncodedCommand` policy rejection at `L1`/`L2`, and `Autopilot -> Manual` keystroke preemption `< 50 ms`.
4. **Native `.vox` & Slash State Parity:** Multi-block variable persistence (`let x = 10` in Block 1 -> `x + 32` in Block 2), non-`Int` returns (`Str`, `List`, `Map`, `Unit`), capability scoping matching `L1..L5`, and `/skills` / `/memory` / `/budget` / `/context` live parity in `vox-term` vs. GUI.

### 8.2 Quantitative Competitor Benchmark Suite (`vox bench terminal`)
Write machine-readable JSON reports to `contracts/reports/terminal-bench.v1.json` across standardized workloads (`100k-line SGR cat`, `vtebench-style unicode scroll`, `100-block rapid OSC 633 loop`, `vim/htop ALT_SCREEN toggle`, `agent exec -> read cycle`) and compare against Nov 2026 baselines (**Ghostty 1.1+**, **Warp 2026**, **WezTerm**, **Kitty**, **Alacritty**, **Zed / VS Code Integrated Terminal**):

| Metric Key | Unit | How Measured in Vox Harness | Nov 2026 Target (Best-in-Class Parity / Stretch) |
|---|---|---|---|
| `input_to_frame_p50_ms` / `p99_ms` | `ms` | Synthetic keystroke timestamp -> DOM/VT canvas paint commit (`requestAnimationFrame` + Playwright trace) | `p50 <= 4.0 ms`, `p99 <= 12.0 ms` (vs. Ghostty/Alacritty `2–5 ms`, VS Code/Warp `6–16 ms`) |
| `pty_scroll_throughput_mb_s` | `MiB/s` | Sustained `cat` of `50 MiB` mixed ASCII + 24-bit TrueColor SGR lines with WebGL2 + `8 ms` coalescing | `>= 85 MiB/s` raw VT / `>= 40 MiB/s` Block mode (`>= 2x` Electron/DOM xterm) |
| `alt_screen_switch_ms` | `ms` | Elapsed from `?1049h` PTY byte arrival to VT canvas visible (`p99`) | `< 12.0 ms` (`< 16 ms` single 60 Hz frame budget) |
| `agent_exec_roundtrip_p50_ms` | `ms` | `vox_terminal_exec("true")` via `TerminalBridge` (`Notify`-woken vs. legacy `20 ms` poll) | `< 8.0 ms` overhead above shell fork+hook cost |
| `tokio_worker_stall_max_ms` | `ms` | Max heartbeat timer jitter on Tokio executor during `sleep 2` PTY command submission | `< 2.0 ms` (eliminating current `5,000 ms` `Condvar` freeze) |
| `osc633_block_accuracy_pct` | `%` | Exact command/exit/cwd/branch attribution across 1,000 rapid/escaped/multi-line commands | `>= 99.9%` (`100%` on POSIX + PowerShell + Nushell) |
| `session_rss_kb_at_1k_blocks` | `KiB` | Host + WebView heap/RSS delta across `1,000` blocks (`2 MiB` replay cap, virtualized list) | `<= 45 MiB` total delta per session pane |
