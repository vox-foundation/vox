# Phase 79: Core Daily Driver Stability, Daemon Auto-Bootstrap & Gateway/IPC Parity — Context

**Gathered:** 2026-10-16
**Status:** Ready for planning
**Mode:** Autonomous (`/gsd-autonomous`)

<domain>
## Phase Boundary

Deliver zero-friction cold start, automatic daemon recovery, and full local/remote gateway RPC parity for the highest-frequency daily driver surfaces:
1. **`CORE-01`**: `vox-gui` (Axis), `vox chat`, and `vox term` auto-bootstrap or reconnect to `vox-orchestrator-d` without manual daemon startup (`vox-terminal-core::daemon_host::resolve_session_host` auto-spawns or adopts `vox-orchestrator-d` when not `force_local`).
2. **`CORE-02`**: Chat (`ChatSurface`), Terminal (`Console` / `TerminalTab`), and Command Palette (`Omnibar`) execute end-to-end against live `vox-orchestrator-d` and `VoxDb` with zero silent stalls or dropped events.
3. **`CORE-03`**: Close the 5 remaining `REMOTE_UNROUTED_DAEMON_MODULES` (`chat`, `chat_turn`, `control_plane`, `mission_control`, `research`) by adding `/api/v2/orchestrator/{tasks/submit,subagent-tree,approvals,mesh-policy}` and `/api/v2/research/run` gateway routes and wiring `remote_route` in `crates/vox-gui/src/remote/gateway_client.rs` (including the missing `orch.set_mesh_policy` handler in `orch_daemon`).
4. **`CORE-04`**: Complete `gui_db_pool.rs` migration off direct `VoxDb` (`class: db` -> `class: shell` in `contracts/gui/command-backends.v1.yaml`), dropping the launch-time persistent `GuiDbPool` state from `crates/vox-gui/src/main.rs` so `vox-gui` holds zero open `VoxDb` handles while `vox-orchestrator-d` runs.
5. **`CORE-05`**: Daemon disconnect/reconnect state is surfaced in `BottomStatusBar` and `Console` with automatic exponential-backoff reconnection and zero lost in-flight turns.

</domain>

<decisions>
## Implementation Decisions

### 1. Eliminating `GuiDbPool` Persistent Connection (`CORE-04`)
- `crates/vox-gui/src/commands/gui_db_pool.rs` has zero remaining Tauri command consumers (`chat.rs`, `llm_spend.rs`, `chat_turn.rs` all assert `!body.contains("GuiDbPool")`).
- Replace `GuiDbPool::connect_workspace()` in `crates/vox-gui/src/main.rs` with `commands::gui_db_pool::reset_drive_store_if_requested()`, which runs the `VOX_GUI_DRIVE=1` + `VOX_GUI_DRIVE_ALLOW_STORE_RESET=1` schema check/reset in a short-lived scope and drops the connection before `daemon.ensure()` spawns `vox-orchestrator-d`.
- Reclassify `gui_db_pool` in `contracts/gui/command-backends.v1.yaml` from `class: db` to `class: shell`, bringing `class: db` count in `command-backends.v1.yaml` to **0**.

### 2. Closing Unrouted Remote Daemon Modules (`CORE-03`)
- Implement `Orchestrator::set_task_mesh_policy(TaskId, MeshPolicy)` in `crates/vox-orchestrator/src/orchestrator/accessors.rs` and wire `orch_daemon_method::SET_MESH_POLICY` in `crates/vox-orchestrator/src/orch_daemon/mod.rs`.
- Add `/api/v2/orchestrator/tasks/submit` (POST), `/api/v2/orchestrator/subagent-tree` (GET), `/api/v2/orchestrator/approvals` (GET), `/api/v2/orchestrator/mesh-policy` (POST) to `crates/vox-orchestrator-mcp/src/gui_ops/orchestrator.rs`, and `/api/v2/research/run` (POST) to `crates/vox-orchestrator-mcp/src/gui_ops/research.rs`.
- Map `SUBMIT_TASK`, `SUBAGENT_TREE`, `LIST_PENDING_APPROVALS`, `SET_MESH_POLICY`, and `dei_method::RESEARCH_RUN` in `crates/vox-gui/src/remote/gateway_client.rs::remote_route`.
- Promote `chat`, `chat_turn`, `control_plane`, `mission_control`, and `research` from `REMOTE_UNROUTED_DAEMON_MODULES` to `REMOTE_ROUTED_DAEMON_MODULES` in `crates/vox-gui/src/remote/manifest.rs` (`REMOTE_UNROUTED_DAEMON_MODULES` becomes empty `&[]`).

### 3. Cold-Start Auto-Bootstrap & Reconnect (`CORE-01`, `CORE-02`, `CORE-05`)
- Update `vox-terminal-core::daemon_host::resolve_session_host` so that when `!force_local` and the initial 250ms ping fails, it attempts `ensure_daemon_running()` (spawning `vox-orchestrator-d` if found on PATH or sibling to current exe) before falling back to `LocalHost`.
- Ensure `BottomStatusBar` and `useOrchestratorStatus` clearly surface daemon connection/reconnection state.

</decisions>
