---
phase: 86-multi-spoke-empirical-evaluation-suite-longitudinal-scorecards-promotion-gates
verified: 2026-10-16T14:12:30Z
status: passed
score: 5/5 must-haves verified
---

# Phase 86: Multi-Spoke Empirical Evaluation Suite, Longitudinal Scorecards & Promotion Gates — Verification

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `crates/vox-eval/src/mens.rs` evaluates all 5 spokes with `pass@1`, `pass@k`, syntax, semantic, and execution rates (`EVAL-01`) | VERIFIED | `evaluate_mens_five_spokes` + `evaluate_mens_five_spokes_computes_per_spoke_rates_and_coverage` |
| 2 | `crates/vox-eval/src/compare.rs` computes paired deltas, W/T/L counts, and non-inferiority (`EVAL-02`) | VERIFIED | `compare_mens_five_spokes` + `SpokeDeltaSummary` |
| 3 | `crates/vox-eval/src/trial_flywheel.rs` persists longitudinal scorecards linked to `snapshot_uri` and `adapter_trusty_uri` (`EVAL-03`) | VERIFIED | `MensLongitudinalScorecardV1` + `MensLongitudinalLedger` |
| 4 | `PromotionGateVerdict` blocks promotion on spoke regression or non-improving overall `pass@1` (`EVAL-04`) | VERIFIED | `compare_mens_five_spokes_enforces_non_inferiority_and_promotion_gate` |
| 5 | `crates/vox-eval/src/report.rs` renders Markdown and JSON comparison reports (`EVAL-05`) | VERIFIED | `render_multi_spoke_comparison_markdown_and_json_formats_table_and_verdict` |

## Verification Command Output

- `timeout 900s cargo test -p vox-eval --lib`: `test result: ok. 109 passed; 0 failed; 4 ignored; finished in 0.01s`
