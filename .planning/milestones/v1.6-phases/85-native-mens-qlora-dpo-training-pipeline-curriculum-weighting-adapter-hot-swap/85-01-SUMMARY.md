---
phase: 85-native-mens-qlora-dpo-training-pipeline-curriculum-weighting-adapter-hot-swap
plan: 01
subsystem: corpus
tags: [mens, training, sft, dpo, curriculum, preflight]
requires: [78-02]
provides:
  - ChosenRejectedPairV2, SftDpoBatch, stream_snapshot_batches, prepare_training_batch
  - SpokeCurriculumWeights, compute_curriculum_weights, sample_curriculum_records
  - PreflightSafetyVerdict, validate_snapshot_preflight_gate, validate_snapshot_safety_gate
affects: [crates/vox-corpus]
key-files:
  created:
    - crates/vox-corpus/src/training/curriculum.rs
  modified:
    - crates/vox-corpus/src/training/mix_prepare.rs
    - crates/vox-corpus/src/training/preflight.rs
    - crates/vox-corpus/src/training/mod.rs
    - crates/vox-corpus/src/lib.rs
key-decisions:
  - "Enforced a >= 10% minimum sampling floor across all 5 spokes in SpokeCurriculumWeights while allocating remaining gradient mass inversely to pass@1."
requirements-completed: [TRAIN-01, TRAIN-02, TRAIN-05]
completed: 2026-10-16
---

# Phase 85 Plan 01: Dataset Snapshot SFT/DPO Batch Bridge, Pre-Flight Safety Gate, and Spoke-Failure-Weighted Curriculum Sampler Summary

## Accomplishments
- Implemented `ChosenRejectedPairV2`, `SftDpoBatch`, `stream_snapshot_batches`, and `prepare_training_batch` (`TRAIN-01`) with deterministic token-budget accounting.
- Implemented `SpokeCurriculumWeights`, `compute_curriculum_weights`, and `sample_curriculum_records` (`TRAIN-02`) with a strict `>= 0.10` per-spoke floor.
- Implemented `validate_snapshot_preflight_gate` and `validate_snapshot_safety_gate` (`TRAIN-05`) blocking training on holdout contamination, missing spoke coverage, or unverified oracle receipts.
