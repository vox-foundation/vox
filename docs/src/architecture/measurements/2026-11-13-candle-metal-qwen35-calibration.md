---
title: "Candle-Metal QLoRA Memory Calibration & Qwen3.5/3.8 Ladder Floors (Nov 2026)"
description: "Two-point difference-quotient activation slope calibration for candle-metal with gradient checkpointing on Apple M5 Max and measured peak memory floors across Qwen3.5-2B/4B/9B and Qwen3.8-27B."
category: "Architecture SSOTs"
status: "current"
topic: "mens-training"
domain: "ml"
authorship: "llm-generated"
authorship_model: "google-deepmind-agent"
training_eligible: false
training_rationale: "Hardware calibration telemetry and memory-floor measurement record."
---

# Candle-Metal QLoRA Memory Calibration & Qwen3.5/3.8 Ladder Floors (Nov 2026)

<!-- kb:summary -->
Records the November 2026 live Metal calibration of Vox MENS QLoRA training with gradient checkpointing and chunked Gated DeltaNet on Apple M5 Max. Fits `act_bytes_per_lht = 162.48240311940512` bytes per `(layer * hidden * token)` in `contracts/mens/memory-model.v1.yaml` via two-point difference quotient on `Qwen/Qwen3.5-2B`, documents measured peak memory floors across the `Qwen3.5-2B/4B/9B` and `Qwen3.8-27B` ladder in `mens/config/gpu-specs.yaml`, and provides empirical wall-clock ETAs for full ladder training.
<!-- /kb:summary -->

## Host & Kernel Context

- **Accelerator:** Apple M5 Max (Metal unified memory), static `recommendedMaxWorkingSetSize = 115,448,725,504` bytes (`107.52 GiB` / `110,101` MiB), `maxBufferLength = 86,586,540,032` bytes (`80.64 GiB`).
- **Linear-Attention Speedup:** Vectorized causal depthwise conv1d + `DELTANET_CHUNK_SIZE = 64` chunked Gated DeltaNet (`crates/vox-plugin-mens-candle-metal/src/model.rs`) reduced `Qwen/Qwen3.5-2B` (`seq_len = 512`) first backward step wall time from `~21.3s/step` to `~2.8–3.2s/step`.
- **Snapshot Location:** Cached text-only snapshots under `~/.cache/huggingface/hub/models--Qwen--Qwen3.*`.
- **Raw Measurement File:** [`2026-11-13-candle-metal-qwen35-calibration.json`](./2026-11-13-candle-metal-qwen35-calibration.json) (`target/mens-cal/*/run_summary.json`).

## Two-Point Slope Derivation (`candle-metal`, `gradient_checkpointing = true`)

From `fit_a_lane` (`crates/vox-populi/src/mens/tensor/calibration.rs`) on `Qwen/Qwen3.5-2B` (`layers = 24`, `hidden = 2048`):

- `tokens_per_step = 512` (`target/mens-cal/2b-s512/run_summary.json`): `peak_bytes = 9,633,126,196` B (`9,187` MiB)
- `tokens_per_step = 2048` (`target/mens-cal/2b-s2048/run_summary.json`): `peak_bytes = 21,900,136,876` B (`20,886` MiB)

$$
\text{act\_bytes\_per\_lht} = \frac{21{,}900{,}136{,}876 - 9{,}633{,}126{,}196}{(2048 - 512) \times 24 \times 2048} = \frac{12{,}267{,}010{,}680}{75{,}497{,}472} = 162.48240311940512\;\text{B/LHT}
$$

## Ladder Floor & Timing Summary

| Run (`target/mens-cal/*`) | Model | Weight Mode | Shape (`L x H`, `rank`, `seq`) | Peak Bytes | Peak MiB | Floor MiB | Step Wall Time |
|---|---|---|---|---:|---:|---:|---:|
| `2b-s512` | `Qwen/Qwen3.5-2B` | `cache_dequantized=true` (BF16) | `24 x 2048`, r16, `seq=512` | `9,633,126,196` | `9,187` | — | `2.80 s/step` |
| `2b-preset` | `Qwen/Qwen3.5-2B` | `cache_dequantized=true` (BF16) | `24 x 2048`, r16, `seq=1024` | `19,649,785,102` | `18,740` | `19,500` | `3.20 s/step` |
| `2b-s2048` | `Qwen/Qwen3.5-2B` | `cache_dequantized=true` (BF16) | `24 x 2048`, r16, `seq=2048` | `21,900,136,876` | `20,886` | — | `12.72 s/step` |
| `4b-preset` | `Qwen/Qwen3.5-4B` | `cache_dequantized=true` (BF16) | `32 x 2560`, r16, `seq=1024` | `34,697,153,390` | `33,090` | `34,000` | `10.22 s/step` |
| `9b-preset` | `Qwen/Qwen3.5-9B` | `cache_dequantized=true` (BF16, untied 248k head) | `32 x 4096`, r16, `seq=1024` | `35,281,560,652` | `33,648` | `36,000` | `14.01 s/step` |
| `27b-s1024` | `Qwen/Qwen3.8-27B` | `cache_dequantized=false` (NF4 + BF16 embed/head) | `64 x 5120`, r32, `seq=1024` | `77,073,260,172` | `73,503` | `74,000` | `41.40 s/step` |
| `27b-preset` (spill ref) | `Qwen/Qwen3.8-27B` | `cache_dequantized=false` (NF4 + BF16 embed/head) | `64 x 5120`, r32, `seq=2048` | `118,280,866,252` | `112,802` | (exceeded `110,101` MiB) | `143.42 s/step` |

### Resident Weight Physics (`from_model_dir_for_qlora`)

- Models with `approx_linear_params <= 10B` (`2B`, `4B`, `9B`) set `cache_dequantized = true`, holding dequantized BF16 weights on GPU (`9B` untied `248,320` vocab embeddings/head = `19.31` GB + activations -> `33,648` MiB peak = `7.84x` nominal NF4 bare weights).
- Models above `10B` linear params (`Qwen3.8-27B`, `20.13B` approx linear params) keep `cache_dequantized = false`: GPU-resident NF4 linear blocks (`0.5 B/param` + FP32 absmax scale per 64 weights) plus BF16 embeddings and untied `lm_head` total `~19.28 GB`, predicting `~46.5 GB` (`43.3 GiB`) at `seq_len = 512` and `~72.6 GB` at `seq_len = 1024` (matching measured `77.07 GB` / `71.78 GiB` within `6%`).
- Switching `qwen38_27b` (`preset_schema.rs`) from `seq_len = 2048` to `seq_len = 1024` eliminated unified-memory swap thrashing, cutting step latency by `3.46x` (`143.42s` -> `41.40s/step`) and peak Metal allocation by `39,300` MiB (`112,802` -> `73,503` MiB).

## Empirical Phase 428 / Phase 429 Wall-Clock ETAs (`seq_len = 1024`, `batch_size = 1`)

- **Throughput rate per 1,000 micro-steps (empirical M5 Max):**
  - `Qwen/Qwen3.5-2B`: `~53 min` (`3.20 s/step`)
  - `Qwen/Qwen3.5-4B`: `~2.84 hours` (`10.22 s/step`)
  - `Qwen/Qwen3.5-9B`: `~3.89 hours` (`14.01 s/step`)
  - `Qwen/Qwen3.8-27B`: `~11.50 hours` (`41.40 s/step`)
- **Dogfood baseline (`target/dogfood/train.jsonl`, `1,485` rows -> `~1,411` packed steps/epoch, `3` epochs = `4,233` steps):**
  - Full 3-epoch wall time: `2B` `~3.8h`, `4B` `~12.0h`, `9B` `~16.5h`, `27B` `~48.7h` (vs `~168h` prior to `seq_len = 1024`). Phase 428 should explicitly budget optimizer steps / `max_runtime_secs` per rung on the full mixed corpus (`train_mixed_vox_lang.jsonl` = `7,089` lines, `train_mixed_rust.jsonl` = `25,440` lines).
