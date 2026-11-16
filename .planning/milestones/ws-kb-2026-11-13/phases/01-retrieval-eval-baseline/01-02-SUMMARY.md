---
phase: 01-retrieval-eval-baseline
plan: 02
subsystem: retrieval-eval
tags: [vox-search, vox-cli-research, eval, kb]
requires: []
provides:
  - "vox-cli-research::answer_overlap (crate-private answer_token_overlap, answer_keyword_hit_rate)"
  - "research-eval report key `answer_token_overlap` in code and both contracts"
affects: [vox-search evaluation.rs, research-eval contracts]
tech-stack:
  added: []
  patterns: ["honestly named, crate-private helpers instead of IR-named public fns"]
key-files:
  created:
    - crates/vox-cli-research/src/answer_overlap.rs
  modified:
    - crates/vox-search/src/evaluation.rs
    - crates/vox-search/src/semcov_wave21_tests.rs
    - crates/vox-cli-research/src/lib.rs
    - crates/vox-cli-research/src/eval.rs
    - contracts/reports/research-eval/results.v1.schema.json
    - contracts/reports/research-eval/baseline.v1.json
key-decisions:
  - "Helper bodies moved verbatim (bit-identical values); only names and docs changed"
  - "recall_at_5 sample field and DB column left alone (needs a migration, AP-04)"
requirements-completed: [EVAL-05]
status: complete
duration: ~15m
completed: 2026-10-04
commits: 2
plan_head_before: 4b4ca34af5d29e08467af6e2fb9480cba4be9046
actuals:
  tokens: 6000
  tasks: 2
  commits: 2
---

# Phase 1 Plan 02: Remove misnamed word-overlap helpers Summary

The two word-overlap helpers that were named like IR metrics are gone from `vox-search` and now live, crate-private and honestly named, in `vox-cli-research`; the averaged report key is `answer_token_overlap`.

## What was built

- **Task 1 (`2f8f90585`)**: new `answer_overlap.rs` with `answer_token_overlap(model, gold)` and `answer_keyword_hit_rate(model, evidence)` plus 6 tests (5 relocated edge cases and one overlap test, now asserting the exact value 0.8). `eval.rs` calls them directly and lost the `compute_token_recall` wrapper and its test. `evaluation.rs` keeps only the planner-intent fixtures; `semcov_wave21_tests.rs` lost the import and the "Evaluation helpers" block.
- **Task 2 (`0b671697f`)**: `citation_recall` renamed to `answer_token_overlap` in the eval report, `results.v1.schema.json` (required + properties) and `baseline.v1.json`.

## Verification

- `cargo nextest run -p vox-cli-research answer_overlap`: 6 passed. `cargo nextest run -p vox-search --lib evaluation semcov_wave21`: 20 passed.
- `cargo clippy -p vox-cli-research -p vox-search --all-targets -- -D warnings`: clean.
- Acceptance: no `calculate_recall_at_5|calculate_groundedness` anywhere in `crates/`; no `citation_recall` in `crates/vox-cli-research` or `contracts/reports/research-eval`; `jq empty` parses both contracts; grep counts 2 / 1 / 1; eval.rs has 457 non-blank lines.
- Mutation proof: appended `pub fn calculate_recall_at_5() {}` to `evaluation.rs` (presence count 0 -> 1); the absence grep then printed the stub and exited 1, i.e. the gate fails. Restored with `git checkout -- <file>` (count back to 0).

## Deviations from Plan

### Deferred / out of scope

**1. Pre-existing failure: `eval_e2e_test::test_eval_executes_real_research_pipeline_not_web_lines`**
- `cargo nextest run -p vox-cli-research` exits non-zero (21 passed, 1 failed) with `synthesis failed: research runtime feature is disabled` (`vox-research-shim` stages.rs:596). The test needs a research-runtime feature this build does not enable; it is unrelated to the helper move or key rename (it fails before any overlap code runs). Task 2's literal criterion "`cargo nextest run -p vox-cli-research` exits 0" is therefore not met; all other 21 tests pass. Not fixed (scope boundary).

Otherwise none; plan executed as written. Task 2's `eval.rs` key edit was applied after Task 1's commit so each task commit is self-contained.

## Known Stubs

None.

## Threat Flags

None. T-01-06 mitigated (helpers deleted from vox-search, crate-private in vox-cli-research); T-01-07 accepted.

## Self-Check: PASSED

- FOUND: crates/vox-cli-research/src/answer_overlap.rs
- FOUND commits: 2f8f90585, 0b671697f
