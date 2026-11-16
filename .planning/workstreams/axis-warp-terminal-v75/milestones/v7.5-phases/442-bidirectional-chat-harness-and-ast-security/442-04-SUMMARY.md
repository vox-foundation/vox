---
phase: 442-bidirectional-chat-harness-and-ast-security
plan: 04
subsystem: mcp-terminal-tools-and-contracts
tags: [terminal, mcp, hitl, operations-catalog, clavis, tool-selection]
dependency_graph:
  requires: [442-01, 442-02, 442-03]
  provides:
    - vox_terminal_exec
    - vox_terminal_read
    - vox_terminal_send_input
    - vox_terminal_sessions
    - TERMINAL_EXEC_TIMEOUT
  affects:
    - crates/vox-orchestrator-mcp
    - crates/vox-orchestrator
    - contracts/operations/catalog.v1.yaml
    - contracts/mcp/tool-registry.canonical.yaml
    - contracts/capability/capability-registry.yaml
    - contracts/mcp/http-read-role-governance.yaml
    - contracts/orchestration/permission-modes.v2.yaml
tech_stack:
  added: []
  patterns:
    - Strict Draft-07 JSON Schema validation (`additionalProperties: false`) for stateful terminal MCP tools
    - Dual-tier HITL approval coordination (`outer_hitl_approved` pass-through preventing double-parking under `PermissionMode::Ask`, plus session-level `TermControlMode` + `SecurityTier` + `ExecPolicyV1` parking under `PermissionMode::AcceptAll`)
    - 3-stage output scrubbing (`strip_ansi_sequences` -> `vox_secrets::scrub_resolved_secrets` -> `vox_redact::redact_owned`)
key_files:
  created:
    - crates/vox-orchestrator-mcp/src/input_schemas/terminal.rs
    - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs
  modified:
    - crates/vox-orchestrator-mcp/src/input_schemas.rs
    - crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs
    - crates/vox-orchestrator-mcp/src/dispatch.rs
    - crates/vox-orchestrator/src/oversight/floor.rs
    - crates/vox-orchestrator-mcp/src/permission_modes.rs
    - crates/vox-orchestrator-mcp/src/dispatch_timeout.rs
    - crates/vox-orchestrator-mcp/src/llm_bridge/tool_selection.rs
    - contracts/operations/catalog.v1.yaml
    - contracts/mcp/http-read-role-governance.yaml
    - contracts/orchestration/permission-modes.v2.yaml
    - contracts/mcp/tool-registry.canonical.yaml
    - contracts/capability/capability-registry.yaml
    - contracts/reports/operations-catalog-inventory.v1.json
decisions:
  - Threaded `outer_hitl_approved: bool` from `dispatch_tool_call` into `handle_tool_call_inner` -> `terminal_tools::dispatch` so `PermissionMode::Ask` parks once at the outer gate and never double-parks inside `terminal_tools.rs`, while `PermissionMode::AcceptAll` delegates HITL parking to the session's `TermControlMode` (`manual` / `co_pilot` + `SecurityTier` / `ExecPolicyV1`).
  - Kept `floor_categories.history_rewrite.tools` in `permission-modes.v2.yaml` untouched (since `pending_approvals_tests::floor_categories_yaml_matches_rust_floor` compares it strictly against `HISTORY_REWRITE_TOOLS`) and added `vox_terminal_exec` solely to `SHELL_COMMAND_TOOLS` in `floor.rs` with `floor: null` on the `risk_classes` rows.
metrics:
  duration_minutes: 22
  completed_date: "2026-11-15"
---

# Phase 442 Plan 04: Terminal MCP Tools, Catalog Projections & HITL Governance Summary

Registered and wired the four stateful Axis terminal MCP tools (`vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, `vox_terminal_sessions`), synchronized the canonical operations catalog and generated registry projections, and locked `PendingApprovals` HITL, safety-floor, timeout, and ranked tool-selection surfaces in lockstep.

## Completed Tasks

### Task 1: `vox_terminal_*` Input Schemas, Dispatch Domain, Sticky Thread Binding, Scrubber & HITL Lockstep
- Created [`terminal.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/input_schemas/terminal.rs) defining strict (`additionalProperties: false`) Draft-07 input schemas for `vox_terminal_exec` (`wait_ms: 0..=120000`, `background`, `chat_thread_id`, `session_id`, `cwd`, `rationale`, `agent_id`), `vox_terminal_read` (`block_id`, `after_seq`, `line_start`, `line_end`, `tail_blocks: 1..=100`, `pinned_only`, `failed_only`), `vox_terminal_send_input` (`required: ["session_id", "data"]`), and `vox_terminal_sessions`.
- Wired `mod terminal;` and `#[cfg(any())] match name` scanner anchors in [`input_schemas.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/input_schemas.rs).
- Created [`terminal_tools.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs) (`487` non-blank LoC, `toestub` exit `0` with `0` errors) implementing:
  - 4 literal match arms (`"vox_terminal_exec" =>`, `"vox_terminal_read" =>`, `"vox_terminal_send_input" =>`, `"vox_terminal_sessions" =>`) satisfying `yaml_registry_tools_have_dispatch_match_arms` and `operations-verify`.
  - Sticky `chat_thread_id` session binding via `ServerState::bind_or_resolve_terminal_session` (defaulting to `"default"` when both `session_id` and `chat_thread_id` are omitted, and honoring initial `cwd` when opening an unbound session).
  - Session-level HITL parking on `state.pending_approvals` (recording durable `hitl_approval_record`/`hitl_approval_resolve` + `ApprovalRequested`/`ApprovalResolved` op-log entries) when `!outer_hitl_approved` and `control_mode == "manual"`, `evaluate_command_ast` reports non-`"command-not-allowed"` violations, or `control_mode == "co_pilot"` fails `evaluate_security_tier`.
  - Fail-closed `CREDENTIAL_PROMPT_USER_ONLY` rejection in `vox_terminal_send_input` whenever `target.credential_prompt` is true.
  - 3-stage output scrubbing (`scrub_terminal_mcp_str`: ANSI/OSC strip -> `vox_secrets::scrub_resolved_secrets` -> `vox_redact::redact_owned`).
  - Deterministic mock `TerminalBridge` unit test covering schema bounds, 3-stage scrubbing, `Autopilot` sticky `cwd` across turns, `Manual` HITL parking under both `accept_all` and `ask` (proving zero double-parking), and `CREDENTIAL_PROMPT_USER_ONLY` fail-closed behavior.
- Wired `pub mod terminal_tools;` in [`mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs) and updated [`dispatch.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/dispatch.rs) (`outer_hitl_approved` threading, `SKIP_DISPATCH_PROBE`, `yaml_registry_tools_have_dispatch_match_arms`, and `tool_call_kind_for`).

### Task 2: Operations Catalog Sync, Safety Floor, Permission Modes, Timeouts & Ranked Tool Selection
- Inserted `terminal.exec`, `terminal.read`, `terminal.send.input`, and `terminal.sessions` in lexicographical `id` order in [`catalog.v1.yaml`](file:///Users/brbrainerd/dev/vox/contracts/operations/catalog.v1.yaml) (+84 lines, targeted edit) and added `vox_terminal_read` and `vox_terminal_sessions` to [`http-read-role-governance.yaml`](file:///Users/brbrainerd/dev/vox/contracts/mcp/http-read-role-governance.yaml).
- Ran `env VOX_SKIP_FRESHNESS_CHECK=1 vox ci operations-sync --target all --write` and `env VOX_SKIP_FRESHNESS_CHECK=1 vox ci operations-verify` (`operations-verify OK (1426 catalog rows, inventory report updated)`), updating `tool-registry.canonical.yaml` (+35 lines), `capability-registry.yaml` (+80 lines), and `operations-catalog-inventory.v1.json` (+3/-3 lines).
- Added `"vox_terminal_exec"` to `SHELL_COMMAND_TOOLS` in [`floor.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/oversight/floor.rs) with unit assertions verifying `classify_floor` returns `None` on empty/safe args and `Some(FloorCategory::HistoryRewrite)` on chained `git push --force`.
- Added `vox_terminal_exec` and `vox_terminal_send_input` (`Destructive`, `reversible: false`, `always_requires_approval: false`, `floor: null`) in lockstep to [`permission-modes.v2.yaml`](file:///Users/brbrainerd/dev/vox/contracts/orchestration/permission-modes.v2.yaml) and [`permission_modes.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/permission_modes.rs).
- Added `TERMINAL_EXEC_TIMEOUT = Duration::from_secs(150)` (`> 120s` max `wait_ms`, `<= 195s` daemon client read deadline) in [`dispatch_timeout.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/dispatch_timeout.rs).
- Added `vox_terminal_*` priority boosts to `StructuredShell`, `RustWorkspace`, `AgentHarness`, and `CORE_DAILY_DRIVER_TOOLS` (leaving `FROZEN_PREFIX_CORE_TOOLS` untouched) plus unit tests for ranked selection and `READ_ONLY_PERMISSION_MODE` (`"plan"`) filtering in [`tool_selection.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/llm_bridge/tool_selection.rs).

## Verification

### Completed Non-Cargo Verification
- `rustfmt --edition 2024` across all 9 modified/created `.rs` files: exit `0`.
- `toestub crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs crates/vox-orchestrator-mcp/src/input_schemas/terminal.rs`: exit `0` (`0 critical, 0 error`; `terminal_tools.rs` at `487 < 500` non-blank lines, `terminal.rs` clean).
- `env VOX_SKIP_FRESHNESS_CHECK=1 vox ci operations-sync --target all --write`: exit `0`.
- `env VOX_SKIP_FRESHNESS_CHECK=1 vox ci operations-verify`: exit `0` (`operations-verify OK (1426 catalog rows, inventory report updated)`).
- `git diff --numstat -- contracts/`: surgical tens-of-lines diffs confirmed (`+84` in `catalog.v1.yaml`, `+35` in `tool-registry.canonical.yaml`, `+80` in `capability-registry.yaml`, `+21` in `permission-modes.v2.yaml`, `+2` in `http-read-role-governance.yaml`).

### Queued for Wave 3 Unified Batched Cargo Pass (Parent Orchestrator)
Per Wave 3 Cargo contention coordination (`442-04`, `442-05`, `442-06` sharing `vox-orchestrator-mcp`), Cargo test execution was intentionally deferred to the parent orchestrator's unified verification batch:
- `timeout 900s cargo test -p vox-orchestrator --lib oversight::floor`
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib dispatch_domains::terminal_tools`
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib dispatch_timeout`
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib llm_bridge::tool_selection`
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib registry_dispatch_tests`
- `timeout 900s cargo test -p vox-orchestrator-mcp --test pending_approvals_tests`

### Note for Parent Orchestrator (Out-of-Scope Snapshot Fixture)
- `crates/vox-orchestrator-mcp/src/registry.rs::profile_backcompat_tests` compares `tool_registry()` against `crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt` (not in `442-04-PLAN.md` `files_modified`, so left untouched per strict file-scope discipline). Before running an unfiltered `cargo test -p vox-orchestrator-mcp --lib`, append `vox_terminal_exec`, `vox_terminal_read`, `vox_terminal_send_input`, and `vox_terminal_sessions` in sorted order to `crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt`.

## Deviations from Plan

- **Adapted to Verified Wave 2 (`442-03`) Async Signatures:** `ServerState::resolve_terminal_bridge(&self)` and `ServerState::bind_or_resolve_terminal_session(&self, chat_thread_id, explicit_session_id)` in `server_state.rs` are `async fn`s returning `Arc<dyn TerminalBridge>` and `anyhow::Result<(Arc<dyn TerminalBridge>, String)>`, and `TerminalBridge::send_input` returns `anyhow::Result<()>` (followed by `bridge.read(...)` for the updated block snapshot). Wired `terminal_tools.rs` directly against those exact signatures.
