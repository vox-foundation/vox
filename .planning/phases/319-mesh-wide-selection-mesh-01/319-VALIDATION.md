---
phase: "319"
slug: "mesh-wide-selection-mesh-01"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-30"
---

# Phase 319 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Requirement: MESH-01. Reviewed committed tests in `a5d60a1e8..f2ab6c443`. This audit did not re-execute Cargo and did not add tests.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (lib unit tests) |
| **Config file** | `.config/nextest.toml` (phase verify blocks use `cargo test`, not nextest) |
| **Quick run command** | Feature-qualified filters below. A default filter that prints `0 tests` and exits 0 is not a pass. |
| **Full suite command** | `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_ -- --test-threads=8` and `cargo test -p vox-plugin-populi-mesh --lib place_qwen3_ -- --test-threads=8` and `cargo test -p vox-plugin-populi-mesh --lib select_best_node_ignores_accelerator_free_mb -- --test-threads=8` and `cargo test -p vox-plugin-populi-mesh --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it -- --test-threads=8` and `cargo test -p vox-populi --lib accelerator_free_mb_from_reading_omits_on_failure -- --test-threads=8` and `cargo test -p vox-populi --features mens --lib explicit_qwen3_code_qlora_for_free_mb_refuses_over_floor -- --test-threads=8` and `cargo test -p vox-populi --features transport --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it -- --test-threads=8` and `cargo test -p vox-orchestrator --features populi-transport --lib vram_admission_allows_task_when_node_meets_requirement -- --test-threads=8` |
| **Estimated runtime** | Minutes when `mens`, `transport`, or `populi-transport` must compile. The plugin default lib already enables `mens` on `vox-populi`. |

`vox-plugin-populi-mesh` depends on `vox-populi` with `features = ["mens"]`, so `mesh_pick_` and `place_qwen3_` compile on the plugin's default lib. `spoke_base_resolver` is `#[cfg(feature = "mens")]` inside `vox-populi`, so the explicit-size test there needs `--features mens`. `vox-populi` transport handlers are `#[cfg(feature = "transport")]`. `vram_admission_allows_task_when_node_meets_requirement` is `#[cfg(feature = "populi-transport")]`.

---

## Sampling Rate

- **After every task commit:** Run that task's feature-qualified command in the map. Discard a `0 tests` result.
- **After every plan wave:** Run the full suite command.
- **Before `/gsd-verify-work`:** Full suite must be green, including `--features mens`, `--features transport`, and `--features populi-transport` where the map requires them.
- **Max feedback latency:** Minutes, bounded by the feature compile.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 319-01-01 | 01 | 1 | MESH-01 | T-319-02 | 70000 MB and 25000 MB yield node `wide` at 32B, preset `qwen3_96g`, floor 60000, matching `select_qwen3_code_qlora_for_free_mb(70000)`. A missing JSON key leaves `accelerator_free_mb` as `None`. `memory_free_bytes` stays system RAM. | unit | `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_routes_largest_floor_to_the_node_that_holds_it -- --test-threads=8` | ✅ | ✅ green |
| 319-01-01b | 01 | 1 | MESH-01 | T-319-01 | The orchestrator admission fixture still builds with `accelerator_free_mb: None`. The module compiles only with `populi-transport`. | unit | `cargo test -p vox-orchestrator --features populi-transport --lib vram_admission_allows_task_when_node_meets_requirement -- --test-threads=8` | ✅ | ✅ green |
| 319-01-02 | 01 | 1 | MESH-01 | T-319-02 | Equal 70000 MB readings tie-break to cpu 10. Quarantine, maintenance, a missing required label, and an empty donation slot list each drop the 70000 node and leave the 25000 node at 14B. Total VRAM, system RAM, and `min_vram_mb` do not select 32B. | unit | `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_ -- --test-threads=8` | ✅ | ✅ green |
| 319-02-01 | 02 | 2 | MESH-01 | T-319-04 | A 70000 reading at age 3_600_001 ms loses to a fresh 25000 reading. `None` and `Some(0)` both use 3_600_000 ms, and age 3_600_000 ms still selects 32B. Stale plus missing returns `NoFreshReading`. | unit | `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_ignores_stale_and_missing_free_memory -- --test-threads=8 && cargo test -p vox-plugin-populi-mesh --lib mesh_pick_unset_window_uses_dispatch_hour -- --test-threads=8` | ✅ | ✅ green |
| 319-02-02 | 02 | 2 | MESH-01 | T-319-05 | `explicit_qwen3_code_qlora_for_free_mb("32B", 25000)` errors. The same spelling at 70000 returns 32B / `qwen3_96g` / floor 60000. On the mesh, explicit 32B on a 25000 node is `ExplicitDoesNotFit` and the automatic pick on that node is 14B. Explicit 32B routes to the 70000 node. | unit | `cargo test -p vox-populi --features mens --lib explicit_qwen3_code_qlora_for_free_mb_refuses_over_floor -- --test-threads=8 && cargo test -p vox-plugin-populi-mesh --lib mesh_pick_explicit_size_ -- --test-threads=8` | ✅ | ✅ green |
| 319-02-03 | 02 | 3 | MESH-01 | T-319-03 | `accelerator_free_mb_from_reading` maps `Some(14000)` to `Some(14000)` and `None` to `None`. Both heartbeat merges assign the field on every merge: omission clears it, `Some(8192)` sets it, and `gpu_vram_total_mb` plus `memory_free_bytes` stay. | unit | `cargo test -p vox-populi --lib accelerator_free_mb_from_reading_omits_on_failure -- --test-threads=8 && cargo test -p vox-populi --features transport --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it -- --test-threads=8 && cargo test -p vox-plugin-populi-mesh --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it -- --test-threads=8` | ✅ | ✅ green |
| 319-03-01 | 03 | 3 | MESH-01 | T-319-06 | `select_best_node` returns the idle node with no free reading. `select_qwen3_mesh` on the same nodes returns the busy 70000 node at 32B. An empty registry and a no-fresh registry use the injected local reading. `local_free_mb: None` returns `NoFreshReading`. | unit | `cargo test -p vox-plugin-populi-mesh --lib select_best_node_ignores_accelerator_free_mb -- --test-threads=8 && cargo test -p vox-plugin-populi-mesh --lib place_qwen3_ -- --test-threads=8` | ✅ | ✅ green |
| 319-03-02 | 03 | 3 | MESH-01 | T-319-05 | One fresh node `local-1` at 70000 MB is a mesh 32B pick while the injected local reading is 25000. Explicit 32B on a fresh 25000 node stays `ExplicitDoesNotFit` when the local reading is 70000. An empty registry and a stale-or-missing registry apply the same explicit rule and do not step 32B down or 14B up. | unit | `cargo test -p vox-plugin-populi-mesh --lib place_qwen3_only_this_node_is_a_mesh_pick -- --test-threads=8 && cargo test -p vox-plugin-populi-mesh --lib place_qwen3_explicit_miss_does_not_fall_back_local -- --test-threads=8 && cargo test -p vox-plugin-populi-mesh --lib place_qwen3_ -- --test-threads=8` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

Runs this audit accepted. This audit did not re-execute Cargo.

| Command | Result |
|---------|--------|
| `cargo test -p vox-plugin-populi-mesh --lib place_qwen3_ -- --test-threads=8` | 8 passed, 0 failed, on the default lib after `ceead59d7` and `f2ab6c443`. Covers 319-03-01 and 319-03-02, including the three CR-01 tests. |
| `cargo test -p vox-plugin-populi-mesh --lib mesh_pick_ -- --test-threads=8` | 9 passed, 0 failed, on the default lib after those commits. Covers 319-01-01, 319-01-02, 319-02-01, the mesh half of 319-02-02, and WR-01 (`mesh_pick_drops_overflowing_free_memory_reading`, including the all-overflow `ReadingExceedsU32` assertion). |
| `cargo test -p vox-plugin-populi-mesh --lib select_best_node_ignores_accelerator_free_mb` | 1 passed at `09485e150` (`319-03-SUMMARY`). `select_best_node` is unchanged from `ab1b17c3f` through `f2ab6c443`. |
| `cargo test -p vox-plugin-populi-mesh --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it` | 1 passed at `c54c973fc` (`319-02-SUMMARY`). `nodes.rs` is unchanged after that commit. |
| `cargo test -p vox-populi --lib accelerator_free_mb_from_reading_omits_on_failure` | 1 passed on the default lib (`319-02-SUMMARY`). The helper is not feature-gated. The production assignment stays inside `#[cfg(feature = "mens")]` and the test does not call `read_accelerator_free_mb`. |
| `cargo test -p vox-populi --features transport --lib merge_clears_accelerator_free_mb_when_later_heartbeat_omits_it` | 1 passed (`319-02-SUMMARY`). |
| `cargo test -p vox-populi --features mens --lib explicit_qwen3_code_qlora_for_free_mb_refuses_over_floor` | 1 passed (`319-02-SUMMARY`). `spoke_base_resolver` is unchanged after `de6e63b1a`. |
| `cargo test -p vox-orchestrator --lib vram_admission_allows_task_when_node_meets_requirement` | 0 tests, exit 0. Not a pass. |
| `cargo test -p vox-orchestrator --features populi-transport --lib vram_admission_allows_task_when_node_meets_requirement` | 1 passed. |

`place_qwen3_` at `f2ab6c443` is these eight tests: `place_qwen3_empty_registry_uses_local_reading`, `place_qwen3_no_fresh_reading_uses_local_reading`, `place_qwen3_without_local_reading_does_not_probe`, `place_qwen3_only_this_node_is_a_mesh_pick`, `place_qwen3_explicit_miss_does_not_fall_back_local`, `place_qwen3_empty_explicit_32b_does_not_step_down`, `place_qwen3_empty_explicit_14b_does_not_step_up`, `place_qwen3_stale_or_missing_explicit_size_uses_local_rule`.

`mesh_pick_` at `f2ab6c443` is these nine tests: `mesh_pick_routes_largest_floor_to_the_node_that_holds_it`, `mesh_pick_tie_breaks_on_lowest_cpu`, `mesh_pick_skips_quarantine_maintenance_labels_and_donation`, `mesh_pick_ignores_total_vram_system_ram_and_min_vram`, `mesh_pick_ignores_stale_and_missing_free_memory`, `mesh_pick_unset_window_uses_dispatch_hour`, `mesh_pick_explicit_size_that_no_node_holds_is_an_error`, `mesh_pick_drops_overflowing_free_memory_reading`, `mesh_pick_explicit_size_routes_to_a_node_that_holds_it`.

T-319-01 and T-319-SC are accepted. These plans add no package install. D-07 forbids the tests from calling `read_accelerator_free_mb`. The mens block in `node_record_for_current_process` is the production call, and the omit mapping is the helper test.

`NothingFits` is the automatic picker's error when a fresh reading fits no measured floor. `place_qwen3_code_qlora` returns every mesh error other than `NoFreshReading`. `place_qwen3_explicit_miss_does_not_fall_back_local` locks that arm. No additional test is required.

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements.

- [x] No new test file. The MESH-01 tests are already in the committed crates listed above.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency: minutes, bounded by the `mens`, `transport`, or `populi-transport` compile
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-10-30

New tests required: no.

## Validation Audit 2026-10-30

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
