---
phase: 548-harder-execution-verified-eval-heldout-bench-v2-verified-cor
workstream: mens-eval-memory-scaling-v94
status: completed
completed_at: "2026-11-19T18:58:00-07:00"
---

# Phase 548 Summary: Harder Execution-Verified Eval (`heldout_bench_v2`), Verified Corpus Expansion & `0.8B` Canary Proof

## What Changed
- `metrics.rs` / `eval_local.rs`: Added `run_harness_assertions` and `verify_completion_with_harness` enforcing interpreter test assertions and `semantic_expected_contains`, with `16 MiB` stack guard and `max_eval_depth = 96`.
- `mens/data/heldout_bench_v2/manifest.json` (`166` tasks) & `mens/data/heldout_bench_v2_canary/manifest.json` (`64` tasks): Self-verified `166/166` reference pass and `166/166` wrong-stub fail.
- `mens/data/verified_vox_codegen_v2.jsonl` (`1,012` unique verified rows, `0` leakage against v1+v2): Filtered `47.1%` (`895/1,902`) failing `dogfood_v4` rows.
- `contracts/reports/mens-execution-eval-rescore-v2.json` & `mens-canary-0.8b-before-after-v2.json`: Re-scored stored 5 rungs (`0.8B 21.7% .. 27B 69.6%` strict) and recorded live `0.8B` canary Before/After (`3/64 = 4.7%` -> `9/64 = 14.1%`).
