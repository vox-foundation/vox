---
phase: 314
phase_name: "held-out-reasoning-set-bench-02-bench-03"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 16
  lessons: 11
  patterns: 6
  surprises: 3
missing_artifacts:
  - "UAT.md"
---

# Phase 314 Learnings: held-out-reasoning-set-bench-02-bench-03

## Decisions

### gsm8k_think cut to the first 32 rows byte-for-byte; only the source suff...
gsm8k_think cut to the first 32 rows byte-for-byte; only the source suffix changed from 64 to 32

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 314-01-SUMMARY.md

---

### math500_think content and source unchanged; no licence claim added
math500_think content and source unchanged; no licence claim added

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 314-01-SUMMARY.md

---

### vox_think gold = value returned by the interpreter for main, read from t...
vox_think gold = value returned by the interpreter for main, read from the oracle failure output, never hand-computed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 314-01-SUMMARY.md

---

### Size cap enforced with Read::take(MAX_SLICE_BYTES + 1); no new crate edg...
Size cap enforced with Read::take(MAX_SLICE_BYTES + 1); no new crate edge

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 314-01-SUMMARY.md

---

### .gitignore negation !mens/data/*_think/reasoning.jsonl tracks the slices...
`.gitignore` negation `!mens/data/*_think/reasoning.jsonl` tracks the slices; `mens/data/train.jsonl` stays ignored.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 314-01-SUMMARY.md

---

### gsm8k_think cut to gsm8k-test-0000..0031; math500_think unchanged (test/...
`gsm8k_think` cut to `gsm8k-test-0000..0031`; `math500_think` unchanged (`test/precalculus/807.json` .. `test/prealgebra/1302.json`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 314-01-SUMMARY.md

---

### vox_think: 11 trace, 11 debug, 10 plan-check items (ids vox-think-0000.....
`vox_think`: 11 trace, 11 debug, 10 plan-check items (ids `vox-think-0000..0031`), pure programs using `fn`/`type`/`match`/lists/loops, each question ending with the fixed final-integer line.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 314-01-SUMMARY.md

---

### Oracle test runs every program through lex, parse, lower and eval and co...
Oracle test runs every program through lex, parse, lower and eval and compares `VoxValue::Int` to the gold.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 314-01-SUMMARY.md

---

### VOX_REASONING_EVAL_LIMIT deleted (it truncated the slice and conflicts w...
VOX_REASONING_EVAL_LIMIT deleted (it truncated the slice and conflicts with expected_n 32); VOX_REASONING_EVAL_MAX_TOKENS kept and registered as kind int, owner vox-ml-cli

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 314-02-SUMMARY.md

---

### Whitespace-only generated_text counts as empty, so the whole run is NotS...
Whitespace-only generated_text counts as empty, so the whole run is NotScored

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 314-02-SUMMARY.md

---

### Missing adapter path returns Err with 'scored 0 of 32 items' and writes/...
Missing adapter path returns Err with 'scored 0 of 32 items' and writes/creates nothing

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 314-02-SUMMARY.md

---

### score_loaded reads the token cap itself (max_tokens()) so its signature ...
score_loaded reads the token cap itself (max_tokens()) so its signature stays (bench, items, infer)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 314-02-SUMMARY.md

---

### BenchOutcome in vox-eval
`BenchOutcome` in `vox-eval`: `from_score` keeps the empty-batch `{0.0, 0.0, 0}` and any partial count out of `Scored`; `to_json` omits score keys for `NotScored`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 314-02-SUMMARY.md

---

### reasoning_eval.rs is in git
`reasoning_eval.rs` is in git. It loads slices through `load_slice(.., SLICE_SIZE)`, stops at the first inference error / unparseable / empty response without grading that item, and builds `ReasoningReport` only from `Scored`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 314-02-SUMMARY.md

---

### The truncation override is gone; the max-tokens override is registered i...
The truncation override is gone; the max-tokens override is registered in the same commit that first tracked the scorer.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 314-02-SUMMARY.md

---

### No model weights, numeric baseline, serve gate, or before/after comparis...
No model weights, numeric baseline, serve gate, or before/after comparison were produced.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 314-02-SUMMARY.md

## Lessons

### Task 1's committed test module held only its 3 tests; Task 2's 7 tests w...
Task 1's committed test module held only its 3 tests; Task 2's 7 tests were added in its own commit, as the plan splits them.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 314-01-SUMMARY.md

---

### Interpreter quirks found while authoring, and avoided in the data
Interpreter quirks found while authoring, and avoided in the data: `.pop()` returns a bare value (not `Option`), `state` is a reserved word, list slicing `xs[0..n]` is unsupported, and struct construction by `Point(3, 4)` fails at run time. Programs use `.push`, indexed assignment, `.unwrap()` on indexing, `range`, ADTs with payloads, and `match`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 314-01-SUMMARY.md

---

### The programs were run only through the interpreter; vox check (typecheck...
The programs were run only through the interpreter; `vox check` (typecheck) was not run on them.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 314-01-SUMMARY.md

---

### StrReplace was not used; all file writes were whole-file writes or sed/a...
`StrReplace` was not used; all file writes were whole-file writes or sed/awk against on-disk bytes, with header and line-count checks after each.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 314-01-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Pre-commit hook reordered the mod lines. The Task...
**1. [Rule 3 - Blocking] Pre-commit hook reordered the `mod` lines.** The Task 1 commit's `mens/mod.rs` has `pub(crate) mod reasoning_eval;` (with its `#[cfg(feature = "gpu")]`) at its rustfmt-sorted position after `plugin_heal` instead of after `eval_collateral`. Still exactly two added lines; no other `mod` moved. The index briefly held the pre-sort blob; I unstaged that path (`git reset -q -- mod.rs`) before the Task 2 commit, which then held exactly its two paths.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 314-02-SUMMARY.md

---

### 2. [Tooling] StrReplace on reasoning_bench.rs wrote a context-compressed...
**2. [Tooling] `StrReplace` on `reasoning_bench.rs` wrote a context-compressed view back to disk** (file shrank to 231 lines with a tool stub at the end). Restored with `git checkout -- crates/vox-eval/src/reasoning_bench.rs` (file was clean) and redid the edits by splicing files from the shell. No commit was affected.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 314-02-SUMMARY.md

---

### 3. Test count. Added 7 ml-cli tests in Task 1-2 (plan minimum 4) and 2 i...
**3. Test count.** Added 7 ml-cli tests in Task 1-2 (plan minimum 4) and 2 in Task 3; `three_of_32_correct_stores_exact_accuracy`, `empty_missing_or_unparseable_text_is_not_scored`, and `an_empty_slice_is_never_scored` are extra pins on the locked rules.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 314-02-SUMMARY.md

---

### None. WR-02, WR-03, IN-01, IN-02, IN-03, IN-04 were out of scope per the...
None. WR-02, WR-03, IN-01, IN-02, IN-03, IN-04 were out of scope per the override and not touched.

**Context:** Surfaced in structured phase code review.
**Source:** 314-REVIEW-FIX.md

---

### _Fixed: 2026-10-26_ _Fixer: Claude (gsd-code-fixer)_ _Iteration: 1_
_Fixed: 2026-10-26_ _Fixer: Claude (gsd-code-fixer)_ _Iteration: 1_

**Context:** Surfaced in structured phase code review.
**Source:** 314-REVIEW-FIX.md

---

### No must-have is missing
No must-have is missing. Phase 314 delivers three tracked 32-item slices with a pinned loader, interpreter-derived Vox golds, and a scorer that fails closed. The two things it deliberately does not do are running a model and writing a numeric baseline. A real before/after score is Phase 316 work, and the serve gate is Phase 315.

**Context:** Verified in phase acceptance checks.
**Source:** 314-VERIFICATION.md

---

### Advisory items, not blocking
Advisory items, not blocking: WR-02 and WR-03 are deferred per the disposition. IN-01 to IN-04 are skipped info findings.

**Context:** Verified in phase acceptance checks.
**Source:** 314-VERIFICATION.md

## Patterns

### load_slice refuses missing/empty id (no item-N default), duplicate ids, ...
load_slice refuses missing/empty id (no item-N default), duplicate ids, blank question/answer, wrong count, oversize file

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 314-01-SUMMARY.md

---

### slice loader with pinned count
slice loader with pinned count

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 314-01-SUMMARY.md

---

### size cap and id uniqueness; gold derived from interpreter run
size cap and id uniqueness; gold derived from interpreter run

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 314-01-SUMMARY.md

---

### ReasoningReport is built in exactly one place, from BenchOutcome::Scored
ReasoningReport is built in exactly one place, from BenchOutcome::Scored

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 314-02-SUMMARY.md

---

### NotScored JSON omits accuracy and think_initiation_rate (absent, not nul...
NotScored JSON omits accuracy and think_initiation_rate (absent, not null or 0)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 314-02-SUMMARY.md

---

### outcome enum with no numeric stand-in for not-scored; score_loaded seam ...
outcome enum with no numeric stand-in for not-scored; score_loaded seam taking an infer closure so the failure paths are tested without a model

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 314-02-SUMMARY.md

## Surprises

### Estimate variance on 314-01
Estimate variance on 314-01: planned 70000 tokens vs 28000 actual (0.40x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 314-01-PLAN.md, 314-01-SUMMARY.md

---

### Estimate variance on 314-02
Estimate variance on 314-02: planned 60000 tokens vs 17000 actual (0.28x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 314-02-PLAN.md, 314-02-SUMMARY.md

---

### 1. [Rule 3 - Blocking] Pre-commit hook reordered the mod lines. The Task...
**1. [Rule 3 - Blocking] Pre-commit hook reordered the `mod` lines.** The Task 1 commit's `mens/mod.rs` has `pub(crate) mod reasoning_eval;` (with its `#[cfg(feature = "gpu")]`) at its rustfmt-sorted position after `plugin_heal` instead of after `eval_collateral`. Still exactly two added lines; no other `mod` moved. The index briefly held the pre-sort blob; I unstaged that path (`git reset -q -- mod.rs`) before the Task 2 commit, which then held exactly its two paths.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 314-02-SUMMARY.md

