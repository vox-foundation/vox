---
phase: "320"
slug: "train-the-sizes-size-03"
status: draft
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-27"
---

# Phase 320 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest (Rust unit tests in-crate; one vox-populi integration target), jq gates over run artifacts for the operations plans |
| **Config file** | `.config/nextest.toml` (existing) |
| **Quick run command** | the task's scoped `timeout <n>s cargo nextest run -p <crate> [--features ...] --lib -E '<filter>'`, checked with the inline awk pass check (`n == passed and n >= MIN`) |
| **Full suite command** | not run locally (hosted CI is the gate); per-plan scoped sets: vox-ml-cli `--features cloud` `test(plan_only) / test(schola::train) / test(train_arm) / test(train_record) / test(reasoning_compare)`; vox-corpus `test(preflight)`; vox-populi `--features mens-train` `test(hub::) / test(size_select) / test(spoke_base_resolver)` and `--test training_presets_yaml_contract`; vox-plugin-mens-candle-core and -metal `--lib` |
| **Estimated runtime** | 2-15 min per scoped set on a warm target dir; operations stages from minutes (S0-S2) to many hours (S4) |

---

## Sampling Rate

- **After every code task commit:** the task's scoped nextest set plus its end-to-end check (plan-only binary run for Plan 01).
- **After every code plan:** scoped clippy on the touched crates with `-p` and the plan's features, and `rustfmt --edition 2024 --config-path /Users/brbrainerd/dev/vox-mens-compare --check` on touched files.
- **Operations plans:** each stage writes its evidence file and a last line `Stage S<n>: PASS`; the next stage's first task has a `<precondition>` on that line (D-01).
- **Before `/gsd-verify-work`:** `evidence/size-03-check.txt` ends in `SIZE-03: PASS` (Plan 09).
- **Max feedback latency:** about 15 minutes for code tasks; operations stages report at each interval check.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 320-01-01 | 01 | 1 | SIZE-03 | T-320-02, T-320-03 | `--plan-only` resolves everything and stops before any download, write or heal; refusals match a real run | unit + e2e | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(plan_only) \| test(schola::train)'` (MIN 27) then the debug-binary plan-only run with `VOX_MENS_NO_DOWNLOAD=1` and jq | new `plan_only.rs` (test-first in task) | pending |
| 320-01-02 | 01 | 1 | SIZE-03 | T-320-01, T-320-04 | rows containing a held-out question are refused with their line numbers | unit + e2e | `cargo nextest run -p vox-corpus --lib -E 'test(preflight)'` (MIN 10), clean and dirty plan-only runs; M1, M2 | yes | pending |
| 320-01-03 | 01 | 1 | SIZE-03 | T-320-02 | `base_cached` true only for the pinned revision; plan-only stop proven | unit + e2e | `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(hub::)'` (MIN 15); M3, M4; scoped clippy and rustfmt | yes | pending |
| 320-02-01 | 02 | 2 | SIZE-03 | T-320-05, T-320-06, T-320-07 | a local cap stops only at an optimizer boundary after one step and is reported as `capped` | unit | `cargo nextest run -p vox-plugin-mens-candle-metal --lib`; core `-E 'test(max_runtime_secs) \| test(checkpoint)'`; populi `-E 'test(training_config) \| test(finetune_contract) \| test(manifest)'`; M5 | yes | pending |
| 320-02-02 | 02 | 2 | SIZE-03 | T-320-08 | OOM events and step telemetry carry the real peak; summary has truncation, loss and wall time | unit + grep | `cargo nextest run -p vox-plugin-mens-candle-metal --lib`; greps for `observed_peak_bytes()` and summary keys; scoped clippy on four crates | yes | pending |
| 320-03-01 | 03 | 3 | SIZE-03 | T-320-09, T-320-10, T-320-11 | the record's status comes only from this run's evidence | unit | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(train_record) \| test(train_arm)'` (MIN 14); M6, M7 | new `train_record.rs` (test-first in task) | pending |
| 320-03-02 | 03 | 3 | SIZE-03 | — | how-to documents the new operator surface | doc lint | `cargo run -q -p vox-doc-pipeline -- --lint-only --paths how-to/how-to-train-mens-macos-metal.md` plus the term grep loop | yes | pending |
| 320-04-01 | 04 | 4 | SIZE-03 | T-320-12, T-320-15 | binaries from HEAD; installed plugin byte-identical to the metal build; S0 live pick and refusal parity | ops | Task 1 verify in `320-04-PLAN.md` (version hash, `cmp` of dylibs, pick line, `cmp` of refusals) | n/a | pending |
| 320-04-02 | 04 | 4 | SIZE-03 | T-320-14 | corpus has no empty or duplicate rows, at least 100 rows, think/direct counted | ops | Task 2 verify (jq over the chosen corpus) | n/a | pending |
| 320-04-03 | 04 | 4 | SIZE-03 | T-320-13 | every size's plan is contamination-clean over 96 questions with its pin | ops | Task 3 verify (jq per size, `Stage S1: PASS`) | n/a | pending |
| 320-05-01 | 05 | 5 | SIZE-03 | T-320-19 | 0.6B smoke: loss falls, pin in manifest and summary, peak above zero | ops | Task 1 verify (jq over record, summary, telemetry, loss) | n/a | pending |
| 320-05-02 | 05 | 5 | SIZE-03 | T-320-16, T-320-17, T-320-18 | resume refuses another base and keeps the checkpoint; serve refused at the gate; adapter answers | ops | Task 2 verify (`cmp` of checkpoint base, exit codes, eval-local jq) | n/a | pending |
| 320-05-03 | 05 | 5 | SIZE-03 | T-320-16 | D-05 serve item settled by the user, gate not bypassed | decision | manual (blocking-human checkpoint) | n/a | pending |
| 320-06-01 | 06 | 6 | SIZE-03 | T-320-20, T-320-23 | ladder file starts from S2; 1.7B verdict from measurement | ops | Task 1 verify (jq over `ladder.json`) | n/a | pending |
| 320-06-02 | 06 | 6 | SIZE-03 | T-320-21, T-320-22, T-320-23 | climb stops at the first non-fit; every verdict measured | ops | Task 2 verify (jq ladder ordering and measurement rules) | n/a | pending |
| 320-06-03 | 06 | 6 | SIZE-03 | — | S3 evidence committed inside the phase directory | ops | Task 3 verify (`Stage S3: PASS`, commit scope) | n/a | pending |
| 320-07-01 | 07 | 7 | SIZE-03 | T-320-24, T-320-26 | full 0.6B pass, pin named, loss falls | ops | Task 1 verify | n/a | pending |
| 320-07-02 | 07 | 7 | SIZE-03 | T-320-24, T-320-25, T-320-26, T-320-27 | every fitting size trained (or capped for 14B/32B) with its pin, or a recorded divergence/OOM | ops | Task 2 verify (loop over fitting sizes) | n/a | pending |
| 320-07-03 | 07 | 7 | SIZE-03 | T-320-25 | non-fitting sizes have measured not-trained records and no adapter | ops | Task 3 verify (`Stage S4: PASS`) | n/a | pending |
| 320-08-01 | 08 | 8 | SIZE-03 | T-320-28, T-320-29 | measured rungs exactly match `MEASURED_QLORA_PINS`; floors only rise | integration | `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract` (MIN 5); M8 | yes (assertion rewritten test-first) | pending |
| 320-08-02 | 08 | 8 | SIZE-03 | T-320-30, T-320-31 | ladder and resolver tests state measured floors; 6144 MB fail-closed kept | unit | `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(size_select) \| test(spoke_base_resolver)'` (MIN 55); clippy | yes | pending |
| 320-08-03 | 08 | 8 | SIZE-03 | T-320-31 | vox-ml-cli floor-dependent tests agree with the measurement | unit | `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(schola::train) \| test(train_arm) \| test(train_record) \| test(reasoning_compare)'` (MIN 58); clippy | yes | pending |
| 320-09-01 | 09 | 9 | SIZE-03 | T-320-32 | SC1 and SC2 checked against the real run directories | ops | Task 1 verify (`SIZE-03: PASS`, six size lines, five stage files) | n/a | pending |
| 320-09-02 | 09 | 9 | SIZE-03 | T-320-33 | report complete, commit scoped to the phase directory | ops | Task 2 verify | n/a | pending |

*Status: pending · green · red · flaky*

---

## Mutation Proofs

Protocol (every row): back up the file; count the guard (expect the stated value); mutate; run the named test with `--no-fail-fast` (expect FAIL); restore from the backup and `cmp`; recount; re-run (expect PASS). All five values go in the plan's SUMMARY.

| ID | Plan | Guard (verbatim) | Mutation | Must fail |
|----|------|------------------|----------|-----------|
| M1 | 01 | `if !hits.is_empty() {` | `if false {` | `heldout_question_in_a_row_is_refused_with_its_line` |
| M2 | 01 | the filter dropping questions whose normalized form is empty | line removed | `punctuation_only_question_matches_nothing` |
| M3 | 01 | `if let Some(rev) = repo_revision(repo_id) {` | `if let Some(_rev) = None::<&str> {` | `pinned_id_needs_its_own_revision_in_cache` |
| M4 | 01 | `if opts.plan_only {` (gpu.rs) | `if false {`, rebuild | plan-only e2e run must hit `Downloading base model` and the `VOX_MENS_NO_DOWNLOAD is set` refusal |
| M5 | 02 | `if optimizer_steps == 0 \|\| !at_optimizer_boundary {` | `if false {` | `cap_waits_for_the_first_optimizer_step` |
| M6 | 03 | `if modified < started {` | `if false {` | `stale_summary_from_an_earlier_run_is_ignored` |
| M7 | 03 | `if ts < started_ms {` | `if false {` | `stale_telemetry_rows_do_not_count` |
| M8 | 08 | `floor_measured: true` on the 0.6B rung of `gpu-specs.yaml` | `floor_measured: false` (evidence fields removed) | `gpu_specs_qwen3_code_ladder_pins_all_six_dense_sizes` |

---

## Stage Gates (D-01)

| Stage | Plan | Evidence | Gate line | Next stage's precondition |
|-------|------|----------|-----------|---------------------------|
| S0 plan-only, binaries, plugin | 04 | `320-S0-EVIDENCE.md`, `evidence/s0-*` | `Stage S0: PASS` | Plan 04 Task 2 |
| S1 corpus and per-size plans | 04 | `320-S1-CORPUS.md`, `evidence/s1-*` | `Stage S1: PASS` | Plan 05 Task 1 |
| S2 0.6B smoke | 05 | `320-S2-SMOKE.md`, `evidence/s2-*` | `Stage S2: PASS` (after the D-05 decision) | Plan 06 Task 1 |
| S3 probes | 06 | `320-S3-PROBES.md`, `evidence/s3-*`, `evidence/ladder.json` | `Stage S3: PASS` | Plan 07 Task 1 (plus the 24 h projection bound) |
| S4 runs | 07 | `320-S4-RUNS.md`, `evidence/s4-*` | `Stage S4: PASS` | Plan 08 Task 1 |
| SIZE-03 | 09 | `evidence/size-03-check.txt`, `320-LADDER-REPORT.md` | `SIZE-03: PASS` | `/gsd-verify-work` |

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. New tests are written first inside their own tasks (`plan_only.rs`, `train_record.rs`, the preflight, hub, cap and round-trip tests), per the repository's test-first policy.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| How D-05's serve item is satisfied | SIZE-03 (D-05) | `vox mens serve` is refused by the Phase 315 gate until Phase 321's HARN-01 chain runs; changing what satisfies a locked decision needs the user | Plan 05 Task 3: read `evidence/s2-serve.txt` and `evidence/s2-eval-local.json`, reply `accept-eval-local`, `gate-chain-now` or `stop` |
| A full run projected over 24 h | SIZE-03 (D-07) | spending more than a day of the Mac is the user's call | Plan 07 Task 1 precondition halts with the projection from `evidence/ladder.json` |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or are a decision checkpoint
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none)
- [x] No watch-mode flags
- [x] Feedback latency bounded per task (scoped sets under `timeout`)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
