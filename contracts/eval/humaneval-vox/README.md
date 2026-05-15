# `contracts/eval/humaneval-vox/` — HumanEval-Vox

Canonical benchmark for [CR-L1](../../../docs/src/architecture/v1-release-criteria.md). Anchored at 164 problems for direct comparability with HumanEval-Python (Chen et al., 2021).

## Gate

- **Bar:** ≥ 80% compile + test-pass rate across the reference LLM panel (median scoring).
- **Sub-bar (demote):** < 60%.

## Fixture format

Each fixture is `problems/<id>.spec.toml`:

```toml
# vox:skip   (shown for illustration; actual fixtures must compile)
id = "humaneval-vox-001-fizzbuzz"
training_eligible = true                       # or false for held-out set

prompt = """
Write a Vox function `fizzbuzz(n: int) -> list[str]` that returns the
classic FizzBuzz sequence up to and including n. Use Vox's @pure
annotation if applicable.
"""

[reference_solution]
path = "problems/001-fizzbuzz/reference.vox"

[tests]
path = "problems/001-fizzbuzz/tests.vox"
expected_exit = 0
```

## Held-out subset

30 of the 164 problems carry `training_eligible: false`. The CI gate in `crates/vox-corpus/` verifies these are never ingested by MENS training pipelines. This is the corpus-contamination guard ([implementation plan R1](../../../docs/src/architecture/v1-llm-target-implementation-plan-2026.md#5-risk-register-cr-l-specific)) — without it, CR-L1 numbers are leaked-evaluation marketing.

## Status

**Empty as of 2026-05-15.** Manifest enforces `corpus_size: 0` until P3.1 lands fixtures (target 2026-09).

## Provenance

Each fixture must declare its provenance to avoid contamination:
- `derived_from: "examples/golden/<file>"` — if mechanically lifted from existing examples (these become `training_eligible: false` by default since MENS likely trained on them).
- `derived_from: "hand-authored-2026-MM"` — net-new problems.
- `derived_from: "ast-mutation-of/<source>"` — programmatic mutations.
