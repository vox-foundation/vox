# Phase 315: Serve Gate - Context

**Gathered:** 2026-10-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Point the existing collateral serve gate at accuracy and think-initiation for the checked-in reasoning slices, and make the collateral report say those benches are 32-item slices. This phase does not run a model at serve time and does not write a base-versus-adapter score.

</domain>

<decisions>
## Implementation Decisions

### Keys compared
- Compare accuracy and think-initiation separately on each reasoning slice.
- Both use the existing 5% relative bar (`max_degradation_rate` 0.05).
- A drop on either key fails the adapter.
- Non-reasoning benches keep their single score.

### Missing or unscored slices
- A missing slice report, a `not_scored` report, or any `n` other than 32 is a hard error.
- Serve refuses.
- Those cases are not stored as accuracy 0.0 or think-initiation 0.0.

### Non-finite and zero baselines
- A non-finite accuracy or think-initiation refuses the adapter.
- A reasoning pre-score of zero refuses the adapter.
- The current "pre-score near zero means no drop" path does not apply to these reasoning keys.
- That generic path may stay for non-reasoning benches. The existing zero-pre test on `eval_collateral_damage` is not a license to pass a reasoning key whose pre-score is zero.

### Slice label
- The collateral report says these benches are 32-item slices, not full GSM8K or MATH-500, and it records `n`.
- Serve only reads the report already on disk.
- No model run in this phase.
- No before/after numbers in this phase. That is phase 316.

### Claude's Discretion
- Whether each reasoning slice is two baseline keys (`gsm8k_think` and `gsm8k_think_init`) or one key the gate expands, as long as both scores are compared and a drop on either fails. `score_bench` already treats a `_init` suffix as think-initiation.
- The exact refusal text, as long as serve does not bind and the report names the benches as slices and records `n`.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` refuses local serve when `collateral_damage_report.json` is missing, and when its `status` is not `pass`.
- `eval_collateral_damage` compares one pre/post pair at 5% relative degradation. Improvement clamps to zero degradation. A pre-score near zero yields `degradation_rate` 0 and does not exceed the threshold.
- `score_bench` loads `mens/data/<name>/reasoning.jsonl` through `score_reasoning_file`. A `_init` suffix selects `think_initiation_rate`; otherwise it selects `accuracy`.
- Phase 314 `BenchOutcome::NotScored` has `n: 0` and no accuracy or think-initiation keys, and `score_reasoning_file` returns `Err`.

### Established Patterns
- The three slices are `gsm8k_think`, `math500_think`, and `vox_think`, 32 rows each, checked in.
- A green unit test is not a model score. Qwen3-8B weights are not required for this phase.

### Integration Points
- Serve must keep refusing a missing `collateral_damage_report.json`.
- A reasoning failure must not write a passing collateral report.
- Phase 316 is the only phase that may write numeric before/after scores, and only when the pinned Qwen3-8B snapshot and an adapter are on disk.

</code_context>

<specifics>
## Specific Ideas

The user accepted the recommended answers: both keys at the existing 5% bar, hard failure on a missing or not-scored slice or any `n` other than 32, refusal on non-finite values and on a reasoning pre-score of zero, and a slice label with `n` on the report. Serve reads the report. It does not run a model.

</specifics>

<deferred>
## Deferred Ideas

- The live base-versus-adapter comparison, or an honest "snapshot absent" record with no invented numbers (phase 316).
- A report marker for an unclosed think block (review WR-02, deferred to phase 316).
- MATH-500 equivalent surface forms (review WR-03). The grader and the slice stay locked.
- Swapping the pinned Qwen3-8B base, or trying a new training technique.

</deferred>
