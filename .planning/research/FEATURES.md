# Feature Research

**Domain:** Preserving reasoning on Mens hub and agent spokes
**Researched:** 2026-10-26
**Confidence:** HIGH

## Feature Landscape

### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Codegen stays non-thinking | A Vox code spoke that rambles inside `<think>` breaks "respond with code" | LOW | Format already implemented. The missing piece is binding it to the codegen spoke only. |
| Hub and agent spokes can still open `<think>` | Those lanes plan, debug, and route | MEDIUM | Serving them through `assemble_prompt` today forces `/no_think`. They need the bare thinking prefix. |
| Same items, two scores | Otherwise "before/after" is two different tests | LOW | GSM8K slice, MATH-500 slice, plus a small Vox task slice. |
| Think-initiation is its own number | Accuracy can stay flat while the model stops thinking | LOW | Baseline keys `*_init`. Empty `<think>\n\n</think>` does not count. |
| Serve refuses a drop | A report that nobody reads does not preserve anything | LOW | Existing collateral gate. Wire the real keys. |

### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| One report shows both numbers side by side | The operator sees "accuracy held, thinking died" without opening two files | LOW | Sidecar `reasoning_report.json` already records both; the gate should read both. |
| Vox items in the same harness | Reasoning about Vox is the product, not only grade-school math | MEDIUM | A short checked-in set of plan/debug prompts with a checkable answer, not the 9k training rows. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Swap in the best Oct 2026 base | "Intelligence up" sounds like a newer model | Confounds the measurement. A gain might be the base, not the lane policy. | Deferred until this before/after exists. |
| Mix in 75% generic reasoning data | Unsloth's notebook says so | The 75% figure is not a measured requirement. It also needs `ce_last_k = 0` and a long sequence or the trace is unsupervised. | Out of scope. Policy and measurement first. |
| Delete thinking to make codegen cleaner | Code-only outputs look tidier | That is how the old prefix trained the thinking-mode entry point to skip `<think>` | Non-thinking is a prefix, not a global off switch. |
| Grade the full GSM8K and MATH-500 on every serve | The number would be less noisy | Hours per gate on an 8B local run, and the weights are not always present | Checked-in slices, labeled as slices, with the noise stated. |

## Feature Dependencies

```
Lane policy
    └──requires──> Two prefixes (thinking vs non-thinking) that cannot be mixed
                       └──already in tree──> qwen_reasoning + assemble_prompt

Serve gate
    └──requires──> Base scores on the same items
                       └──requires──> Checked-in benches + a base-model run

Vox reasoning items
    └──enhances──> The math slices
    └──must not reuse──> The fixed three-line "Plan / Syntax / Implementation" template
```

### Dependency Notes

- **Serve gate requires base scores:** A post-adapter number with no pre-score cannot say whether reasoning was preserved.
- **Lane policy conflicts with a single global prompt:** `assemble_prompt` currently wraps every raw user string as a direct answer. Hub and agent calls must opt into the thinking prefix.

## What's Out of Scope

| Feature | Why Out of Scope |
|---------|------------------|
| New base model | User locked scope to lanes, benches, and the gate |
| New training techniques (self-distill, replay, KL anchor) | Same lock. They are the next milestone if the gate shows a drop that format alone does not fix |
| Re-scoring adapters trained before 2026-10-26 as if they used the new prefix | They did not |

## Sources

- Prior Mens audit (2026-09-21) and the 2026-10-26 format change
- Qwen3 tech report Stage 3; Unsloth 75/25 guidance (heuristic, not adopted)
