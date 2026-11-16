---
phase: 321-harness-before-after-harn-01-bench-04
status: passed
verified: 2026-11-01
requirements:
  - HARN-01
  - BENCH-04
---

# Phase 321 Verification: Harness Before/After (`HARN-01`, `BENCH-04`)

## Goal-Backward Verification

| Requirement / Must-Have | Evidence | Status |
|---|---|---|
| `HARN-01`: Chosen Qwen3 size resolves to pinned base snapshot or verified adapter behind `assert_collateral_gate` + `assert_serve_preconditions` | `commands::mens::harness_before_after::tests::harn_01_serves_chosen_size_base_and_gated_adapter` | Passed |
| `BENCH-04`: Consolidated `harness_before_after.v1` report holds base and adapter reasoning accuracy + think-initiation on the 3 slices (`n=32`) and intelligence (`pass_rate_at_1`, `n`) for every trained size (`>= 2` sizes), while untrained sizes carry `status: "not_trained"` and no numeric score keys | `commands::mens::harness_before_after::tests::bench_04_two_trained_sizes_report_reasoning_and_intelligence_and_untrained_have_no_scores` | Passed |
