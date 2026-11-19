---
phase: 85
phase_name: "native-mens-qlora-dpo-training-pipeline-curriculum-weighting-adapter-hot-swap"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 8
  lessons: 1
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 85 Learnings: native-mens-qlora-dpo-training-pipeline-curriculum-weighting-adapter-hot-swap

## Decisions

### Enforced a >= 10% minimum sampling floor across all 5 spokes in SpokeCur...
Enforced a >= 10% minimum sampling floor across all 5 spokes in SpokeCurriculumWeights while allocating remaining gradient mass inversely to pass@1.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-01-SUMMARY.md

---

### Implemented ChosenRejectedPairV2, SftDpoBatch, stream_snapshot_batches, ...
Implemented `ChosenRejectedPairV2`, `SftDpoBatch`, `stream_snapshot_batches`, and `prepare_training_batch` (`TRAIN-01`) with deterministic token-budget accounting.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-01-SUMMARY.md

---

### Implemented SpokeCurriculumWeights, compute_curriculum_weights, and samp...
Implemented `SpokeCurriculumWeights`, `compute_curriculum_weights`, and `sample_curriculum_records` (`TRAIN-02`) with a strict `>= 0.10` per-spoke floor.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-01-SUMMARY.md

---

### Implemented validate_snapshot_preflight_gate and validate_snapshot_safet...
Implemented `validate_snapshot_preflight_gate` and `validate_snapshot_safety_gate` (`TRAIN-05`) blocking training on holdout contamination, missing spoke coverage, or unverified oracle receipts.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-01-SUMMARY.md

---

### Persisted MensCheckpointManifestV1 in VoxDb content-addressed storage wi...
Persisted MensCheckpointManifestV1 in VoxDb content-addressed storage with Trusty-URI adapter digests and calibrated per-spoke pass@1 model registry entries.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 85-02-SUMMARY.md

---

### Implemented MensCheckpointManifestV1, run_deterministic_qlora_dpo_step, ...
Implemented `MensCheckpointManifestV1`, `run_deterministic_qlora_dpo_step`, `persist_checkpoint_to_db`, and `load_checkpoint_from_db` (`TRAIN-03`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-02-SUMMARY.md

---

### Implemented MensAdapterRegistryEntry / MensAdapterRegistryEntryV1, calib...
Implemented `MensAdapterRegistryEntry` / `MensAdapterRegistryEntryV1`, `calibrate_adapter_from_scorecard`, and `build_adapter_registry_entry` (`TRAIN-04`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-02-SUMMARY.md

---

### Verified all 230 unit tests in vox-corpus --features database --lib pass...
Verified all 230 unit tests in `vox-corpus --features database --lib` pass.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 85-02-SUMMARY.md

## Lessons

### timeout 900s cargo test -p vox-corpus --features database --lib -> 230 p...
`timeout 900s cargo test -p vox-corpus --features database --lib` -> `230 passed; 0 failed; 1 ignored`

**Context:** Verified in phase acceptance checks.
**Source:** 85-VERIFICATION.md

## Patterns

## Surprises

