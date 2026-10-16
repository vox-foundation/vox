# Phase 08..11 & 42 Summary: Paired Benchmarks, Memory Utility, Training Flywheel, Verification & Graduation

**Status:** Complete
**Requirements Covered:** `BENCH-01`, `BENCH-02`, `MEM-01`, `MEM-02`, `MEM-03`, `TRAIN-01`, `TRAIN-02`, `TRAIN-03`, `VERIF-01`, `VERIF-02`, `VERIF-03`, `GRAD-01`, `GRAD-02`

## Deliverables

1. **Paired Benchmarks (`BENCH-01`, `BENCH-02`)**:
   - Implemented `PairedItemScore`, `paired_bootstrap_ci` (deterministic xorshift64 paired bootstrap 95% CI), `paired_sign_test_p_value` (exact two-sided binomial sign test), `benjamini_hochberg_adjust` (step-up monotonicity-preserving FDR adjustment), and `validate_paired_arm_assignment` in [`crates/vox-orchestrator/src/preregistration/trial_core.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trial_core.rs).
2. **Memory Utility & Ablation (`MEM-01`, `MEM-02`, `MEM-03`)**:
   - Implemented `FrozenKbSnapshotManifest`, `verify_snapshot_immutability`, `MemoryAblationCondition`, and `compute_memory_utility_delta` in [`crates/vox-orchestrator/src/preregistration/trial_core.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trial_core.rs).
3. **Training Flywheel & Two-Campaign Promotion (`TRAIN-01`, `TRAIN-02`, `TRAIN-03`)**:
   - Implemented `PreferencePairSample`, `audit_preference_pair`, `TrainingProvenanceTag`, `validate_training_provenance`, `CampaignPromotionEvidence`, and `evaluate_two_campaign_promotion` in [`crates/vox-orchestrator/src/preregistration/trial_core.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trial_core.rs).
4. **Offline Evidence Verification & Calibration (`VERIF-01`, `VERIF-02`, `VERIF-03`)**:
   - Implemented `SignedEvidenceBundle`, `sign_evidence_bundle`, `verify_evidence_bundle_offline`, `detect_prompt_contamination`, and `calibrate_type_i_error` in [`crates/vox-orchestrator/src/preregistration/trial_core.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trial_core.rs).
5. **Flywheel Graduation (`GRAD-01`, `GRAD-02`)**:
   - Implemented `FlywheelGraduationInput`, `FlywheelGraduationReport`, and `evaluate_flywheel_graduation` in [`crates/vox-orchestrator/src/preregistration/trial_core.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trial_core.rs).

## Verification

- `cargo test -p vox-orchestrator --lib -- preregistration` (`48 passed; 0 failed`).
