---
phase: 319-mesh-wide-selection-mesh-01
verified: 2026-10-29
status: passed
score: 12/12 must-haves verified
behavior_unverified: 0
overrides_applied: 0
ported_from: "cursor/t3-deeper-extraction-v55-510f @ 38001f8dc (fixes ceead59d7, f2ab6c443)"
covered_files:
  - crates/vox-orchestrator/src/orchestrator/tests/populi_single_owner.rs
  - crates/vox-plugin-populi-mesh/Cargo.toml
  - crates/vox-plugin-populi-mesh/src/transport/handlers/dispatch.rs
  - crates/vox-plugin-populi-mesh/src/transport/handlers/nodes.rs
  - crates/vox-populi-types/src/node_record.rs
  - crates/vox-populi/src/lib.rs
  - crates/vox-populi/src/transport/handlers/nodes.rs
---

# Phase 319: Mesh-Wide Selection Verification Report (ported)

**Phase Goal:** Across a mesh, the selector picks the largest size that fits on any node's current free memory and routes the work there.
**Status:** passed, on `gsd/mens-compare-316`, against this branch's `size_select::select_size` and measured floors.
**Cursor's original report** (against its own selector and placeholder floors) is kept in git at `38001f8dc:.planning/phases/319-mesh-wide-selection-mesh-01/319-VERIFICATION.md`. Its mapping to this branch is in `319-PORT.md`.

Floors used by the tests (`mens/config/gpu-specs.yaml`, measured on silicon-gpu-metal): 1.7B 27007 (`qwen3_6g`) and 4B 33037 (`qwen3_12g`). The node readings are 34000 (picks 4B) and 28000 (picks 1.7B). No test depends on the 8B+ estimates.

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Heartbeats carry current free accelerator memory, not only total VRAM. | VERIFIED | `NodeRecord.accelerator_free_mb: Option<u64>` sits beside `gpu_vram_total_mb`. The mens block of `node_record_for_current_process` sets it from `size_select::read_free_memory().map(\|f\| f.mb)`. Both heartbeat merges assign it unconditionally, so omission clears it. Tests: `merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it` in vox-populi and in the plugin. |
| 2 | The largest size that fits some node is picked, and the job is routed to a node that holds it. | VERIFIED | `select_qwen3_mesh` takes the max of the usable readings and makes one `select_size` call, then keeps the nodes at or above `floor_mb` and takes the lowest CPU. `mesh_pick_routes_largest_floor_to_the_node_that_holds_it`: wide=34000 gives 4B on `wide`, pin `Qwen/Qwen3-4B@1cfa9a72...`. The rung equals the local `select_size` pick on the same reading. |
| 3 | A stale or missing reading is not free memory. The local fallback works with no mesh. | VERIFIED | `mesh_pick_ignores_stale_and_missing_free_memory` and `place_qwen3_empty_registry_uses_local_reading` / `place_qwen3_no_fresh_reading_uses_local_reading`. |
| 4 | Policy exclusions and the CPU tie-break apply. | VERIFIED | `mesh_pick_skips_quarantine_maintenance_labels_and_donation` (each case selects `narrow` at 1.7B) and `mesh_pick_tie_breaks_on_lowest_cpu`. `node_passes_dispatch_policy` is shared with `select_best_node`. |
| 5 | Total VRAM, system RAM and `min_vram_mb` do not choose the size. | VERIFIED | `mesh_pick_ignores_total_vram_system_ram_and_min_vram`. |
| 6 | An unset or zero window uses 3_600_000 ms. | VERIFIED | `mesh_pick_unset_window_uses_dispatch_hour`. |
| 7 | An explicit size that no node holds is an error. The automatic pick steps down. | VERIFIED | `mesh_pick_explicit_size_that_no_node_holds_is_an_error`: `Qwen3-4B` on 28000 gives `ExplicitDoesNotFit`, and automatic gives 1.7B. `mesh_pick_explicit_size_routes_to_a_node_that_holds_it`. New on this branch: `mesh_pick_explicit_off_ladder_base_is_refused` (`gpt2` gives `NotALadderSize`, because `select_size` passes a non-ladder base through with no floor). |
| 8 | `select_best_node` does not use the free reading. | VERIFIED | `select_best_node_ignores_accelerator_free_mb`. |
| 9 | A one-node fresh registry is a mesh pick. An explicit miss does not fall back to local. | VERIFIED | `place_qwen3_only_this_node_is_a_mesh_pick` and `place_qwen3_explicit_miss_does_not_fall_back_local`. |
| 10 | Tests inject records, clocks, readings and the overlay. They do not probe a GPU or touch HF. | VERIFIED | The overlay comes from the repo `gpu-specs.yaml` via `load_ladder_overlay(Some(root))`. Local readings are injected `FreeMemory` values. |
| 11 | An explicit size on the local fallback is refused, never stepped down (CR-01). | VERIFIED | `place_qwen3_empty_explicit_4b_does_not_step_down`, `place_qwen3_empty_explicit_1_7b_does_not_step_up`, `place_qwen3_stale_or_missing_explicit_size_uses_local_rule`. |
| 12 | A reading that overflows u32 is dropped before the max (WR-01). | VERIFIED | `mesh_pick_drops_overflowing_free_memory_reading`: `u64::MAX` beside 28000 gives `usable` at 1.7B. A lone `u64::MAX` gives `ReadingExceedsU32`. |

## Test evidence (2026-10-29)

- **RED** (guards removed: stale filter, u32 filter, both merge lines): plugin, 15 passed and 5 failed. The failures were `mesh_pick_drops_overflowing_free_memory_reading`, `mesh_pick_ignores_stale_and_missing_free_memory`, `mesh_pick_unset_window_uses_dispatch_hour`, `place_qwen3_stale_or_missing_explicit_size_uses_local_rule`, and the plugin `merge_clears_...`. vox-populi: `merge_clears_...` FAILED.
- **GREEN:** `cargo test -p vox-plugin-populi-mesh --lib -- qwen3_mesh accelerator_free_mb select_best_node` gave 20 passed. `cargo test -p vox-populi --features transport,mens --lib merge_clears_accelerator` gave 1 passed.
- **Mutation, stale guard only:** 3 failed (`mesh_pick_ignores_stale_and_missing_free_memory`, `mesh_pick_unset_window_uses_dispatch_hour`, `place_qwen3_stale_or_missing_explicit_size_uses_local_rule`). Presence was 0 while mutated and 1 after restore.
- **Mutation, u32 guard only:** 1 failed (`mesh_pick_drops_overflowing_free_memory_reading`). Presence was 0 while mutated and 1 after restore. The file is byte-identical to GREEN after restore.

**Score:** 12/12
