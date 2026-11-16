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
  - contracts/mcp/orch-daemon-rpc-methods.schema.json
---

# Phase 12 Plan 08: Comprehensive Origin Invariant Summary

## Overview

Plan 12-08 established the origin boundary across all remaining entry points (`http_gateway`, stdio-forwarded daemon routing, and daemon RPC), ensuring that:
1. External HTTP gateway requests (`http_gateway/rpc_tools.rs`) run inside `with_agent_origin`, preventing any external caller from self-approving floored actions through JSON-RPC.
2. The stdio bridge forwarder (`daemon_route.rs`) sets `agent_origin: true` on `orch.tool_call` parameters.
3. The daemon RPC dispatcher (`daemon_extra.rs`) honors `agent_origin: true` by wrapping execution in `with_agent_origin`, while preserving human GUI client approval privileges when `agent_origin` is omitted or false.
4. A static entry point invariant test (`tests/floor_entry_points_tests.rs`) guarantees every production `handle_tool_call` caller site in the repository is explicitly classified and monitored against accidental unclassified entry points.

## Changes

1. **`crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs`**:
   - Wrapped `handle_tool_call(state, name, args).await` with `crate::floor_gate::with_agent_origin(...)` in `call_tool_json`.

2. **`crates/vox-orchestrator-mcp/src/http_gateway/floor_origin_tests.rs` & `mod.rs`**:
   - Added `floor_origin_tests` testing `gateway_cannot_approve_floored_park`.
   - Spawns test execution in a thread with a 32MB stack to prevent macOS unoptimized debug future stack overflows.

3. **`crates/vox-orchestrator-mcp/src/daemon_route.rs`**:
   - Updated `call_tool_via_daemon` to include `"agent_origin": true` in `orch.tool_call` RPC params.

4. **`crates/vox-orchestrator-mcp/src/daemon_extra.rs`**:
   - Parsed `agent_origin: bool` from `orch.tool_call` params (defaulting to `false`).
   - If `agent_origin == true`, wrapped `handle_tool_call_with_mode` with `with_agent_origin`.

5. **`contracts/mcp/orch-daemon-rpc-methods.schema.json`**:
   - Documented the optional `agent_origin: boolean` property in the `orch.tool_call` parameter schema.

6. **`crates/vox-orchestrator-mcp/tests/stdio_daemon_routing_tests.rs`**:
   - Added `stdio_forwarded_resolve_cannot_approve_floored_park` to verify that stdio-forwarded callers cannot self-approve floored actions.

7. **`crates/vox-orchestrator-mcp/tests/floor_entry_points_tests.rs`**:
   - Added comprehensive structural audit test `entry_point_callers_are_classified`.
   - Verifies all production caller sites of `handle_tool_call`, `handle_tool_call_with_mode`, and `handle_tool_call_with_receipt` against an allowlist and asserts that required origin tags (`with_agent_origin` / `"agent_origin": true`) are present in source.

## Verification Evidence

- `cargo test -p vox-orchestrator-mcp --lib -- http_gateway::floor_origin_tests`: passed (1 passed, 0 failed).
- `cargo test -p vox-orchestrator-mcp --test stdio_daemon_routing_tests`: passed (6 passed, 0 failed).
- `cargo test -p vox-orchestrator-mcp --test daemon_extra_tests`: passed (10 passed, 0 failed).
- `cargo test -p vox-orchestrator-mcp --test floor_entry_points_tests`: passed (1 passed, 0 failed).
- `cargo clippy --no-deps -p vox-orchestrator-mcp --all-targets -- -D warnings`: passed with 0 warnings.
- `cargo run -q -p vox-cli -- ci ssot-drift`: clean (no drift).
- `cargo run -q -p vox-cli -- ci crate-edges`: clean (699 live in-tree edges within baseline).
