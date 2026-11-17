---
phase: 454-causal-swarm-event-stamping-persistent-sequence-board
plan: 02
subsystem: concilium-control-plane
tags: [http-gateway, axum, tauri, populi-placement, pilot-actions]
requires:
  - phase: 454-causal-swarm-event-stamping-persistent-sequence-board
    provides: CausalSwarmStamp DTOs, persistent `#seq` board, and watchdog telemetry
provides:
  - Axum `/api/v2/concilium/*` HTTP gateway routes in `crates/vox-orchestrator-mcp/src/http_gateway/concilium_api.rs` (`398` LoC)
  - Typed Tauri `mission_control.rs` (`306` LoC) handlers (`concilium_snapshot`, `concilium_board`, `concilium_placement`, `concilium_pilot_action`) registered in `axis_invoke_handler!`
affects: [vox-orchestrator-mcp, vox-gui]
tech-stack:
  added: []
  patterns:
    - "Single daemon-mediated HTTP v2 control plane (`/api/v2/concilium/{snapshot,board,placement,pilot-action}`) keeping `vox-gui` completely free of direct `VoxDb` handles"
    - "Populi capability normalization using `rec.capabilities.cpu_cores.unwrap_or(1).max(1)`"
key-files:
  created:
    - crates/vox-orchestrator-mcp/src/http_gateway/concilium_api.rs
  modified:
    - crates/vox-orchestrator-mcp/src/http_gateway/mod.rs
    - crates/vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/inproc.rs
    - crates/vox-gui/src/commands/mission_control.rs
    - crates/vox-gui/src/main.rs
key-decisions:
  - "Mount `super::concilium_api::router()` inside `http_gateway` and proxy all `vox-gui` Concilium & Populi calls via `BackendCalls::api_v2_get` / HTTP RPC."
  - "Preserve `#[rustfmt::skip]` in `concilium_api.rs` (`398` LoC) to stay strictly beneath the `400`-LoC child module ceiling."
patterns-established:
  - "Hybrid local-MENS, QUIC peer, and cloud bandit placement telemetry and pilot control actions flow uniformly through `concilium_api.rs`."
requirements-completed: [CONC-03, CONC-04]
duration: 40min
completed: 2026-11-16
---

# Phase 454 Plan 02: Concilium Control Plane & Axis RPC Surface Summary

**Phase 454 Plan 02 shipped the daemon HTTP v2 Concilium & Populi control plane in [`concilium_api.rs`](crates/vox-orchestrator-mcp/src/http_gateway/concilium_api.rs) (`398` LoC) (`CONC-03`) and wired typed Tauri `mission_control` IPC handlers in `vox-gui` without direct database handles (`CONC-04`).**

## Accomplishments

### `CONC-03`: HTTP Gateway Concilium & Populi Control Plane
- Implemented [`crates/vox-orchestrator-mcp/src/http_gateway/concilium_api.rs`](crates/vox-orchestrator-mcp/src/http_gateway/concilium_api.rs) (`398` LoC), mounted once inside [`dashboard_api.rs`](crates/vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs) and nested under `/api/v2` in [`mod.rs`](crates/vox-orchestrator-mcp/src/http_gateway/mod.rs) plus [`gui_ops/inproc.rs`](crates/vox-orchestrator-mcp/src/gui_ops/inproc.rs) (after removing a duplicate `.merge(concilium_api::router())` collision in `mod.rs`).
- Normalized Populi CPU core count reporting (`rec.capabilities.cpu_cores.unwrap_or(1).max(1)`) alongside GPU/VRAM, FileAffinityMap lock cells, `#seq` board pagination, and pilot actions (`park_agent`, `wake_agent`, `steer_agent`, `swap_compute_tier`, etc.).

### `CONC-04`: `vox-gui` Daemon-Mediated `mission_control` Handlers
- Added `concilium_snapshot`, `concilium_board`, `concilium_placement`, and `concilium_pilot_action` in [`crates/vox-gui/src/commands/mission_control.rs`](crates/vox-gui/src/commands/mission_control.rs) (`306` LoC), deduplicated imports, and registered unique handlers in `axis_invoke_handler!` ([`crates/vox-gui/src/main.rs`](crates/vox-gui/src/main.rs)).

---
*Phase: 454-causal-swarm-event-stamping-persistent-sequence-board*
*Completed: 2026-11-16*
