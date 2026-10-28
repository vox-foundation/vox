---
phase: 07-trial-identity-contracts-observational-baseline
plan: 01
subsystem: research-trials
tags: [trials, preregistration, trusty-uri, ed25519, baseline-97, atomic-persistence, store-01, store-02]
requires: []
provides:
  - "Baseline 97 Tier-A relational schema (`trial_campaigns`, `trial_campaigns_log`, `trial_runs`, `trial_samples`, `llm_interactions.trial_run_label`)"
  - "Atomic `GuardedConnection::transaction` persistence with `PersistFailpoint` rollback verification (`STORE-02`)"
  - "Pure L2 `VerifiedCampaign`, `SignerAllowlist`, `SpendReservationLedger`, `GroupSequentialGate`, `EnvironmentFingerprint` (`TRIAL-01..05`)"
  - "Four-axis `score_research_result` evaluator (`07-12`) preserving `Option<f64>` missingness on judge errors"
affects:
  - crates/vox-db
  - crates/vox-db-types
  - crates/vox-orchestrator
  - crates/vox-research-shim
  - contracts/db/baseline-version-policy.yaml
key-files:
  created:
    - crates/vox-db/src/schema/domains/trials.rs
    - crates/vox-db/src/trials.rs
    - crates/vox-db-types/src/store_types/trials.rs
    - crates/vox-orchestrator/src/preregistration/trial_core.rs
    - crates/vox-research-shim/src/research/trial_scoring.rs
  modified:
    - crates/vox-db/src/schema/domains/mod.rs
    - crates/vox-db/src/schema/domains/agents.rs
    - crates/vox-db/src/schema/manifest.rs
    - crates/vox-db/src/codex_legacy.rs
    - crates/vox-db/src/store/types/error.rs
    - crates/vox-db/src/store/open.rs
    - crates/vox-db/src/lib.rs
    - contracts/db/baseline-version-policy.yaml
    - docs/src/architecture/research-trial-tier-a-schema-2026.md
completed: 2026-10-15
---

# Phase 07 Summary — Trial Identity, Contracts & Observational Baseline (`STORE-01`, `STORE-02`, `TRIAL-01..05`)

## What Was Built
1. **Tier-A Trial Schema (`07-06`, `STORE-01`):**
   - Added [`crates/vox-db/src/schema/domains/trials.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-db/src/schema/domains/trials.rs) (`SCHEMA_TRIALS`, `TRIAL_REQUIRED_TABLES`) and `llm_interactions.trial_run_label` (`idx_llm_interactions_trial`), bumped `BASELINE_VERSION` to `97`, updated [`contracts/db/baseline-version-policy.yaml`](file:///Users/brbrainerd/dev/vox/contracts/db/baseline-version-policy.yaml) (`0x538a3cfd1faf6bde088bc55401bbda62836a15eedc79417f60a88a3ebd2624fa`), and added the `StoreError::BaselineStampedWithoutTables` check in `store/open.rs`.
2. **Atomic Trial Persistence & Failpoints (`07-07`, `STORE-02`):**
   - Added `GuardedConnection::transaction` (`BEGIN IMMEDIATE ... COMMIT / ROLLBACK` under the connection mutex) and [`crates/vox-db/src/trials.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-db/src/trials.rs) (`register_campaign`, `seal_campaign`, `transition_campaign`, `record_trial_run_batch`, `partition_llm_spend`) with `PersistFailpoint` fault injection.
3. **Pure L2 Pre-Registration & Stopping Gates (`07-01`, `07-05`, `07-08..11`, `TRIAL-01..05`):**
   - Added generic `canonical_json_bytes` and `compute_value_trusty_uri` in [`crates/vox-orchestrator/src/preregistration/trusty_uri.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trusty_uri.rs) and `CampaignPreregV1`, `VerifiedCampaign`, `SignerAllowlist`, `SpendReservationLedger`, `GroupSequentialGate`, and `EnvironmentFingerprint` in [`crates/vox-orchestrator/src/preregistration/trial_core.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trial_core.rs).
4. **Four-Axis Research Trial Scoring (`07-12`):**
   - Added [`crates/vox-research-shim/src/research/trial_scoring.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-research-shim/src/research/trial_scoring.rs) (`TrialSampleEvaluation`, `score_research_result`).

## Verification Evidence
- `cargo --config 'build.rustc-wrapper=""' test -p vox-db --lib -- trials` -> `8 passed; 0 failed`
- `cargo --config 'build.rustc-wrapper=""' test -p vox-db --lib -- baseline` -> `7 passed; 0 failed`
- `cargo --config 'build.rustc-wrapper=""' test -p vox-db --lib --features legacy-import -- legacy_export_covers_all_baseline_tables` -> `1 passed; 0 failed`
