---
phase: 12-code-gen-benchmark-plumbing-provenance
plan: 08
subsystem: testing
tags: [eval-corpus, compiler-feedback, repair-turns, pass-at-1, bootstrap-ci]
requires:
  - phase: 12-02
    provides: draw_samples, sample_seed, SampleRecord
  - phase: 12-04
    provides: EvalCorpusReport::assemble, arm_stats, the completeness gate
  - phase: 12-07
    provides: collect_provenance, ConfigKnobs/config_digest, typed report output
provides:
  - "`vox model eval-corpus --feedback-turns K` (0..=3, default 0): failed samples get up to K repair turns carrying the verifier diagnostic"
  - "SampleRecord.passed_by_turn / repair_tokens / repair_cost_usd (serde default)"
  - "SamplingRecord.feedback_turns (serde default) and feedback_turns in config_digest"
  - "EvalCorpusReport.pass_at_1_after_feedback: one ArmStats (bootstrap CI, MDD) per turn"
  - "feedback_prompt, repair_seed, MAX_FEEDBACK_TURNS"
affects: [12-09]
status: complete
actuals:
  tokens: 7700
  tasks: 2
  commits: 2
plan_head_before: aadb3239c24c68b77f2271e7347e5b19da13b066
tech-stack:
  added: []
  patterns:
    - "repair outcomes live in a separate field; the AttemptOutcome that feeds pass@1/pass@k is always the original attempt"
    - "trajectory padded with resize(K+1, last): carries a pass forward (cumulative) and carries the last value after an early stop"
key-files:
  created: []
  modified:
    - crates/vox-cli/src/commands/model/eval_corpus.rs
    - crates/vox-eval/src/report.rs
    - crates/vox-eval/src/provenance.rs
    - crates/vox-cli/src/commands/model/eval_compare.rs
key-decisions:
  - "repair turns reuse the single llm_chat path (generate_chat), sending [user prompt, assistant raw completion, user feedback]; no vendor code"
  - "a repair candidate is classified by the same classify_outcome as the original; only ParseError/CheckError/RuntimeError/WrongAnswer are repaired, and a pass, cheat or timeout ends that sample's repairs"
  - "feedback_turns is always part of config_digest (so K=0 digests differ from pre-12-08 digests; schema-2 reports are new in phase 12)"
  - "--from-dir runs record feedback_turns 0 (pre-generated solutions cannot be re-prompted)"
requirements-completed: [BENCH-13, BENCH-03]
---

# Phase 12 Plan 08: Compiler-feedback repair turns Summary

eval-corpus can now measure pass@1 after k compiler-feedback turns (design section 8): failed samples are re-prompted with the verifier's diagnostic, and the typed report carries one bootstrap-CI ArmStats per turn while pass@1 and pass@k stay on the original attempts.

## Accomplishments

- **Tracer (871cee721):** `--feedback-turns K` (validated 0..=3). In `draw_samples`, after the original attempt is verified, a failed live sample (parse, check, runtime or wrong-answer) loops over turns 1..=K: messages are the original prompt, the previous raw completion as the assistant, and `feedback_prompt(previous detail)`; the config is cloned with `seed = repair_seed(base, fixture, turn, sample_index)`; the new candidate is verified with the same closure and classified with the same `classify_outcome`. Repair tokens and cost go on the sample and into the spend counter (cap hit ends the loop and marks the run incomplete). Pass, cheat or timeout ends a sample's repairs; an infra error increments `n_infra_errors` and ends them; the remaining `passed_by_turn` entries carry the last value forward. K = 0 sends no repair request. The `AttemptOutcome` is never replaced.
- **Per-turn statistics (a5dc95191):** `assemble` reads K from `provenance.sampling.feedback_turns`; for each turn and each scorable fixture the value is (verified samples true at index t) / (verified samples), then `arm_stats`. Unscored fixtures are excluded; K = 0 yields an empty vec. `print_summary` prints a line per turn.

## Mutation proof for T-12-08-03 (advisory adopted)

The guard is `feedback_never_replaces_the_original_attempt` (asserts `(n, c) == (1, 0)`, `attempts[0].tests_passed == false`, record `tests_passed`/`compiled` false, class still `CheckError`, even though `passed_by_turn == [false, true]`).

1. Before: file saved; `grep -c MUTATION` = 0.
2. Mutation: the `AttemptOutcome` push became `tests_passed: outcome.tests_passed || <sample's last passed_by_turn is true>` (marker `// MUTATION`, presence check = 1).
3. Result: the guard test FAILED (3 tries, assertion at the `(o.n, o.c)` check); the other three feedback tests still passed.
4. Restore: marker count 0 and the file is byte-identical to the pre-mutation copy (`diff` clean); the 6-test run (`feedback_*`, `repair_seed_*`, `sample_args_reject_*`) passes.

## Verification

- `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::tests::feedback_) | test(eval_corpus::tests::repair_seed_) | test(eval_corpus::tests::sample_args_reject_more_than_three_feedback_turns)'`: 6 passed (plan required at least 5).
- `cargo nextest run -p vox-eval --lib`: 101 passed (3 new `report::tests::pass_after_feedback_*` / `no_feedback_turns_*`; existing `report::` tests still pass).
- `cargo nextest run -p vox-cli --lib -E 'test(eval_corpus::) | test(eval_compare::)'`: 42 passed.
- Touched files pass `rustfmt --check` (never `cargo fmt --all`). No clippy or full-workspace run (lean verification; CI covers the rest).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] eval_compare.rs test literal needed the new field**
- **Found during:** Task 1
- **Issue:** `SamplingRecord { .. }` in `eval_compare.rs` tests is a struct literal; adding `feedback_turns` broke compilation.
- **Fix:** added `feedback_turns: 0` to that one literal.
- **Files modified:** crates/vox-cli/src/commands/model/eval_compare.rs (not in the plan's files_modified)
- **Commit:** 871cee721

**2. [Rule 2 - Missing] `Generated.raw` and `feedback_turns` in `ConfigKnobs`**
- The assistant turn must carry the model's raw completion, not the fence-stripped code, so `Generated` gained a `raw` field; `ConfigKnobs` gained `feedback_turns` (and the digest test variant array grew to 12) to satisfy "feedback_turns is part of config_digest".

**3. Scope note:** the pin-guard script was run in a condensed form (same git-vs-git root comparison, without the Windows drive-form branches) from the scratchpad; it printed `PIN_OK` before each write phase and commit.

## Known Stubs

None.

## Threat Flags

None. The diagnostic sent to the provider is the verifier's existing one-line detail (T-12-08-01 accepted); K is capped at 3 and repair cost counts toward the spend cap (T-12-08-02).

## Issues Encountered

- A pre-existing uncommitted change to `crates/vox-cli/src/commands/model/classify.rs` (another session) was left untouched and unstaged.

## Self-Check: PASSED

- FOUND commits 871cee721, a5dc95191 on feat/research-trial-flywheel.
- FOUND `feedback_turns` in provenance.rs, `passed_by_turn` and `pass_at_1_after_feedback` in report.rs, `fn repair_seed` and `fn feedback_prompt` in eval_corpus.rs.
