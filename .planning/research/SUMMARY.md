# Project Research Summary

**Project:** Vox
**Domain:** Lane-scoped reasoning measurement for Mens
**Researched:** 2026-10-26
**Confidence:** HIGH

## Executive Summary

Mens already fine-tunes Qwen3-8B with Candle QLoRA. Direct-answer rows now use Qwen's non-thinking prefix (`/no_think` and a prefilled empty think block that is not in the loss). That prefix is also what inference and chat completions send for every raw user string. Hub and agent spokes, which are the lanes that need to think, currently have no way to ask for the bare thinking-mode entry point.

The milestone is not a new model and not a new trainer. It is a lane switch, a fixed before/after on real items, and a serve gate that reads those numbers. The pieces of the scorer and the gate exist. What is missing is the spoke branch, a Vox item file that is not the boilerplate think template, and a base-model run. The Qwen3-8B weights are not on this machine, so no honest before/after number exists yet.

The main risk is measuring thinking mode through the codegen prefix, which forces the empty think block and makes think-initiation look like zero. The second risk is trusting a 32- or 64-item slice at a 5% regression threshold. The report has to say it is a slice.

## Key Findings

### Recommended Stack

Stay inside the repo. No lm-eval sidecar, no Unsloth run, no new base checkpoint.

**Core technologies:**
- Pinned Qwen3-8B: the only legal "before" in this milestone
- `vox_eval::qwen_reasoning`: prefixes, grader, think-initiation
- Existing collateral serve gate: refuse a missing or failing report

### Expected Features

**Must have (table stakes):**
- Codegen uses the non-thinking prefix; hub and agent use the bare thinking prefix
- One item set for base and adapter, with accuracy and think-initiation stored separately
- Serve blocks a drop on those keys

**Should have (competitive):**
- A short Vox plan/debug slice with checkable answers, beside GSM8K and MATH-500
- The report states the slice size so the 5% gate is not mistaken for a full-benchmark claim

**Defer (v2+):**
- Base-model swap
- New training techniques (replay, self-distill, KL anchor, the 75% mix)

### Architecture Approach

The spoke tag picks the prefix. Training and serve share that picker. A runner scores base and adapter on `reasoning.jsonl` files. The collateral report is the only artifact serve reads.

**Major components:**
1. Prefix picker — codegen versus hub/agent
2. Bench files — `gsm8k_think`, `math500_think`, and a new `vox_think`
3. Collateral gate — `bench` is accuracy, `bench_init` is think-initiation

### Critical Pitfalls

1. **Thinking benches that go through `assemble_prompt`** — pass a finished ChatML string ending at bare `assistant\n`
2. **Vacuous baselines** — only real bench keys, and a load failure writes no report
3. **Noisy 5% gate on a slice** — record `n` and do not treat the slice as GSM8K-full
4. **One global direct-answer wrapper** — hub prompts must not contain `/no_think`
5. **Boilerplate `<think>` rows** — keep them out of the held-out set

## Implications for Roadmap

Phases continue at 313.

### Phase 313: Lane prefix
**Rationale:** Until hub and codegen use different prompts, every later number is about the wrong mode.
**Delivers:** A spoke-selected prefix and tests for both strings.
**Addresses:** Lane policy
**Avoids:** One global `/no_think` wrapper

### Phase 314: Held-out set
**Rationale:** The gate needs items before it needs a threshold.
**Delivers:** Vox items plus the existing GSM8K and MATH-500 slices, and the baseline file shape with empty scores refused.
**Addresses:** Same items, two metrics
**Avoids:** The boilerplate think template and invented base numbers

### Phase 315: Serve gate
**Rationale:** The report only matters if serve reads it.
**Delivers:** Collateral keys for accuracy and think-initiation, including the slice-size note.
**Addresses:** Refuse a drop
**Avoids:** Config-field benchmarks and a silent pass when the model did not load

### Phase 316: Show the comparison
**Rationale:** The milestone promised a before/after, not only the wiring.
**Delivers:** One base run and one adapter run on the same files, written to the report, when the weights are present. If the weights are absent, the phase stops with that fact instead of a filled-in score.
**Addresses:** The operator can see the comparison
**Avoids:** Copying pre into post

## Sources

- `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md`
- Qwen3 Technical Report, arXiv:2505.09388
- In-tree `qwen_reasoning`, `reasoning_eval`, and `eval_collateral` as of 2026-10-26
