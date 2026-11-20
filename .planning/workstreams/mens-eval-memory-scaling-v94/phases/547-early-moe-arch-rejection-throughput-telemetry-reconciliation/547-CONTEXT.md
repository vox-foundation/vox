---
phase: 547-early-moe-arch-rejection-throughput-telemetry-reconciliation
workstream: mens-eval-memory-scaling-v94
milestone: v9.4
status: completed
---

# Phase 547 Context: Early MoE/Arch Rejection & Throughput Telemetry Reconciliation

## Goal
Reject unsupported MoE (`qwen3_5_moe*`, `qwen3_next`, `glm4_moe`, etc.) and unsupported non-Qwen architectures (`qwen4_exp*`, `gpt_oss`, `gemma3*`, `gemma4*`, `mamba`, `rwkv`) early at `config.json` validation in `vox-hf-layout`, and fix the `grad_accum` omission in `telem_helpers::build_train_step_payload` so `telemetry.jsonl` reconciles `30.77 raw` / `28.39 packed` vs `3.85 supervised tok/s`.

## Requirements Covered
- `ARCH-GUARD-01..03`: Early config-parse rejection + `classify_hf_architecture` exclusion + unit tests in `vox-hf-layout` and `vox-populi`.
- `TELEM-TOK-01..03`: `grad_accum` scaling on `tokens_per_sec`, explicit `supervised_tokens_per_sec` emission, and exact reconciliation test against `mens/runs/qwen3.8-27b`.
