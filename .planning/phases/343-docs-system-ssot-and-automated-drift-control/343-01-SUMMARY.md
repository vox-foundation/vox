# Phase 343 Plan 01 Summary: SSOT Drift Stage Table and Automated Regeneration Parity

## Overview

Plan `343-01-PLAN.md` has been executed in full.
- Replaced the inline 35-call `ds!` chain in `run_ssot_drift` with a structured `SsotDriftStage` table in `crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs`.
- `run_stages` executes every stage unconditionally, aggregates all failures, and reports every failing stage together with elapsed runtimes.
- Added `ssot_regen_parity` as the final stage of `ssot_drift_stages()`, verifying both `scripts/ssot-regen.vox` and the pre-push retry chain in `lefthook.yml`.
- Reconciled `scripts/ssot-regen.vox` by adding `crate-api-sync --write` and `wiki-sync --write`.
- Updated documentation in `docs/src/architecture/docs-system-ssot.md` and `docs/src/contributors/detector-coverage-ledger.md`.

## Task 1: Stage Table & Regeneration Parity Tracer

- **RED phase**: Added unit test suite in `ssot_drift.rs` covering all required behaviors (run stages aggregation, regen parity parser, unknown argv, missing argv, real-tree contract). Verified failures before implementation.
- **GREEN phase**:
  - Implemented `SsotDriftStage` with fields `name`, `run`, and `regen`.
  - Implemented `run_stages` with timing and complete failure aggregation.
  - Implemented `regen_parity_failures` parsing `scripts/ssot-regen.vox` calls to `regen(...)` and checking two-way parity with stages having `Some(regen)`.
  - Reconciled `scripts/ssot-regen.vox` to include `crate-api-sync --write` and `wiki-sync --write`.
  - Refactored `run_ssot_drift` in `docs.rs` to call `run_stages(root, auto_heal, &ssot_drift_stages())?`.
- **Mutation Proof 1 (scripts/ssot-regen.vox)**:
  - Deleted `wiki-sync` call from `scripts/ssot-regen.vox`.
  - Ran `cargo nextest run -p vox-cli --lib ssot_drift::tests::real_tree_regen_parity`.
  - Test failed with exit code 100 citing `stage has regen argv but scripts/ssot-regen.vox does not call it: vox ci wiki-sync --write`.
  - Restored `scripts/ssot-regen.vox` and confirmed diff clean.
- **Mutation Proof 2 (ssot_drift_stages table)**:
  - Temporarily set `crate_api_docs` stage `regen` to `None`.
  - Ran `real_tree_regen_parity`.
  - Test failed with exit code 100 citing `scripts/ssot-regen.vox calls regen argv that matches no stage: vox ci crate-api-sync --write`.
  - Restored stage definition.

## Task 2: Decision on Lefthook Retry Parity

- **Selection**: `option-a` ("keep the hand list, and make the ssot_regen_parity stage check it (every `vox ci ...` generator in the retry chain must equal a stage regen argv; the hook may run a subset)").
- Pre-authorized by user instruction for AFK execution.

## Task 3: Lefthook Parity & Docs Update

- **Implementation**:
  - Implemented `lefthook_retry_failures` parsing `lefthook.yml` under `pre-push -> commands -> pre-push -> run`, splitting retry commands on `&&`, filtering non-SSOT commands (`scripts/fmt.vox`, `line-endings`, `sync-ignore-files`, `pre-push`), and asserting each `vox ci <cmd>` matches a stage's `regen` argv.
  - Wired into `ssot_regen_parity` function alongside `regen_parity_failures`.
  - Added unit test `lefthook_retry_failures_option_a_success`, `lefthook_retry_failures_option_a_unknown_generator`, and `real_tree_lefthook_parity`.
- **Mutation Proof 3 (lefthook.yml)**:
  - Appended `&& vox ci not-a-stage --write` into `lefthook.yml`'s pre-push retry chain.
  - Ran `cargo nextest run -p vox-cli --lib ssot_drift::tests::real_tree_lefthook_parity`.
  - Test failed with exit code 100 citing `generator call in lefthook retry chain matches no stage regen argv: vox ci not-a-stage --write`.
  - Restored `lefthook.yml` and verified diff clean.
- **Documentation Updates**:
  - `docs/src/architecture/docs-system-ssot.md`:
    - §3 "Regeneration loop": Documented `run_stages` and `ssot_regen_parity` verification.
    - §4 table: Updated `vox ci ssot-drift` description with stage count (≈40 stages, `ssot_drift_stages()`), full-batch aggregation, and `ssot_regen_parity`.
    - §4 note: Added note that pre-push retry generators are checked against `ssot_drift_stages()` by `ssot_regen_parity`.
    - §6: Removed the "Hand-kept sync lists" gap bullet.
    - §7: Added instructions to register stages in `ssot_drift_stages()` and `scripts/ssot-regen.vox`.
  - `docs/src/contributors/detector-coverage-ledger.md`:
    - Added row for `ssot_regen_parity` (blocking, pre-push fast / CI).
  - Verified with `vox-doc-pipeline --lint-only` (exit 0).

## Verification Results

- `cargo nextest run -p vox-cli --lib ssot_drift`: 12 passed, 0 failed.
- `vox check scripts/ssot-regen.vox`: exit 0.
- `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/docs-system-ssot.md contributors/detector-coverage-ledger.md`: exit 0 (no hard errors).
