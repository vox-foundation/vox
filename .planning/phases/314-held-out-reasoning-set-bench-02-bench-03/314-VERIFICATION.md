---
phase: 314-held-out-reasoning-set-bench-02-bench-03
verified: 2026-10-26T21:10:00Z
status: passed
score: 8/8 must-haves verified
covered_files:
  - ".gitignore"
  - ".planning/phases/314-held-out-reasoning-set-bench-02-bench-03/314-01-PLAN.md"
  - ".planning/phases/314-held-out-reasoning-set-bench-02-bench-03/314-01-SUMMARY.md"
  - ".planning/phases/314-held-out-reasoning-set-bench-02-bench-03/314-02-PLAN.md"
  - ".planning/phases/314-held-out-reasoning-set-bench-02-bench-03/314-02-SUMMARY.md"
  - "contracts/config/env-vars.v1.yaml"
  - "crates/vox-compiler/tests/vox_think_oracle_test.rs"
  - "crates/vox-eval/src/lib.rs"
  - "crates/vox-eval/src/reasoning_bench.rs"
  - "crates/vox-ml-cli/src/commands/mens/eval_collateral.rs"
  - "crates/vox-ml-cli/src/commands/mens/mod.rs"
  - "crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs"
  - "mens/data/gsm8k_think/reasoning.jsonl"
  - "mens/data/math500_think/reasoning.jsonl"
  - "mens/data/vox_think/reasoning.jsonl"
covered_digest: "v3:sha256:e78965e3f4caca6b105a45c1364e5b502e385e09b3974aa8c25ea5c6a746257b"
behavior_unverified: 0
overrides_applied: 0
deferred:
  - truth: "A truncated/unclosed think block is reported with a marker in the report (WR-02)"
    addressed_in: "Phase 316"
    evidence: "314-REVIEW-DISPOSITION.md: deferred; truncation count belongs with the before/after comparison. Not a Phase 314 must-have."
  - truth: "MATH-500 equivalent surface forms grade correct (WR-03)"
    addressed_in: "Phase 316 (known ceiling, grader locked)"
    evidence: "314-REVIEW-DISPOSITION.md: MATH-500 content and grader are locked this phase."
---

# Phase 314: Held-out Reasoning Set Verification Report

**Phase Goal:** The same GSM8K, MATH-500, and Vox items exist for a before/after, and a missing model cannot write a passing score.
**Verified:** 2026-10-26 (re-verified after phase 315)
**Status:** passed
**Re-verification:** Yes. The prior report went stale when phase 315 commits 6600e61a6, b31d48077, 5c0006c88, 3d9e9a467 and 84711c6c7 touched covered files. All 8 truths were re-checked on the current tree (HEAD 733bbb4e7).

No model was run and no numeric baseline exists. That is the phase contract. A green test here is not a model score.

## Re-verification Notes (after phase 315)

What 315 changed in 314-covered files, from `git diff 3bbaf52db HEAD` (3bbaf52db was the last 314 commit to these files):

- `reasoning_eval.rs`: a doc comment on `ReasoningReport.n` only (the `#[allow(dead_code)]` came off because the collateral writer now reads `n`). No logic change. `fail_closed`, `score_reasoning_file_with` (non-directory adapter bails before any write; bad cap, slice load and backend errors go through `fail_closed`), `ReasoningReport {` built in the one `Scored` arm, and `parse_max_tokens` are as verified before.
- `eval_collateral.rs`: +508 lines. `score_bench` still routes reasoning benches through `cached_reasoning_report`; the cache stores only `Ok` reports and `?` propagates any Err, so a failed or missing model still yields no score and (new in 315) no report file. 315 adds a stricter `report.n != SLICE_SIZE` bail and six-key reporting. This strengthens truth 5 and does not weaken it. `an_unloadable_adapter_fails_loudly_and_writes_no_passing_report` and `a_stale_passing_report_is_removed_when_scoring_fails` are present.
- `vox-eval/src/lib.rs`: +1 line, `pub mod reasoning_slice;` (315 code). `reasoning_bench.rs`, `mod.rs`, `env-vars.v1.yaml`, `.gitignore`, the three slices and the oracle test: no diff since 314.

Re-run this session: `cargo test -p vox-compiler --test vox_think_oracle_test` gives 3 passed. Slice files are tracked, 32 rows each, and match HEAD. `VOX_REASONING_EVAL_LIMIT` has 0 hits; `VOX_REASONING_EVAL_MAX_TOKENS` is registered once. No TBD/FIXME/XXX in the reasoning files.

Caveat: `cargo nextest run -p vox-ml-cli reasoning_eval eval_collateral` does not compile in this worktree. `crates/vox-code-audit/src/detectors/mod.rs:159` declares `pub mod ui_perf_hazards;` (added by commit 5164b5263, phase 305), and that file is not tracked in any commit. It exists only as an untracked file in the main checkout. This is unrelated to 314 or 315, but it blocks the vox-ml-cli test build here. So the vox-ml-cli test results below are by test-presence inspection, not a fresh run. The 12 named 314 tests are all present in `reasoning_eval.rs` and `reasoning_bench.rs`. The 315 verification records those suites passing.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: a checked-in Vox slice has a checkable answer on every item (BENCH-02) | ✓ VERIFIED | `mens/data/vox_think/reasoning.jsonl` is tracked (`git ls-files`), 32 rows, ids `vox-think-0000`..`0031`, 32 unique. Every row has an integer answer and a `program`. Oracle test `every_vox_think_program_returns_its_gold` runs each program through lex, parse, lower and the interpreter and requires `VoxValue::Int` equal to the gold. I re-ran it this session: 3 passed. |
| 2 | SC1: the Vox slice does not reuse the Plan / Syntax / Implementation template | ✓ VERIFIED | `rg -i 'Plan:\|Syntax considerations\|Implementation:\|construct in vox'` finds 0 hits in the file. Kinds are 11 debug, 11 trace, 10 plan-check. All 32 questions end with the same final-answer instruction. Guards `vox_think_avoids_the_template` and `vox_think_is_not_a_golden_copy` (golden-file substring and pairwise-distinct checks) exist. |
| 3 | SC2: the GSM8K and MATH-500 slices are the checked-in files, not a new sample | ✓ VERIFIED | `gsm8k_think` tracked, 32 rows, `gsm8k-test-0000`..`gsm8k-test-0031`, no `first 64` text, LF endings, trailing newline. `math500_think` tracked, 32 rows, `test/precalculus/807.json`..`test/prealgebra/1302.json`, no licence claim added. `.gitignore:199` is `!mens/data/*_think/reasoning.jsonl`. `git check-ignore` exits 1 for all three slices and 0 for `mens/data/train.jsonl`. `git diff HEAD -- mens/data` is empty, so the committed files match the tree. |
| 4 | One validating loader pins the slices (`load_slice`) | ✓ VERIFIED | `reasoning_bench.rs` reads through `Read::take(MAX_SLICE_BYTES + 1)`. It refuses a missing or empty id, blank question or answer, duplicate ids (both line numbers named), and any count other than `expected_n`. Tests exist for each (`wrong_count_is_refused`, `duplicate_id_is_refused`, `size_cap_boundary`, and the rest). The oracle and the scorer both use this one loader. |
| 5 | SC3 / BENCH-03: a failed or missing model load writes no passing score and records how many items were scored | ✓ VERIFIED | `score_reasoning_file_with` returns Err for a non-directory adapter without writing anything. Otherwise any failure (bad cap, slice load, plugin lookup, `load_model`) goes through `fail_closed`, which writes `BenchOutcome::NotScored`. That JSON has `status: not_scored`, `n: 0`, `n_requested`, and `reason` containing `scored 0 of N items`, with no `accuracy` or `think_initiation_rate` key. It then returns Err. Tests: `an_adapter_dir_without_a_loadable_model_writes_not_scored`, `a_missing_adapter_path_is_err_and_not_created`, `not_scored_json_has_no_score_keys`. |
| 6 | A partial or empty run can never be `Scored` | ✓ VERIFIED | `score_loaded` stops at the first inference error, unparseable body, or empty `generated_text`, does not grade or count that item, and returns NotScored naming the item id. `from_score` is `Scored` only when `n_requested >= 1` and `score.n == n_requested`. `ReasoningReport {` is constructed in exactly one place (the `Scored` arm, `reasoning_eval.rs:86`). Tests: `an_inference_error_on_any_item_is_not_scored`, `empty_missing_or_unparseable_text_is_not_scored`, `an_empty_slice_is_never_scored`, `a_wrong_size_slice_is_not_scored`, `from_score_requires_every_requested_item`. |
| 7 | A not-scored write replaces a stale scored report | ✓ VERIFIED | `write_outcome` uses `std::fs::write`, which truncates. `a_not_scored_write_replaces_a_stale_scored_report` asserts the score keys are gone afterwards. |
| 8 | Scored reports record `n`, with exact accuracy and no truncation env var | ✓ VERIFIED | `scored_json_carries_n_and_exact_accuracy` checks 3 of 32 gives `3.0/32.0` exactly and `n == 32`. `VOX_REASONING_EVAL_LIMIT` has 0 hits in `crates/vox-ml-cli/src` and `contracts/config`. `VOX_REASONING_EVAL_MAX_TOKENS` is registered once (`env-vars.v1.yaml:3192`, owner `vox-ml-cli`, kind `int`). WR-01 is fixed in `3bbaf52db`: `parse_max_tokens` returns Err for a set non-integer, unset gives 2048, and values below 1 become 1. Tests: `max_tokens_parse_rules` and `an_unparseable_max_tokens_is_not_scored_and_never_2048`. |

**Score:** 8/8 truths verified (0 present but behavior-unverified)

The phase's runtime transitions (load failure, per-item error, stale overwrite) are exercised by tests through the `score_loaded` seam and a tempdir adapter. They need no plugin or model. The one thing no test reaches is a real model loading and scoring. That is outside this phase by contract.

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | WR-02: report marker for an unclosed think | Phase 316 | `314-REVIEW-DISPOSITION.md` records it as deferred. The grader is locked. |
| 2 | WR-03: MATH-500 surface-form fragility | Phase 316 (known ceiling) | `314-REVIEW-DISPOSITION.md` records it as deferred. Slice content and grader are locked. |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `mens/data/gsm8k_think/reasoning.jsonl` | 32 tracked rows | ✓ VERIFIED | Committed in `72f08843c`. |
| `mens/data/math500_think/reasoning.jsonl` | 32 tracked rows, unchanged | ✓ VERIFIED | Committed in `72f08843c`. |
| `mens/data/vox_think/reasoning.jsonl` | 32 interpreter-checked rows | ✓ VERIFIED | Committed in `b44742263`. |
| `crates/vox-eval/src/reasoning_bench.rs` | `load_slice`, `BenchOutcome`, tests | ✓ VERIFIED | Substantive and wired. Used by the oracle test and by `reasoning_eval.rs`. |
| `crates/vox-eval/src/lib.rs` | `pub mod reasoning_bench;` | ✓ VERIFIED | |
| `crates/vox-compiler/tests/vox_think_oracle_test.rs` | Interpreter oracle | ✓ VERIFIED | 3 tests pass. |
| `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` | Tracked, fail-closed scorer | ✓ VERIFIED | Committed in `a02bdfc63`, hardened by `448899eb0`, `50ab81321`, `3bbaf52db`. |
| `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` | Routes reasoning benches through the scorer | ✓ VERIFIED | `score_bench` calls `cached_reasoning_report`. The cache stores only `Ok` reports, and an Err propagates, so no passing score appears on failure. |
| `crates/vox-ml-cli/src/commands/mens/mod.rs` | `reasoning_eval` registered under the gpu feature | ✓ VERIFIED | `mod.rs:40`. |
| `contracts/config/env-vars.v1.yaml` | Max-tokens env var registered | ✓ VERIFIED | Present once. |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| `reasoning_eval.rs` | `reasoning_bench::load_slice` | `load_slice(items_path, SLICE_SIZE)` | WIRED |
| `reasoning_eval.rs` | `BenchOutcome::from_score` | final line of `score_loaded` | WIRED |
| `eval_collateral.rs::score_bench` | `reasoning_eval::score_reasoning_file` | `cached_reasoning_report` | WIRED |
| `vox_think_oracle_test.rs` | `vox_eval::reasoning_bench::load_slice` | same loader as the scorer | WIRED |
| Slice directory names | `SLICE_NAMES` | `gsm8k_think`, `math500_think`, `vox_think` | WIRED |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real Data | Status |
|----------|------|--------|-----------|--------|
| `reasoning_report.json` (Scored) | `accuracy`, `n` | `score_completions` over real completions, passed through `from_score` | Yes, never a literal | ✓ FLOWING |
| `reasoning_report.json` (NotScored) | `n: 0`, `reason` | `fail_closed` / `score_loaded` | Yes, and there are no score keys | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Interpreter returns each gold | `cargo test -p vox-compiler --test vox_think_oracle_test` (run by me) | 3 passed | ✓ PASS |
| Slice shape, ids, ignore rules, template markers | `jq`, `rg`, `git ls-files`, `git check-ignore` (run by me) | Match the plan's pinned values | ✓ PASS |
| Scorer, collateral, qwen_reasoning, reasoning_bench | Not re-run by me: vox-ml-cli test build is blocked by the untracked `ui_perf_hazards.rs` (see notes). Earlier reported counts: `reasoning_eval` 12, `eval_collateral` 4 (now more after 315), `qwen_reasoning` 19, `reasoning_bench` 17 | Tests present; static inspection of the code paths matches | ? NOT RE-RUN (not a gap) |

### Probe Execution

SKIPPED. The phase declares no probe scripts.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| BENCH-02 | 314-01 | Held-out Vox slice with checkable answers, not the fixed template | ✓ SATISFIED | Truths 1 to 4. |
| BENCH-03 | 314-02 | Missing model or failed load writes no passing score and records the number of items scored | ✓ SATISFIED | Truths 5 to 8. |

No orphaned requirements. REQUIREMENTS.md maps only BENCH-02 and BENCH-03 to Phase 314. Their checkboxes and "Pending" status are for the orchestrator to update, and I did not edit them.

### Anti-Patterns Found

None blocking. There are no `TBD`, `FIXME` or `XXX` markers in the phase's implementation files, and no stubs. The `0.0` values in `score_completions` for an empty batch cannot reach a report, because `from_score` rejects `n == 0`.

### Human Verification Required

None.

### Gaps Summary

No must-have is missing. Phase 314 delivers three tracked 32-item slices with a pinned loader, interpreter-derived Vox golds, and a scorer that fails closed. The two things it deliberately does not do are running a model and writing a numeric baseline. A real before/after score is Phase 316 work, and the serve gate is Phase 315.

Advisory items, not blocking: WR-02 and WR-03 are deferred per the disposition. IN-01 to IN-04 are skipped info findings.

---

_Verified: 2026-10-26 (re-verification)_
_Verifier: Claude (gsd-verifier)_
