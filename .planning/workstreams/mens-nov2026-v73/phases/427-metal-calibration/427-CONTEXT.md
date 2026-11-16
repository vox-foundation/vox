# Phase 427 Context: Metal Memory Model Slope Calibration & Ladder Floor Measurement

- **Workstream:** `mens-nov2026-v73` (`Milestone v7.3`)
- **Goal:** Fit `candle-metal` `act_bytes_per_lht` from two `Fits` points on `Qwen3.5-2B`, measure real Metal QLoRA peak floors across `4B`, `9B`, `27B`, record machine-portable measurement artifacts, and update `contracts/mens/memory-model.v1.yaml` and `mens/config/gpu-specs.yaml`.
- **Execution Mode:** Strictly sequential single-GPU Metal execution.
