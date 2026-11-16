---
phase: 315-serve-gate-gate-01-gate-02
fixed_at: 2026-10-26T18:30:00-07:00
review_path: .planning/phases/315-serve-gate-gate-01-gate-02/315-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 315: Code Review Fix Report

**Fixed at:** 2026-10-26
**Source review:** 315-REVIEW.md
**Iteration:** 1

**Scope override:** only WR-01 and WR-04 were in scope. WR-02, WR-03, IN-01, IN-02, IN-03 were deliberately not applied.

**Verification ran in:** main checkout (dirty tree, no worktree), explicit-path commits only.

**Summary:**
- Findings in scope: 2
- Fixed: 2
- Skipped: 0

## Fixed Issues

### WR-01: how-to overstates "hand-edited pass is refused"

**Files modified:** `docs/src/how-to/how-to-train-mens-macos-metal.md`
**Commit:** bb0dad31d
**Applied fix:** Narrowed the sentence. Serve recomputes from the stored pre and post numbers; a flipped status, a missing slice block, an old-format report, or numbers that recompute past 5% are refused. A consistent rewrite of pre and post is a new baseline and is not detected as an edit. Kept the "32-item" and "status alone" phrases. No sample baseline or score added. Architecture doc and `scripts/mens-macos-metal-e2e.vox` untouched.

### WR-04: range-guard test fixture rejected earlier by serde_json

**Files modified:** `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs`
**Commit:** 84711c6c7
**Applied fix:** `an_out_of_range_stored_number_is_refused` now stores `1.5` (valid JSON, outside [0, 1]) instead of `1e999`, and asserts the error contains `outside [0, 1]` and `gsm8k_think`, so only the range guard can satisfy it. rustfmt (edition 2024) clean.

**Mutation check (guard in `crates/vox-eval/src/reasoning_slice.rs`):** changed the `(0.0..=1.0)` check to `if false && (...)`. Result: `an_out_of_range_stored_number_is_refused` FAILED (11 passed, 1 failed). Guard restored; file shows no diff against HEAD; re-run green.

## Test results

- `cargo test -p vox-ml-cli --features gpu --lib -- collateral_gate`: 12 passed, 0 failed (before and after mutation cycle)
- `cargo test -p vox-eval --lib reasoning_slice`: 26 passed, 0 failed (file not modified in the commits)

---

_Fixed: 2026-10-26_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
