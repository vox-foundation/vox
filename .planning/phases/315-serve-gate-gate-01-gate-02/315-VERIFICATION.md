---
phase: 315-serve-gate-gate-01-gate-02
verified: 2026-10-26T18:45:00Z
status: passed
score: 9/9 must-haves verified
covered_files:
  - ".planning/phases/315-serve-gate-gate-01-gate-02/315-01-PLAN.md"
  - ".planning/phases/315-serve-gate-gate-01-gate-02/315-01-SUMMARY.md"
  - ".planning/phases/315-serve-gate-gate-01-gate-02/315-02-PLAN.md"
  - ".planning/phases/315-serve-gate-gate-01-gate-02/315-02-SUMMARY.md"
  - "crates/vox-eval/src/lib.rs"
  - "crates/vox-eval/src/reasoning_slice.rs"
  - "crates/vox-ml-cli/src/commands/mens/eval_collateral.rs"
  - "crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs"
  - "crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs"
  - "docs/src/how-to/how-to-train-mens-macos-metal.md"
covered_digest: "v3:sha256:c3aea58e6d0fe0f80c0637400ee882f8bed3c290bc2b88e8801b406277267333"
behavior_unverified: 0
overrides_applied: 0
re_verification: false
---

# Phase 315: Serve gate (GATE-01, GATE-02) Verification Report

**Phase Goal:** Serve refuses an adapter whose collateral report is missing or worse on accuracy or think-initiation, and the report labels the benches as slices.
**Verified:** 2026-10-26
**Status:** passed
**Re-verification:** No, initial verification

I read the committed code, not the SUMMARY claims. The phase files have no uncommitted diff. The dirty files (`eval_local_prompt.rs`, `train_arm.rs`) are outside this phase and were not judged. No model was run and no numeric baseline exists. A green test here is not a model score.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Serve refuses when the collateral report is missing | ✓ VERIFIED | `assert_collateral_gate` (dispatch.rs:801) bails with `eval_collateral_damage check not found!` when `collateral_damage_report.json` is absent. It is called once in the serve arm (dispatch.rs:357) before `assert_serve_preconditions`. Test `a_missing_report_is_refused`. |
| 2 | Serve refuses a non-pass, malformed, or old-format report | ✓ VERIFIED | Unreadable or non-JSON report is an error. `status != "pass"` gives the verbatim `check FAILED.` text. A missing `reasoning_slices` block is refused by `verify_reasoning_slices`. Tests: `a_failed_status_is_refused_with_the_existing_text`, `a_malformed_collateral_report_is_refused`, `an_old_format_passing_report_is_refused`. |
| 3 | Serve refuses an adapter worse than 5% on accuracy or think-initiation, recomputed from stored numbers and not from `status` | ✓ VERIFIED | `verify_reasoning_slices` (reasoning_slice.rs:204-271) iterates `SLICE_NAMES`, reads the four stored numbers, and calls `eval_reasoning_slice`. It ignores stored `degradation_rate`, `exceeds_threshold` and `status`. It returns Err naming `failed_key()`. Accuracy gives `<slice>` and think-initiation gives `<slice>_init`. Threshold uses the strict `>` in the unchanged `eval_collateral_damage` (lib.rs:308). Tests: `a_hand_edited_pass_over_a_ten_percent_accuracy_drop_is_refused`, `a_hand_edited_pass_over_a_think_initiation_drop_is_refused`, plus the vox-eval recompute tests. Both mutation-verified per 315-02-SUMMARY, and I confirmed the tests are present and pass. |
| 4 | Writer compares accuracy and think-initiation separately per slice; exactly 5% passes; bad inputs return Err, never 0.0 | ✓ VERIFIED | `eval_reasoning_slice` (reasoning_slice.rs:116-154) refuses an unknown slice, `n != SLICE_SIZE`, non-finite values, `pre <= 0.0`, and values outside [0, 1]. All of these run before the arithmetic. It then calls the generic function once per metric. `exactly_five_percent_passes_on_both_metrics` exists. |
| 5 | Writer requires all six baseline keys, refuses scores with no `n` or an `n` mismatch, and removes the stale report first | ✓ VERIFIED | `require_reasoning_keys` runs before any scoring (eval_collateral.rs, after the empty check). `remove_file` is the first statement of `run_collateral_damage_with`. `n: None` or a mismatch between the accuracy and think-initiation counts is a bail. Writer tests cover the stale-removal and incomplete-baseline cases. |
| 6 | Report labels the benches as 32-item slices and records `n`, on pass and on fail | ✓ VERIFIED | `reasoning_slices_json` is written on every report with `scope_note` ("fixed 32-item slices, not full GSM8K or MATH-500"), `slice_size` 32, and `benches.<slice>.n`. Serve refuses a report whose `scope_note`, `slice_size`, or `n` is missing or wrong (reasoning_slice.rs:221-245). Reasoning entries in `reports[]` carry `slice: true` and `n`. Generic entries carry neither. |
| 7 | Serve re-checks each slice's `reasoning_report.json` (status scored, n 32); a missing, not-scored, wrong-n, or malformed one is refused | ✓ VERIFIED | The loop in `assert_collateral_gate` builds paths only from `SLICE_NAMES` and ensures `status == "scored"` and `n == SLICE_SIZE`. Tests: `a_missing_slice_report_is_refused_by_name`, `a_not_scored_slice_report_is_refused`, `a_slice_report_with_n_31_is_refused`, `an_unparseable_slice_report_is_refused`. |
| 8 | Generic `eval_collateral_damage` is unchanged | ✓ VERIFIED | The function is at lib.rs:296-318 with the original zero-pre path (`0.0` when `pre <= EPSILON`). `lib.rs` differs only by `pub mod reasoning_slice;` (line 16). The zero-pre test was recorded as passing. |
| 9 | The operator how-to names the six keys, the hard error, and the serve re-check | ✓ VERIFIED | The how-to section (lines 59-90) lists the six keys. It contains `32-item` and `status alone`. WR-01 (bb0dad31d) narrowed the claim so it no longer says every hand-edited pass is refused. The `jq '.status'` example is kept. No numeric baseline is written. |

**Score:** 9/9 truths verified (0 present, behavior-unverified)

Truths 1-7 are runtime behavior. Each is exercised by a named test that I confirmed exists. The mutation checks recorded in the SUMMARYs (finite guard, zero-pre guard, `remove_file`, refusal skip, slice status, slice `n`) and the WR-04 range-guard mutation all failed the expected tests.

### Prohibitions

| ID | Statement | Status | Evidence |
|----|-----------|--------|----------|
| P-315-01 | A missing, not-scored, non-finite, zero-baseline, or wrong-n slice is never stored as 0.0 and never produces a pass | ✓ VERIFIED | The scorer's `Err`, the key rule, the guards, and `n: None` bails all return before the write at the end of `run_collateral_damage_with`. Serve independently refuses the same cases. The test tier has wired tests, so this is not an unverified fail-closed item. |
| P-315-02 | No numeric baseline, no model run, no new env var or crate dependency, research doc untouched | ✓ VERIFIED | The how-to has no numbers. `gate` code reads files only. `reasoning_slice.rs` imports only `crate::` items and `serde_json`. The research doc has no diff in the phase files. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/vox-eval/src/reasoning_slice.rs` | strict evaluator, key rules, report block, verifier, tests | ✓ VERIFIED | 549 lines. Contains `pub fn eval_reasoning_slice` and `pub fn verify_reasoning_slices`. 26 tests. No uncommitted diff. |
| `crates/vox-eval/src/lib.rs` | one added line | ✓ VERIFIED | `pub mod reasoning_slice;` at line 16. |
| `.../mens/eval_collateral.rs` | `BenchScore`, six-key rule, stale removal, strict compare, `reasoning_slices` | ✓ VERIFIED | Wired as described. 715 lines. |
| `.../mens/reasoning_eval.rs` | `n` read; dead-code allow removed | ✓ VERIFIED | `allow(dead_code)` count is 0. `report.n` is read in `score_bench`. |
| `.../populi/dispatch.rs` | `assert_collateral_gate`, serve arm wired, tests | ✓ VERIFIED | Defined once, called once. 12 `collateral_gate` tests. |
| how-to doc | six keys, `32-item`, `status alone` | ✓ VERIFIED | See truth 9. |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| `run_collateral_damage_with` | `eval_reasoning_slice` | one call per `SLICE_NAMES` entry with both metrics and the scorer `n` | ✓ WIRED |
| `eval_reasoning_slice` | `eval_collateral_damage` | guards, then the generic arithmetic once per metric | ✓ WIRED |
| `score_bench` | `ReasoningReport.n` | `BenchScore { value, n: Some(report.n) }` | ✓ WIRED |
| serve arm (dispatch.rs:357) | `assert_collateral_gate` | direct call inside the marker-file condition | ✓ WIRED |
| `assert_collateral_gate` | `verify_reasoning_slices` | called with `CollateralDamageConfig::default()`, error mapped to a refusal | ✓ WIRED |
| `verify_reasoning_slices` | `eval_reasoning_slice` | same function the writer uses | ✓ WIRED |
| writer | serve | the writer writes `collateral_eval/<slice>/reasoning_report.json` (reasoning_eval.rs:239-262) and serve reads the same path | ✓ WIRED |

### Data-Flow Trace (Level 4)

The values serve judges come from the stored `pre`/`post` numbers in the report on disk. The writer produces them from the scorer's output, which comes from `score_reasoning_file`. Neither side uses a hardcoded stand-in. Whether those numbers reflect a real model is phase 316's concern and is not claimed here.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Slice evaluator and verifier | `cargo test -p vox-eval --lib reasoning_slice` | 26 passed | ✓ PASS |
| Serve gate | `cargo test -p vox-ml-cli --features gpu --lib -- collateral_gate` | 12 passed | ✓ PASS |

I ran only those two. The `eval_collateral` (17), `collateral_damage` (10) and `serve_preconditions` (3) results come from the same-day runs you reported; I did not re-run them. I did not run a workspace test.

### Probe Execution

SKIPPED. The phase declares no `probe-*.sh` scripts.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| GATE-01 | 315-01, 315-02 | Serve refuses when the report is missing or accuracy/think-initiation drops past the threshold | ✓ SATISFIED | Truths 1-5 and 7. |
| GATE-02 | 315-01, 315-02 | The report says these benches are slices | ✓ SATISFIED | Truth 6, and the serve re-check of the label. |

No orphaned requirements. Both phase 315 IDs are claimed by both plans.

### Anti-Patterns Found

None in the phase files. There are no TBD/FIXME/XXX markers, no stub returns, and no `unwrap` in the verifier's parsing path (it uses `get().and_then()`).

### Locked Limits (recorded, not gaps)

- The gate runs only when `training_manifest.json` or `candle_qlora_adapter.safetensors` exists.
- Serve checks slice `status` and `n`, not numeric equality with the collateral report. A consistent rewrite of both pre and post is a new baseline and is not detected as an edit (the how-to says so after WR-01).
- WR-02 (token cap not on the report) and WR-03 (exact `scope_note` match) are deferred. Info findings are skipped.
- `serve_preconditions` has 3 tests. The plan asked for 4 because it miscounted the baseline. Nothing was removed.
- Scoped clippy with `-D warnings` fails on three errors in files this phase did not touch (`qwen_reasoning.rs:178`, `trial_flywheel.rs:208`, `trial_flywheel.rs:784`). I did not re-run clippy, so that comes from the SUMMARYs only. It is outside this phase's goal.

### Human Verification Required

None. Every truth is covered by a test that exercises the behavior. The real before/after model scores belong to phase 316.

### Gaps Summary

No gaps. No must-have is missing.

---

_Verified: 2026-10-26_
_Verifier: Claude (gsd-verifier)_
