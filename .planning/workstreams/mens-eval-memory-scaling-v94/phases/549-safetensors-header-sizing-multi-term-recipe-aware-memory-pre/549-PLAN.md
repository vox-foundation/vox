---
phase: 549-safetensors-header-sizing-multi-term-recipe-aware-memory-pre
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/vox-populi/src/mens/tensor/memory_model.rs
  - crates/vox-populi/src/mens/tensor/calibration.rs
  - contracts/mens/memory-model.v1.yaml
  - docs/src/architecture/measurements/2026-11-19-candle-metal-multi-term-calibration.json
autonomous: true
requirements:
  - MEM-FIT-01
  - MEM-FIT-02
  - MEM-FIT-03
  - MEM-FIT-04
must_haves:
  truths:
    - "project_qlora_shape reads safetensors header parameter counts so MoE checkpoints resolve cache_dequantized=false."
    - "predict_bytes_with_recipe matches 27B r16 --lora-exclude-lm-head at 43.28 GB (1.000x) and covers r32 lm_head 77.07 GB and 0.8B..9B BF16-cached peaks."
---

# Phase 549 Plan (Backfilled Record)

Implemented safetensors JSON-header sizing and multi-term recipe-aware memory fitting across 11 measured Metal runs in `memory_model.rs`, `calibration.rs`, and `contracts/mens/memory-model.v1.yaml`.
