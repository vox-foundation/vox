# Requirements: Milestone v7.3 (`mens-nov2026-v73`)

## Phase 425 — Stage D Pre-Flight Code Seams
- **MENS-NOV26-PREFLIGHT-01:** `vox_populi::mens::hub::download_model` fetches `generation_config.json` and `special_tokens_map.json` when available on HF and exposes them on `DownloadedModelFiles`.
- **MENS-NOV26-PREFLIGHT-02:** `vox_ml_cli::commands::mens::reasoning_compare::detect::snapshot_complete` filters `weight_map` shards through `!vox_hf_layout::is_vision_or_mtp_key(key)` (via `vox_populi::mens::hub::is_text_tower_weight_key`) so text-only VLM snapshots pass completeness checks.
- **MENS-NOV26-PREFLIGHT-03:** `vox_populi::mens::hub_card::facts::compare_json_path`, `size_train`, and `size_select` resolve `Qwen3.5-{2B,4B,9B}` and `Qwen3.8-27B` while maintaining backwards-compatible `qwen3_code` tests.
- **MENS-NOV26-PREFLIGHT-04:** `vox-ml-cli mens hub download` supports `--cache-only` without copying shards and verifies snapshot completeness.

## Phase 426 — Parallel HF Prefetch & Corpus Regeneration
- **MENS-NOV26-PREP-01:** Authorized HF text-only snapshots (`Qwen3.5-2B@15852e8c...`, `Qwen3.5-4B@851bf6e8...`, `Qwen3.5-9B@c2022362...`, `Qwen3.8-27B@37561816...`, plus `Qwen3-0.6B`) are cached with `generation_config.json` and zero vision-only shards.
- **MENS-NOV26-PREP-02:** Fresh compiler-verified Vox/Rust/Docs/GSD/Synthetic/Agent corpora are extracted and validated into `mens/data/`.
- **MENS-NOV26-PREP-03:** Spoke mix configs (`vox-lang`, `rust`, `tool-selection`, `argument-generation`, `agents`, `harness`) build non-empty training datasets with zero held-out benchmark overlap.

## Phase 427 — Metal Calibration & Floor Measurement
- **MENS-NOV26-CAL-01:** Fit `candle-metal` `act_bytes_per_lht` from two distinct `tokens_per_step` runs on `Qwen3.5-2B` via `fit_a_lane`.
- **MENS-NOV26-CAL-02:** Record empirical `peak_mb`, safe `floor_mb`, `floor_measured: true`, and `measured_on: silicon-gpu-metal` across `2B`, `4B`, `9B`, `27B` in `mens/config/gpu-specs.yaml` and `contracts/mens/memory-model.v1.yaml`.
- **MENS-NOV26-CAL-03:** Store machine-portable measurement artifact & doc under `docs/src/architecture/measurements/` with relative paths only.

## Phase 428 — Packed QLoRA Ladder Training & Adapter Merge
- **MENS-NOV26-TRAIN-01:** Fine-tune `Qwen3.5-2B` (`--lora-exclude-lm-head`) with packed sequences & rsLoRA and verify `merge-qlora`.
- **MENS-NOV26-TRAIN-02:** Fine-tune `Qwen3.5-4B` (`--lora-exclude-lm-head`) with packed sequences & rsLoRA and verify `merge-qlora`.
- **MENS-NOV26-TRAIN-03:** Fine-tune `Qwen3.5-9B` (with `lm_head`) with packed sequences & rsLoRA and verify `merge-qlora`.
- **MENS-NOV26-TRAIN-04:** Fine-tune `Qwen3.8-27B` (with `lm_head`) cleanly within M5 Max Metal memory ceiling and verify `merge-qlora`.

## Phase 429 — Paired Evaluation & Gate Receipts
- **MENS-NOV26-EVAL-01:** Run base vs candidate held-out `eval-local` on all trained rungs.
- **MENS-NOV26-EVAL-02:** Run hash-pinned `eval-reasoning-compare` and `eval-collateral-damage` McNemar checks.
- **MENS-NOV26-EVAL-03:** Generate `eval-gate` receipts (`gate_receipt.json`) consumed dynamically by `hub_card::facts` (`size-train` retired in commit `9a429892f`).

## Phase 430 — Promotion, Hub Cards & Milestone Audit
- **MENS-NOV26-PROMOTE-01:** Promote gate-verified rungs in `hub_repo.rs`, `mens/config/hub-cards.yaml`, and card renderers (strict validator compliance).
- **MENS-NOV26-PROMOTE-02:** Sync `docs/src/reference/mens-models.md` and `docs/src/architecture/mens-model-selection-nov-2026-research-2026-11-13.md` with empirical Stage D outcomes.
- **MENS-NOV26-PROMOTE-03:** Pass `vox-arch-check`, `vox-doc-pipeline --lint-only`, card lints, hardcoded-path audit, and complete `v7.3-MILESTONE-AUDIT.md`.
