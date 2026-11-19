---
phase: 79
phase_name: "core-daily-driver-stability-daemon-auto-bootstrap-gateway-ipc-parity"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 9
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 79 Learnings: core-daily-driver-stability-daemon-auto-bootstrap-gateway-ipc-parity

## Decisions

### Eliminated GuiDbPool persistent SQLite connection in vox-gui (CORE-04)
**Eliminated `GuiDbPool` persistent SQLite connection in `vox-gui` (`CORE-04`)**: Added `GuiDbPool::reset_drive_store_if_requested()` and removed `app.manage(pool)` from `crates/vox-gui/src/main.rs`, ensuring `vox-gui` never competes with `vox-orchestrator-d` for workspace SQLite locks.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 79-01-SUMMARY.md

---

### Zero class: db command modules (CORE-04): Reclassified gui_db_pool to cl...
**Zero `class: db` command modules (`CORE-04`)**: Reclassified `gui_db_pool` to `class: shell` in `contracts/gui/command-backends.v1.yaml` and tightened `gui_command_backends.rs` and `commandBackendsMigration.test.ts` to assert zero `class: db` modules remain.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 79-01-SUMMARY.md

---

### vox term cold-start daemon auto-bootstrap (CORE-01)
**`vox term` cold-start daemon auto-bootstrap (`CORE-01`)**: Updated `vox_terminal_core::daemon_host::resolve_session_host` to locate and auto-spawn `vox-orchestrator-d` with exponential readiness polling (`50ms -> 800ms`) before falling back to an in-process `TerminalHost`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 79-01-SUMMARY.md

---

### Verified vox-gui and vox chat auto-bootstrap & reconnect (CORE-01, CORE-...
**Verified `vox-gui` and `vox chat` auto-bootstrap & reconnect (`CORE-01`, `CORE-02`, `CORE-05`)**: Verified `PersistentDaemon::ensure()`, `spawn_daemon_watchdog` exponential backoff + `orchestrator:daemon-status` event emission, and `BottomStatusBar` reconnecting banner tests (`BottomStatusBar.test.tsx` 31/31 passing).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 79-01-SUMMARY.md

---

### Implemented Orchestrator::set_task_mesh_policy and orch.set_mesh_policy ...
**Implemented `Orchestrator::set_task_mesh_policy` and `orch.set_mesh_policy` (`CORE-03`)**: Added `set_task_mesh_policy` in `crates/vox-orchestrator/src/orchestrator/accessors.rs` and wired `orch_daemon_method::SET_MESH_POLICY` in `crates/vox-orchestrator/src/orch_daemon/mod.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 79-02-SUMMARY.md

---

### GET /api/v2/orchestrator/tasks (LIST_TASKS)
`GET /api/v2/orchestrator/tasks` (`LIST_TASKS`)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 79-02-SUMMARY.md

---

### POST /api/v2/orchestrator/tasks/{submit,edit,cancel,interrupt,reorder,do...
`POST /api/v2/orchestrator/tasks/{submit,edit,cancel,interrupt,reorder,doubt,overrule,approve-plan,skip-verify,force-verify}`

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 79-02-SUMMARY.md

---

### POST /api/v2/orchestrator/agents/{pause,resume}
`POST /api/v2/orchestrator/agents/{pause,resume}`

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 79-02-SUMMARY.md

---

### GET /api/v2/orchestrator/subagent-tree (SUBAGENT_TREE)
`GET /api/v2/orchestrator/subagent-tree` (`SUBAGENT_TREE`)

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 79-02-SUMMARY.md

## Lessons

## Patterns

## Surprises

