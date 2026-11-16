---
phase: 314-held-out-reasoning-set-bench-02-bench-03
plan: 02
subsystem: testing
tags: [eval, reasoning-bench, fail-closed, not-scored, vox-eval, vox-ml-cli, mens-collateral]

requires:
  - phase: 314-01
    provides: load_slice, SLICE_SIZE, ReasoningItem and the three 32-item slices
provides:
  - vox_eval::reasoning_bench::BenchOutcome { Scored, NotScored } with from_score and to_json
  - tracked reasoning_eval.rs scorer that loads through load_slice and fails closed
  - not_scored reasoning_report.json (n 0, no score keys) on every failure with an existing adapter dir
  - VOX_REASONING_EVAL_MAX_TOKENS registered in contracts/config/env-vars.v1.yaml
affects: [315, 316]

actuals:
  tokens: 17000
  tasks: 3
  commits: 3
plan_head_before: 6f15ec58f154a0c03dfac896a6f84ba7bf394882
plan_head_after: 50ab81321490d30e7eca80a5390933e006125629

tech-stack:
  added: []
  patterns: [outcome enum with no numeric stand-in for not-scored; score_loaded seam taking an infer closure so the failure paths are tested without a model]

key-files:
  created:
    - crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs
  modified:
    - crates/vox-eval/src/reasoning_bench.rs
    - crates/vox-ml-cli/src/commands/mens/mod.rs
    - crates/vox-ml-cli/src/commands/mens/eval_collateral.rs
    - contracts/config/env-vars.v1.yaml

key-decisions:
  - "VOX_REASONING_EVAL_LIMIT deleted (it truncated the slice and conflicts with expected_n 32); VOX_REASONING_EVAL_MAX_TOKENS kept and registered as kind int, owner vox-ml-cli"
  - "Whitespace-only generated_text counts as empty, so the whole run is NotScored"
  - "Missing adapter path returns Err with 'scored 0 of 32 items' and writes/creates nothing"
  - "score_loaded reads the token cap itself (max_tokens()) so its signature stays (bench, items, infer)"

patterns-established:
  - "ReasoningReport is built in exactly one place, from BenchOutcome::Scored"
  - "NotScored JSON omits accuracy and think_initiation_rate (absent, not null or 0)"

requirements-completed: [BENCH-03]

coverage:
  - id: D1
    description: "A model or slice that does not load writes a not_scored report (n 0, no score keys) when the adapter dir exists and returns Err containing 'scored 0 of 32'; a missing adapter path is Err and not created"
    requirement: "BENCH-03"
    verification:
      - kind: unit
        ref: "cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval (an_adapter_dir_without_a_loadable_model_writes_not_scored, a_missing_adapter_path_is_err_and_not_created)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Any per-item inference error, unparseable response, or empty text makes the run NotScored; from_score yields Scored only when n == n_requested >= 1; 3 of 32 stores accuracy 3.0/32.0 exactly"
    requirement: "BENCH-03"
    verification:
      - kind: unit
        ref: "cargo test -p vox-eval --lib reasoning_bench; cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval"
        status: pass
    human_judgment: false
  - id: D3
    description: "A not_scored write replaces a stale scored report; a 31-row slice is not scored; existing eval_collateral tests stay green"
    requirement: "BENCH-03"
    verification:
      - kind: unit
        ref: "cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral (4 passed); a_not_scored_write_replaces_a_stale_scored_report; a_wrong_size_slice_is_not_scored"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-10-26
status: complete
---

# Phase 314 Plan 02: Fail-Closed Reasoning Scorer Summary

**The reasoning scorer now records a score only for a full, error-free 32-item run; every other outcome writes `status: not_scored, n: 0` with no score keys and returns `Err`.**

## Performance

- **Duration:** ~45 min
- **Completed:** 2026-10-26
- **Tasks:** 3
- **Files modified:** 5 (one newly tracked)

## Accomplishments

- `BenchOutcome` in `vox-eval`: `from_score` keeps the empty-batch `{0.0, 0.0, 0}` and any partial count out of `Scored`; `to_json` omits score keys for `NotScored`.
- `reasoning_eval.rs` is in git. It loads slices through `load_slice(.., SLICE_SIZE)`, stops at the first inference error / unparseable / empty response without grading that item, and builds `ReasoningReport` only from `Scored`.
- The truncation override is gone; the max-tokens override is registered in the same commit that first tracked the scorer.
- No model weights, numeric baseline, serve gate, or before/after comparison were produced.

## Task Commits

1. **Task 1 (tracer): fail closed when the model does not load** - `a02bdfc63` (feat)
2. **Task 2: treat any inference error as not scored** - `448899eb0` (feat)
3. **Task 3: pin stale-report overwrite and slice-size refusal** - `50ab81321` (test)

## Test Evidence

| Command | Result |
|---|---|
| `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` | 9 passed, 0 failed (min 6) |
| `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` | 4 passed, 0 failed (min 4) |
| `cargo test -p vox-eval --lib reasoning_bench` | 17 passed, 0 failed (min 17) |
| `cargo check -p vox-ml-cli --features gpu --tests` | Finished, no errors |

Intermediate gates: Task 1 saw 2 passed (ml-cli) and 15 passed (vox-eval); Task 2 saw 7 passed and 17 passed. Red runs were observed first: ml-cli tests failed 0 passed / 2 failed against the old scorer; the vox-eval tests failed to compile on missing `BenchOutcome` / `from_score`.

## Mutation Check (Task 3)

Restored after each; `cmp` against a saved copy and a grep for the mutation text both confirmed it was gone before staging.

1. **Load-failure branch (`fail_closed`) writes `Scored{0.0, 0.0, 0}` instead of `NotScored`:** 7 passed, 2 FAILED — `an_adapter_dir_without_a_loadable_model_writes_not_scored`, `a_wrong_size_slice_is_not_scored`.
2. **`write_outcome` rewrites any `NotScored` as `Scored{0.0, 0.0, 0}`:** 6 passed, 3 FAILED — the two above plus `a_not_scored_write_replaces_a_stale_scored_report`.

The plan named the stale-overwrite test as failing under mutation 1; it calls `write_outcome` directly and does not go through `fail_closed`, so it only fails under mutation 2. Both mutations were run to cover it.

## Deviations from Plan

**1. [Rule 3 - Blocking] Pre-commit hook reordered the `mod` lines.** The Task 1 commit's `mens/mod.rs` has `pub(crate) mod reasoning_eval;` (with its `#[cfg(feature = "gpu")]`) at its rustfmt-sorted position after `plugin_heal` instead of after `eval_collateral`. Still exactly two added lines; no other `mod` moved. The index briefly held the pre-sort blob; I unstaged that path (`git reset -q -- mod.rs`) before the Task 2 commit, which then held exactly its two paths.

**2. [Tooling] `StrReplace` on `reasoning_bench.rs` wrote a context-compressed view back to disk** (file shrank to 231 lines with a tool stub at the end). Restored with `git checkout -- crates/vox-eval/src/reasoning_bench.rs` (file was clean) and redid the edits by splicing files from the shell. No commit was affected.

**3. Test count.** Added 7 ml-cli tests in Task 1-2 (plan minimum 4) and 2 in Task 3; `three_of_32_correct_stores_exact_accuracy`, `empty_missing_or_unparseable_text_is_not_scored`, and `an_empty_slice_is_never_scored` are extra pins on the locked rules.

## Deferred Issues

- `cargo clippy -p vox-eval --lib -- -D warnings` fails with 3 errors, none in `reasoning_bench.rs`: `qwen_reasoning.rs:178` (collapsible `if`, grading file — not to be edited here), `trial_flywheel.rs:208` (`is_multiple_of`) and `trial_flywheel.rs:784` (collapsible `if`; dirty unrelated file). Out of scope; not fixed.
- No end-to-end run against a real Candle plugin and adapter was done; the plugin-backed path (`run_backend`) is exercised only up to the model-load failure.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- `a02bdfc63`, `448899eb0`, `50ab81321` are ancestors of HEAD; each holds exactly its task's paths.
- `reasoning_eval.rs`, `reasoning_bench.rs`, `env-vars.v1.yaml` entry (exactly one `VOX_REASONING_EVAL_MAX_TOKENS`, zero `VOX_REASONING_EVAL_LIMIT`) present.
- `eval_local_prompt.rs`, `populi/train_arm.rs`, `trial_flywheel.rs` remain uncommitted.
