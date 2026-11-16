---
phase: 79-core-daily-driver-stability-daemon-auto-bootstrap-gateway-ipc-parity
plan: 02
subsystem: gateway-ipc-parity
tags: [remote-axis, gateway, ipc, orchestrator, research, mission-control]
requires: [79-01]
provides:
  - "100% remote routing parity across all daemon-class modules in vox-gui (REMOTE_UNROUTED_DAEMON_MODULES is empty)"
  - "Orchestrator::set_task_mesh_policy and orch.set_mesh_policy daemon RPC handler"
  - "/api/v2/orchestrator/* and /api/v2/research/{run,persist-claims} HTTP gateway routes"
affects:
  - crates/vox-orchestrator/src/orchestrator/accessors.rs
  - crates/vox-orchestrator/src/orch_daemon/mod.rs
  - crates/vox-orchestrator-mcp/src/gui_ops/mod.rs
  - crates/vox-orchestrator-mcp/src/gui_ops/orchestrator.rs
  - crates/vox-orchestrator-mcp/src/gui_ops/research.rs
  - crates/vox-gui/src/remote/gateway_client.rs
  - crates/vox-gui/src/remote/manifest.rs
  - crates/vox-gui/src/remote/backend.rs
  - crates/vox-gui/src/commands/mission_control.rs
key-files:
  modified:
    - crates/vox-orchestrator/src/orchestrator/accessors.rs
    - crates/vox-orchestrator/src/orch_daemon/mod.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/mod.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/orchestrator.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/research.rs
    - crates/vox-gui/src/remote/gateway_client.rs
    - crates/vox-gui/src/remote/manifest.rs
    - crates/vox-gui/src/remote/backend.rs
    - crates/vox-gui/src/commands/mission_control.rs
decisions:
  - "Delegated /api/v2/orchestrator/* routes in gui_ops/orchestrator.rs to vox_orchestrator::orch_daemon::dispatch_request and /api/v2/research/{run,persist-claims} to McpExtraDispatch::try_handle so HTTP gateway and local IPC share one canonical implementation."
  - "Promoted chat, chat_turn, control_plane, mission_control, and research into REMOTE_ROUTED_DAEMON_MODULES, reducing REMOTE_UNROUTED_DAEMON_MODULES to &[] (zero)."
requirements-completed: [CORE-03]
completed: 2026-10-16
---

# Phase 79 Plan 02: Gateway / IPC Parity Summary

## Accomplishments
- **Implemented `Orchestrator::set_task_mesh_policy` and `orch.set_mesh_policy` (`CORE-03`)**: Added `set_task_mesh_policy` in `crates/vox-orchestrator/src/orchestrator/accessors.rs` and wired `orch_daemon_method::SET_MESH_POLICY` in `crates/vox-orchestrator/src/orch_daemon/mod.rs`.
- **Exposed `/api/v2/orchestrator/*` and `/api/v2/research/*` HTTP Gateway Routes (`CORE-03`)**:
  - `GET /api/v2/orchestrator/tasks` (`LIST_TASKS`)
  - `POST /api/v2/orchestrator/tasks/{submit,edit,cancel,interrupt,reorder,doubt,overrule,approve-plan,skip-verify,force-verify}`
  - `POST /api/v2/orchestrator/agents/{pause,resume}`
  - `GET /api/v2/orchestrator/subagent-tree` (`SUBAGENT_TREE`)
  - `GET /api/v2/orchestrator/approvals` (`LIST_PENDING_APPROVALS`)
  - `POST /api/v2/orchestrator/mesh-policy` (`SET_MESH_POLICY`)
  - `POST /api/v2/research/run` (`dei_method::RESEARCH_RUN`)
  - `POST /api/v2/research/persist-claims` (`dei_method::RESEARCH_PERSIST_CLAIMS`)
- **Zero Unrouted Daemon Modules (`CORE-03`)**: Mapped all methods in `crates/vox-gui/src/remote/gateway_client.rs::remote_route` and moved `chat`, `chat_turn`, `control_plane`, `mission_control`, and `research` into `REMOTE_ROUTED_DAEMON_MODULES` (`REMOTE_UNROUTED_DAEMON_MODULES = &[]`).
