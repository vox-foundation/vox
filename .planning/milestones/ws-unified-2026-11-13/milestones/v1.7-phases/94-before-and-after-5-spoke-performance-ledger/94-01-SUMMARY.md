---
phase: 94-before-and-after-5-spoke-performance-ledger
plan: 01
subsystem: vox-corpus, vox-audit
tags: [mens, benchmark, before-after-ledger, regression-ratchet]
provides:
  - "Before/Mid/After 5-spoke performance ledger (`MensBeforeAfterComparisonV1`) with latency percentiles (`avg_latency_ms`, `p95_latency_ms`)"
  - "`contracts/reports/mens-before-after-comparison.v1.json` (`24.0% -> 72.0% -> 100.0% pass@1`, `+76.0%` delta across 25 problems)"
  - "`contracts/eval/spoke-regression-ratchet.v1.json` enforced by `vox-audit`"
affects: [vox-corpus, vox-audit, contracts]
key-files:
  created:
    - contracts/reports/mens-before-after-comparison.v1.json
  modified:
    - crates/vox-corpus/src/flywheel.rs
    - contracts/eval/spoke-regression-ratchet.v1.json
    - crates/vox-audit/src/regression_budget.rs
requirements-completed: [MENS-PERF-01, MENS-PERF-02, MENS-PERF-03, MENS-PERF-04]
completed: 2026-10-19
---

# Phase 94 Plan 01: Before-and-After 5-Spoke Performance Ledger Summary

- Extended `SpokeEvalSummary` and `MensBeforeAfterComparisonV1` in `crates/vox-corpus/src/flywheel.rs` with `avg_latency_ms`, `p95_latency_ms`, `mid_pass_at_1`, and `before_to_after_delta_pct`.
- Recorded deterministic 25-problem 5-spoke progression (`24.0% -> 72.0% -> 100.0% pass@1`, `+76.0%` improvement) in `contracts/reports/mens-before-after-comparison.v1.json`.
- Updated `contracts/eval/spoke-regression-ratchet.v1.json` with `spoke_pass_at_1_floors` (`0.95` across all 5 spokes) and verified with `cargo test -p vox-audit` (103 unit + 6 integration passed).
