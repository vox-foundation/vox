---
phase: 319-mesh-wide-selection-mesh-01
plan: 02
subsystem: mesh
tags: [qwen3, qlora, stale-window, explicit-size, heartbeat]

requires:
  - phase: 319-mesh-wide-selection-mesh-01
    provides: select_qwen3_mesh and accelerator_free_mb from plan 01
provides:
  - Size candidates limited to accelerator_free_mb readings inside the stale window
  - explicit_qwen3_code_qlora_for_free_mb, which does not step down
  - Heartbeat fill from the phase 318 reading, and a clear when a later heartbeat omits the field
affects: [319-03]

actuals:
  tokens: 3212
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "The caller passes the stale window. select_qwen3_mesh does not read server_stale_prune_ms."
    - "None and Some(0) both use 3_600_000 ms. No new env var."

key-files:
  created: []
  modified:
    - crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs
    - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
    - crates/vox-populi/src/lib.rs
    - crates/vox-populi/src/transport/handlers/nodes.rs
    - crates/vox-plugin-populi-mesh/src/transport/handlers/nodes.rs

key-decisions:
  - "An explicit size that does not fit returns ExplicitDoesNotFit and does not call the automatic picker."
  - "A heartbeat that omits accelerator_free_mb assigns None. gpu_vram_total_mb and memory_free_bytes are not cleared by that assignment."

patterns-established:
  - "Stale means now.saturating_sub(last_seen) > window. The boundary age of 3_600_000 ms is still fresh."

requirements-completed: [MESH-01]

coverage:
  - id: D1
    description: "Stale and missing free-memory readings are not size candidates. An unset window is 3_600_000 ms."
    requirement: MESH-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-populi-mesh --lib mesh_pick_ignores_stale_and_missing_free_memory"
        status: pass
    human_judgment: false
  - id: D2
    description: "An explicit size no eligible node can hold is an error. The automatic pick on the same nodes steps down."
    requirement: MESH-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-populi-mesh --lib mesh_pick_explicit_size_"
        status: pass
      - kind: unit
        ref: "cargo test -p vox-populi --features mens --lib explicit_qwen3_code_qlora_for_free_mb_refuses_over_floor"
        status: pass
    human_judgment: false
  - id: D3
    description: "A failed probe omits accelerator_free_mb. A later heartbeat that omits it clears the stored field."
    requirement: MESH-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-populi --lib accelerator_free_mb_from_reading_omits_on_failure"
        status: pass
      - kind: unit
        ref: "cargo test -p vox-populi --features transport --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it"
        status: pass
      - kind: unit
        ref: "cargo test -p vox-plugin-populi-mesh --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it"
        status: pass
    human_judgment: false

duration: 19min
completed: 2026-10-30
status: complete
plan_head_before: 6c43f9dee63a965f31ac981ebf169e07c4229bbd
plan_head_after: c54c973fccdf49fd63d36ce4d881dcb74eb1a2ae
---

# Phase 319 Plan 02: Fresh readings and explicit sizes Summary

**Size candidates need a free-memory reading inside the stale window, an explicit size that nobody can hold is an error, and a heartbeat that omits `accelerator_free_mb` clears it.**

## Performance

- **Duration:** 19 min
- **Started:** 2026-10-30T02:37:54Z
- **Completed:** 2026-10-30T02:57:02Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- A node is a size candidate only when `accelerator_free_mb` is present and `last_seen_unix_ms` is inside the injected window. `None` and `Some(0)` use 3_600_000 ms.
- `explicit_qwen3_code_qlora_for_free_mb` loads the same overlay as the automatic picker and refuses a floor above the reading. `select_qwen3_mesh` maps that failure to `ExplicitDoesNotFit` and does not step down.
- `accelerator_free_mb_from_reading` maps the phase 318 reading. `None` stays omitted. Both heartbeat merges assign the field unconditionally, so a later omission clears it. Total VRAM still copies only when present. System RAM is untouched.

## Task Commits

1. **Task 1: stale and missing readings** - `6c01d3eab` (feat)
2. **Task 2: explicit size refusal** - `de6e63b1a` (feat)
3. **Task 3: heartbeat fill and clear** - `c54c973fc` (feat)

## Files Created/Modified

- `crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs` - window filter and explicit branch
- `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` - `explicit_qwen3_code_qlora_for_free_mb`
- `crates/vox-populi/src/lib.rs` - reading helper and the mens-block assignment
- `crates/vox-populi/src/transport/handlers/nodes.rs` - omission clears the field
- `crates/vox-plugin-populi-mesh/src/transport/handlers/nodes.rs` - same clear on the plugin merge

## Decisions Made

- The caller passes the window. `select_qwen3_mesh` does not call `server_stale_prune_ms`.
- `Some(0)` is the unset window, matching the locked 3_600_000 ms default. No env var was added.

## Deviations from Plan

### Auto-fixed Issues

**1. Tests and implementation share one commit per task**
- **Found during:** Tasks 1–3
- **Issue:** Same as plan 01. Tasks are marked `tdd="true"` on an `execute` plan, and the run asked for one commit per task.
- **Fix:** Tests landed in the green commit.
- **Committed in:** `6c01d3eab`, `de6e63b1a`, `c54c973fc`

**Total deviations:** 1
**Impact on plan:** No behavior change beyond the commit shape. `Some(0)` is the locked default, asserted inside `mesh_pick_unset_window_uses_dispatch_hour`.

## Issues Encountered

The populi explicit-size test needs `--features mens` because `mens` is not a default feature. That command was run with the feature and passed 1 test.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Fresh readings and the explicit-size error are in place. Plan 03 decides when the caller uses the local reading instead.

## Self-Check: PASSED

- FOUND: crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
- FOUND: crates/vox-populi/src/transport/handlers/nodes.rs
- FOUND: crates/vox-plugin-populi-mesh/src/transport/handlers/nodes.rs
- FOUND: 6c01d3eab
- FOUND: de6e63b1a
- FOUND: c54c973fc

## Test commands

- `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_ignores_stale_and_missing_free_memory` — 1 passed
- `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_unset_window_uses_dispatch_hour` — 1 passed
- `cargo test -p vox-populi --features mens --lib explicit_qwen3_code_qlora_for_free_mb_refuses_over_floor` — 1 passed
- `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_explicit_size_` — 2 passed
- `cargo test -p vox-populi --lib accelerator_free_mb_from_reading_omits_on_failure` — 1 passed
- `cargo test -p vox-populi --features transport --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it` — 1 passed
- `cargo test -p vox-plugin-populi-mesh --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it` — 1 passed

---
*Phase: 319-mesh-wide-selection-mesh-01*
*Completed: 2026-10-30*
