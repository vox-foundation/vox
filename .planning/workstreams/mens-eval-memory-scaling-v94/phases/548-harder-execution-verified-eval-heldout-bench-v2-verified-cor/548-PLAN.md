---
phase: 548-harder-execution-verified-eval-heldout-bench-v2-verified-cor
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/vox-ml-cli/src/commands/mens/metrics.rs
  - crates/vox-ml-cli/src/commands/mens/eval_local.rs
  - crates/vox-ml-cli/src/commands/mens/eval_local_prompt.rs
  - crates/vox-ml-cli/src/commands/mens/v94_eval_corpus.rs
  - crates/vox-ml-cli/src/commands/mens/v94_eval_tasks.rs
  - crates/vox-ml-cli/src/commands/mens/v94_eval_tasks_algo.rs
  - crates/vox-ml-cli/src/commands/mens/v94_eval_tasks_domain.rs
  - crates/vox-ml-cli/src/commands/corpus/source_pool.rs
  - mens/config/vox-source-pool.yaml
  - mens/data/heldout_bench_v2/manifest.json
  - mens/data/heldout_bench_v2_canary/manifest.json
  - mens/data/verified_vox_codegen_v2.jsonl
  - contracts/reports/mens-execution-eval-rescore-v2.json
  - contracts/reports/mens-canary-0.8b-before-after-v2.json
  - contracts/reports/mens-verified-corpus-v2.json
autonomous: true
requirements:
  - EVAL-HARD-01
  - EVAL-HARD-02
  - EVAL-HARD-03
  - EVAL-HARD-04
  - DATA-VER-01
  - DATA-VER-02
  - DATA-VER-03
  - DATA-VER-04
must_haves:
  truths:
    - "166 heldout_bench_v2 tasks and 64 canary tasks pass 100% reference execution and fail wrong-return stubs."
    - "verified_vox_codegen_v2.jsonl compiles 100% cleanly with 0 overlap against v1 and v2 heldout sets."
---

# Phase 548 Plan (Backfilled Record)

Implemented interpreter `test_harness` scoring (with `max_eval_depth = 96` + `16 MiB` scoped stack), generated `heldout_bench_v2` / `heldout_bench_v2_canary` and `verified_vox_codegen_v2.jsonl`, re-scored 5 stored rungs offline, and ran the live `0.8B` canary via debug CLI without touching `PID 34157`.
