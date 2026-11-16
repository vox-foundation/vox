# Phase 337 Plan 10 Summary: Single-Load Continuous Decode Streams and Collateral Fail-Fast Cancel

## Overview
Implemented single-load continuous decode streams and fail-fast cancellation across reasoning compare and collateral damage evaluation:
- `prefill_benches` in `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs`: opens checkpoints with `reasoning_checkpoint(..., settings, force_restart)`, gathers pending items across all benches, announces complete benches with `on_record(bench, true)` first, and runs a single continuous `decode_stream` across all pending items. Records into checkpoints and informs callers of per-bench completion via `on_record(bench, complete)` respecting returned `StreamControl`.
- Single-load reasoning compare in `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs`: added `score_side_with` calling `prefill_benches` once per side with `force_restart`, followed by `score_slices_checkpointed` with `force_restart: false` (retaining decoded items without discarding them again). `score_side_with_backend` wraps this in a single `with_backend` call per side so two models are never resident simultaneously.
- Single-load collateral damage with fail-fast cancellation in `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs`:
  - `prefill_collateral_base`: prefills base model items across all slices in a single continuous stream with `force.for_base()`.
  - `prefill_collateral_adapter`: prefills adapter model items with `force.for_adapter()`, judging completed slices in fail-fast order via `slice_verdict` against checkpoints. On the first failing slice, immediately returns `StreamControl::Cancel` which issues `batch_cancel` to the backend.
  - `prefill_collateral_with`: testable core orchestrating base prefill followed by adapter prefill.
  - `run_collateral_damage`: invokes `with_backend` exactly twice (once for base prefill, once for adapter prefill), then runs the scoring pass with `ForceRestart { adapter: false, base: false }` directly from checkpoints. Base generations are prefilled for all pending slices in one stream because they are cached per snapshot and shared by every adapter.

## Verification & Tests
- `reasoning_compare::tests::compare_decodes_one_stream_per_side`: verifies scoring one side sends exactly one batch request carrying all prompts across all slices, records each into checkpoints, and a second scoring of the same side sends zero requests.
- `reasoning_compare::tests::a_restart_discards_once`: verifies `force_restart: true` discards once in prefill and does not discard again in the subsequent scoring pass.
- `eval_collateral::tests::collateral_decodes_one_stream_per_side`: verifies base receives one batch request for all prompts, adapter receives one batch request for all prompts, and report generated from checkpoints matches expectation.
- `eval_collateral::tests::a_failing_first_slice_cancels_the_adapter_stream`: verifies fail-fast early stop emits `batch_cancel` to the adapter stream after the first failing slice, stops streaming, marks later slices `not_run`, and reports failure.
- `eval_collateral::tests::a_restart_discards_once`: verifies `ForceRestart { adapter: true, base: false }` discards only the adapter checkpoint once in prefill, leaves base checkpoint untouched, and the scoring pass reads records without discarding again.

## Mutation Proofs
1. **Mutation Proof (a) - Fail-Fast Stream Cancellation**:
   - Checked `grep -c 'StreamControl::Cancel' crates/vox-ml-cli/src/commands/mens/eval_collateral.rs`: 1 before, 1 after.
   - Mutation: replaced `return Ok(super::reasoning_eval::StreamControl::Cancel);` with `return Ok(super::reasoning_eval::StreamControl::Continue);`.
   - Result: `a_failing_first_slice_cancels_the_adapter_stream` FAILED (`assertion left == right failed: left: 0, right: 1`).
   - Restored and re-verified: test passed green.
2. **Mutation Proof (b) - Discard Once Guard**:
   - Checked `grep -c 'force_restart = false: scoring pass keeps every record' crates/vox-ml-cli/src/commands/mens/eval_collateral.rs`: 1 before, 1 after.
   - Mutation: replaced `false, // force_restart = false: scoring pass keeps every record` with `true, // force_restart = false: scoring pass keeps every record`.
   - Result: `a_restart_discards_once` FAILED (`inference error: scoring pass must not generate; left: 0, right: 128`).
   - Restored and re-verified: test passed green.
