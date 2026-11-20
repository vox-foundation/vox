---
phase: 548-harder-execution-verified-eval-heldout-bench-v2-verified-cor
workstream: mens-eval-memory-scaling-v94
milestone: v9.4
status: completed
---

# Phase 548 Context: Harder Execution-Verified Eval (`heldout_bench_v2`), Verified Corpus Expansion & `0.8B` Canary Proof

## Goal
Replace the saturated compile-only 52-task eval (`94.2%` on `27B`) with execution-verified interpreter harness scoring (`heldout_bench_v2`, 166 tasks + 64-task `heldout_bench_v2_canary`), build a zero-leakage compiler/execution-verified codegen corpus (`verified_vox_codegen_v2.jsonl`, 1,012 rows), re-score all 5 stored checkpoints on CPU, and establish a live `Qwen3.5-0.8B` Before/After canary baseline without disturbing `PID 34157`.

## Requirements Covered
- `EVAL-HARD-01..04`: Harness execution scoring (`test_harness`), 166/64 self-verified eval manifests, 5-rung CPU rescore, live `0.8B` canary Before/After (`3/64 -> 9/64`).
- `DATA-VER-01..04`: Leakage guard covering v1+v2 heldout manifests, `verified_vox_codegen_v2.jsonl` build & provenance report.
