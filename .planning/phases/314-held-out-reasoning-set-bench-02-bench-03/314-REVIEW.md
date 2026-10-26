---
phase: 314-held-out-reasoning-set-bench-02-bench-03
reviewed: 2026-10-26T23:50:00Z
depth: standard
files_reviewed: 11
files_reviewed_list:
  - crates/vox-eval/src/reasoning_bench.rs
  - crates/vox-eval/src/lib.rs
  - crates/vox-compiler/tests/vox_think_oracle_test.rs
  - mens/data/gsm8k_think/reasoning.jsonl
  - mens/data/math500_think/reasoning.jsonl
  - mens/data/vox_think/reasoning.jsonl
  - .gitignore
  - crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs
  - crates/vox-ml-cli/src/commands/mens/mod.rs
  - crates/vox-ml-cli/src/commands/mens/eval_collateral.rs
  - contracts/config/env-vars.v1.yaml
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
status: issues_found
---

# Phase 314: Code Review Report

**Reviewed:** 2026-10-26
**Depth:** standard
**Files Reviewed:** 11 (read as committed on `cursor/t3-deeper-extraction-v55-510f`)

## Summary

The fail-closed contract holds in the code I traced. `BenchOutcome::from_score` refuses `n == 0` and any partial count. `NotScored` serialises with `n: 0` and no `accuracy` or `think_initiation_rate` keys. `ReasoningReport` is built only from `BenchOutcome::Scored`. Every `NotScored` path returns `Err`, and a missing adapter directory is never created. `load_slice` enforces count, unique ids, non-blank fields and a size cap. The three slices have 32 rows each and the `vox_think` programs are unique. `vox_think_oracle_test.rs` checks every gold against the interpreter. The new env var is registered in `env-vars.v1.yaml`. The `.gitignore` exception is correct.

I found no BLOCKERs. The three warnings are about whether a before/after comparison is trustworthy, not whether a missing model can write a passing score.

## Warnings

### WR-01: Token cap is not recorded in the report and silently falls back on a bad value

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs:95-100` (and the `Scored` report body, around `:227`)
**Issue:** `max_tokens()` falls back to 2048 when `VOX_REASONING_EVAL_MAX_TOKENS` fails to parse, for example `4k` or `4096 `. It does this without a warning. The `Scored` report records `temperature` and `prompt` but not `max_tokens`. The goal of this phase is a before/after comparison on the same items. Thinking-mode accuracy depends heavily on the cap (see WR-02). A "before" run with one cap and an "after" run with another, or a typo that silently reverts to 2048, produces two reports that look comparable and are not.
**Fix:**
```rust
// in write_outcome, Scored branch (thread max_tokens in, or re-read it once in score_reasoning_file)
map.insert("max_tokens".to_string(), serde_json::json!(max_tokens()));
```
Also `eprintln!` a warning, or return `Err`, when the env var is set but unparseable. That fits the fail-closed stance of the module.

### WR-02: Truncated reasoning is graded as a wrong answer with no marker in the report

**File:** `crates/vox-eval/src/qwen_reasoning.rs:153-160` (`answer_region`), reached from `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs:155-205`
**Issue:** If a completion opens `<think>` and never closes it, `answer_region` returns `""`, so `grade_final_answer` returns `false`. `score_loaded` counts that item as correctly scored and wrong, and `opened_nonempty_think` counts it as thinking. With a 2048-token cap on MATH-500 items, a model that thinks longer is scored the same as a model that is wrong. The phase locks make an inference error `not_scored`, but this case is neither an error nor a distinguishable result. Accuracy falls and think-rate rises, and the before/after delta then partly measures verbosity against the cap. The grader is unchanged by lock, so I am not asking for a grading change. The run should at least record how many items were truncated or unclosed.
**Fix:** Add a per-item `closed_think: bool` (or `truncated`) to the `items` array. Add an aggregate `unclosed_think_count` to the `Scored` body. Use `completion.contains("<think>") && !completion.contains("</think>")`. This does not change the score, but it makes WR-01 and WR-02 visible to whoever reads the before/after pair.

### WR-03: About a third of the MATH-500 golds are format-fragile under the string-match grader

**File:** `mens/data/math500_think/reasoning.jsonl` (rows with non-numeric gold); grader at `crates/vox-eval/src/qwen_reasoning.rs:169-181`
**Issue:** Non-numeric golds are matched by whitespace/`$`/comma-stripped string equality against the boxed body or the last line. Many gold strings are one of several equivalent surface forms: `x=5`, `\text{Evelyn}`, `90^\circ`, `11\sqrt2`, `52_8`, `1,-2`, `p - q`, `6+9i`, `-2 + 7i`, `\left( 3, \frac{\pi}{2} \right)`. A correct `\boxed{5}`, `\boxed{Evelyn}`, `\boxed{90}`, `\boxed{11\sqrt{2}}` or `\boxed{\dfrac{14}{3}}` scores wrong. `every_gold_round_trips_through_the_grader` only checks that the exact gold string grades true. It does not check that a plausible equivalent form does. With n = 32, each item is about 3.1 points, so several unreachable-by-formatting items add noise and a ceiling to the delta. This is a benchmark-quality issue, not a fail-closed issue.
**Fix:** Either prefer slice items whose gold is a bare integer or simple fraction, or add a normaliser pass for `\dfrac`/`\frac`, `\text{}`, `^\circ`, and `\sqrt2`/`\sqrt{2}`. Alternatively record the limitation in the slice `source` or `README` so a low MATH score is not read as a regression.

## Info

### IN-01: `bench` name from the baseline file is joined into filesystem paths unsanitised

**File:** `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs:143-166`
**Issue:** `bench` is a key from the operator's pre-score JSON. It is used in `Path::new(BENCH_ROOT).join(dir_name)` and `adapter.join("collateral_eval").join(bench)`. A key such as `../../x` or an absolute path escapes the intended root, and `create_dir_all` and `write` then run there. Input is operator-controlled, so this is low risk. The non-reasoning branch already behaved this way, but the reasoning branch adds a second write path.
**Fix:** Reject names containing a path separator, `..`, or an absolute path at the top of `score_bench`.

### IN-02: A failed `write_outcome` after a scored run can leave an older `scored` report on disk

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs:66`
**Issue:** `write_outcome(...)?` returns `Err` without going through `fail_closed`. If the write fails, for example on a read-only file, a previous `reasoning_report.json` with `status: scored` remains next to a run that returned `Err`. The serve gate does not read this file, and `fail_closed` already covers the other error paths. The stale report is still misleading to an operator.
**Fix:** On write error, make a best-effort `std::fs::remove_file` of the report path before returning `Err`.

### IN-03: `normalize_answer` strips commas, so `1,-2` and `1-2` compare equal

**File:** `crates/vox-eval/src/qwen_reasoning.rs:229`; gold `test/intermediate_algebra/1388.json` (`1,-2`)
**Issue:** Stripping commas is right for GSM8K thousands separators. It also collapses a two-element answer list into a different expression. Gold `1,-2` becomes `1-2`, and a model answering the arithmetic expression `1-2` would match. This is unlikely in practice. The cleaner fix is to replace that one item (WR-03).
**Fix:** Drop the item or special-case list golds.

### IN-04: Reasoning-bench routing depends on the process working directory

**File:** `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs:9,148`
**Issue:** `BENCH_ROOT` is the relative path `mens/data`. Run from another directory, `reasoning.is_file()` is false. The code falls through to the manifest lookup and fails with "has no manifest", which hides that the real problem is cwd. The failure is still closed, so this is not a scoring bug. The existing `eval-local` convention uses the same relative root.
**Fix:** Mention the cwd requirement in the manifest-missing error, or resolve `BENCH_ROOT` against the repo root.

---

_Reviewed: 2026-10-26_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
