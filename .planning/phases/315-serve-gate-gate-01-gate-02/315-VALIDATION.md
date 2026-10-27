---
phase: "315"
slug: "serve-gate-gate-01-gate-02"
status: draft
nyquist_compliant: true
wave_0_complete: false
created: "2026-10-26"
---

# Phase 315 — Validation Strategy

> Per-phase validation contract. Commands come from `315-RESEARCH.md`. No model is loaded. A green test is not a score.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | built-in `cargo test` |
| **Config file** | none |
| **Quick run command** | `cargo test -p vox-eval --lib reasoning_slice` |
| **Full suite command** | `cargo test -p vox-eval --lib collateral_damage` and `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral`, then `cargo test -p vox-ml-cli --features gpu --lib -- collateral_gate` |
| **Estimated runtime** | ~180 seconds |

---

## Sampling Rate

- **After every task commit:** Run that task's automated command
- **After every plan wave:** Run every command in the map
- **Before `/gsd-verify-work`:** Every command in the map must be green, with at least one passing test
- **Max feedback latency:** 180 seconds

`vox-ml-cli` filters must be `cargo test -p vox-ml-cli --features gpu --lib -- <filter>`. A line that says `0 passed` is a failed run. The generic zero-pre test on `eval_collateral_damage` must stay green.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 315-01-01 | 01 | 0 | GATE-01 | — | Accuracy drop above 5% on any slice fails; exactly 5% passes | unit | `cargo test -p vox-eval --lib reasoning_slice` | ❌ Wave 0 | ⬜ pending |
| 315-01-02 | 01 | 0 | GATE-01 | — | Think-initiation drop above 5% fails while accuracy is flat | unit | `cargo test -p vox-eval --lib reasoning_slice` | ❌ Wave 0 | ⬜ pending |
| 315-01-03 | 01 | 0 | GATE-01 | — | NaN, infinity, pre-score 0, and `n` other than 32 return Err | unit | `cargo test -p vox-eval --lib reasoning_slice` | ❌ Wave 0 | ⬜ pending |
| 315-01-04 | 01 | 0 | GATE-01 | — | Generic zero-pre behavior for non-reasoning benches stays | unit | `cargo test -p vox-eval --lib collateral_damage_zero_pre_score_avoids_divide_by_zero` | ✅ | ⬜ pending |
| 315-01-05 | 01 | 0 | GATE-01 | — | A missing partner key, zero reasoning pre, bad `n`, or failed score writes no report | unit | `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` | ✅ | ⬜ pending |
| 315-01-06 | 01 | 0 | GATE-01 | — | A pre-seeded passing report is removed when scoring fails | unit | `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` | ❌ Wave 0 | ⬜ pending |
| 315-02-01 | 02 | 2 | GATE-01 | — | Serve refuses a missing report, a fail status, a hand-edited pass, a missing slice, `not_scored`, and `n` 31 | unit | `cargo test -p vox-ml-cli --features gpu --lib -- collateral_gate` | ❌ Wave 0 | ⬜ pending |
| 315-02-02 | 02 | 2 | GATE-01 | — | Serve mutation checks and scoped clippy stay red when a guard is dropped, then green when restored | unit | `cargo test -p vox-ml-cli --features gpu --lib -- collateral_gate` | ❌ Wave 0 | ⬜ pending |
| 315-01-08 | 01 | 0 | GATE-02 | — | The report says 32-item slices, not a full set, and records `n` on pass and fail | unit | `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` | ❌ Wave 0 | ⬜ pending |
| 315-01-09 | 01 | 0 | GATE-02 | — | A non-reasoning bench keeps one score and has no slice field | unit | `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` | ❌ Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Strict reasoning evaluator tests in `vox-eval`, same file as the new function
- [ ] Writer tests in `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs`
- [ ] Serve gate tests in `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs`
- [ ] No framework install

New tests are written inside the task that needs them.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| None | — | Every gate behavior above has an automated command | — |

A green run is not a model score. Do not load Qwen3-8B.

---

## Validation Sign-Off

- [x] All tasks have automated verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 180s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
