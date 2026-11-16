# Phase 321: Harness Before/After (`HARN-01`, `BENCH-04`) - Context

**Gathered:** 2026-11-01
**Status:** Ready for execution

<domain>
## Phase Boundary

1. **`HARN-01` (Chosen-Size Serve Resolution Behind the Collateral Gate):**
   - Resolve a chosen Qwen3 size (`0.6B`, `8B`, `14B`, `32B`, etc., or pinned `hf_id@sha`) to either its pinned base snapshot on disk or its adapter run directory on disk.
   - When serving a base snapshot, require `snapshot_complete` at the exact pinned revision.
   - When serving an adapter for a chosen size, require `adapter_manifest_names_snapshot` to match that size's pinned base snapshot AND pass both `assert_collateral_gate` and `assert_serve_preconditions` before `ServeConfig` is constructed.
2. **`BENCH-04` (Multi-Size Harness Before/After Report):**
   - Write one consolidated report (`mens/runs/harness_before_after.json`, schema `harness_before_after.v1`) across all six `qwen3_code` QLoRA sizes (`0.6B`, `8B`, `14B`, `32B`, `1.7B`, `4B`) ordered smallest measured `floor_mb` first, then unmeasured sizes in Qwen3 size order.
   - For every trained size (`status == "trained"` in `size_train` or Ready snapshot + matching adapter): record `base` and `adapter` reasoning accuracy and think-initiation on the three 32-item slices (`gsm8k_think`, `math500_think`, `vox_think` with `n: 32`), plus `intelligence` (`pass_rate_at_1` and `n`) on a codegen/agent benchmark (`heldout_bench`) for both `base` and `adapter`.
   - Sizes that are not trained (`floor_unmeasured`, `does_not_fit`, `base_snapshot_absent`, `train_not_run`) are listed with `status: "not_trained"`, their `reason`, and **no numeric score keys** (`slices` and `intelligence` omitted).
   - Record `trained_scored_count` and `milestone_ready: trained_scored_count >= 2` so the two-trained-sizes bar is explicitly verified.
</domain>
