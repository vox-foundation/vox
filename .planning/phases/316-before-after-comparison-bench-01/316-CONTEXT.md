# Phase 316: Before/After Comparison - Context

**Gathered:** 2026-10-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Score the pinned base and an adapter on the three checked-in 32-item reasoning slices, with accuracy and think-initiation stored as separate keys, when the pinned `Qwen/Qwen3-8B` snapshot and an adapter are both on disk. When the snapshot is absent, record that fact and write no numeric score. This checkout has no Qwen3-8B snapshot, so the run here takes the absent branch. The phase does not train a model, does not download weights, and does not change the serve gate.

</domain>

<decisions>
## Implementation Decisions

### Absent snapshot on this checkout
- This checkout records that the snapshot is absent and writes no accuracy and no think-initiation.
- Do not score Qwen3-0.6B, or any other model, as a stand-in for Qwen3-8B.
- Do not download weights as part of this phase.

### Both branches exist in code
- If the pinned snapshot and an adapter are on disk, score the three checked-in slices (`gsm8k_think`, `math500_think`, `vox_think`) on both sides.
- If either is missing, write the absence record instead.
- Tests for the present branch do not call a model and do not invent numbers.
- The run on this checkout takes the absent branch.

### Absence record shape
- The absence record names `Qwen/Qwen3-8B` and says the snapshot is not on disk.
- It has no `accuracy` key and no `think_initiation_rate` key.
- It is not a collateral pass and is not a serve-gate input.

### Grader, golds, and token cap stay as they are
- Do not change the grader or the slice golds.
- Do not add a token-cap field to the report.
- The deferred token-cap marker (review WR-02 from phase 315), the unclosed-think marker, and MATH-500 equivalent surface forms stay deferred. This phase does not produce a comparison to attach them to.

### Claude's Discretion
- Where the absence record lives on disk and what its file is called, as long as it cannot be mistaken for `collateral_damage_report.json` or a per-slice `reasoning_report.json` that serve reads.
- How "snapshot on disk" is detected (Hugging Face cache layout, an explicit path, or both), as long as detection never loads or runs the model and the pinned revision is the one checked.
- Whether the present branch reuses the phase 314 scorer and the phase 315 per-slice report shape directly, as long as both sides use the same checked-in items and the same keys.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `vox_populi::mens::DEFAULT_MODEL_ID` is the pinned base. `mens/config/gpu-specs.yaml` pins the Qwen3-8B rung as `Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218`.
- `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` scores one slice and writes `reasoning_report.json`. Its `NotScored` outcome has `n: 0`, `status: not_scored`, and no accuracy or think-initiation keys.
- `crates/vox-eval/src/reasoning_slice.rs` holds the slice names, the six reasoning keys, and the 32-item slice note used by the phase 315 gate.
- The three slices are checked in at `mens/data/{gsm8k,math500,vox}_think/reasoning.jsonl`, 32 rows each.

### Established Patterns
- A missing model or a failed load writes no passing score (phase 314, `BENCH-03`).
- A green unit test is not a model score.
- `VOX_REASONING_EVAL_MAX_TOKENS` defaults to 2048 and is not stored on the report.

### Integration Points
- Serve reads `collateral_damage_report.json` and `collateral_eval/<slice>/reasoning_report.json` only when `training_manifest.json` or `candle_qlora_adapter.safetensors` is present. The absence record must not land where serve would read it as a pass.
- The local Hugging Face cache on this checkout has `models--Qwen--Qwen3-0.6B` and no `models--Qwen--Qwen3-8B`.

</code_context>

<specifics>
## Specific Ideas

The user accepted the four recommended answers: record absence here with no stand-in model, keep both branches in code with model-free tests for the present branch, an absence record with no score keys that is not a gate input, and no grader, gold, or token-cap changes.

</specifics>

<deferred>
## Deferred Ideas

- Pick a hub and its spokes together from available VRAM. Today spoke training bases resolve through `pick_base` on `train_bases.agentic_default` in `mens/config/gpu-specs.yaml`, but every spoke hard-codes `preset: qwen3_16g`, and the hub model has no VRAM entry (the `hub:` block in `mens/config/domain-profiles.yaml` lists only the embedder). This needs its own phase. Note that the `Qwen/Qwen3.8-27B` rung has an unmeasured floor and no license entry.
- Store the token cap on the report (review WR-02 from phase 315).
- A report marker for an unclosed think block.
- MATH-500 equivalent surface forms.
- Swapping the pinned Qwen3-8B base, or trying a new training technique.

</deferred>
