---
phase: 12-safety-floor-lowering-paths
plan: 08
type: execute
completed_at: 2026-10-14T09:14:00Z
files_modified:
  - crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs
  - crates/vox-orchestrator-mcp/src/http_gateway/floor_origin_tests.rs
  - crates/vox-orchestrator-mcp/src/http_gateway/mod.rs
  - crates/vox-orchestrator-mcp/src/daemon_route.rs
  - crates/vox-orchestrator-mcp/src/daemon_extra.rs
  - crates/vox-orchestrator-mcp/tests/stdio_daemon_routing_tests.rs
  - crates/vox-orchestrator-mcp/tests/floor_entry_points_tests.rs
  - contracts/orchestration/orch-daemon-rpc-methods.schema.json
---

# Phase 12 Plan 08: Entry-Point Refusal Summary

## Overview

Closed all external tool-execution entry points into the orchestrator so that remote HTTP gateway clients and `vox mcp` stdio clients cannot approve or modify floored parks (resolving Research OQ4 and T-12-25 across external entry boundaries). Also established a comprehensive AST/source scan integration test that pins every in-process dispatcher and daemon client.

## Changes

1. **`crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs`**:
   - Wrapped `handle_tool_call` inside `call_tool_json` with `crate::floor_gate::with_agent_origin(...)`.
   - Ensured all HTTP gateway routes (`/ln`, `/v1/tools/call`, `/v1/ws`, status probes, and `/v1/mobile`) are treated as agent-origin by construction.
   - Added unit test module `crates/vox-orchestrator-mcp/src/http_gateway/floor_origin_tests.rs` verifying that `POST /v1/tools/call` cannot approve or modify a floored park (`FLOOR_SELF_APPROVAL_REFUSED`), while GUI/daemon reject resolves cleanly.

2. **`crates/vox-orchestrator-mcp/src/daemon_route.rs`**:
   - In `call_tool_via_daemon`, passed `"agent_origin": true` in the `orch.tool_call` payload. Documented that client LLMs compose only `args`, and this flag can only restrict capabilities.

3. **`crates/vox-orchestrator-mcp/src/daemon_extra.rs`**:
   - In the `orch_daemon_method::TOOL_CALL` match arm, extracted `agent_origin` from `req.params`.
   - When `agent_origin == true`, executed the tool-call future wrapped in `crate::floor_gate::with_agent_origin(fut).await`.
   - Documented the rationale for carrying the flag in `params` (it only restricts permissions).

4. **`contracts/orchestration/orch-daemon-rpc-methods.schema.json`**:
   - Updated `params` description to document `orch.tool_call`'s optional `agent_origin: true` flag.

5. **`crates/vox-orchestrator-mcp/tests/stdio_daemon_routing_tests.rs`**:
   - Added `stdio_forwarded_resolve_cannot_approve_floored_park` integration test verifying that tool calls routed through the stdio server cannot approve or modify floored parks even when passing falsified args (`"agent_origin": false`, `"origin": "human"`).

6. **`crates/vox-orchestrator-mcp/tests/floor_entry_points_tests.rs`**:
   - Created source-scan integration test `entry_point_callers_are_classified` verifying that:
     - Part (a): All non-test `handle_tool_call*` callers belong to the approved classification list and are wrapped with `with_agent_origin`, or have `agent_origin` checks in `daemon_extra.rs`.
     - Part (b): All non-test `orch.tool_call` senders belong to the approved classification list and `daemon_route.rs` sets `"agent_origin": true`.
     - Part (c): `server.rs` routes tool calls through `daemon_route::call_tool_via_daemon(`.
   - Verified that the test fails if either the Task 1 wrapper or Task 2 flag is removed.

## Verification Evidence

- `cargo test -p vox-orchestrator-mcp --lib -- http_gateway::floor_origin_tests::gateway_cannot_approve_floored_park`: passed (1 passed, 0 failed).
- `cargo test -p vox-orchestrator-mcp --test stdio_daemon_routing_tests -- stdio_forwarded_resolve_cannot_approve_floored_park`: passed (1 passed, 0 failed).
- `cargo test -p vox-orchestrator-mcp --test daemon_extra_tests -- daemon_gui_path_can_approve_floored_park`: passed (1 passed, 0 failed).
- `cargo test -p vox-orchestrator-mcp --test floor_entry_points_tests`: passed (1 passed, 0 failed).
- Proved test failure modes:
  - Part (a) fails on missing wrapper in `rpc_tools.rs` with `Expected with_agent_origin wrapping call`.
  - Part (b) fails on missing `agent_origin: true` in `daemon_route.rs` with `daemon_route.rs must contain "agent_origin": true`.
- `cargo clippy --no-deps -p vox-orchestrator-mcp --all-targets -- -D warnings`: 0 warnings, clean.
- `cargo run -q -p vox-cli -- ci ssot-drift`: clean, all SSOT guards OK.
- `cargo run -q -p vox-cli -- ci crate-edges`: 699 live in-tree edges within baseline, OK.
