---
phase: 319-mesh-wide-selection-mesh-01
plan: 03
subsystem: mesh
tags: [qwen3, qlora, local-fallback, place_qwen3]

requires:
  - phase: 319-mesh-wide-selection-mesh-01
    provides: select_qwen3_mesh with the stale window and ExplicitDoesNotFit from plan 02
provides:
  - place_qwen3_code_qlora, mesh pick when a fresh reading exists and local pick otherwise
  - select_best_node still ignores accelerator_free_mb
affects: []

actuals:
  tokens: 1411
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "Empty registry or NoFreshReading uses the caller-supplied local_free_mb. local_free_mb None does not probe."
    - "ExplicitDoesNotFit and NothingFits propagate. They do not become a local pick."

key-files:
  created: []
  modified:
    - crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs

key-decisions:
  - "A registry of only this node with a fresh reading is a mesh pick of this node, even when a smaller local reading was also supplied."
  - "The automatic picker failure is NothingFits, so a too-small fresh reading is not NoFreshReading and does not use the local reading."

patterns-established:
  - "place_qwen3_code_qlora never calls read_accelerator_free_mb. The local number is an argument."

requirements-completed: [MESH-01]

coverage:
  - id: D1
    description: "select_best_node ignores accelerator_free_mb. Empty or stale registries use the injected local reading and do not probe."
    requirement: MESH-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-populi-mesh --lib select_best_node_ignores_accelerator_free_mb"
        status: pass
      - kind: unit
        ref: "cargo test -p vox-plugin-populi-mesh --lib place_qwen3_"
        status: pass
    human_judgment: false
  - id: D2
    description: "One fresh node is a mesh pick of that node. An explicit miss does not fall back to the local reading."
    requirement: MESH-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-populi-mesh --lib place_qwen3_only_this_node_is_a_mesh_pick"
        status: pass
      - kind: unit
        ref: "cargo test -p vox-plugin-populi-mesh --lib place_qwen3_explicit_miss_does_not_fall_back_local"
        status: pass
    human_judgment: false

duration: 10min
completed: 2026-10-30
status: complete
plan_head_before: c54c973fccdf49fd63d36ce4d881dcb74eb1a2ae
plan_head_after: ab1b17c3f2d09d14c03dd9870278cff8ddf77583
---

# Phase 319 Plan 03: Local fallback and one-node mesh pick Summary

**`place_qwen3_code_qlora` uses the mesh pick whenever a fresh reading exists, including a registry of only this node, and uses the injected local reading only when the mesh has none.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-10-30T02:57:02Z
- **Completed:** 2026-10-30T03:06:54Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- `select_best_node` still returns the lowest CPU and does not call the qwen3 selector. A node with no free-memory reading can win that path while `select_qwen3_mesh` picks the node that has free memory.
- An empty registry, or `NoFreshReading`, with `local_free_mb: Some` returns the local variant from `select_qwen3_code_qlora_for_free_mb`. `local_free_mb: None` returns `NoFreshReading` and does not probe.
- One fresh node is a mesh pick of that node. A smaller local reading does not replace it.
- `ExplicitDoesNotFit` and `NothingFits` propagate. A local reading that would have fit is not used.

## Task Commits

1. **Task 1: local fallback** - `09485e150` (feat)
2. **Task 2: one-node mesh pick and explicit miss** - `ab1b17c3f` (feat)

## Files Created/Modified

- `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs` - `Qwen3Placement` and `place_qwen3_code_qlora`

## Decisions Made

- The automatic picker error is `NothingFits`. `PickFailed` remains for a local selector failure.
- `Qwen3Placement` derives `Debug` so the tests can use `expect_err`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `expect_err` required `Debug` on the placement enum**
- **Found during:** Task 1
- **Issue:** `place_qwen3_without_local_reading_does_not_probe` did not compile. `Result::expect_err` needs `Debug` on the success type.
- **Fix:** `#[derive(Debug)]` on `Qwen3Placement`. The inner types already derive `Debug`.
- **Files modified:** crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs
- **Verification:** `place_qwen3_` 3 passed, then 5 passed after task 2
- **Committed in:** `09485e150`

**2. Tests and implementation share one commit per task**
- **Found during:** Tasks 1 and 2
- **Issue:** Same execute-plan commit shape as plans 01 and 02.
- **Fix:** Green commit per task.
- **Committed in:** `09485e150`, `ab1b17c3f`

**Total deviations:** 2
**Impact on plan:** The derive is a compile fix. Mapping the automatic picker error to `NothingFits` is the error the plan names, so a too-small fresh reading does not fall through to the local reading.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

MESH-01 placement is covered by injected tests. `select_best_node` and `dispatch_script` are unchanged. Phase 322 was not touched.

## Self-Check: PASSED

- FOUND: crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs
- FOUND: 09485e150
- FOUND: ab1b17c3f

## Test commands

- `cargo test -p vox-plugin-populi-mesh --lib select_best_node_ignores_accelerator_free_mb --quiet` — 1 passed
- `cargo test -p vox-plugin-populi-mesh --lib place_qwen3_ --quiet` — 3 passed after task 1, 5 passed after task 2
- `cargo test -p vox-plugin-populi-mesh --lib place_qwen3_only_this_node_is_a_mesh_pick --quiet` — 1 passed
- `cargo test -p vox-plugin-populi-mesh --lib place_qwen3_explicit_miss_does_not_fall_back_local --quiet` — 1 passed

---
*Phase: 319-mesh-wide-selection-mesh-01*
*Completed: 2026-10-30*
