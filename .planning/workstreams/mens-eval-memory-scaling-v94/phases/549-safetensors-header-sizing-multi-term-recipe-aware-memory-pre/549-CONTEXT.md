---
phase: 549-safetensors-header-sizing-multi-term-recipe-aware-memory-pre
workstream: mens-eval-memory-scaling-v94
milestone: v9.4
status: completed
---

# Phase 549 Context: Safetensors Header Sizing & Multi-Term Recipe-Aware Memory Predictor Recalibration

## Goal
Replace the single-slope `2B`-only Metal predictor (`73.80 GB` predicted vs `43.28 GB` actual on `27B` `r16 --lora-exclude-lm-head`) with a safetensors-header-backed, recipe-aware multi-term NNLS/envelope model fitted across all 11 measured Apple M5 Max runs, and ensure MoE checkpoints route to `cache_dequantized=false`.

## Requirements Covered
- `MEM-FIT-01..04`: Safetensors JSON-header parameter reader, 11-run Metal calibration dataset, `TrainingRecipe` + `MultiTermMemoryModel` (`predict_bytes_with_recipe`), and `derive_floor_mb`.
