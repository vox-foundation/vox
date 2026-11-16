---
phase: 12-safety-floor-lowering-paths
plan: 07
type: execute
completed_at: 2026-10-14T08:45:00Z
files_modified:
  - crates/vox-orchestrator-mcp/src/floor_gate.rs
  - crates/vox-orchestrator-mcp/src/dispatch.rs
  - crates/vox-orchestrator-mcp/src/autonomous_tool_dispatch.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs
  - crates/vox-orchestrator-mcp/tests/autonomous_tool_dispatch_tests.rs
  - crates/vox-orchestrator-mcp/tests/daemon_extra_tests.rs
  - .planning/workstreams/autonomy-ux/phases/12-safety-floor-lowering-paths/12-CONTEXT.md
  - .planning/workstreams/unified/phases/12-safety-floor-lowering-paths/12-CONTEXT.md
---

# Phase 12 Plan 07: In-Process Agent Self-Approval Refusal Summary

## Overview

Stopped in-process agent dispatchers from approving or modifying their own floored parks (resolving Research OQ4 and the Revision-1 blocker).

Agent origin is established strictly by construction via a `tokio::task_local!` (`AGENT_ORIGIN`), entered only by vox's own in-process execution loops (`McpToolDispatcher::dispatch` and `run_agent_turn`). Wire parameters, caller claims (`origin: "human"`), and `permission_mode` are never consulted for determining agent origin.

When an agent-origin caller attempts `vox_resolve_approval` on a floored action with `approve` or `modify`, the call is refused with `FLOOR_SELF_APPROVAL_REFUSED`. Rejections (`outcome: "reject"`) and resolution of unfloored actions remain permitted. The human GUI path and daemon RPC (`orch.resolve_approval`) remain fully operational.

## Changes

1. **`crates/vox-orchestrator-mcp/src/floor_gate.rs`**:
   - Declared private `tokio::task_local! static AGENT_ORIGIN: bool`.
   - Exported `with_agent_origin`, `in_agent_origin`, `FLOOR_SELF_APPROVAL_REFUSED`.
   - Implemented `refuse_agent_floor_resolution(state, approval_id, outcome)`:
     - Short-circuits immediately if `!in_agent_origin()` or if `outcome == Rejected`.
     - Identifies floored status via `info.summary.starts_with("[floor:") || name_is_floored(&info.tool)`.
     - Added unit tests `agent_origin_scope_is_by_construction_only` and `refuse_agent_floor_resolution_matrix`.

2. **`crates/vox-orchestrator-mcp/src/dispatch.rs`**:
   - At the entry of the `vox_resolve_approval` arm, checked `refuse_agent_floor_resolution(state, approval_id, outcome)` before resolving.

3. **`crates/vox-orchestrator-mcp/src/autonomous_tool_dispatch.rs`**:
   - Wrapped `handle_tool_call_with_mode` inside `McpToolDispatcher::dispatch` with `crate::floor_gate::with_agent_origin(...)`.

4. **`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`**:
   - Wrapped `handle_tool_call_with_receipt` in `run_agent_turn` with `crate::floor_gate::with_agent_origin(...)`.
   - Added test `agent_loop_cannot_approve_floored_park`.

5. **`crates/vox-orchestrator-mcp/tests/autonomous_tool_dispatch_tests.rs`**:
   - Added test `autonomous_dispatch_cannot_approve_floored_park`.

6. **`crates/vox-orchestrator-mcp/tests/daemon_extra_tests.rs`**:
   - Added test `daemon_gui_path_can_approve_floored_park`.

7. **`12-CONTEXT.md`**:
   - Appended Revision 1 resolutions recording OQ4 resolution, entry points handoff to 12-08, and residual T-12-25 owned by Phase 14.

## Caller Classification Table

Every non-test `handle_tool_call*` call site in the workspace was inspected and classified:

| File and Line | Call Expression | Classification | Status |
|---------------|-----------------|----------------|--------|
| `autonomous_tool_dispatch.rs:78` | `handle_tool_call_with_mode` | In-process autonomous runtime bridge (`McpToolDispatcher`) | Wrapped with `with_agent_origin` in 12-07 |
| `chat_tools/chat/agent_loop.rs:977` | `handle_tool_call_with_receipt` | In-process chat agent iteration loop (`run_agent_turn`) | Wrapped with `with_agent_origin` in 12-07 |
| `http_gateway/rpc_tools.rs:158` | `handle_tool_call` | External HTTP gateway (remote MCP clients) | Scheduled for agent-origin wrapping in 12-08 |
| `daemon_extra.rs:232` | `handle_tool_call_with_mode` | Daemon RPC `orch.tool_call` (GUI path / stdio bridge) | GUI path is human; stdio client gets `params.agent_origin` in 12-08 |
| `dispatch.rs:41` | `handle_tool_call_with_mode` | Internal dispatch delegation chain | Safe (delegates to internal router) |
| `dispatch.rs:59` | `handle_tool_call_with_receipt` | Internal dispatch delegation chain | Safe (delegates to internal router) |

All other matches are inside `#[cfg(test)]` modules (`receipt_tools.rs`, `floor_gate.rs`, `handlers_preferences.rs`, `dispatch.rs`, `kb_tools/library.rs`, `scope_guard.rs`, `command_compliance/tests.rs`).

## Verification Evidence

- `cargo test -p vox-orchestrator-mcp --test autonomous_tool_dispatch_tests`: 6 passed, 0 failed.
- `cargo test -p vox-orchestrator-mcp --test daemon_extra_tests`: 10 passed, 0 failed.
- `cargo test -p vox-orchestrator-mcp --lib -- floor_gate:: agent_loop::tests`: 26 passed, 0 failed.
- `cargo clippy --no-deps -p vox-orchestrator-mcp --all-targets -- -D warnings`: 0 warnings, clean.
- `cargo run -q -p vox-cli -- ci crate-edges`: 699 live in-tree edges within baseline, OK.
