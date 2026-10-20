# Phase 85: Native Mens QLoRA + DPO Training Pipeline, Curriculum Weighting & Adapter Hot-Swap — Context

**Gathered:** 2026-10-16
**Status:** Ready for execution
**Mode:** Autonomous (`/gsd-autonomous`)

<domain>
## Phase Boundary

Deliver `TRAIN-01..05` in `crates/vox-corpus/src/training/`:
- `TRAIN-01`: `ChosenRejectedPairV2` and `stream_snapshot_batches` in `mix_prepare.rs` bridging `TrustyDatasetSnapshot` into deterministic SFT + DPO batches with token-budget accounting.
- `TRAIN-02`: `SpokeCurriculumWeights` and `compute_curriculum_weights` in `curriculum.rs` weighting spokes inversely to `MensSpokeScorecardV1` `pass@1` with a strict `>= 10%` per-spoke floor.
- `TRAIN-03`: `MensCheckpointManifestV1` in `adapter_registry.rs` recording dataset `snapshot_uri`, adapter digest URI, per-spoke weights, and SFT/DPO loss trajectory (plus `VoxDb` CAS persistence).
- `TRAIN-04`: `MensAdapterRegistryEntry` and `calibrate_adapter_from_scorecard` in `adapter_registry.rs` producing empirical per-spoke benchmark calibration scores for `vox-orchestrator::models`.
- `TRAIN-05`: `validate_snapshot_safety_gate` in `preflight.rs` blocking training if holdout contamination `> 0`, any of the 5 spokes has `0` rows, or any record fails oracle/digest integrity.

</domain>
