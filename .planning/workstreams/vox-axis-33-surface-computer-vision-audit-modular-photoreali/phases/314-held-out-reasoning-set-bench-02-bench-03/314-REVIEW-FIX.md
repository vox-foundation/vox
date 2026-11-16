---
phase: 314-held-out-reasoning-set-bench-02-bench-03
fixed_at: 2026-10-26T16:55:00-07:00
review_path: /Users/brbrainerd/dev/vox/.planning/phases/314-held-out-reasoning-set-bench-02-bench-03/314-REVIEW.md
iteration: 1
findings_in_scope: 1
fixed: 1
skipped: 0
status: all_fixed
---

# Phase 314: Code Review Fix Report

**Fixed at:** 2026-10-26
**Source review:** 314-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 1 (scope override: WR-01 only)
- Fixed: 1
- Skipped: 0

## Fixed Issues

### WR-01: Unparseable `VOX_REASONING_EVAL_MAX_TOKENS` silently became 2048

**Files modified:** `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs`
**Commit:** 3bbaf52db
**Applied fix:** `max_tokens()` is replaced by a pure `parse_max_tokens(Option<&str>) -> Result<usize>`:
unset -> 2048; integer >= 1 -> that integer; integer < 1 (including 0 and negatives) -> 1;
set but not an integer after trim -> `Err`. `score_reasoning_file` now delegates to
`score_reasoning_file_with(.., raw_max_tokens)`, which parses the cap right after the adapter
check and routes a parse failure through `fail_closed` (NotScored report: `n` 0, no `accuracy`,
no `think_initiation_rate`, no `items`; returns `Err` naming the variable). The parsed cap is
passed explicitly to `run_backend` and `score_loaded`, so `score_loaded` no longer reads the
environment. A non-UTF-8 value counts as set (and fails). The registry text in
`contracts/config/env-vars.v1.yaml` is unchanged and still accurate. Scored reports do not
record `max_tokens` (optional; not added).

Verification (run in the main checkout, not a worktree; `workflow.use_worktrees` not consulted
because the tree is dirty and the orchestrator owns commits of the report):
- `rustfmt --edition 2024 --check` clean.
- `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval`: 12 passed, 0 failed
  (was 9; added `max_tokens_parse_rules`, `an_unparseable_max_tokens_is_not_scored_and_never_2048`,
  `the_token_cap_reaches_the_prompt`).
- Mutation check: making a parse failure fall back to 2048 made 2 of the new tests fail; reverted
  and re-run green.
- Tests pass the raw value as an argument and never mutate the process environment, so nothing
  leaks into other tests.
- `vox-eval` not touched, so `reasoning_bench` was not re-run.

## Skipped Issues

None. WR-02, WR-03, IN-01, IN-02, IN-03, IN-04 were out of scope per the override and not touched.

---

_Fixed: 2026-10-26_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
