# Architecture Research

**Domain:** Mens hub-and-spoke reasoning policy
**Researched:** 2026-10-26
**Confidence:** HIGH

## Standard Architecture

### System Overview

```
Spoke tag (codegen | hub | agent)
        |
        v
Prefix picker
        |-- codegen --> /no_think + empty think block  (loss starts at the answer)
        |-- hub/agent -> bare assistant\n              (model may open <think>)
        v
Train rows and serve prompts use the same picker
        |
        v
Before/after runner (base dir, then adapter dir)
        |-- gsm8k_think, math500_think, vox_think
        |-- keys: accuracy and *_init
        v
collateral_damage_report.json
        |
        v
vox mens serve refuses a missing or failing report
```

### Component Responsibilities

| Component | Responsibility |
|-----------|----------------|
| `vox_eval::qwen_reasoning` | Format helpers, thinking-mode prompt, grader, think-initiation |
| `training_text` (candle-core and populi) | Apply fusion on direct-answer rows; leave real think traces alone |
| `assemble_prompt` (CUDA and Metal) | Direct-answer serve prefix. Must not be the hub/agent path |
| `reasoning_eval.rs` | Generate on the thinking prefix and write `reasoning_report.json` |
| `eval_collateral.rs` | Map `bench` to accuracy and `bench_init` to think rate |
| Serve dispatch | Read `collateral_damage_report.json` before binding the adapter |

## Integration Points

- **Train:** `try_encode_training_step` already calls the fusion helpers. A spoke tag has to decide whether the row is a direct answer. Today every non-trace row is a direct answer, which is correct for codegen and wrong if hub rows are mixed into the same file without a tag.
- **Serve:** chat completions and `assemble_prompt` always prefill the empty think block. Hub and agent entry points need a branch, not a second trainer.
- **Gate:** `score_bench` already special-cases `reasoning.jsonl`. A Vox slice is another `reasoning.jsonl` with checkable answers, not a second harness.

## Suggested Build Order

1. Name the lanes and the prefix each one uses. Add a test that a hub prompt does not contain `/no_think` and a codegen prompt does.
2. Add the Vox item file and the base-score file shape. Do not invent the base numbers.
3. Point the collateral pre-score at `gsm8k_think`, `gsm8k_think_init`, `math500_think`, `math500_think_init`, and the Vox pair.
4. Run base, then adapter, when the Qwen3-8B snapshot is on disk. Until that run exists, the gate has nothing honest to compare.

## Sources

- `crates/vox-plugin-mens-candle-core/src/training_text.rs`
- `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs`
- `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs`
