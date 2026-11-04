---
phase: 319-mesh-wide-selection-mesh-01
plan: 01
subsystem: mesh
tags: [qwen3, qlora, accelerator_free_mb, node-record, mens]

requires:
  - phase: 318
    provides: CUDA-then-Metal free-memory reading via read_accelerator_free_mb
provides:
  - accelerator_free_mb on NodeRecord, in megabytes beside gpu_vram_total_mb
  - select_qwen3_mesh, one pick_base call on the largest eligible free-memory reading
  - CPU tie-break and the existing quarantine, maintenance, label, and donation filters
affects: [319-02, 319-03]

actuals:
  tokens: 3908
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "Size comes from accelerator_free_mb through select_qwen3_code_qlora_for_free_mb. Total VRAM, system RAM, and min_vram_mb do not choose the size."
    - "vox-plugin-populi-mesh enables the existing mens feature on vox-populi. No new workspace crate edge."

key-files:
  created: []
  modified:
    - crates/vox-populi-types/src/node_record.rs
    - crates/vox-populi/src/lib.rs
    - crates/vox-orchestrator/src/orchestrator/tests/populi_single_owner.rs
    - crates/vox-plugin-populi-mesh/Cargo.toml
    - crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs

key-decisions:
  - "Missing cpu_usage_pct sorts as 100.0 so a node that reported CPU wins the tie."
  - "A free-memory reading that does not fit u32 is ReadingExceedsU32. It is not truncated."

patterns-established:
  - "Mesh tests inject NodeRecord values, a clock, and a stale window. They do not probe a GPU."

requirements-completed: [MESH-01]

coverage:
  - id: D1
    description: "The largest measured qwen3_code QLoRA floor is routed to a node whose accelerator_free_mb holds it."
    requirement: MESH-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-populi-mesh --lib mesh_pick_routes_largest_floor_to_the_node_that_holds_it"
        status: pass
    human_judgment: false
  - id: D2
    description: "Tie-break is lowest cpu_usage_pct. Quarantine, maintenance, labels, and donation policy still exclude a node."
    requirement: MESH-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-populi-mesh --lib mesh_pick_"
        status: pass
    human_judgment: false

duration: 50min
completed: 2026-10-30
status: complete
plan_head_before: a5d60a1e8743ef708dd5a26ed061bbfb27a612b2
plan_head_after: 6c43f9dee63a965f31ac981ebf169e07c4229bbd
---

> **Port note (2026-10-29):** This is Cursor's record, copied verbatim. On `gsd/mens-compare-316`, the selector names and floors it cites are re-targeted onto `size_select::select_size` and the measured floors. See `319-PORT.md`.

# Phase 319 Plan 01: Mesh pick of the largest measured floor Summary

**`select_qwen3_mesh` places the largest measured qwen3_code QLoRA floor on the eligible node whose `accelerator_free_mb` holds it, with the existing policy filters and a lowest-CPU tie-break.**

## Performance

- **Duration:** 50 min
- **Started:** 2026-10-30T01:48:20Z
- **Completed:** 2026-10-30T02:37:54Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- `NodeRecord.accelerator_free_mb` is optional megabytes beside `gpu_vram_total_mb`. Missing JSON stays `None`. `memory_free_bytes` is still system RAM.
- `select_qwen3_mesh` calls `select_qwen3_code_qlora_for_free_mb` once on the largest eligible reading. That function calls `pick_base`. The ladder is not copied.
- Two holders of the same floor break on the lower `cpu_usage_pct`. Quarantine, maintenance, required labels, and donation policy exclude a node before the pick.
- The mesh pick ignores `gpu_vram_total_mb`, `memory_free_bytes`, and `capabilities.min_vram_mb`.

## Task Commits

1. **Task 1: route the largest measured floor** - `4cd8c11ac` (feat)
2. **Task 2: policy filters and CPU tie-break** - `6c43f9dee` (feat)

## Files Created/Modified

- `crates/vox-populi-types/src/node_record.rs` - `accelerator_free_mb: Option<u64>` with serde default
- `crates/vox-populi/src/lib.rs` - struct literal field so the current-process record still builds
- `crates/vox-orchestrator/src/orchestrator/tests/populi_single_owner.rs` - two struct literals gain the field
- `crates/vox-plugin-populi-mesh/Cargo.toml` - existing `vox-populi` dependency enables `mens`
- `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs` - `select_qwen3_mesh` and the policy helper shared with `select_best_node`

## Decisions Made

- A missing CPU sample sorts as 100.0, so a node that reported CPU wins the tie.
- Readings above `u32::MAX` return `ReadingExceedsU32` instead of being truncated into the picker.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Orchestrator admission test is cfg-gated**
- **Found during:** Task 1
- **Issue:** `cargo test -p vox-orchestrator --lib vram_admission_allows_task_when_node_meets_requirement` ran 0 tests. The test is `#[cfg(feature = "populi-transport")]`, and that feature is not default.
- **Fix:** Reran with `--features populi-transport`. 1 passed, 1465 filtered.
- **Files modified:** none
- **Verification:** 1 passed
- **Committed in:** not a code change; recorded against `4cd8c11ac`

**2. Tests and implementation share one commit per task**
- **Found during:** Tasks 1 and 2
- **Issue:** The plan marks tasks `tdd="true"`, and the plan type is `execute`. A compile-fail RED commit is not a valid gate, and the execution order asked for one commit per task.
- **Fix:** Each task's tests were written in the same change as the implementation and the commit landed after the green run.
- **Committed in:** `4cd8c11ac`, `6c43f9dee`

**Total deviations:** 2
**Impact on plan:** The feature flag makes the planned test command actually compile the module. The commit shape matches one commit per task.

## Issues Encountered

None in the shipped commits.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

`select_qwen3_mesh` names a node from injected readings. Plan 02 adds the stale window, the explicit-size error, and the heartbeat clear.

## Self-Check: PASSED

- FOUND: crates/vox-populi-types/src/node_record.rs
- FOUND: crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs
- FOUND: 4cd8c11ac
- FOUND: 6c43f9dee

## Test commands

- `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_routes_largest_floor_to_the_node_that_holds_it --quiet` — 1 passed
- `cargo test -p vox-orchestrator --lib vram_admission_allows_task_when_node_meets_requirement --quiet` — 0 passed (module not compiled)
- `cargo test -p vox-orchestrator --features populi-transport --lib vram_admission_allows_task_when_node_meets_requirement --quiet` — 1 passed
- `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_ --quiet` — 4 passed

---
*Phase: 319-mesh-wide-selection-mesh-01*
*Completed: 2026-10-30*
