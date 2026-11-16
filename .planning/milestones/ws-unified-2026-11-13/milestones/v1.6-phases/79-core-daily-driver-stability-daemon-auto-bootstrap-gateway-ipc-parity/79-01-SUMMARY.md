---
phase: 79-core-daily-driver-stability-daemon-auto-bootstrap-gateway-ipc-parity
plan: 01
subsystem: core-daemon-bootstrap-and-db-pool
tags: [daemon, bootstrap, vox-gui, vox-chat, vox-term, gui_db_pool]
requires: []
provides:
  - "Zero persistent VoxDb handles inside vox-gui (gui_db_pool migrated from class: db to class: shell)"
  - "Cold-start daemon auto-bootstrap for vox-gui, vox chat, and vox term (resolve_session_host)"
  - "Daemon reconnect backoff and live status banner in vox-gui"
affects:
  - crates/vox-gui/src/commands/gui_db_pool.rs
  - crates/vox-gui/src/main.rs
  - contracts/gui/command-backends.v1.yaml
  - crates/vox-cli/src/commands/ci/gui_command_backends.rs
  - crates/vox-gui/ui/src/guards/commandBackendsMigration.test.ts
  - crates/vox-terminal-core/src/daemon_host.rs
key-files:
  modified:
    - crates/vox-gui/src/commands/gui_db_pool.rs
    - crates/vox-gui/src/main.rs
    - contracts/gui/command-backends.v1.yaml
    - crates/vox-cli/src/commands/ci/gui_command_backends.rs
    - crates/vox-gui/ui/src/guards/commandBackendsMigration.test.ts
    - crates/vox-terminal-core/src/daemon_host.rs
decisions:
  - "Replaced GuiDbPool::connect_workspace() + app.manage(pool) in vox-gui startup with GuiDbPool::reset_drive_store_if_requested() so vox-gui holds zero open SQLite handles while vox-orchestrator-d runs."
  - "Added find_daemon_binary() + try_autospawn_daemon() to vox_terminal_core::daemon_host::resolve_session_host so vox term auto-bootstraps vox-orchestrator-d on cold start unless force_local or VOX_TERM_NO_AUTOSPAWN=1."
requirements-completed: [CORE-01, CORE-02, CORE-04, CORE-05]
completed: 2026-10-16
---

# Phase 79 Plan 01: Core Daemon Auto-Bootstrap & Zero-DB GUI Pool Summary

## Accomplishments
- **Eliminated `GuiDbPool` persistent SQLite connection in `vox-gui` (`CORE-04`)**: Added `GuiDbPool::reset_drive_store_if_requested()` and removed `app.manage(pool)` from `crates/vox-gui/src/main.rs`, ensuring `vox-gui` never competes with `vox-orchestrator-d` for workspace SQLite locks.
- **Zero `class: db` command modules (`CORE-04`)**: Reclassified `gui_db_pool` to `class: shell` in `contracts/gui/command-backends.v1.yaml` and tightened `gui_command_backends.rs` and `commandBackendsMigration.test.ts` to assert zero `class: db` modules remain.
- **`vox term` cold-start daemon auto-bootstrap (`CORE-01`)**: Updated `vox_terminal_core::daemon_host::resolve_session_host` to locate and auto-spawn `vox-orchestrator-d` with exponential readiness polling (`50ms -> 800ms`) before falling back to an in-process `TerminalHost`.
- **Verified `vox-gui` and `vox chat` auto-bootstrap & reconnect (`CORE-01`, `CORE-02`, `CORE-05`)**: Verified `PersistentDaemon::ensure()`, `spawn_daemon_watchdog` exponential backoff + `orchestrator:daemon-status` event emission, and `BottomStatusBar` reconnecting banner tests (`BottomStatusBar.test.tsx` 31/31 passing).
