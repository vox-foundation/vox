# Pitfalls Research

**Domain:** Before/after reasoning gates on a hybrid Qwen3 adapter
**Researched:** 2026-10-26
**Confidence:** HIGH

## Critical Pitfalls

### Pitfall 1: Measuring thinking mode with the non-thinking prefix

**What goes wrong:**
The bench "proves" the model no longer thinks, because every prompt prefilled `<think>\n\n</think>\n\n` or appended `/no_think`.

**Why it happens:**
`assemble_prompt` and chat completions now do that on purpose for codegen. A reasoning bench that reuses them is not in thinking mode.

**How to avoid:**
Thinking-mode items pass a full ChatML string that starts with `<|im_start|>` and ends at a bare `assistant\n`. `thinking_mode_prompt` already does this. Keep it that way.

**Warning signs:**
Think-initiation is 0% and every completion starts with the answer or with an empty think block the prompt already contained.

**Phase to address:**
The bench phase, before any serve gate is trusted.

---

### Pitfall 2: A passing report that did not load the model

**What goes wrong:**
`collateral_damage_report.json` says pass because pre and post were copied from the same JSON, including `max_tokens` and `temperature`.

**Why it happens:**
The old pre-score parser treated every top-level number as a benchmark. That parser was fixed. A hand-written baseline can still repeat the mistake.

**How to avoid:**
The baseline file has a `benchmarks` object whose keys are only `gsm8k_think`, `math500_think`, `vox_think`, and the matching `*_init` keys. A load failure writes no report.

**Warning signs:**
Pre and post match to many decimals on keys that are not task scores.

**Phase to address:**
The gate phase.

---

### Pitfall 3: Treating a 64-item slice as a stable 5% gate

**What goes wrong:**
One or two items flip and a good adapter is blocked, or a real drop hides inside the noise.

**Why it happens:**
The collateral default is a 5% relative drop. On 64 GSM8K items that is a handful of answers. MATH-500's 32-item slice is worse.

**How to avoid:**
Publish the slice size in the report. Use think-initiation (a coarse rate) as the reasoning alarm, and treat accuracy on the slice as a smoke bar, not a paper claim. Do not silently enlarge the gate to the full sets in this milestone.

**Warning signs:**
Two runs of the same weights disagree by more than the threshold.

**Phase to address:**
The report phase. State the limit in the artifact.

---

### Pitfall 4: One prompt policy for every spoke

**What goes wrong:**
Hub and agent lanes lose the thinking entry point because codegen's direct-answer wrapper became global.

**Why it happens:**
The format fix was applied at `assemble_prompt`, which every raw user string hits.

**How to avoid:**
The spoke chooses the prefix. Codegen uses fusion. Hub and agent use the bare thinking prefix. A test locks both strings.

**Warning signs:**
A hub request's prompt contains `/no_think`.

**Phase to address:**
The lane-policy phase. This is the actual product change.

---

### Pitfall 5: Scoring the boilerplate think template as reasoning

**What goes wrong:**
The 104 training rows whose `<think>` body is "1. Plan / 2. Syntax / 3. Implementation" count as preserved reasoning.

**Why it happens:**
`opened_nonempty_think` is true for any non-whitespace body. Those rows are non-whitespace.

**How to avoid:**
Do not put that generator's output in the held-out set. Grade held-out items on the answer, and count initiation only on those items.

**Warning signs:**
Think-initiation is high and the trace is the same three sentences with the construct name swapped.

**Phase to address:**
The Vox item phase.

## Sources

- `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md`
- `generate_cot_organic_corpus` in `crates/vox-corpus/src/codegen_vox/part_03.rs`
- Collateral threshold `max_degradation_rate` default 0.05 in `vox-eval`
