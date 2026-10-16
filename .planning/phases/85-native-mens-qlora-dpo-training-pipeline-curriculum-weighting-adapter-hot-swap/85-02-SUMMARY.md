---
phase: 85-native-mens-qlora-dpo-training-pipeline-curriculum-weighting-adapter-hot-swap
plan: 02
subsystem: corpus
tags: [mens, training, qlora, dpo, checkpoint, voxdb, adapter-registry]
requires: [85-01]
provides:
  - MensCheckpointManifestV1, run_deterministic_qlora_dpo_step, persist_checkpoint_to_db, load_checkpoint_from_db
  - MensAdapterRegistryEntry, MensAdapterRegistryEntryV1, calibrate_adapter_from_scorecard, build_adapter_registry_entry
affects: [crates/vox-corpus]
key-files:
  created:
    - crates/vox-corpus/src/training/adapter_registry.rs
  modified:
    - crates/vox-corpus/src/training/curriculum.rs
    - crates/vox-corpus/src/training/mod.rs
    - crates/vox-corpus/src/lib.rs
key-decisions:
  - "Persisted MensCheckpointManifestV1 in VoxDb content-addressed storage with Trusty-URI adapter digests and calibrated per-spoke pass@1 model registry entries."
requirements-completed: [TRAIN-03, TRAIN-04]
completed: 2026-10-16
---

# Phase 85 Plan 02: MensCheckpointManifestV1 VoxDb Provenance and Adapter Calibration Bridge Summary

## Accomplishments
- Implemented `MensCheckpointManifestV1`, `run_deterministic_qlora_dpo_step`, `persist_checkpoint_to_db`, and `load_checkpoint_from_db` (`TRAIN-03`).
- Implemented `MensAdapterRegistryEntry` / `MensAdapterRegistryEntryV1`, `calibrate_adapter_from_scorecard`, and `build_adapter_registry_entry` (`TRAIN-04`).
- Verified all 230 unit tests in `vox-corpus --features database --lib` pass.
