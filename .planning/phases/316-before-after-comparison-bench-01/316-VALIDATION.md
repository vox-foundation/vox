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
| 316-01-01 | 01 | 1 | BENCH-01 | T-316 absence record | The absence record names `Qwen/Qwen3-8B` and the pinned revision, has status `not_compared`, and a recursive key walk finds no `accuracy`, no `think_initiation_rate`, and no `status: scored`. The compared shape has separate accuracy and think-initiation keys per slice and side | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ W0 | ⬜ pending |
| 316-01-02 | 01 | 1 | BENCH-01 | T-316 detection | Only the exact pinned snapshot directory counts as present. An empty hub root, a hub root with only Qwen3-0.6B, another revision, a pin without a 40-hex revision, a missing tokenizer, or a dangling shard is not present. The adapter needs both files and a manifest `base_model` equal to the pinned snapshot | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_compare` | ❌ W0 | ⬜ pending |
| 316-01-03 | 01 | 1 | BENCH-01 | T-316 scorer and output | With a fake scorer, both sides get identical checked-in items and all three slices on both sides are scored. One `NotScored` gives `incomplete` and `Err`. The absent branch never calls the scorer. No branch writes `collateral_damage_report.json` or `collateral_eval/`. An `--out` naming a serve-read file is refused. A stale compared report is replaced | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_compare` | ❌ W0 | ⬜ pending |
| 316-01-04 | 01 | 1 | BENCH-01 | — | The reused scorer still passes its existing tests | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` | ✅ | ⬜ pending |
| 316-01-05 | 01 | 1 | BENCH-01 | T-316 no invented score | The real run on this checkout takes the absent branch, writes the record, and the record has no score key | smoke | `cargo run -p vox-ml-cli --features gpu -- mens eval-reasoning-compare`, then `rg -c 'accuracy\|think_initiation' mens/runs/v1/reasoning_before_after.json` must find nothing (exit 1) | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

Task IDs are provisional until the plan exists. The planner may retag rows to match its tasks.

---

## Wave 0 Requirements

- [ ] Shape builders and tests in `crates/vox-eval/src/reasoning_bench.rs`
- [ ] `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` with its test module (tests first, then implementation)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| A real compared report with numeric scores | BENCH-01 (SC1) | Needs the pinned Qwen3-8B snapshot and an adapter, which are not on this checkout, and weights must not be downloaded in this phase | When both exist, run `vox mens eval-reasoning-compare --adapter <run dir>` and confirm all three slices on both sides carry separate accuracy and think-initiation keys |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 200s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
