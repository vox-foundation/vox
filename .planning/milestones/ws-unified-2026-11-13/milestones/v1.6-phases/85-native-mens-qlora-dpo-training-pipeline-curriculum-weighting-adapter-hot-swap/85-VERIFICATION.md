---
phase: 85-native-mens-qlora-dpo-training-pipeline-curriculum-weighting-adapter-hot-swap
verified: 2026-10-16T14:09:30Z
status: passed
score: 5/5 must-haves verified
---

# Phase 85: Native Mens QLoRA + DPO Training Pipeline, Curriculum Weighting & Adapter Hot-Swap — Verification Report

**Phase Goal:** Bridge `VoxDb` `TrustyDatasetSnapshot` directly into SFT + DPO training batches, dynamically weight spokes inversely by `MensSpokeScorecardV1` `pass@1` (`>= 10%` floor), persist `MensCheckpointManifestV1` with Trusty-URI digests, and register empirical per-spoke adapter scores.
**Verified:** 2026-10-16
**Status:** passed

## Observable Truths & Requirements Coverage (`TRAIN-01..05`)

| Requirement | Description | Status | Evidence |
|-------------|-------------|--------|----------|
| `TRAIN-01` | `TrustyDatasetSnapshot` streams into SFT (`TrainingRecordV2`) and DPO (`ChosenRejectedPairV2`) batches with token accounting | ✓ VERIFIED | `crates/vox-corpus/src/training/mix_prepare.rs` (`stream_snapshot_batches`), `curriculum.rs` (`prepare_training_batch`) |
| `TRAIN-02` | Dynamic inverse-failure curriculum weighting (`SpokeCurriculumWeights`) with `>= 10%` minimum floor per spoke | ✓ VERIFIED | `crates/vox-corpus/src/training/curriculum.rs` (`compute_curriculum_weights`, `sample_curriculum_records`) |
| `TRAIN-03` | `MensCheckpointManifestV1` records `snapshot_uri`, `adapter_uri`, spoke weights, and monotonic loss trajectory in `VoxDb` | ✓ VERIFIED | `crates/vox-corpus/src/training/adapter_registry.rs`, `curriculum.rs` (`run_deterministic_qlora_dpo_step`, `persist_checkpoint_to_db`) |
| `TRAIN-04` | Calibrated `MensAdapterRegistryEntry` with empirical per-spoke `pass@1` scores for `vox-orchestrator` model routing | ✓ VERIFIED | `crates/vox-corpus/src/training/adapter_registry.rs` (`calibrate_adapter_from_scorecard`), `curriculum.rs` (`build_adapter_registry_entry`) |
| `TRAIN-05` | Pre-flight safety gate blocks training on holdout contamination, missing spoke coverage, or unverified oracle receipts | ✓ VERIFIED | `crates/vox-corpus/src/training/preflight.rs` (`validate_snapshot_safety_gate`), `curriculum.rs` (`validate_snapshot_preflight_gate`) |

## Verification Command Output

- `timeout 900s cargo test -p vox-corpus --features database --lib` -> `230 passed; 0 failed; 1 ignored`
