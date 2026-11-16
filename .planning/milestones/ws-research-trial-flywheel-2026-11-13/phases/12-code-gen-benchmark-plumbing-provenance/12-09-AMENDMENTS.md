# 12-09 amendments (from Phase 7 planning, 2026-10-12)

Apply these to `12-09-PLAN.md` after the Phase 7 STORE-01 gate (07-04-T2) is decided and before 12-09 executes. They reconcile 12-09's assumed Phase 7 interface with what Phase 7 actually plans. Items 6–7 assume the gate picks item-batch grain and `model_benchmark_results` option A. If it picks otherwise, re-derive them.

1. **Canonicalizer (12-09 item 3).** Call `vox_crypto::canonical::canonical_json_bytes<T: Serialize + DeserializeOwned + PartialEq>(&T) -> Result<Vec<u8>, CanonicalError>`.
   - Derive `PartialEq` through the full type trees of `EvalCorpusReport` and `BenchBundle`.
   - NaN fails with `CanonicalError::Lossy`.
2. **Allowlist (12-09 item 4).** There is no ambient `verify_signer_allowlisted`.
   - Use `vox_research_trial_core::SignerAllowlist::contains(&self, pubkey_hex)`, passed explicitly into `verify_bundle(signed, allow: &SignerAllowlist)`.
   - The CLI loads the allowlist with `vox_cli_research::trial::load_signer_allowlist()`.
3. **Signer (12-09 item 2).** Phase 7 D-11 supersedes 12-09's "no new Clavis SecretId".
   - Load the signer with `vox_cli_research::trial::load_trial_signer() -> TrialSigner`, which reads `VoxResearchTrialSigningKeyHex`.
   - Sign with `TrialSigner::sign_hex(&bytes)`, and set `signer_key_id = pubkey_hex()`.
   - Tests use `TrialSigner::generate_ephemeral()`.
   - Verify with `vox_crypto::facades::verify_signature_hex`.
4. **Identity (12-09 item 1).** The type is `vox_research_trial_core::RunIdentity { campaign_id, run_id, arm_id, replicate_id: u32, attempt_index: u32, retry_of_run_id }`, with `first`, `retry` and `session_key`.
   - Map it to `vox_eval::provenance::RunIdentity` by dropping `arm_id`.
   - `preregistration_id` is the campaign_id (Trusty URI).
   - `--campaign` loads the SignedCampaign and runs `verify_campaign(&signed, &allow)` before opening a run. The campaign must be in state `running`.
5. **Crate path.** Use `vox_cli_research::trial_core::*`. vox-cli needs no new crate edge.
6. **Persist (12-09 item 5).**
   - Call `VoxDb::open_trial_run(&TrialRunRecord)` before sampling and `VoxDb::persist_trial_batch(&TrialRunBatch, PersistFailpoint)` after. The failpoint is `vox_db::PersistFailpoint::{None, AfterStatements(u32)}`, not `AfterRunRow`.
   - `provenance_json` and the scalars syntax_fingerprint, model_id, condition and problem_set_hash go on the run record. `run_complete` goes on `TrialRunOutcome`.
   - With item-batch grain:
     - one run per invocation and arm;
     - `item_set_hash` = problem_set_hash;
     - samples keyed by (fixture_id, sample_index).
7. **model_benchmark_results (option A).**
   - Drop the "projection" must-have, the Task 3 test `benchmark_results_projection_matches_samples`, and its precondition.
   - Replace them with a test that `list_trial_samples(run_id).len()` equals the report's sample count.
   - Keep "never writes model_benchmark_results". That table feeds live routing.
8. **Baseline wording.** Drop the obsolete "must not reuse 95" and "fold the uncommitted table" text. Phase 7 uses baseline 96, and 12-09 touches no schema files.
9. **Preconditions.** Name the 07-01, 07-05, 07-07 and 07-09 SUMMARYs and the 07-04 Decision record.
10. **Campaign kind.** The vocabulary is observational, exploratory and holdout_confirmation. 12-09 uses `observational` and calls the DB API directly, not the research `run_campaign`.
