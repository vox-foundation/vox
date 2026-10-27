---
phase: "316"
slug: "before-after-comparison-bench-01"
status: draft
nyquist_compliant: true
wave_0_complete: false
created: "2026-10-26"
---

# Phase 316 — Validation Strategy

> Per-phase validation contract. Commands come from `316-RESEARCH.md`. No model is loaded and no weights are downloaded. A green test is not a score.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | built-in `cargo test`, `tempfile` for filesystem fixtures |
| **Config file** | none |
| **Quick run command** | `cargo test -p vox-eval --lib reasoning_bench` |
| **Full suite command** | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_compare`, then `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval`, then `cargo check -p vox-ml-cli --features gpu` |
| **Estimated runtime** | ~200 seconds warm (a cold `vox-ml-cli` compile took 3m12s) |

---

## Sampling Rate

- **After every task commit:** Run that task's automated command
- **After every plan wave:** Run every command in the map
- **Before `/gsd-verify-work`:** Every command in the map must be green, with at least one passing test, plus the real absent-branch run and its key grep
- **Max feedback latency:** 200 seconds

`vox-ml-cli` filters must be `cargo test -p vox-ml-cli --features gpu --lib -- <filter>`. A line that says `0 passed` is a failed run. Do not run workspace `cargo test`. The three pre-existing clippy errors (`qwen_reasoning.rs:178`, `trial_flywheel.rs:208`, `trial_flywheel.rs:784`) are not phase work.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 316-01-01 | 01 | 1 | BENCH-01 | T-316 absence record, detection, output | Tracer. The absence record names the requested pinned base, has status `not_compared`, and a recursive key walk finds no `accuracy`, no `think_initiation_rate`, and no `status: scored`. Only the exact pinned snapshot directory counts. An empty hub root, a root with only Qwen3-0.6B, another revision, or a pin without a 40-hex revision is not present. The absent branch never calls the scorer. An `--out` naming `collateral_damage_report.json`, `reasoning_report.json`, a `collateral_eval` path, the cache, or the adapter directory is refused and writes nothing. A stale compared report is replaced. The refusal, absent-branch, and stale tests go through `run_eval_reasoning_compare_with`. `default_out_is_outside_a_runs_v1_adapter` and `report_file_name_is_per_base_and_path_safe` pass. `reasoning_bench` reaches at least 21 | unit | `cargo test -p vox-eval --lib reasoning_bench` and `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_compare` | ❌ W0 | ⬜ pending |
| 316-01-01 | 01 | 1 | BENCH-01 | T-316 no invented score | The real run on this checkout prints `NOT COMPARED: base_snapshot_absent`, writes the record, and the record has no score key | smoke | `cargo run -p vox-ml-cli --features gpu -- mens eval-reasoning-compare`, then `rg -c 'accuracy\|think_initiation' mens/runs/reasoning_compare/Qwen3-8B-b968826d9c46.json` must find nothing (exit 1) | ❌ W0 | ⬜ pending |
| 316-01-02 | 01 | 1 | BENCH-01 | T-316 completeness, adapter pin, scorer | A missing config or tokenizer or a dangling shard is incomplete, not present. The adapter needs both files and a manifest `base_model` that resolves to the pinned snapshot. With a fake scorer, both sides get identical checked-in items and all six slice-side entries carry separate accuracy and think-initiation keys. One `NotScored` gives `incomplete` and `Err`. `slice_load_failure_is_not_scored_on_both_sides` and `scorer_outcomes_are_matched_by_name` pass; detection tests live under `reasoning_compare::detect`. No branch writes `collateral_damage_report.json` or `collateral_eval/` | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_compare` | ❌ W0 | ⬜ pending |
| 316-01-02 | 01 | 1 | BENCH-01 | — | The reused scorer still passes its existing tests | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` | ✅ | ⬜ pending |
| 316-01-03 | 01 | 1 | BENCH-01 | T-316 base resolution | `--base` accepts a pinned `hf_id@sha` or a size name looked up in `train_bases`, defaults to `DEFAULT_MODEL_ID`, and records an unknown or ambiguous name as `pin_unverifiable`. `cli_entry_records_the_resolved_pin` passes. Each of the eight guards, including the two call sites, fails its test when broken on purpose. Each new file has 1 to 449 non-blank lines before `#[cfg(test)]`. The how-to paragraph has no numbers | unit + mutation + lint | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_compare`, `cargo clippy -p vox-eval --lib --tests -- -D warnings`, and the scoped `vox-ml-cli` clippy log check | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

Rows are tagged to the three tasks in `316-01-PLAN.md`.

---

## Wave 0 Requirements

- [ ] Shape builders and tests in `crates/vox-eval/src/reasoning_bench.rs`
- [ ] `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` with its test module (tests first, then implementation)
- [ ] `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs` with its test module

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| A real compared report with numeric scores | BENCH-01 (SC1) | Needs the pinned Qwen3-8B snapshot and an adapter, which are not on this checkout, and weights must not be downloaded in this phase | When both exist, run `vox mens eval-reasoning-compare --adapter <run dir>` and confirm `mens/runs/reasoning_compare/<size>-<rev12>.json` shows that all three slices on both sides carry separate accuracy and think-initiation keys |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 200s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
