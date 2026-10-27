---
phase: 320-train-the-sizes-size-03
plan: 02
subsystem: mens-training
tags: [mens, qlora, metal, run-cap, peak-memory, truncation, size-03]
status: complete
requires: [320-01]
provides:
  - "--max-runtime-secs caps a local Metal run at an optimizer-step boundary (capped: true)"
  - "LoraTrainingConfig.max_runtime_secs (host and plugin, serde default, same wire name)"
  - "run_summary.json / TRAIN_COMPLETE: capped, wall_secs, final_loss_ema, truncated_rows, prompt_trimmed_rows"
  - "peak_metal_allocated_bytes in every TRAIN_STEP row; OOM event records the sampler's peak"
affects: [320-03, 320-05, 320-06, 320-07]
tech-stack:
  added: []
  patterns: ["cap checked only where an optimizer step just completed", "labeled break ends the run without validation or finish_epoch"]
key-files:
  created: []
  modified:
    - crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
    - crates/vox-ml-cli/src/commands/schola/train/mod.rs
    - crates/vox-ml-cli/src/commands/schola/train/gpu.rs
    - crates/vox-populi/src/mens/tensor/training_config.rs
    - crates/vox-plugin-mens-candle-core/src/config.rs
    - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/mod.rs
    - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs
    - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/finalize.rs
decisions:
  - "The cap uses a labeled `break 'epochs` instead of `break` plus `if capped { break; }` after the pair loop: same control flow (no validation pass, no finish_epoch, no further epoch), fewer lines"
  - "Truncation is counted once per encoded row during the first epoch this process runs (epoch == start_epoch); a resumed or capped run counts only the rows it actually encoded"
  - "wall_secs is computed once before TRAIN_COMPLETE and reused for run_summary.json and the returned TrainingSummary"
metrics:
  duration: "~10 min"
  completed: 2026-10-27
actuals:
  tokens: 4100
  tasks: 2
  commits: 2
plan_head_before: 47362e01f845429bbf873ff849625a4f5d42ef32
plan_head_after: afedc44ebd34fedbdebb71107c3d128082551e50
---

# Phase 320 Plan 02: Metal run cap and run measurements Summary

`vox mens train --max-runtime-secs N` now caps a local Metal run. Training stops at the first optimizer-step boundary after N seconds of training (model load excluded, at least one optimizer step always completes) and finalizes normally, with `capped: true`. Every run also leaves peak Metal memory on disk while it runs (TRAIN_STEP rows) and on OOM. When it ends, it records wall time, final EMA loss and the D-04 truncation counts.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | 695df837d | feat(320-02): cap local Metal runs with --max-runtime-secs |
| 2 | afedc44eb | feat(320-02): record peak memory, truncation and final loss for Metal runs |

## Tests

| Set | Result |
|-----|--------|
| `cargo nextest run -p vox-plugin-mens-candle-metal --lib` (after Task 1) | 64/64 pass (MIN 5); 4 `cap_tests::` lines |
| `cargo nextest run -p vox-plugin-mens-candle-core --lib -E 'test(max_runtime_secs) \| test(checkpoint)'` | 15/15 pass (MIN 2) |
| `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(training_config) \| test(finetune_contract) \| test(manifest)'` | 32/32 pass (MIN 1) |
| `cargo check -p vox-ml-cli --features cloud --tests` | exit 0 |
| `cargo nextest run -p vox-plugin-mens-candle-metal --lib` (after Task 2) | 65/65 pass (MIN 6); includes `truncation_flags_cover_fit_trimmed_and_cut` |

New tests (6):
- Metal `cap_tests`: `no_cap_never_stops`, `cap_waits_for_the_first_optimizer_step`, `cap_stops_only_at_an_optimizer_boundary`, `cap_not_reached_before_its_time`, `truncation_flags_cover_fit_trimmed_and_cut`
- core `config::tests::max_runtime_secs_defaults_to_none_and_round_trips`
- host `training_config::semcov_wave26_tests::default_config_serializes_max_runtime_secs`

RED first:
- Against a `false` stub of `runtime_cap_reached`, `cap_stops_only_at_an_optimizer_boundary` failed (exit 100). The other three pass by design against a constant `false`.
- The core config test failed to compile (E0609/E0560, no field `max_runtime_secs`).
- Against a `(false, false)` stub, `truncation_flags_cover_fit_trimmed_and_cut` failed (exit 100, left `(false, false)` right `(true, false)`).
- The host populi test was not run separately in RED. Its RED state is the same missing field as core's.

Lint: scoped clippy `--lib --tests` on all four crates finished. The plan's features were used: none for metal and core, `cloud` for vox-ml-cli, `mens-train` for vox-populi. No finding names a touched file. `rustfmt --check` is clean on all 9 files.

## Mutation proofs

| ID | Guard | Count before | Mutated count | Named test under mutation | Restore (diff + shasum) / count | Re-run |
|----|-------|--------------|---------------|---------------------------|----------------------------------|--------|
| M5 | `if optimizer_steps == 0 \|\| !at_optimizer_boundary {` -> `if false {` | 1 | 0 | `cap_waits_for_the_first_optimizer_step` FAIL (exit 100, 3 tries) | equal, sha fe94821f…, count 1, `git status` clean | PASS |

## Acceptance

- Flag-to-loop chain grep: OK.
  - `pub max_runtime_secs: Option<u64>` appears once each in core config.rs and in host training_config.rs.
  - gpu.rs has `max_runtime_secs: opts.max_runtime_secs`.
  - The loop calls `runtime_cap_reached(`.
  - finalize.rs has `"capped": stats.capped`.
- Task 2 greps: OK.
  - `observed_bytes: 0,` count is 0.
  - `observed_peak_bytes()` appears 3 times in the loop (two OOM sites and the step payload).
  - finalize.rs has all five keys (`wall_secs`, `final_loss_ema`, `truncated_rows`, `prompt_trimmed_rows`, `capped`).
- `git diff 47362e01f -- '*.toml' contracts/` is empty. No crate edge, Cargo feature or `VOX_*` env var was added (P-320-04).
- `capped` is set only by the loop that stopped (P-320-05).
- No other full `LoraTrainingConfig` struct literal needed `max_runtime_secs: None`. The only full literal is gpu.rs's, and the rest use `..Default::default()`. The CUDA plugin has its own `TrainingLoopStats` and never reads the new field.

## Deviations from Plan

**1. [Budget] Metal `training_loop/mod.rs` line growth over the plan's budget**
- Non-blank lines against the anchor:
  - 694 -> 744 after Task 1 (+50, budget 40)
  - 744 -> 766 after Task 2 (+22, total +72, budget 60)
- I compressed what could be compressed:
  - a one-line test helper, `cap(...)`, so rustfmt does not split each assert over five lines
  - the labeled break
  - a tuple declaration for the two counters
  - the 4-line ponytail comment above the OOM event replaced by 1 line
- The remaining code is the plan's six tests, the two helpers and the wiring, as rustfmt lays them out. The 6-line `runtime_cap_reached(` call and the 4-line log are fixed by `fn_call_width`.
- I did not move the tests to a separate file just to pass the metric. The budget explicitly includes the test module.

**2. [Design within latitude] Labeled break instead of `break` + `if capped { break; }`**
- `break 'epochs` leaves both loops at once. Behaviour matches the plan: no validation pass, no `finish_epoch`, no further epoch, and the code after the epoch loop finalizes.

**3. Verification harness substitutions (environment, not code)**
- The lean-ctx shell allowlist blocks `perl -pi` and `sh -c`. The M5 mutation was applied with the Edit tool, restored with `cp` from a backup, and checked with `diff -q`, `shasum` and `git status`.
- Scratch logs went to `$TMPDIR/gsd-320-02/` instead of `/tmp/320-02-*`.
- The plan's awk pass checks were evaluated by reading each nextest `Summary` line.

**Not done here (by plan):** the live proof that a capped Metal run stops and writes `capped: true` is the S2 smoke run (Plan 05). No GPU training was run.

## Known Stubs

None.

## Threat Flags

None. The change adds one optional config field that crosses the existing JSON plugin boundary with `serde(default)`, plus extra keys in files the trainer already writes.

## Self-Check: PASSED

- FOUND commits: 695df837d, afedc44eb (both ancestors of HEAD)
- FOUND: all 9 modified files; no file created
