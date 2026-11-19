---
phase: 7
phase_name: "trial-identity-contracts-observational-baseline"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 14
  lessons: 3
  patterns: 0
  surprises: 1
missing_artifacts:
  - "UAT.md"
---

# Phase 7 Learnings: trial-identity-contracts-observational-baseline

## Decisions

### D-11: separate SecretIds from the nanopub key; the private key gets its ...
D-11: separate SecretIds from the nanopub key; the private key gets its own metadata arm (allow_env_in_strict=false, allow_compat_sources_in_strict=false, shareable=false), copied from VoxUserRsaNanopubPrivateKeyB64

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 07-02-SUMMARY.md

---

### The allowlist falls through to the default metadata arm, as VoxZenodoUpl...
The allowlist falls through to the default metadata arm, as VoxZenodoUploadAllowlist does (Operator class, env allowed); it holds only public keys

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 07-02-SUMMARY.md

---

### Task 1 (8aaaca4f7), tracer
**Task 1 (8aaaca4f7), tracer:** added both `SecretId` variants and `SecretSpec` entries, plus a dedicated `metadata()` arm for the signing key. Three tests (`research_trial_*`) pass. Regenerated the three `contracts/secrets/*` files with `vox ci secrets-contracts`. The only drift was the two new entries and the `generated_at_ms` field. `secrets-parity` and `secret-env-guard` both pass.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-02-SUMMARY.md

---

### Task 2 (9a62b1e2f)
**Task 2 (9a62b1e2f):** added one row to the Secret Inventory table in `docs/src/reference/secrets-ssot.md`. `secret-env-guard` passes, and the scoped doc lint reports no hard errors.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-02-SUMMARY.md

---

### STORE-01 gate (D-04) approved by the user on 2026-10-12
STORE-01 gate (D-04) approved by the user on 2026-10-12: item-batch run grain, model_benchmark_results option A (separate writer with a documented boundary)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 07-04-SUMMARY.md

---

### Task 1 (409a3a5d0)
**Task 1 (409a3a5d0):** contract doc for the `trials` domain at baseline 96.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-04-SUMMARY.md

---

### Defines trial_campaigns, trial_campaigns_log, trial_runs and trial_sampl...
Defines `trial_campaigns`, `trial_campaigns_log`, `trial_runs` and `trial_samples`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-04-SUMMARY.md

---

### No CHECK constraints, no triggers, no content columns
No CHECK constraints, no triggers, no content columns. Money is stored as integer micros, and NULL means Missing.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-04-SUMMARY.md

---

### Includes Phase 12's provenance_json column plus indexed scalars.
Includes Phase 12's `provenance_json` column plus indexed scalars.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-04-SUMMARY.md

---

### Adds a BaselineStampedWithoutTables guard.
Adds a `BaselineStampedWithoutTables` guard.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-04-SUMMARY.md

---

### Added [crates/vox-db/src/schema/domains/trials.rs](file:///Users/brbrain...
Added [`crates/vox-db/src/schema/domains/trials.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-db/src/schema/domains/trials.rs) (`SCHEMA_TRIALS`, `TRIAL_REQUIRED_TABLES`) and `llm_interactions.trial_run_label` (`idx_llm_interactions_trial`), bumped `BASELINE_VERSION` to `97`, updated [`contracts/db/baseline-version-policy.yaml`](file:///Users/brbrainerd/dev/vox/contracts/db/baseline-version-policy.yaml) (`0x538a3cfd1faf6bde088bc55401bbda62836a15eedc79417f60a88a3ebd2624fa`), and added the `StoreError::BaselineStampedWithoutTables` check in `store/open.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-SUMMARY.md

---

### Added GuardedConnection::transaction (BEGIN IMMEDIATE ..
Added `GuardedConnection::transaction` (`BEGIN IMMEDIATE ... COMMIT / ROLLBACK` under the connection mutex) and [`crates/vox-db/src/trials.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-db/src/trials.rs) (`register_campaign`, `seal_campaign`, `transition_campaign`, `record_trial_run_batch`, `partition_llm_spend`) with `PersistFailpoint` fault injection.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-SUMMARY.md

---

### Added generic canonical_json_bytes and compute_value_trusty_uri in [crat...
Added generic `canonical_json_bytes` and `compute_value_trusty_uri` in [`crates/vox-orchestrator/src/preregistration/trusty_uri.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trusty_uri.rs) and `CampaignPreregV1`, `VerifiedCampaign`, `SignerAllowlist`, `SpendReservationLedger`, `GroupSequentialGate`, and `EnvironmentFingerprint` in [`crates/vox-orchestrator/src/preregistration/trial_core.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/preregistration/trial_core.rs).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-SUMMARY.md

---

### Added [crates/vox-research-shim/src/research/trial_scoring.rs](file:///U...
Added [`crates/vox-research-shim/src/research/trial_scoring.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-research-shim/src/research/trial_scoring.rs) (`TrialSampleEvaluation`, `score_research_result`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 07-SUMMARY.md

## Lessons

### [Rule 3 - Blocking] Doc lint path
**[Rule 3 - Blocking] Doc lint path.** The plan's `--paths docs/src/reference/secrets-ssot.md` fails with "did not match any file or directory", because the tool resolves `--paths` relative to `docs/src`. I used `--paths reference/secrets-ssot.md` instead, and it passed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 07-02-SUMMARY.md

---

### Test-first was not shown as a separate red run
**Test-first was not shown as a separate red run.** The tests and the implementation were written in one edit, then run green. The mutation proof above is the evidence that the key test fails when the guard is broken.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 07-02-SUMMARY.md

---

### The commit hook ran rustfmt
**The commit hook ran rustfmt.** `fmt-fix` reformatted one assert in my new test, and the reformatted file landed in the same commit (8aaaca4f7). No extra commit was needed.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 07-02-SUMMARY.md

## Patterns

## Surprises

### [Rule 3 - Blocking] Doc lint path
**[Rule 3 - Blocking] Doc lint path.** The plan's `--paths docs/src/reference/secrets-ssot.md` fails with "did not match any file or directory", because the tool resolves `--paths` relative to `docs/src`. I used `--paths reference/secrets-ssot.md` instead, and it passed.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 07-02-SUMMARY.md

