---
phase: "314"
slug: "held-out-reasoning-set-bench-02-bench-03"
status: draft
nyquist_compliant: true
wave_0_complete: false
created: "2026-10-26"
---

# Phase 314 — Validation Strategy

> Per-phase validation contract. Commands come from `314-RESEARCH.md`. No model is loaded. A green test is not a score.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | built-in `cargo test` |
| **Config file** | none |
| **Quick run command** | `cargo test -p vox-eval --lib reasoning_bench` |
| **Full suite command** | every command in the map below, then `cargo check -p vox-ml-cli --features gpu --tests` |
| **Estimated runtime** | ~120 seconds |

---

## Sampling Rate

- **After every task commit:** Run that task's automated command
- **After every plan wave:** Run every command in the map for plans in that wave
- **Before `/gsd-verify-work`:** Every command in the map must be green, with at least one passing test
- **Max feedback latency:** 120 seconds

`vox-ml-cli` filters must be `cargo test -p vox-ml-cli --features gpu --lib -- <filter>`. A line that says `0 passed` is a failed run.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 314-01-01 | 01 | 0 | BENCH-02 | — | `vox_think` has 32 rows, unique ids, non-empty question and answer | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 | ⬜ pending |
| 314-01-02 | 01 | 0 | BENCH-02 | — | No Vox question uses the Plan / Syntax / Implementation template | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 | ⬜ pending |
| 314-01-03 | 01 | 0 | BENCH-02 | — | Every `vox_think` program returns its gold answer | integration | `cargo test -p vox-compiler --test vox_think_oracle_test` | ❌ Wave 0 | ⬜ pending |
| 314-01-04 | 01 | 0 | BENCH-02 | — | GSM8K and MATH-500 slices are 32 tracked rows | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 | ⬜ pending |
| 314-01-05 | 01 | 0 | BENCH-02 | — | The three jsonl files are not gitignored | shell | `git check-ignore -q mens/data/vox_think/reasoning.jsonl` exits 1 | ❌ Wave 0 | ⬜ pending |
| 314-01-06 | 01 | 0 | BENCH-02 | — | All 96 golds round-trip through `grade_final_answer` | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 | ⬜ pending |
| 314-02-01 | 02 | 1 | BENCH-03 | — | `NotScored` JSON has `n: 0` and no accuracy or think-initiation key | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 | ⬜ pending |
| 314-02-02 | 02 | 1 | BENCH-03 | — | `Scored` cannot be built from an empty batch | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 | ⬜ pending |
| 314-02-03 | 02 | 1 | BENCH-03 | — | A 31-row or 33-row slice is refused | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 | ⬜ pending |
| 314-02-04 | 02 | 1 | BENCH-03 | — | A not-scored write replaces a stale scored report | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` | ❌ Wave 0 | ⬜ pending |
| 314-02-05 | 02 | 1 | BENCH-03 | — | A missing adapter path returns an error and is not created | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` | ❌ Wave 0 | ⬜ pending |
| 314-02-06 | 02 | 1 | BENCH-03 | — | A scorer error writes no collateral report | unit | `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vox-eval/src/reasoning_bench.rs` and `pub mod reasoning_bench` in `lib.rs`
- [ ] `crates/vox-compiler/tests/vox_think_oracle_test.rs`
- [ ] Tests in `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs`
- [ ] A `.gitignore` negation so the three `reasoning.jsonl` files can be committed

New tests are written inside the tasks that need them. No framework install.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Changing one Vox gold makes the oracle fail | BENCH-02 | Mutation check, then restore | Edit one answer, run the oracle test, expect failure, restore |
| Replacing NotScored with a zero score fails the key-absence test | BENCH-03 | Mutation check, then restore | Make the change, run the reasoning_eval filter, expect failure, restore |

All other phase behaviors have automated verification. A green run is not a model score.

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
