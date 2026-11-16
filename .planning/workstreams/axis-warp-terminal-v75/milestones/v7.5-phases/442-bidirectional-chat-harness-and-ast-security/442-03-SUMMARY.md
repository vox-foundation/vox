---
phase: 442-bidirectional-chat-harness-and-ast-security
plan: 03
subsystem: terminal-harness
tags: [terminal, pty, osc633, pager-guard, stall-detector, terminal-bridge, vox-terminal-core, vox-orchestrator, vox-orchestrator-mcp, vox-orchestrator-d]

requires:
  - phase: 442-01
    provides: ExecPolicyV1, evaluate_command_ast, evaluate_security_tier, extract_effective_commands in vox_orchestrator::oversight::exec_policy
  - phase: 442-02
    provides: BlockStatus::AwaitingInput, Block.{pinned,credential_prompt}, strip_ansi_and_normalize, scrub_ansi_and_redact, truncate_terminal_block, slice_lines_1based, vox_secrets::scrub_resolved_secrets
provides:
  - 5-shell (zsh, bash, fish, pwsh, nu) non-subshell __vox_pager_enter / __vox_pager_restore pager neutralization preserving cwd/env across turns
  - Quiescence + ALT_SCREEN interactive stall classifier (stall_detector::classify_stall, PromptKind, StallSignal, DEFAULT_STALL_QUIESCENCE_MS = 1_500)
  - Stateful PTY agent execution & input handoff (LocalHost::{open_with_cwd, exec_agent_command, agent_send_input, pin_block, promote_bg}) gated by exec_policy and refusing credential prompts (CREDENTIAL_PROMPT_USER_ONLY)
  - SessionEvent::{BlockAwaitingInput, BlockPinned, SecurityLevelChanged, ControlModeChanged} + SessionHost / DaemonHost RPC parity (term.security, term.control, term.pin, term.promote_bg)
  - TerminalBridge trait + OrchDaemonTerminalBridge + EphemeralTerminalBridge in vox_orchestrator::terminal_bridge
  - ServerState interior-mutable terminal_bridge + sticky chat_terminal_sessions binding table (set_terminal_bridge, resolve_terminal_bridge, bind_or_resolve_terminal_session)
  - LocalHostTerminalBridge (3-stage ANSI + vox-redact + Clavis scrubbing) and shared Arc<LocalHost> wiring in vox-orchestrator-d + TermDispatch (term.agent_exec, term.agent_send_input, term.read, term.promote_bg, term.pin, Stage-3 term.context)
affects: [442-04, 442-05, 443, 445, vox-terminal-core, vox-orchestrator, vox-orchestrator-mcp, vox-orchestrator-d, vox-term, vox-gui]

actuals:
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Non-subshell current-shell pager guard (__vox_pager_enter saves/exports 6 pager vars to cat; precmd/prompt __vox_pager_restore restores exact prior state immediately after exit-code capture)"
    - "ALT_SCREEN + 1_500 ms quiescence stall classifier pausing blocks in BlockStatus::AwaitingInput and failing closed on credential_prompt == true (CREDENTIAL_PROMPT_USER_ONLY)"
    - "Single shared Arc<LocalHost> across TermDispatch and ServerState::terminal_bridge inside vox-orchestrator-d on loopback/stdio transports with 3-stage output scrubbing"

key-files:
  created:
    - crates/vox-terminal-core/src/stall_detector.rs
    - crates/vox-terminal-core/src/agent_exec.rs
    - crates/vox-orchestrator/src/terminal_bridge.rs
    - crates/vox-orchestrator-d/src/term_bridge_impl.rs
  modified:
    - crates/vox-terminal-core/src/pty.rs
    - crates/vox-terminal-core/src/shell_init.rs
    - crates/vox-terminal-core/src/lib.rs
    - crates/vox-terminal-core/src/session.rs
    - crates/vox-terminal-core/src/host.rs
    - crates/vox-terminal-core/src/daemon_host.rs
    - crates/vox-orchestrator/src/lib.rs
    - crates/vox-orchestrator-mcp/src/server_state.rs
    - crates/vox-orchestrator-d/src/lib.rs
    - crates/vox-orchestrator-d/src/term_dispatch.rs
    - crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs

key-decisions:
  - "Kept OpenSpec { cols, rows, shell } unchanged for downstream vox-gui/vox-term compatibility and added LocalHost::open_with_cwd(&self, spec: OpenSpec, cwd: Option<&Path>) while accepting optional params.cwd on term.open"
  - "Wrapped ServerState::terminal_bridge, chat_terminal_sessions, and ephemeral_terminal_bridge in Arc<parking_lot::RwLock<...>> across all 3 constructors so set_terminal_bridge after DaemonBootCtx propagates immediately to every pre-existing ServerState clone"
  - "Kept all vox_secrets::scrub_resolved_secrets Stage-3 calls in crates that already depend on vox-secrets (vox-orchestrator, vox-orchestrator-d) so zero Cargo.toml or crate-edges changes were required"

patterns-established:
  - "Agent PTY commands execute via __vox_pager_enter; <cmd> in the live shell rather than a subshell so cd and export persist while git/man/bat/gh/systemd pagers never hang"
  - "User terminal submissions and raw PTY keystrokes automatically demote TermControlMode::Autopilot to TermControlMode::Manual and emit SessionEvent::ControlModeChanged"

requirements-completed:
  - TERM-HARNESS-01
  - TERM-HARNESS-02
  - TERM-HARNESS-03

coverage:
  - id: D1
    description: "5-shell non-subshell __vox_pager_enter/__vox_pager_restore pager neutralization, quiescence/ALT_SCREEN stall detector, and exec_policy-backed agent_exec module"
    requirement: TERM-HARNESS-02
    verification:
      - kind: unit
        ref: "crates/vox-terminal-core/src/shell_init.rs#tests::five_shells_define_non_subshell_pager_enter_and_restore"
        status: pending-cargo-lock
      - kind: unit
        ref: "crates/vox-terminal-core/src/stall_detector.rs#tests"
        status: pending-cargo-lock
    human_judgment: false
  - id: D2
    description: "AST-delegated TermSecurityLevel::allows_agent_command, stateful LocalHost::{exec_agent_command,agent_send_input,pin_block,promote_bg}, SessionEvent/DaemonHost parity, and vox_orchestrator::terminal_bridge"
    requirement: TERM-HARNESS-03
    verification:
      - kind: unit
        ref: "crates/vox-terminal-core/src/agent_exec.rs#tests::live_pty_exec_preserves_state_handles_prompts_and_refuses_credentials"
        status: pending-cargo-lock
      - kind: unit
        ref: "crates/vox-orchestrator/src/terminal_bridge.rs#tests::ephemeral_bridge_persists_cwd_reads_slices_and_enforces_policy"
        status: pending-cargo-lock
    human_judgment: false
  - id: D3
    description: "ServerState interior-mutable terminal_bridge + sticky chat_terminal_sessions, LocalHostTerminalBridge 3-stage scrubbing, TermDispatch RPCs, and single shared Arc<LocalHost> in vox-orchestrator-d"
    requirement: TERM-HARNESS-01
    verification:
      - kind: unit
        ref: "crates/vox-orchestrator-mcp/src/server_state.rs#tests::terminal_bridge_propagates_across_clones_and_binds_sticky_thread_sessions"
        status: pending-cargo-lock
      - kind: integration
        ref: "crates/vox-orchestrator-d/src/term_dispatch.rs#tests::orch_daemon_terminal_bridge_exec_read_pin_and_three_stage_scrub_over_tcp"
        status: pending-cargo-lock
    human_judgment: false

completed: 2026-11-15
status: complete-pending-foreground-cargo-check
---

# Phase 442 Plan 03: Stateful PTY Pager Guard, Stall Detector, `TerminalBridge` & Daemon RPC Wiring Summary

**5-shell non-subshell `__vox_pager_enter`/`__vox_pager_restore` pager guard, `ALT_SCREEN` + `1,500 ms` quiescence stall detector, AST-gated stateful `LocalHost::exec_agent_command` + `TerminalBridge` seam, and single-`Arc<LocalHost>` `vox-orchestrator-d` + `ServerState` wiring with 3-stage secret scrubbing (`TERM-HARNESS-01`, `TERM-HARNESS-02`, `TERM-HARNESS-03`)**

## Performance

- **Tasks:** 3 / 3 implemented & formatted (`rustfmt --edition 2024 --check` clean across all 15 files)
- **Files created/modified:** 15 (`4` created, `11` modified; `0` `Cargo.toml` / `Cargo.lock` changes; `0` new `crate-edges`)
- **Commits:** skipped (shared git working tree & staged index rule)

## Accomplishments

- **Task 1 — 5-Shell Pager Guard, Stall Detector & `agent_exec` Core (`vox-terminal-core`):**
  - Retained `shell_name` on `PtyHandle` (`crates/vox-terminal-core/src/pty.rs`).
  - Added non-subshell `__vox_pager_enter` and `__vox_pager_restore` across all 5 shell shims (`zsh`, `bash`, `fish`, `pwsh`, `nu` in `crates/vox-terminal-core/src/shell_init.rs`) covering `PAGER`, `GIT_PAGER`, `SYSTEMD_PAGER`, `BAT_PAGER`, `MANPAGER`, and `GH_PAGER`, restoring user pager settings immediately after `$?`/`$status`/`$LASTEXITCODE`/`$env.LAST_EXIT_CODE` capture and before `OSC 633;D` emission so `cd`/`export` persist in the current shell while human interactive commands retain their normal pagers.
  - Created `crates/vox-terminal-core/src/stall_detector.rs` (`DEFAULT_STALL_QUIESCENCE_MS = 1_500`, `PromptKind`, `StallSignal`, `classify_stall`) detecting `ALT_SCREEN` immediately and classifying credential (`Password:`, `passphrase`, `[sudo]`, `Enter PIN`, `verification code`, `2FA`/`OTP`), yes/no, press-enter, selection, and interactive prompts only on unterminated non-newline tails after `>= quiescence_ms` silence (never false-firing on newline-terminated `rustc` `error[E0308]:` diagnostics).
  - Created `crates/vox-terminal-core/src/agent_exec.rs` (`AgentExecRequest`, `AgentExecStatus`, `AgentExecOutcome`, `allows_agent_command` delegating to `vox_orchestrator::oversight::exec_policy`, `format_agent_command_for_shell`) and re-exported both modules in `crates/vox-terminal-core/src/lib.rs`.
- **Task 2 — AST Security Delegation, Stateful PTY Execution & `TerminalBridge` (`vox-terminal-core` + `vox-orchestrator`):**
  - Replaced ~230 lines of inline heuristic parsing in `crates/vox-terminal-core/src/host.rs` by delegating `TermSecurityLevel::allows_agent_command` to `crate::agent_exec::allows_agent_command`, shrinking `host.rs` and upgrading `redacted_context_bundle` to `scrub_ansi_and_redact` + `truncate_terminal_block`.
  - Extended `SessionEvent` (`crates/vox-terminal-core/src/session.rs`) with `BlockAwaitingInput`, `BlockPinned`, `SecurityLevelChanged`, and `ControlModeChanged`; wired `Autopilot -> Manual` auto-demotion events on user submissions/keystrokes; and extended `SessionHost` (`LocalHost` + `DaemonHost`) with `set_security_level`, `set_control_mode`, `pin_block`, `promote_bg`, and `route()` decoding for `"block_awaiting_input"`, `"block_pinned"`, `"security_changed"`, and `"control_changed"`.
  - Implemented `LocalHost::{open_with_cwd, exec_agent_command, agent_send_input, pin_block, promote_bg}` in `agent_exec.rs`, enforcing `CREDENTIAL_PROMPT_USER_ONLY` fail-closed rejection when `credential_prompt == true`.
  - Created `crates/vox-orchestrator/src/terminal_bridge.rs` (exported unconditionally via `pub mod terminal_bridge;` in `crates/vox-orchestrator/src/lib.rs`) defining `TerminalBridge`, DTOs (`TermExecRequest`, `TermExecResponse`, `TermSendInputRequest`, `TermReadRequest`, `TermBlockDto`, `TermReadResponse`, `TermSessionSummary`), `OrchDaemonTerminalBridge`, and `EphemeralTerminalBridge` (sticky per-session `cwd` via `__VOX_EPHEM_PWD__:`, 6 `PAGER=cat` env vars, `exec_policy` AST enforcement, and ANSI + `PiiFilter::redact` + `vox_secrets::scrub_resolved_secrets` scrubbing).
- **Task 3 — `ServerState` Bridge Binding, `LocalHostTerminalBridge`, `TermDispatch` & Daemon Binary Wiring (`vox-orchestrator-mcp` + `vox-orchestrator-d`):**
  - Added interior-mutable `terminal_bridge`, `chat_terminal_sessions`, and `ephemeral_terminal_bridge` on `ServerState` (`crates/vox-orchestrator-mcp/src/server_state.rs`) across `new_full`, `new_for_daemon_gated`, and `hermetic_stub`, plus `set_terminal_bridge`, `resolve_terminal_bridge`, and `bind_or_resolve_terminal_session`.
  - Exported `pub mod term_bridge_impl;` and `pub mod term_dispatch;` from `crates/vox-orchestrator-d/src/lib.rs` and removed `#[path = "../term_dispatch.rs"]` from `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs`.
  - Implemented `LocalHostTerminalBridge` (`crates/vox-orchestrator-d/src/term_bridge_impl.rs`) applying all 3 scrubbing stages (`scrub_ansi_and_redact` -> `vox_secrets::scrub_resolved_secrets`).
  - Extended `TermDispatch` (`crates/vox-orchestrator-d/src/term_dispatch.rs`) with `TermDispatch::with_bridge`, `"term.agent_exec"`, `"term.agent_send_input"`, `"term.read"`, `"term.promote_bg"`, `"term.pin"`, optional `cwd` on `"term.open"`, Stage-3 `scrub_resolved_secrets` on `"term.context"`, and JSON event serialization for the 4 new `SessionEvent` variants while preserving `refuse_agent_origin` on `"term.input"`, `"term.submit"`, and `"term.signal"`.
  - Wired one shared `Arc<LocalHost>` into both `state.set_terminal_bridge(...)` and `TermDispatch::with_bridge(...)` on loopback/stdio transports in `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs`.

## Exported Signatures for Downstream Plans (`442-04`, `442-05`)

- `vox_orchestrator::terminal_bridge`:
  - `pub trait TerminalBridge: Send + Sync` (`exec`, `send_input`, `read`, `list_sessions`, `open_session`, `promote_bg`, `pin_block`)
  - `pub struct TermExecRequest`, `pub struct TermExecResponse`, `pub struct TermSendInputRequest`, `pub struct TermReadRequest`, `pub struct TermBlockDto`, `pub struct TermReadResponse`, `pub struct TermSessionSummary`
  - `pub struct OrchDaemonTerminalBridge`, `pub struct EphemeralTerminalBridge`
- `vox_orchestrator_mcp::ServerState`:
  - `pub terminal_bridge: Arc<PrRwLock<Option<Arc<dyn TerminalBridge>>>>`
  - `pub chat_terminal_sessions: Arc<PrRwLock<HashMap<String, String>>>`
  - `pub fn set_terminal_bridge(&self, bridge: Arc<dyn TerminalBridge>)`
  - `pub async fn resolve_terminal_bridge(&self) -> Arc<dyn TerminalBridge>`
  - `pub async fn bind_or_resolve_terminal_session(&self, chat_thread_id: Option<&str>, explicit_session_id: Option<&str>) -> anyhow::Result<(Arc<dyn TerminalBridge>, String)>`
- `vox_terminal_core`:
  - `stall_detector::{DEFAULT_STALL_QUIESCENCE_MS, PromptKind, StallSignal, classify_stall}`
  - `agent_exec::{AgentExecRequest, AgentExecStatus, AgentExecOutcome, allows_agent_command, format_agent_command_for_shell}`
  - `SessionEvent::{BlockAwaitingInput, BlockPinned, SecurityLevelChanged, ControlModeChanged}`
  - `SessionHost::{set_security_level, set_control_mode, pin_block, promote_bg}`
  - `daemon_host::{SECURITY, CONTROL, PIN, PROMOTE_BG}`
- `vox_orchestrator_d`:
  - `term_bridge_impl::{LocalHostTerminalBridge, scrub_three_stage}`
  - `term_dispatch::TermDispatch::{new, with_bridge}` (`term.agent_exec`, `term.agent_send_input`, `term.read`, `term.promote_bg`, `term.pin`, `term.context`, `term.security`, `term.control`)

## Downstream Exhaustive-Match Note (Outside `442-03` Scope)

- `crates/vox-term/src/app.rs` (outside `442-03` `files_modified`, alongside the existing `vox-term/src/ui/blocks.rs` `BlockStatus::AwaitingInput` note from `442-02-SUMMARY.md`) contains a non-wildcard `match ev` over `SessionEvent` (`L169-252`) that will need arms or `_ => {}` for `BlockAwaitingInput`, `BlockPinned`, `SecurityLevelChanged`, and `ControlModeChanged` during the Wave 3 / orchestrator sweep.

## Verification Status & Lock-Contention Note

- `rustfmt --edition 2024 --check` on all 15 modified/created `.rs` files: **PASS** (`exit 0`).
- Cargo verification (`task-207`, `task-217`, `task-291`):
  - `task-207` caught a `PiiFilter` callsite mismatch in `crates/vox-orchestrator/src/terminal_bridge.rs:361`, which was fixed to `crate::pii_filter::PiiFilter::redact(&plain)`.
  - `task-217` (`timeout 900s cargo test -p vox-terminal-core -p vox-orchestrator-d --lib --tests`) compiled cleanly through `1074/1099: vox-orchestrator` (`0` errors in `vox-orchestrator`) after waiting on `Blocking waiting for file lock on build directory`, then hit `exit 124` due to sibling session `target/.cargo-lock` contention.
  - Static cross-audit also caught and fixed a `$()` command-substitution string inside `agent_exec.rs` `live_pty_exec_preserves_state_handles_prompts_and_refuses_credentials` (`turn 2`) before test execution and added unit tests in `term_bridge_impl.rs` + `CREDENTIAL_PROMPT_USER_ONLY` sentinel in `session.rs`.
  - `task-291` (`timeout 300s cargo test -p vox-terminal-core`) spent the full window on `Blocking waiting for file lock on build directory` (`exit 124`). Per Strict Execution Rule 4, blind Cargo retries were halted so the orchestrator can run the uncontended foreground verification pass once sibling Wave 2 builds release `target/.cargo-lock`:
    - `timeout 900s cargo test -p vox-terminal-core -p vox-orchestrator-d --lib --tests`
    - `timeout 300s cargo test -p vox-orchestrator --lib terminal_bridge`
    - `timeout 900s cargo check -p vox-orchestrator-mcp`
    - `timeout 300s cargo run -p vox-cli -- ci secret-env-guard`
    - `timeout 300s cargo run -p vox-arch-check`
