---
phase: 320-train-the-sizes-size-03
plan: 03
subsystem: mens-training
tags: [mens, qlora, metal, train-record, d-08, size-03]
status: complete
requires: [320-02]
provides:
  - "train-record.json (schema vox_mens_train_record_v1) after every local non-plan-only vox mens train run, success or failure"
  - "vox-ml-cli populi::train_record: RecordInput, build_train_record, write_train_record, report_record"
  - "Metal how-to: --plan-only/--json, contamination refusal, local --max-runtime-secs, --seq-len 3072, run_summary keys, train-record.json"
affects: [320-05, 320-06, 320-07, 320-08, 320-09]
tech-stack:
  added: []
  patterns: ["status from this run's own evidence only (summary mtime >= start, telemetry ts_ms >= start)", "RecordInput::start before the run, finish(..) after, so train_arm grows by 4 lines"]
key-files:
  created:
    - crates/vox-ml-cli/src/commands/mens/populi/train_record.rs
  modified:
    - crates/vox-ml-cli/src/commands/mens/populi/mod.rs
    - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
    - docs/src/how-to/how-to-train-mens-macos-metal.md
decisions:
  - "RecordInput owns its strings (no lifetime), and RecordInput::start/finish replace the plan's separate start/t0 locals plus an inline struct literal, to stay inside train_arm's remaining 5-line budget"
  - "Total wall time is measured from the SystemTime start (started.elapsed()), not a separate Instant"
  - "Telemetry rows without ts_ms count as older (fail-closed); zero byte readings are ignored"
  - "pin/size/explicit come only from the final size pick; a failed run keeps the operator's --model in requested_model"
metrics:
  duration: "~35 min"
  completed: 2026-10-27
actuals:
  tokens: 7600
  tasks: 2
  commits: 2
plan_head_before: ad901aa96f7477efd15bf94b570255e77355f25f
plan_head_after: 5333a44c7f0e115801725092adf91c38279b7ea1
---

# Phase 320 Plan 03: Per-run train records and the Metal how-to Summary

Every local `vox mens train` run that is not `--plan-only` now writes `<output-dir>/train-record.json`, whether it succeeds or fails. The record's `status` is `trained`, `capped` or `not_trained`, and it holds the requested model, pin, size, preset, steps, final loss, peak MB with its source, wall times, truncation counts, host GPU and the error text. Status comes only from evidence this run wrote: a `run_summary.json` whose mtime is before the run's start is ignored, and so are telemetry rows stamped before it. A run that OOMs is `not_trained` with the peak from its own telemetry. The Metal how-to now documents the Phase 320 operator surface.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | 088852e95 | feat(320-03): write train-record.json after every local run |
| 2 | 5333a44c7 | docs(320-03): document plan-only, contamination guard, run cap and train records |

## Tests

| Set | Result |
|-----|--------|
| `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(train_record) \| test(train_arm)'` | 16/16 pass (MIN 14). 7 `train_record::tests::` lines (need ≥6). The plan's awk check: OK |
| RED first | All 7 new tests failed (exit 100) against stubs: `build_train_record` returned `Value::Null`, and `write_train_record` was `todo!()` |

New tests (7): `fresh_summary_marks_trained_with_steps_loss_and_peak`, `capped_summary_marks_capped`, `failed_run_is_not_trained_with_error_and_telemetry_peak`, `ok_without_summary_is_not_trained`, `stale_summary_from_an_earlier_run_is_ignored`, `stale_telemetry_rows_do_not_count`, plus one extra, `write_puts_the_record_in_the_output_dir`. The telemetry fixtures put an unparsable line before each row, so the tests also prove such lines are skipped.

Lint and format:
- `cargo clippy -p vox-ml-cli --features cloud --lib --tests` finished. Its findings are all pre-existing and in other files (vox-corpus, vox-populi cloud, vox-orchestrator, vox-search, vox-ml-cli serve handlers). None names `train_record.rs`, `train_arm.rs` or `mod.rs`.
- `rustfmt --check` is clean on the three touched Rust files.
- `cargo run -p vox-doc-pipeline -- --lint-only --paths how-to/how-to-train-mens-macos-metal.md`: no hard errors. All 7 documented terms are present.

## Mutation proofs

| ID | Guard | Count before | Mutated count | Named test under mutation | Restore (diff + shasum) / count | Re-run |
|----|-------|--------------|---------------|---------------------------|----------------------------------|--------|
| M6 | `if modified < started {` -> `if false {` | 1 | 0 | `stale_summary_from_an_earlier_run_is_ignored` FAIL (exit 100; `left: "trained"`, `right: "not_trained"`) | equal, sha e44634dd…, count 1, `git status` clean | PASS |
| M7 | `if ts < started_ms {` -> `if false {` | 1 | 0 | `stale_telemetry_rows_do_not_count` FAIL (exit 100; `assertion failed: r["peak_mb"].is_null()`) | equal, sha e44634dd…, count 1, `git status` clean | PASS (both tests 2/2) |

## Acceptance

- `write_train_record(` appears exactly once in train_arm.rs. It comes after the `if opts.plan_only {` return and before `write_local_adapter_card(&card_output_dir`. The plan's awk ordering checks pass.
- `train_arm.rs` non-blank non-test lines: 985 at ANCHOR f36074f89, 995 after Plan 01, 999 now (+14 in total, limit 15). This plan added 4 lines (one `use`, `RecordInput::start`, `finish`, `report_record(write_train_record(..))`) and changed one line: `trained_preset` is now `pub(super)` so the record reuses the card's preset rule.
- `wc -l train_record.rs` = 292 (under 300, tests included).
- `git diff f36074f89 -- '*.toml' contracts/` is empty: no crate edge, Cargo feature or env var. `git diff f36074f89 --stat -- docs/` lists only the Metal how-to.
- The how-to no longer implies `--max-runtime-secs` is a no-op locally. A paragraph right after the example says how the local cap works.
- P-320-06 holds: `record_status` returns `trained` or `capped` only when `train_ok` is true and `fresh_summary` returned a summary. M6 proves the freshness guard.

## Deviations from Plan

**1. [Budget] `RecordInput` shape and constructors**
- **Why:** After Plan 01, train_arm.rs had 5 lines of budget left. The plan's version (`let started = SystemTime::now(); let t0 = Instant::now();` plus a 13-field `RecordInput` literal in train_arm) would not fit.
- **What changed:**
  - `RecordInput::start(epochs, seq_len, cap)` runs before `train::run_train`.
  - `.finish(&card_output_dir, &card_model, &card_preset, &train_res)` runs after it. It fills the pick, the preset (through `train_arm::trained_preset`), the host from `probe_gpu()` and the error text (`{e:#}`).
  - The fields own their strings (`Option<String>`/`String`/`PathBuf`) instead of borrowing (`&'a str`).
  - Total wall time is `started.elapsed()`, not a separate `Instant`.
  - The behaviour and the field set match the plan.
- **Files:** train_record.rs, train_arm.rs.
- **Commit:** 088852e95.

**2. [Addition] Extra test and helper**
- `write_puts_the_record_in_the_output_dir` covers the file write and directory creation, which the six planned tests leave untested.
- `report_record` prints the path, or warns, and never changes `train_res`.

**3. Verification harness substitutions (environment, not code)**
- The lean-ctx hook refuses some `cat`/`cp`/`sed` invocations on repo paths ("path escapes project root"), so files were read with the Read tool.
- The mutation backup came from `git show HEAD:<file>`, and restores used `git checkout -- <file>` (that one file only). Each restore was checked with `diff -q` against the backup plus `shasum`.
- Scratch files are under `$TMPDIR/gsd-320-03/`, not `/tmp/320-03-*`.

**Not done here (by plan):** The first real record is written by the S2 smoke run (Plan 05). No GPU training was run.

## Known Stubs

None.

## Threat Flags

None. The record holds paths, pins, the host GPU name and numbers, and no secrets (T-320-11, accepted). It reads only files in the run's own output directory.

## Self-Check: PASSED

- FOUND: crates/vox-ml-cli/src/commands/mens/populi/train_record.rs
- FOUND commits: 088852e95, 5333a44c7 (both ancestors of HEAD)
