# Phase 337 Plan 12 Summary: Sequential McNemar Live-Look Judging and Early Stopping

## Overview
Implemented sequential McNemar live-look judging and early stopping across evaluation and collateral damage checks:
- `judge_slice` in `crates/vox-eval/src/sequential_mcnemar.rs`:
  - Evaluates looks sequentially on the same contiguous pinned-order prefix for accuracy and think-initiation.
  - Stops at the first look where either accuracy or think-initiation rejects (`recomputed_p < look_alpha`), returning `SequentialSliceVerdict { slice, accuracy, think_initiation, stopped_at_look: Some(look_idx_1_based), items_used: look.n }`.
  - On full pass across all looks, returns `stopped_at_look: None, items_used: last_look.n`.
- `verify_sequential_block` in `crates/vox-eval/src/sequential_mcnemar.rs`:
  - Recomputes $p$-values for each look directly from stored `losses` and `gains`.
  - Verifies that stored `stopped_at_look` matches the first rejecting look (1-based), rejecting any regressions or tampered flags as `Err`.
  - On a valid passing sequential report, reconstructs `Vec<ReasoningSliceVerdict>` using final look counts with `PairedMetric::from_counts`.
- `verify_reasoning_slices` in `crates/vox-eval/src/reasoning_slice.rs`:
  - Dispatches `SEQUENTIAL_RULE` ("sequential_mcnemar_v1") to `verify_sequential_block`.
- Sequential collateral damage in `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs`:
  - `sequential_outcomes`: reads the contiguous pinned-order prefix recorded in base and adapter checkpoints without calling inference.
  - Live look judging inside `prefill_collateral_adapter`: on each recorded item, evaluates `rule.judge_slice` on the contiguous prefix. If a boundary is crossed (`verdict.rejected()`), immediately cancels the stream via `StreamControl::Cancel`.
  - Sequential report generation: when policy specifies `sequential_mcnemar_v1`, writes `sequential_slices_json` structure and verifies it with `verify_reasoning_slices`.
  - Replay equals live: offline verification of checkpoints generates identical reports to the live stream.

## Acceptance Criteria
- `grep -c 'fn judge_slice' crates/vox-eval/src/sequential_mcnemar.rs` -> 1
- `grep -c 'sequential_rule()' crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` -> 1
- `grep -v '^[[:space:]]*//' crates/vox-eval/src/sequential_mcnemar.rs | grep -c 'fn futility\|fn curtail'` -> 0
- `grep -c 'fn verify_sequential_block' crates/vox-eval/src/sequential_mcnemar.rs` -> 1
- `grep -c 'verify_sequential_block' crates/vox-eval/src/reasoning_slice.rs` -> 1

## Verification & Tests
- `vox-eval`:
  - `sequential_mcnemar::tests::verify_sequential_block_rejudges_from_counts`: verifies passing blocks, verifies tampered `rejected`/`p` flags are recomputed, verifies tampered losses trigger rejection error naming slice and look, and verifies invalid schedule size fails.
  - `sequential_mcnemar::tests::an_unknown_rule_is_refused`: verifies invalid rule names are rejected.
  - All 43 tests in `cargo test -p vox-eval --lib -- sequential_mcnemar reasoning_slice` pass.
- `vox-ml-cli`:
  - `eval_collateral::tests::a_crossed_boundary_cancels_the_stream`: verifies stream cancellation at early look boundary upon regression.
  - `eval_collateral::tests::replay_equals_live`: verifies replaying from checkpoints produces identical output to live streaming.
  - `eval_collateral::tests::fixed_n_reports_are_unchanged`: verifies non-sequential policies remain completely unchanged.
  - `eval_collateral::tests::a_look_is_judged_only_on_a_contiguous_pinned_order_prefix`: verifies incomplete prefixes before look boundaries do not trigger premature judging.
  - All 43 tests in `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` pass.

## Mutation Proof
- Metric: presence of `stopped_at_look` in `crates/vox-eval/src/sequential_mcnemar.rs`.
- Count before: 18.
- Mutation: changed `first_rej_look = Some(idx + 1);` (1-based) to `first_rej_look = Some(idx);` (0-based) in `verify_sequential_block`.
- Observed failure:
  - `verify_sequential_block_rejudges_from_counts` FAILED (`gsm8k_think: stored stopped_at_look Some(1) does not match recomputed Some(0)`).
  - `replay_equals_live` FAILED (`gsm8k_think: stored stopped_at_look Some(1) does not match recomputed Some(0)`).
- Restored: returned to `first_rej_look = Some(idx + 1);`.
- Recount after: 18.
- Re-run verification: both `verify_sequential_block_rejudges_from_counts` and `replay_equals_live` passed ok.
