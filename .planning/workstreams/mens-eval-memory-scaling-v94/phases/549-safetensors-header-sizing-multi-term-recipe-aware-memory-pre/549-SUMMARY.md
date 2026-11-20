---
phase: 549-safetensors-header-sizing-multi-term-recipe-aware-memory-pre
workstream: mens-eval-memory-scaling-v94
status: completed
completed_at: "2026-11-19T19:01:00-07:00"
---

# Phase 549 Summary: Safetensors Header Sizing & Multi-Term Recipe-Aware Memory Predictor Recalibration

## What Changed
- `crates/vox-populi/src/mens/tensor/{memory_model,calibration}.rs`: Added `safetensors_header_text_params_and_bytes`, `TrainingRecipe`, `MultiTermMemoryModel` (`predict_bytes_with_recipe`), `fit_multi_term_lane`, and `derive_floor_mb`.
- `contracts/mens/memory-model.v1.yaml` & `docs/src/architecture/measurements/2026-11-19-candle-metal-multi-term-calibration.json`: Persisted the 11-run Metal calibration fit (`1.000x` exact on `27B r16` `43.28 GB`, `1.012x` on `27B r32 lm_head` `77.99 GB vs 77.07 GB`).
