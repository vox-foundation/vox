---
phase: 315-serve-gate-gate-01-gate-02
plan: 01
subsystem: eval
tags: [collateral-damage, reasoning-slices, gate, mens, vox-eval, vox-ml-cli]

requires:
  - phase: 314-held-out-reasoning-set-bench-02-bench-03
    provides: SLICE_SIZE, SLICE_NAMES, reasoning scorer with ReasoningReport.n
provides:
  - vox_eval::reasoning_slice strict per-slice evaluator, six-key rule, report block projection
  - eval-collateral-damage writer that refuses incomplete or invalid reasoning input and labels slices with n
affects: [315-02, 316]

plan_head_before: 29f96c4a24205ba3ad0b9ec1de6b9b9c02e6b92e
plan_head_after: b31d480774402a65a3204216a86ec09b8fe725b1

actuals:
  tokens: 7000
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Strict wrapper around the unchanged generic eval_collateral_damage: guards first, arithmetic once per metric"
    - "Reasoning keys classified by name against SLICE_NAMES, never by filesystem probe"

key-files:
  created:
    - crates/vox-eval/src/reasoning_slice.rs
  modified:
    - crates/vox-eval/src/lib.rs
    - crates/vox-ml-cli/src/commands/mens/eval_collateral.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs

key-decisions:
  - "Accuracy and think-initiation compared separately at 5% (strict >); exactly 5% passes"
  - "All six baseline keys required; any missing key is a hard error before scoring"
  - "NaN, infinity, pre <= 0, values outside [0, 1], n != 32, and unknown slice names return Err, never 0.0"
  - "Stale collateral_damage_report.json removed as the first statement of the writer"
  - "failed_on: failing generic bench wins, else first failing reasoning key in SLICE_NAMES order, accuracy before _init"

patterns-established:
  - "Reasoning entries in reports[] carry slice true and n; generic entries carry neither"
  - "Every report (pass and fail) carries reasoning_slices with scope_note, slice_size 32, and per-slice n"

requirements-completed: [GATE-01, GATE-02]

duration: 1 session
completed: 2026-10-26
status: complete
---

# Phase 315 Plan 01: Strict reasoning-slice collateral writer Summary

**`vox mens eval-collateral-damage` now compares accuracy and think-initiation separately on each 32-item reasoning slice, refuses every locked hard-error case without writing a report, and labels both pass and fail reports with the slice scope and `n`.**

## Performance

- **Tasks:** 3 of 3 (tracer + 2 TDD)
- **Files modified:** 4 (1 new)

## Accomplishments

- New pure module `vox_eval::reasoning_slice`: `reasoning_slice_of`, `require_reasoning_keys`, `eval_reasoning_slice`, `ReasoningSliceVerdict`, `reasoning_slices_json`, `REASONING_SLICE_SCOPE_NOTE`. It wraps `eval_collateral_damage` without editing it.
- Writer: `BenchScore { value, n }` from the scorer, six-key rule, stale-report removal, strict comparison, `reasoning_slices` block, and `slice`/`n` on reasoning entries.
- `ReasoningReport.n` is now read by the writer; its dead-code allow is gone.
- No model was run, no baseline number written, no env var or dependency added.

## Task Commits

1. **Task 1: Tracer, six-key baseline writes a labelled passing report** - `6600e61a6` (feat)
2. **Task 2: Strict evaluator refuses non-finite, zero-baseline, out-of-range, wrong-n** - `982e644d2` (feat)
3. **Task 3: Writer refuses incomplete input, removes stale report, labels failing reports** - `b31d48077` (feat)

## Test Commands and Counts

| Command | Result |
|---|---|
| `cargo test -p vox-eval --lib reasoning_slice` | Task 1: 6 passed. Task 2 RED: 8 passed, 7 failed. Task 2 GREEN: 15 passed |
| `cargo test -p vox-eval --lib collateral_damage` | 10 passed (unchanged) |
| `cargo test -p vox-eval --lib collateral_damage_zero_pre_score_avoids_divide_by_zero` | 1 passed |
| `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` | Task 1: 5 passed. Task 3 RED: 14 passed, 3 failed. Task 3 GREEN: 17 passed |
| `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` | 12 passed (gate minimum 6) |
| `cargo check -p vox-ml-cli --features gpu --tests` | exit 0, no errors or warnings |

Awk minimums (5, 15, 17, 10, 6) all satisfied. `crates/vox-eval/src/lib.rs` changed by exactly one added line.

## Mutation Checks

- Finite guard replaced with `if false`: `a_nan_accuracy_post_score_is_refused` and `an_infinite_think_initiation_post_score_is_refused` failed (13 passed, 2 failed). Restored: 15 passed.
- Zero-pre guard replaced with `if false`: `a_zero_accuracy_baseline_is_refused` and `a_zero_think_initiation_baseline_is_refused` failed (13 passed, 2 failed). Restored: 15 passed.
- `remove_file` replaced with a no-op: `a_stale_passing_report_is_removed_when_scoring_fails` and `a_stale_passing_report_is_removed_when_the_baseline_is_incomplete` failed (15 passed, 2 failed). Restored (diff-identical to the pre-mutation file): 17 passed.
- Each mutation had a before/after presence count to rule out a concurrent revert.

## Deviations from Plan

### Auto-fixed Issues

None.

### Notes

- **Task 1 RED:** the Task 1 tests and implementation were written together, so there is no recorded red run for those six tests. RED was recorded for Task 2 (7 failing) and Task 3 (3 failing).
- **Tooling:** `StrReplace` and the reader returned compressed views of the Rust files, so edits were spliced with `sed` from temp files and each file's header and line count were checked afterward. No compressed view was written back.
- **Clippy (scoped):** `cargo clippy -p vox-eval --lib -- -D warnings` exits 101 on three errors in files outside this plan: `qwen_reasoning.rs:178`, `trial_flywheel.rs:208`, `trial_flywheel.rs:784`. None are in `reasoning_slice.rs`. Left alone as out of scope. `trial_flywheel.rs` was dirty before this plan and is not staged.
- Unrelated dirty-tree paths (including the research doc) were never staged. Each commit held exactly its task's paths.

## Known Stubs

None.

## Issues Encountered

None blocking.

## Next Phase Readiness

- Plan 315-02 can make serve re-check the `reasoning_slices` block (`scope_note`, `slice_size`, per-slice `n`) in `collateral_damage_report.json`.
- Phase 316 still owes the real scores. Nothing here is a model result.

## Self-Check: PASSED

- `crates/vox-eval/src/reasoning_slice.rs`: found
- Commits `6600e61a6`, `982e644d2`, `b31d48077`: found in HEAD history
- `allow(dead_code)` in `reasoning_eval.rs`: 0; uncommented `remove_file` in `eval_collateral.rs`: 1
