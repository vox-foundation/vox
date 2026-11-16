---
phase: 117-persistent-chat-task-and-session-context-backend-rpcs
plan: 01
subsystem: gui-chat-backend
tags: [vox-gui, hopper, context-budget, session-context-pins, tauri]
requires: [116-01]
provides:
  - `hopper_rename` Tauri command (`vox_hopper_rename` tool route) in `crates/vox-gui/src/commands/hopper.rs`
  - `set_context_budget_strategy`, `get_session_context_pins`, and `set_session_context_pins` in `crates/vox-gui/src/commands/context_budget.rs`
  - Live IPC persistence in `ChatExecutionRail.tsx` and mock support in `tauriMock.ts`
affects: [vox-gui]
key-files:
  modified:
    - crates/vox-gui/src/commands/hopper.rs
    - crates/vox-gui/src/commands/context_budget.rs
    - crates/vox-gui/src/main.rs
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatExecutionRail.test.tsx
    - crates/vox-gui/ui/e2e/lib/tauriMock.ts
requirements-completed: [CHAT-BACKEND-01, CHAT-BACKEND-02, CHAT-BACKEND-03, CHAT-BACKEND-04]
completed: 2026-10-20
---

# Phase 117 Plan 01: Persistent Backend-Synchronized Chat Task & Session Context RPCs Summary

## Accomplishments
- **`CHAT-BACKEND-01` (`crates/vox-gui/src/commands/hopper.rs`):** Added `rename_impl` and `#[tauri::command] pub async fn hopper_rename` routing through `BackendCalls::call_tool("vox_hopper_rename", ...)` and emitting `emit_tasks_changed(&app_handle)`.
- **`CHAT-BACKEND-02` (`crates/vox-gui/src/commands/context_budget.rs`):** Added `set_strategy_impl` and `#[tauri::command] pub async fn set_context_budget_strategy` validating `balanced | aggressive | conservative` and persisting via `vox_gui_preference_set` (`gui.context_budget.strategy`).
- **`CHAT-BACKEND-03` (`crates/vox-gui/src/commands/context_budget.rs`):** Added `SessionContextPinDto`, `get_pins_impl`, `set_pins_impl`, `#[tauri::command] pub async fn get_session_context_pins`, and `#[tauri::command] pub async fn set_session_context_pins` keyed by `gui.session_context_pins.<session_id>`.
- **`CHAT-BACKEND-04` (`ChatExecutionRail.tsx`, `ChatExecutionRail.test.tsx`, `tauriMock.ts`):** Wired `ChatExecutionRail.tsx` to hydrate session pins on session switch (`get_session_context_pins`), persist pin edits/additions/removals (`set_session_context_pins`), persist task renames (`hopper_rename`), and persist compaction strategy cycling (`set_context_budget_strategy`). Verified `11/11` Rust unit tests (`cargo test -p vox-gui -- context_budget hopper plan_panel`) and `24/24` `ChatExecutionRail.test.tsx` tests.
