# Phase 314: Held-out Reasoning Set - Context

**Gathered:** 2026-10-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Check in the GSM8K, MATH-500, and Vox slices this milestone scores, and make a missing model fail closed. This phase does not run the base-versus-adapter comparison and does not change the serve gate.

</domain>

<decisions>
## Implementation Decisions

### Slice files
- Check in three slices named `gsm8k_think`, `math500_think`, and `vox_think`.
- Tests and the scorer read those checked-in files. They do not download items at test time.
- Every Vox item has a checkable answer.
- Vox items are not the fixed three-line Plan / Syntax / Implementation template.

### Slice size
- Each slice has 32 items.
- The report stores `n`.
- This phase does not run the full GSM8K or MATH-500 sets.

### Missing model
- A failed load writes no accuracy number and no think-initiation number.
- The report records how many items were scored. A failed load scores 0 items.
- A green unit test is not a passing score.

### Out of scope
- No serve gate. That is phase 315.
- No base-versus-adapter run. That is phase 316.
- No new base model and no new training recipe.

### Claude's Discretion
- Exact directory and file extension, as long as the three names above are the identifiers the scorer loads.
- How a Vox item's answer is checked, as long as it is checkable and is not the Plan / Syntax / Implementation template.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `vox_eval::qwen_reasoning::score_completions` returns `ReasoningScore` with `accuracy`, `think_initiation_rate`, and `n`.
- `grade_final_answer` grades a numeric gold from the last number in the answer region, and other gold from a boxed body or the last non-empty line.
- `eval_collateral_damage` compares pre/post benchmark scores. It does not load items or a model.

### Established Patterns
- There is no checked-in GSM8K or MATH-500 item file today. Milestone research names the bench files `gsm8k_think`, `math500_think`, and a new `vox_think`.
- `prepare_bench_item` in `crates/vox-ml-cli/src/commands/mens/eval_local_prompt.rs` builds Vox program prompts. Its uncommitted edit is not part of phase 313 and is not this phase's contract.

### Integration Points
- A missing-model path must not call `score_completions` and then store the empty-batch zeros (`accuracy: 0.0`, `think_initiation_rate: 0.0`) as if the model had scored.
- Phase 316 is the only phase that may write numeric before/after scores, and only when the pinned Qwen3-8B snapshot and an adapter are on disk.

</code_context>

<specifics>
## Specific Ideas

The user accepted the recommended answers: checked-in 32-item slices, `n` stored on the report, and a failed load recorded as zero items scored with no accuracy and no think-initiation.

</specifics>

<deferred>
## Deferred Ideas

- Serve refusal when the collateral report is missing or worse (phase 315).
- The live base-versus-adapter comparison, or an honest "snapshot absent" record with no invented numbers (phase 316).
- Swapping the pinned Qwen3-8B base, or trying a new training technique.

</deferred>
