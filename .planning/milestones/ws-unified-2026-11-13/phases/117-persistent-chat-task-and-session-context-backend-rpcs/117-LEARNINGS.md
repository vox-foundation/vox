---
phase: 117
phase_name: "persistent-chat-task-and-session-context-backend-rpcs"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 4
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
---

# Phase 117 Learnings: persistent-chat-task-and-session-context-backend-rpcs

## Decisions

### CHAT-BACKEND-01 (crates/vox-gui/src/commands/hopper.rs)
**`CHAT-BACKEND-01` (`crates/vox-gui/src/commands/hopper.rs`):** Added `rename_impl` and `#[tauri::command] pub async fn hopper_rename` routing through `BackendCalls::call_tool("vox_hopper_rename", ...)` and emitting `emit_tasks_changed(&app_handle)`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 117-01-SUMMARY.md

---

### CHAT-BACKEND-02 (crates/vox-gui/src/commands/context_budget.rs)
**`CHAT-BACKEND-02` (`crates/vox-gui/src/commands/context_budget.rs`):** Added `set_strategy_impl` and `#[tauri::command] pub async fn set_context_budget_strategy` validating `balanced | aggressive | conservative` and persisting via `vox_gui_preference_set` (`gui.context_budget.strategy`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 117-01-SUMMARY.md

---

### CHAT-BACKEND-03 (crates/vox-gui/src/commands/context_budget.rs)
**`CHAT-BACKEND-03` (`crates/vox-gui/src/commands/context_budget.rs`):** Added `SessionContextPinDto`, `get_pins_impl`, `set_pins_impl`, `#[tauri::command] pub async fn get_session_context_pins`, and `#[tauri::command] pub async fn set_session_context_pins` keyed by `gui.session_context_pins.<session_id>`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 117-01-SUMMARY.md

---

### CHAT-BACKEND-04 (ChatExecutionRail.tsx, ChatExecutionRail.test.tsx, taur...
**`CHAT-BACKEND-04` (`ChatExecutionRail.tsx`, `ChatExecutionRail.test.tsx`, `tauriMock.ts`):** Wired `ChatExecutionRail.tsx` to hydrate session pins on session switch (`get_session_context_pins`), persist pin edits/additions/removals (`set_session_context_pins`), persist task renames (`hopper_rename`), and persist compaction strategy cycling (`set_context_budget_strategy`). Verified `11/11` Rust unit tests (`cargo test -p vox-gui -- context_budget hopper plan_panel`) and `24/24` `ChatExecutionRail.test.tsx` tests.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 117-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

