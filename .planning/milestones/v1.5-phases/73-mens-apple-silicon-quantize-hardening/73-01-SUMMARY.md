---
phase: 73-mens-apple-silicon-quantize-hardening
plan: 01
status: complete
requirements: [MENS-01, MENS-02, MENS-03, MENS-04]
verified: true
date: 2026-10-14
---

# Plan Summary: 73-01 — Mens Apple Silicon Unified Memory, Qwen 3.8 27B & Quantize Hardening

## Overview
Consolidated MENS Apple Silicon calibration (`5a290248e`, `5cb8f9dec`), streaming `vox-quantize` memory bounds, MLX pre-flight refusal, and multi-backend serving/quantization lane unifications across `crates/vox-quantize`, `crates/vox-plugin-mens-candle-metal`, `crates/vox-plugin-mens-candle-cuda`, and `crates/vox-ml-cli`.

## Accomplished
1. **Task 1: Memory-Fit SSOT & Streaming `vox-quantize` (`MENS-01..02`)**:
   - `vox-quantize` walks tensor headers without whole-model resident loads (`open_enumerates_a_single_file_model_without_loading_tensor_data`).
   - Refuses incompatible MLX U32-packed checkpoints at pre-flight (`open_refuses_an_mlx_packed_checkpoint`).
   - Flushes output blocks and shards to disk within tight memory limits (`the_writer_flushes_shards_instead_of_holding_the_whole_output`, `recombine_does_not_hold_the_whole_model_resident`).
   - **Verification**: `cargo test -p vox-quantize --lib` passes with 62 passed, 0 failed.

2. **Task 2: Multi-Backend Serving & Locked `mens-*` Worktree Harvest (`MENS-03..04`)**:
   - Tied `validated.jsonl` and `mix_sources` to `data_dir` in `crates/vox-ml-cli/src/commands/mens/pipeline.rs`, resolving test isolation and ensuring pipeline runs fail closed when expected.
   - Retained and verified the restored `llama.cpp` GGUF lane and architecture-scoped Ollama conversions.
   - Harvested and unified all worktree fixes across `fix/cuda-mens-kv-cache-eos` (`3a015a024`), `fix/metal-hybrid-fallback-routing` (`8fb684d73`), and MLX calibration records.
   - **Verification**: `cargo test -p vox-ml-cli --lib` passes with 184 passed, 0 failed.

## Verification Evidence
- `target/debug/deps/vox_quantize-d0d2552fe7333bd9`: 62 passed, 0 failed (0.02s).
- `target/debug/deps/vox_ml_cli-11b76e5158fd0183`: 184 passed, 0 failed (53.19s).
