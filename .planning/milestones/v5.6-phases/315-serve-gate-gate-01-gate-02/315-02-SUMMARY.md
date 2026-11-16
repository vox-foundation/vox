---
phase: 315-serve-gate-gate-01-gate-02
plan: 02
subsystem: eval
tags: [collateral-damage, reasoning-slices, serve-gate, mens, vox-eval, vox-ml-cli]

requires:
  - phase: 315-serve-gate-gate-01-gate-02
    provides: strict writer, reasoning_slices block, BenchScore, run_collateral_damage_with (plan 01)
provides:
  - vox_eval::reasoning_slice::verify_reasoning_slices, a pure re-validation of a stored collateral report
  - assert_collateral_gate in dispatch.rs, called from the serve arm before assert_serve_preconditions
  - operator how-to text for the six baseline keys and the serve re-check
affects: [316]

plan_head_before: 26a9e90d8c695e0f3cd21e216a1a5d578102d77c
plan_head_after: 6c5bbd7cb6c13bba1cac95dadb8517087e035ff2

actuals:
  tokens: 9000
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Serve recomputes from stored pre/post numbers with the writer's own eval_reasoning_slice; stored status, degradation_rate and exceeds_threshold are ignored"
    - "Slice paths built only from SLICE_NAMES, never from report content"

key-files:
  created: []
  modified:
    - crates/vox-eval/src/reasoning_slice.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
    - docs/src/how-to/how-to-train-mens-macos-metal.md

key-decisions:
  - "Stored threshold must equal CollateralDamageConfig::default(); recompute always uses the default config"
  - "scope_note must equal REASONING_SLICE_SCOPE_NOTE exactly; slice_size and per-slice n must be 32"
  - "Marker condition manifest_path.exists() || adapter_path.exists() is unchanged; only its body was replaced"
  - "No numeric equality between the collateral report and reasoning_report.json (D-10): status scored and n 32 only"

requirements-completed: [GATE-01, GATE-02]

completed: 2026-10-26
status: complete
---

# Phase 315 Plan 02: Serve re-validates the collateral report Summary

**`vox mens serve` now refuses an adapter unless its on-disk collateral report re-validates: the 32-item slice block is labelled and complete, both metrics on all three slices are recomputed from the stored numbers within 5%, and every `collateral_eval/<slice>/reasoning_report.json` is scored with n 32. A hand-edited or old-format `pass` no longer binds.**

## Accomplishments

- `verify_reasoning_slices` in `vox-eval`: checks threshold, `scope_note`, `slice_size`, per-slice `n` and four numbers per slice, iterating `SLICE_NAMES`, then calls `eval_reasoning_slice` and refuses a recomputed drop naming `failed_key()`.
- `assert_collateral_gate` in `dispatch.rs`: missing-report and FAILED messages kept verbatim, malformed JSON refused, per-slice report must be present, valid JSON, `status` `scored`, `n` 32. Reads only; no model, no write.
- Serve arm: the inline status check body is replaced by `assert_collateral_gate(&model)?;` then the unchanged `assert_serve_preconditions(&model)?;`. The condition and marker files are untouched.
- How-to: names the six baseline keys, says a missing key is a hard error, and says serve re-checks the 32-item block and does not trust status alone. No numeric baseline written.
- No model was run, no score invented, no env var or dependency added.

## Task Commits

1. **Task 1: Tracer, serve accepts a writer-produced report and refuses missing or old-format** - `5c0006c88` (feat)
2. **Task 2: Serve refuses tampered, stale, and unscored evidence** - `3d9e9a467` (test)
3. **Task 3: How-to names six keys and the serve re-check** - `6c5bbd7cb` (docs; amended once to make the literal phrase `status alone` appear, before any further commit)

## Test Commands and Counts

| Command | Result |
|---|---|
| `cargo test -p vox-ml-cli --features gpu --lib -- collateral_gate` | Task 1: 3 passed. Task 2: 12 passed |
| `cargo test -p vox-eval --lib reasoning_slice` | Task 1: 18 passed. Task 2: 26 passed |
| `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` | 17 passed (315-01 writer tests intact) |
| `cargo test -p vox-eval --lib collateral_damage_zero_pre_score_avoids_divide_by_zero` | 1 passed |
| `cargo test -p vox-eval --lib collateral_damage` | 10 passed |
| `cargo test -p vox-ml-cli --features gpu --lib -- serve_preconditions` | 3 passed (see Deviations) |
| `cargo check -p vox-ml-cli --features gpu --tests` | exit 0 |
| `cargo test -p vox-gui --bins collateral` | 1 passed (GUI reader, unchanged) |

The Task 3 verify script printed `VERIFY3_OK` (six keys, `32-item`, `status alone`, text outside the section byte-identical, commit holds only the how-to).

## Mutation Checks

Each had a before/after presence count of the mutated text (0 then 1) and was restored by copying the saved original (diff-identical).

- `verify_reasoning_slices` refusal skipped (`failed_key().filter(|_| false)`): vox-eval `a_stored_pass_over_a_degraded_accuracy_is_recomputed_and_refused` and `a_stored_pass_over_a_degraded_think_initiation_is_refused_on_the_init_key` failed (24 passed, 2 failed); dispatch `a_hand_edited_pass_over_a_ten_percent_accuracy_drop_is_refused` and `a_hand_edited_pass_over_a_think_initiation_drop_is_refused` failed (10 passed, 2 failed). Restored: 26 and 12 passed.
- Slice `status` check accepts anything (`true || status == Some("scored")`): `a_not_scored_slice_report_is_refused` failed (11 passed, 1 failed). Restored: 12 passed.
- Slice `n` check skipped (`true || n == Some(..)`): `a_slice_report_with_n_31_is_refused` failed (11 passed, 1 failed). Restored: 12 passed.

## Deviations from Plan

### Auto-fixed Issues

None in code.

### Notes

- **Plan miscount, `serve_preconditions` minimum:** the plan's gate expects at least 4 passed; `serve_preconditions_tests` holds 3 tests at the plan base (verified with `git show HEAD:` before editing) and 3 now. None were lost. The awk `>= 4` gate cannot pass as written; the right baseline is 3.
- **Clippy (scoped):** `cargo clippy -p vox-eval --lib -- -D warnings` and the `vox-ml-cli` run (which compiles `vox-eval`) both exit 101 on the same three errors found in 315-01, all outside this plan's files: `qwen_reasoning.rs:178`, `trial_flywheel.rs:208`, `trial_flywheel.rs:784`. A non-denying `cargo clippy ... --lib --tests` on both crates exited 0 and printed no diagnostic naming `reasoning_slice.rs` or `dispatch.rs`.
- **GUI command:** `cargo test -p vox-gui --lib collateral` fails ("no library targets"); the equivalent `--bins collateral` was run instead (1 passed).
- **Task 1 RED:** tests and implementation were written together; RED/green evidence for the guards is the three mutation checks above.
- **Tooling:** `StrReplace` and the readers returned compressed views, so edits were spliced with shell scripts from fragment files; header lines and line counts were checked after each splice. No compressed view was written back.
- The research doc and `scripts/mens-macos-metal-e2e.vox` were never staged. The tree was dirty with unrelated work; every commit held only its task's paths.

## Known Stubs

None.

## Issues Encountered

None blocking.

## Next Phase Readiness

- Phase 316 still owes real scores. Nothing here is a model result.
- Out of scope, unchanged: an adapter directory with only a different adapter filename (for example `adapter_model.safetensors`) keeps its existing behavior.

## Self-Check: PASSED

- `fn assert_collateral_gate` defined once; `assert_collateral_gate(&model)` called once in the serve arm; `eval_collateral_damage check not found!` and `check FAILED.` once each.
- Commits `5c0006c88`, `3d9e9a467`, `6c5bbd7cb`: found in HEAD history.
