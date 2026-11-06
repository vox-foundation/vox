# Stack Research

**Domain:** Lane-scoped Qwen3 reasoning measurement for Mens hub-and-spoke
**Researched:** 2026-10-26
**Confidence:** HIGH

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Qwen3-8B (pinned snapshot already in `DEFAULT_MODEL_ID`) | `b968826d…` | Base model both sides of the before/after | The milestone forbids a base swap. The comparison is this checkpoint versus an adapter on it. |
| In-tree ChatML fusion (`vox_eval::qwen_reasoning`) | repo | `/no_think` plus empty `<think>\n\n</think>\n\n` for direct answers; bare `assistant\n` for thinking mode | Matches Qwen3 Thinking Mode Fusion. Already used by the trainer, eval-local, and chat completions. |
| `vox mens eval-collateral-damage` | repo | Serve gate | Already refuses a missing or failing `collateral_damage_report.json`. The gap is feeding it real benches, not a new gate. |
| Checked-in slices `mens/data/gsm8k_think` and `mens/data/math500_think` | 64 + 32 rows | Held-out reasoning items | Same items for base and adapter. GSM8K is MIT; MATH-500 is the public subset. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `vox-eval` grader (`grade_final_answer`, `opened_nonempty_think`) | repo | Numeric and boxed-answer match, plus think-initiation | Scoring only. Do not add a second grader. |
| Candle Mens plugin (`assemble_prompt`) | repo | Direct-answer inference prefix | Codegen serve path. Thinking-mode benches must pass a full ChatML prompt so this wrapper does not add `/no_think`. |
| `tokenizers` already in the train image | repo | Optional token-length checks | Only if a test must prove the empty think block is outside the loss. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| `VOX_REASONING_EVAL_LIMIT` | Truncate a bench while wiring | Not a substitute for the full checked-in slice in the report that gates serve. |
| `VOX_REASONING_EVAL_MAX_TOKENS` | Cap thinking generations | Default 2048. Temperature stays 0 so the gate does not flap. |

## Installation

No new packages. Do not add `lm-eval`, `inspect-ai`, or a Python eval sidecar.

```bash
# Already in the workspace. No install step.
cargo test -p vox-eval --lib qwen_reasoning
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| In-tree JSONL slices | Full GSM8K (1319) and full MATH-500 (500) | After the gate works. The 5% collateral threshold is noisy on 32–64 items; say so in the report rather than pretending the slice is the full set. |
| Greedy temperature 0 | Qwen's published thinking sample (0.6, top-p 0.95) | A one-off capability note. Not the serve gate. |
| Existing collateral report | A new scorecard format | Only if the report cannot store accuracy and think-initiation as two baseline keys. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| A new base model (Qwen3.5, larger Qwen, a 2026 reasoning specialist) | Explicitly out of this milestone | The pinned Qwen3-8B snapshot |
| A new trainer, Unsloth, or Axolotl run | The bug is which prefix a lane uses, and whether we measure it | Candle QLoRA already in tree |
| Config-field "benchmarks" (`max_tokens`, `temperature`, `seed_base`) as the pre-score | They made a vacuous pass when pre and post were copied from the same eval settings | `{"benchmarks":{"gsm8k_think":…,"gsm8k_think_init":…}}` |

## Stack Patterns by Variant

**If the lane is codegen:**
- Use `/no_think` and the prefilled empty think block
- Because the product contract is "emit Vox", and that prefix is the one training now supervises

**If the lane is hub or agent:**
- Use the bare `<|im_start|>assistant\n` prompt, no `/no_think`
- Because that is Qwen3's default thinking-mode entry, and these lanes are the ones that must open a non-empty `<think>`

## Version Compatibility

| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| Qwen3-8B chat template | Empty think block `<think>\n\n</think>\n\n` | A single newline (`<think>\n</think>\n`) is a different prefix. Do not mix them. |
| Adapters trained before 2026-10-26 | Old prefix with no `/no_think` | They are not evidence for the new policy. The before/after for this milestone is base versus a new run. |

## Sources

- Qwen3 Technical Report, arXiv:2505.09388 (Thinking Mode Fusion)
- Qwen3-8B `tokenizer_config.json` chat template (`enable_thinking`)
- In-repo audit `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md`
