---
phase: 547-early-moe-arch-rejection-throughput-telemetry-reconciliation
workstream: mens-eval-memory-scaling-v94
status: completed
completed_at: "2026-11-19T18:55:00-07:00"
---

# Phase 547 Summary: Early MoE/Arch Rejection & Throughput Telemetry Reconciliation

## What Changed
- `crates/vox-hf-layout/src/lib.rs`: Added `validate_supported_hf_config` and narrowed `classify_hf_architecture` so `qwen3_next`, `qwen3_5_moe*`, `gpt_oss`, `glm4_moe`, `qwen4_exp*`, `gemma3*`, `gemma4*`, `mamba`, and `rwkv` fail early with explicit `model_type` diagnostics.
- `crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/telem_helpers.rs` + Metal/CUDA training loop callers: Multiplied step throughput by `grad_accum` and added `supervised_tokens_per_sec`, reconciling `30.77` raw / `28.39` packed / `3.85` supervised tok/s across `mens/runs/qwen3.8-27b` (`108,813.49` s).
