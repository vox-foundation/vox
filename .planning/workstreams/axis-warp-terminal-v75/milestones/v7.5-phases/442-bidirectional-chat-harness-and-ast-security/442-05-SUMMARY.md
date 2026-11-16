---
phase: 442-bidirectional-chat-harness-and-ast-security
plan: 05
subsystem: ui
tags: [tauri, ipc, terminal, pty, redaction, clavis, a2a, typescript, vox-gui]

requires:
  - phase: 442-02
    provides: scrub_ansi_and_redact, truncate_terminal_block, BlockStatus::AwaitingInput, Block.{pinned,credential_prompt}, and vox_secrets::scrub_resolved_secrets
  - phase: 442-03
    provides: SessionHost::{set_security_level,set_control_mode,pin_block,promote_bg} and TermReadResponse / vox_terminal_read contract
provides:
  - Tauri IPC commands term_session_set_security, term_session_set_control, term_session_pin, and term_session_promote_bg in commands::pty (ModuleClass::Host) registered in axis_invoke_handler!
  - Backend-resolved 3-stage scrubbed send_terminal_block_to_agent and scrubbed send_to_agent in commands::console_a2a (ModuleClass::Daemon) registered in axis_invoke_handler!
  - Canonical TypeScript wire types (TermSecurityLevelWire, TermControlModeWire, TermBlockStatus with AwaitingInput, TermBlockDto.{pinned,credential_prompt}, TermSessionInfoDto metadata) and typed safeInvoke wrappers in crates/vox-gui/ui/src/transport.ts
affects: [443, 445, vox-gui]

actuals:
  tokens: 5278
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Backend-resolved SendToAgent citations: webview sends { agentId, sessionId, blockId, lineStart, lineEnd, note } and Rust resolves output via vox_terminal_read before running 3-stage ANSI + vox-redact + Clavis scrubbing and 16 KiB head-40/tail-160 truncation"
    - "ModuleClass boundary preservation: PTY session controls stay in commands::pty (ModuleClass::Host) while durable A2A terminal citations live in commands::console_a2a (ModuleClass::Daemon)"

key-files:
  created: []
  modified:
    - crates/vox-gui/src/commands/pty.rs
    - crates/vox-gui/src/commands/console_a2a.rs
    - crates/vox-gui/src/main.rs
    - crates/vox-gui/ui/src/transport.ts

key-decisions:
  - "Added promote_bg_on_host(host, session_id, block_id: Option<u64>) -> Result<bool, String> in commands::pty so term_session_promote_bg supports both Ctrl+B session-level promotion (resolving the active Running/AwaitingInput block via zero-replay attach(Some(u64::MAX)) and returning Ok(false) when idle) and explicit block-level promotion against SessionHost::promote_bg(&self, id: &str, block_id: BlockId) -> Result<()>"
  - "Validated session_id and parse_receiver(&agent_id) in send_terminal_block_to_agent_impl prior to calling vox_terminal_read so empty sessions and reserved/invalid agent IDs fail fast with zero daemon/gateway round-trips"

patterns-established:
  - "All terminal-to-agent A2A notes (both structured send_terminal_block_to_agent citations and free-form send_to_agent bodies) pass through scrub_for_agent_inbox (scrub_ansi_and_redact + vox_secrets::scrub_resolved_secrets) and truncate_terminal_block (head-40/tail-160, 16 KiB) before calling vox_a2a_send with route: \"db\""
  - "All new frontend terminal IPC helpers in transport.ts route exclusively through safeInvoke outside the marked raw-IPC region"

requirements-completed:
  - TERM-HARNESS-03
  - TERM-HARNESS-04

coverage:
  - id: D1
    description: "Tauri PTY session commands (term_session_set_security, term_session_set_control, term_session_pin, term_session_promote_bg) in commands::pty (ModuleClass::Host) with wire-alias parsers and LocalHost unit tests, registered in axis_invoke_handler!"
    requirement: TERM-HARNESS-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/src/commands/pty.rs#tests::parse_security_level_and_control_mode_accept_canonical_and_reject_invalid"
        status: unknown
      - kind: unit
        ref: "crates/vox-gui/src/commands/pty.rs#tests::security_control_pin_and_promote_bg_roundtrip_on_local_host"
        status: unknown
    human_judgment: false
  - id: D2
    description: "Backend-resolved 3-stage scrubbed send_terminal_block_to_agent and scrubbed send_to_agent in commands::console_a2a (ModuleClass::Daemon) with FakeBackend unit tests"
    requirement: TERM-HARNESS-04
    verification:
      - kind: unit
        ref: "crates/vox-gui/src/commands/console_a2a.rs#tests::send_terminal_block_to_agent_resolves_backend_scrubs_and_truncates"
        status: unknown
      - kind: unit
        ref: "crates/vox-gui/src/commands/console_a2a.rs#tests::send_terminal_block_to_agent_validates_inputs_and_missing_blocks"
        status: unknown
      - kind: unit
        ref: "crates/vox-gui/src/commands/console_a2a.rs#tests::send_to_agent_scrubs_ansi_paths_and_tokens_before_a2a_send"
        status: unknown
    human_judgment: false
  - id: D3
    description: "Canonical TypeScript terminal wire types (TermSecurityLevelWire, TermControlModeWire, TermBlockStatus, TermBlockDto, TermSessionInfoDto) and safeInvoke wrappers in transport.ts passing src/guards"
    requirement: TERM-HARNESS-03
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/guards/transportIpcGuard.test.ts"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/ui/src/transport.console.test.ts"
        status: pass
    human_judgment: false

duration: 21min
completed: 2026-11-15
status: complete
---

# Phase 442 Plan 05: Vox Axis Terminal Tauri Commands, Backend-Scrubbed `SendToAgent` & `transport.ts` Wrappers Summary

**`commands::pty` `term_session_{set_security,set_control,pin,promote_bg}` Tauri IPC handlers, backend-resolved 3-stage scrubbed `send_terminal_block_to_agent` in `commands::console_a2a`, and canonical `safeInvoke` wrappers/DTOs in `transport.ts` (`TERM-HARNESS-03`, `TERM-HARNESS-04`)**

## Performance

- **Duration:** 21 min
- **Started:** 2026-11-15T07:57:46Z
- **Completed:** 2026-11-15T08:18:30Z
- **Tasks:** 2 / 2 completed
- **Files modified:** 4 (`0` `Cargo.toml` / `Cargo.lock` changes; `0` new `crate-edges`)
- **Commits:** skipped per orchestrator shared git index rule (orchestrator handles batched Cargo verification and commits)

## Accomplishments

- **Task 1 — PTY Session Controls & Backend-Scrubbed `SendToAgent` (`commands/pty.rs`, `commands/console_a2a.rs`):**
  - Added `parse_security_level` (`"l1_read_only"`..`"l5_autopilot_floor"` plus numeric/short/legacy serde aliases `"1"`..`"5"`, `"L1"`..`"L5"`, `"read_only"`, `"workspace_build"`, `"safe_exec"`, `"structured_default"`, `"workspace_write"`, `"host_gated"`, `"system_exec"`, `"autopilot_floor"`), `parse_control_mode` (`"manual"`, `"co_pilot"` / `"copilot"`, `"autopilot"`), and `promote_bg_on_host` in `crates/vox-gui/src/commands/pty.rs`.
  - Implemented `term_session_set_security`, `term_session_set_control`, `term_session_pin`, and `term_session_promote_bg` in `commands::pty` (`ModuleClass::Host`) resolving the host via `manager.host_for(daemon.inner()).await`, plus unit tests covering alias parsing, rejection of empty/invalid inputs, and `LocalHost` round-trips.
  - Added `scrub_for_agent_inbox` (`scrub_ansi_and_redact` + `vox_secrets::scrub_resolved_secrets`), `send_terminal_block_to_agent_impl`, and `#[tauri::command] pub async fn send_terminal_block_to_agent` in `crates/vox-gui/src/commands/console_a2a.rs` (`ModuleClass::Daemon`), resolving `{ session_id, block_id, line_start, line_end }` via `"vox_terminal_read"`, applying 3-stage scrubbing and head-40/tail-160 (`16 KiB`) truncation to `input`, `output`, and optional `note` (as well as legacy free-form `send_to_agent` bodies), and dispatching `"vox_a2a_send"` with `CONSOLE_OPERATOR_AGENT_ID` (`0`), `parse_receiver`, and `"route": "db"`.
- **Task 2 — Tauri Handler Registration & Typed `transport.ts` Wrappers (`main.rs`, `ui/src/transport.ts`):**
  - Registered `commands::pty::term_session_set_security`, `commands::pty::term_session_set_control`, `commands::pty::term_session_pin`, `commands::pty::term_session_promote_bg`, and `commands::console_a2a::send_terminal_block_to_agent` in `axis_invoke_handler!` in `crates/vox-gui/src/main.rs`.
  - Updated `crates/vox-gui/ui/src/transport.ts` strictly outside the `// __VOX_RAW_IPC_BEGIN__`..`// __VOX_RAW_IPC_END__` guard region: widened `TermBlockStatus` with `'AwaitingInput'`, exported canonical `TermSecurityLevelWire` and `TermControlModeWire`, extended `TermBlockDto` (`pinned?`, `credential_prompt?`) and `TermSessionInfoDto` (`security_level?`, `control_mode?`, `alt_screen?`, `cwd?`, `git_branch?`), and added `termSessionSetSecurity`, `termSessionSetControl`, `termSessionPin`, `termSessionPromoteBg`, `SendTerminalBlockToAgentArgs`, and `sendTerminalBlockToAgent` calling `safeInvoke`.

## Task Commits

Commits skipped per Wave 3 execution rules (shared git index); all 3 modified `.rs` files formatted individually via `rustfmt --edition 2024`.

## Files Created/Modified

- `crates/vox-gui/src/commands/pty.rs` — Added `parse_security_level`, `parse_control_mode`, `promote_bg_on_host`, `term_session_{set_security,set_control,pin,promote_bg}`, and unit tests.
- `crates/vox-gui/src/commands/console_a2a.rs` — Added `scrub_for_agent_inbox`, `dispatch_durable_a2a`, `send_terminal_block_to_agent_impl`, `send_terminal_block_to_agent`, 3-stage scrubbing on `send_to_agent_impl`, and `FakeBackend` unit tests.
- `crates/vox-gui/src/main.rs` — Registered the 5 new terminal/A2A commands in `axis_invoke_handler!`.
- `crates/vox-gui/ui/src/transport.ts` — Added `TermSecurityLevelWire`, `TermControlModeWire`, updated `TermBlockStatus` / `TermBlockDto` / `TermSessionInfoDto`, and added `safeInvoke` wrappers.

## Decisions Made

- Kept all identifiers free of `password_*` / `secret_*` names (`credential_prompt` on DTOs) and annotated test path fixtures with `// vox-arch-check: allow abs-path` so `secret-env-guard` and `vox-arch-check` stay clean.
- Made `block_id: Option<u64>` optional on `term_session_promote_bg` (`blockId?: number | null` on `termSessionPromoteBg`) so both keyboard-shortcut session promotion (`Ctrl+B` without a block ID) and explicit Warp Block Card promotion work over a single IPC command.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Adapted `term_session_promote_bg` to `SessionHost::promote_bg(&self, id: &str, block_id: BlockId) -> Result<()>`**
- **Found during:** Task 1 (`crates/vox-terminal-core/src/host.rs:166` signature inspection)
- **Issue:** `442-05-PLAN.md` sketched `host.promote_bg(&session_id).await` returning `Result<bool, _>`, whereas `SessionHost::promote_bg` in `crates/vox-terminal-core/src/host.rs:166` (delivered by `442-03`) takes `(&self, id: &str, block_id: BlockId) -> Result<()>`.
- **Fix:** Implemented `promote_bg_on_host(host, &session_id, block_id: Option<u64>) -> Result<bool, String>` which uses `BlockId(id)` when `block_id` is supplied or resolves the active `Running | AwaitingInput` block via `host.attach(session_id, Some(u64::MAX)).await` (zero replay copy), returning `Ok(false)` when no block is active and `Ok(true)` once `host.promote_bg(session_id, bid).await` succeeds.
- **Files modified:** `crates/vox-gui/src/commands/pty.rs`, `crates/vox-gui/ui/src/transport.ts`
- **Verification:** Covered by `commands::pty::tests::security_control_pin_and_promote_bg_roundtrip_on_local_host` (idle `Ok(false)`, active `Ok(true)`, missing session `Err`).

---

**Total deviations:** 1 auto-fixed (Rule 1 signature mismatch alignment)
**Impact on plan:** Preserves the exact `Result<bool, String>` / `Promise<boolean>` contract from `442-05-PLAN.md` while compiling cleanly against `442-03`'s `SessionHost` trait and additionally supporting explicit `blockId` targeting.

## Verification Proof (Frontend Guard & Console Transport Suites)

Per Wave 3 Cargo contention coordination rules, `cargo test` / `cargo check` were deferred to the parent orchestrator's unified batched Cargo pass (`cargo test -p vox-gui --bin vox-gui commands::pty`, `commands::console_a2a`, `remote::gate`, and `cargo run -p vox-arch-check`).

### 1. `rustfmt --edition 2024` on Modified Rust Files (Exit 0)
- `rustfmt --edition 2024 crates/vox-gui/src/commands/pty.rs` → `exit 0`
- `rustfmt --edition 2024 crates/vox-gui/src/commands/console_a2a.rs` → `exit 0`
- `rustfmt --edition 2024 crates/vox-gui/src/main.rs` → `exit 0`

### 2. `timeout 300s pnpm --dir crates/vox-gui/ui exec vitest run src/guards` (Exit 0)
```text
 RUN  v3.2.6 /Users/brbrainerd/dev/vox/crates/vox-gui/ui

 ✓ src/guards/transportIpcGuard.test.ts
 ✓ src/guards/commandBackendsMigration.test.ts
 ✓ src/guards/ipcBoundaries.test.ts
 ✓ src/guards/zeroEmojiAndDingbat.test.ts (2 tests) 3156ms
 ✓ src/guards/noRawNativeControlsGuard.test.ts (2 tests) 5669ms
 ✓ src/guards/surfaceRegistryEscape.test.ts (4 tests) 14ms

 Test Files  10 passed (10)
      Tests  40 passed (40)
   Duration  39.89s
```

### 3. `timeout 120s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts` (Exit 0)
```text
 RUN  v3.2.6 /Users/brbrainerd/dev/vox/crates/vox-gui/ui

 ✓ src/transport.console.test.ts (3 tests) 15ms
   ✓ console transport > discoverySuggest forwards typed + limit to invoke 4ms
   ✓ console transport > ptyWrite forwards tab id + data 2ms
   ✓ console transport > termSession* wrappers forward SessionHost IPC commands 1ms

 Test Files  1 passed (1)
      Tests  3 passed (3)
   Duration  24.59s
```

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `vox-gui` Tauri backend commands and `transport.ts` `safeInvoke` wrappers + canonical DTOs (`TermSecurityLevelWire`, `TermControlModeWire`, `TermBlockStatus` `'AwaitingInput'`, `pinned`, `credential_prompt`, `sendTerminalBlockToAgent`) are ready for Phase 443 Warp Block Cards and Phase 445 Console/Chat UI integration once the orchestrator completes the batched Wave 3 Cargo verification pass:
  - `timeout 300s cargo test -p vox-gui --bin vox-gui commands::pty`
  - `timeout 300s cargo test -p vox-gui --bin vox-gui commands::console_a2a`
  - `timeout 300s cargo test -p vox-gui --bin vox-gui remote::gate`
  - `timeout 300s cargo run -p vox-arch-check`

---
*Phase: 442-bidirectional-chat-harness-and-ast-security*
*Completed: 2026-11-15*
