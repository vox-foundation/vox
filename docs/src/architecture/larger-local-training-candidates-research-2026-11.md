---
title: "Larger Local Training Candidates Beyond 27B (November 2026)"
description: "Verified landscape, trainer audit, quantization options and quality evidence for QLoRA-training bases above the Qwen3.8-27B rung on a 128 GB Apple Silicon Mac, with a ranked decision table and a verification-gap list."
category: "Architecture SSOTs"
status: "research"
topic: "mens-training"
domain: "ml"
authorship: "llm-assisted"
authorship_model: "google-deepmind-agent"
training_eligible: false
---

# Larger Local Training Candidates Beyond 27B (November 2026)

<!-- kb:summary -->
Qwen3.8-27B trained at a 43.28 GB Metal peak (`41,274 MiB`) with `--lora-exclude-lm-head`. Phases 547–550 (`mens-eval-memory-scaling-v94`) reconcile throughput (`30.77` raw / `28.39` packed / `3.85` supervised tok/s), add early MoE/unsupported architecture rejection in `vox-hf-layout`, recalibrate Metal memory via multi-term NNLS (`1.00x` on `27B`), ship execution-verified `heldout_bench_v2` (`166` tasks), expand `verified_vox_codegen_v2.jsonl` to `3,338` rows (`2,340` execution-verified, 0.10 USD MIT distillation), cut `0.8B` `seq=1024` chunked `lm_head` peak by `49.8%` (`18.24` to `9.15 GB`), and validate stabilized `0.8B` `r=64` convergence (`3/64` to `8/64` in `30m` capped; `27B` queued post-`PID 34157`).
<!-- /kb:summary -->

Supersedes the earlier draft of this page. Builds on [mens-model-selection-nov-2026-research-2026-11-13](mens-model-selection-nov-2026-research-2026-11-13.md), the [Qwen3.5 Metal calibration](measurements/2026-11-13-candle-metal-qwen35-calibration.md) and the [mens-models reference](../reference/mens-models.md). Rungs: `mens/config/gpu-specs.yaml`; selection: `crates/vox-populi/src/mens/tensor/size_select.rs`.

**Evidence tags:** **[V]** verified from a primary source (HF API / `config.json` fetched 2026-11-19, or repo code/artifacts); **[K]** background knowledge, not re-fetched; **[E]** derived estimate; **[U]** unverified.

## 1. The 128 GB MacBook reality

- Host hardware: 128 GiB unified memory. Default `iogpu.wired_limit_mb` is 0 (OS policy) [V, local read-only `sysctl`]. The calibration host reported a Metal `recommendedMaxWorkingSetSize` of 115,448,725,504 bytes (about 107.5 GiB) [V, calibration JSON], while the effective working set we have been planning against is about **72 GB measured** [V per parent measurement]. Treat 72 GB as the safe budget when anything else (a Metal eval, a browser) is running.
- Raising the cap: `sudo sysctl iogpu.wired_limit_mb=<MB>` on macOS Sonoma+ (`0` restores the default) [U: secondary sources only]. It is **volatile** (resets at reboot unless a LaunchDaemon re-applies it). Published guidance of 104-112 GB on a 128 GB machine is **unverified**; values near 128 GB risk starving the OS (beachballs, swap thrash, kernel panic, lost work), because wired memory cannot be swapped.
- **Policy:** raising the wired limit is an **opt-in, documented, manual step** (needs `sudo`, the operator runs it). Vox must never apply it automatically. If used, start around 104-110 GB (`106496` to `112640`), close other apps, then re-run the free-working-set measurement to confirm Metal reports the new cap before trusting any floor.
- Spill is a cliff, not a slope: 27B at 2048 tokens peaked 118.3 GB over a 115.4 GB working set and ran 143 s versus 41 s per step [V, calibration `spill_observations`]. The predictor's conservatism protects a real cliff; loosening it must be calibrated per token count.

## 2. Memory story and predictor analysis (corrected)

### 2.1 What the 5-rung Metal runs actually measured [V]

| Run | LoRA recipe | Measured Peak & Wall |
| :--- | :--- | :--- |
| `qwen3.5-0.8b` (`run_summary.json`) | rank 16, `cache_dequantized=true`, `--lora-exclude-lm-head`, seq 1024 | 18,241,520,638 B Metal (`17,397` MiB `peak_mb`), wall 12,100.92 s [V] |
| `qwen3.5-2b` (`run_summary.json`) | rank 16, `cache_dequantized=true`, `--lora-exclude-lm-head`, seq 1024 | 22,967,428,158 B Metal (`21,904` MiB `peak_mb`), wall 9,191.43 s [V] |
| `qwen3.5-4b` (`run_summary.json`) | rank 16, `cache_dequantized=true`, `--lora-exclude-lm-head`, seq 1024 | 43,753,238,378 B Metal (`41,727` MiB `peak_mb`), wall 21,883.17 s [V] |
| `qwen3.5-9b` (`run_summary.json`) | rank 16, `cache_dequantized=true`, `--lora-exclude-lm-head`, seq 1024 | 51,987,582,826 B Metal (`49,580` MiB `peak_mb`), wall 44,249.20 s [V] |
| Earlier 27B run (`2026-11-13` PeakSampler) | rank 32, lm_head LoRA included, `cache_dequantized=false` | 77.07 GB Metal allocated (`46,467` MB / `gpu-specs` `73,503` MiB lineage, setting the **74,000 MB floor**) [V] |
| `mens/runs/qwen3.8-27b/` (`run_summary.json`, `telemetry.jsonl`) | rank 16, alpha 8, `--lora-exclude-lm-head`, `cache_dequantized=false`, 2 epochs, seq 1024, grad_accum 8 | `peak_mb` **41,274**; `peak_metal_allocated_bytes` **43,278,282,682** (`43.28` GB / `40.31` GiB); train wall 108,813.49 s (30.2 h); final loss EMA 0.3744 [V] |

**Throughput reconciliation [V]:** Three distinct token denominators existed in `qwen3.8-27b`, plus an `8x` telemetry formula bug (fixed in `crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/telem_helpers.rs` + `candle-metal` + `candle-cuda`):
- **Raw dataset tokens:** `3,347,740` (`total_tokens`) / `108,813.49` s = **30.77 tok/s** (`[V]`).
- **Packed training throughput:** `3,089,400` (`theoretical_tokens`) / `108,813.49` s = **28.39 tok/s** (`[V]`).
- **Supervised non-pad tokens:** `418,896` (`valid_tokens`, `13.56%` supervised ratio) / `108,813.49` s = **3.85 tok/s** (`[V]`).
- **Why `telemetry.jsonl` showed `~4–6 tok/s`:** `telem_helpers::build_train_step_payload` multiplied `ema_steps_per_sec` (optimizer steps/s = `437 / 108,813.49s = 0.004016`) by `batch_size * seq_len` **without multiplying by `grad_accum`** (`8` on 27B -> `~4.1 tok/s`; `4` on earlier presets -> `~6 tok/s`). Now `token_throughput_proxy` includes `grad_accum` (`~32.9 tok/s`) alongside explicit `supervised_tokens_per_sec` (`~4.46 tok/s`) (`92` unit tests + `sameness_gate` passed) [V].

### 2.2 Predictor versus reality (before vs after recalibration) [V]

Previously, `plan_for` used a single-slope linear fit on **Qwen3.5-2B** (`act_bytes_per_lht = 162.482` in `contracts/mens/memory-model.v1.yaml`), predicting `19.3 GB weights + 54.5 GB act = 73.80 GB` for 27B — **1.71× (`43.28 GB` vs `73.80 GB`) over** the lean lm-head-excluded recipe (`1.8×` in GiB terms), while under-counting unmeasured MoE cache terms in `12 x L x d^2`.

### 2.3 Multi-term recalibration landed [V]

`contracts/mens/memory-model.v1.yaml` and `crates/vox-populi/src/mens/tensor/{calibration,memory_model}.rs` now fit a non-negative + upper-envelope multi-term predictor (`docs/src/architecture/measurements/2026-11-19-candle-metal-multi-term-calibration.json`, 11 Metal `Fits` runs across `0.8B`–`27B`):
- Fitted `multi_term` Metal coefficients (`[V]`): `fixed_bytes = 4,597,696,186.12`, `act_bytes_per_lht = 57.8112`, `logits_bytes_per_vt = 0.0`, `cached_attn_bytes_per_lwt = 216.7975`, `lm_head_lora_bytes_per_vt = 136.5176`.
- Full-run predicted vs actual peaks (`[V]`, `212 passed; 0 failed; 1 ignored` in `vox-populi --features mens --lib mens::tensor`):
  - `0.8B` (`cache=true, lmhead_excl=true`): **18.71 GB** vs 18.24 GB (**1.026×**)
  - `2B` (`cache=true, lmhead_excl=true`): **22.97 GB** vs 22.97 GB (**1.000×**)
  - `4B` (`cache=true, lmhead_excl=true`): **47.87 GB** vs 43.75 GB (**1.094×**)
  - `9B` (`cache=true, lmhead_excl=true`): **60.76 GB** vs 51.99 GB (**1.169×**)
  - `27B` (`cache=false, lmhead_excl=true`): **43.28 GB** vs 43.28 GB (**1.000×**, down from 73.80 GB / 1.71×)
  - `27B` (`cache=false, lmhead_excl=false, r32`): **77.99 GB** vs 77.07 GB (**1.012×**)
- **Unmeasured `Qwen3-32B` lean projection (`[E]` from fitted `multi_term`):** ~`43.1 GB` (`40–52 GB` envelope under `cache=false, lmhead_excl=true`), clearing the `72 GB` safe working-set budget versus the stale `295,822 MB` single-slope floor. Also landed `safetensors_header_text_params_and_bytes` and MoE-aware `resolve_cache_dequantized`.

## 3. Quantization

**Loads today [V]:** only NF4 (vendored `qlora-rs 1.0.5`, block 64, double-quant) created **at load time from F32/BF16 HF weights**. No code path ingests already-quantized tensors as the frozen base; `merge.rs` bails on non-F32/BF16/F16 base dtypes. `candle` `QMatMul` is used for inference only (no autograd).

Pre-quantized loaders, ranked by effort-versus-payoff (LoC from the audit [E]):

| Rank | Format | Work | Est. LoC | Notes |
| :--- | :--- | :--- | :--- | :--- |
| 1 | bitsandbytes NF4 safetensors | Map packed U8 + absmax/quant_map straight into the qlora-rs tensor (same NF4 codebook, block 64) | about 300-400 | **Lossless, no requantization**, about 0.55 B/param; best first target |
| 2 | FP8 E4M3 (block-scaled, e.g. `-FP8` repos) | Dequant per tensor to BF16, then NF4 (or keep FP8 resident, about 1 B/param) | about 200 | Near-lossless from vendor; no FP8 matmul on Metal [K] |
| 3 | MXFP4 (gpt-oss experts) | LUT + E8M0 scale dequant | about 250 | Only matters if gpt-oss is pursued |
| 4 | MLX 4-bit affine | Dequant then NF4 at load | about 250 (native kernel 800+) | Small precision loss; unlocks many mlx-community repos |
| Skip | GGUF, AWQ, GPTQ | Different layouts, `g_idx`, no backward through `QMatMul` | 400-600+ each | Little gain over NF4; maintenance cost |

Also required for any of these: `project_qlora_shape` must skip its 9/32 shrink when the source is already 4-bit, `merge.rs` must refuse/dequantize a quantized base, and `bytes_per_dtype` must handle U8/I32 packed dtypes. Cheap training-side wins [K]: rsLoRA scaling (one line, matters at rank 64+), chunked cross-entropy (would let us train lm_head cheaply), LoftQ only below 4 bits. NF4 to bitsandbytes parity claims on 7-65B are from the QLoRA paper [K]. Optimizer state is not the bottleneck (about 1-2 GB for r16 on 27B [E]).

## 4. Candidate landscape (HF-verified, 2026-11-19)

Sizes are HF safetensors element counts; active parameters come from repo names unless noted [U]. Full table with SHAs lives in the research scratch; the decision-relevant rows:

| Model | Total / active | `model_type` | License | 4-bit nominal | Trainer today |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Qwen/Qwen3.8-27B (current) | 27.8B dense | `qwen3_5_text` | Apache-2.0 [V] | about 15 GB | Supported |
| Qwen/Qwen3-32B | 32.8B dense | `qwen3` | Apache-2.0 [V] | about 18 GB | Supported (rung floor unmeasured) |
| Qwen/Qwen3.6-35B-A3B | 36.0B / about 3B [U] | `qwen3_5_moe_text` (30 linear + 10 full layers, 256 experts top-8) | Apache-2.0 [V] | about 20 GB | **MoE missing** |
| Qwen/Qwen3-Next-80B-A3B, Qwen3-Coder-Next | 80-81B / about 3B [U] | `qwen3_next` | Apache-2.0 [V] | 40-46 GB | New arch + MoE |
| Qwen/Qwen3.5-122B-A10B | 125B / about 10B [U] | `qwen3_5_moe_text` | Apache-2.0 [V] | about 65-75 GB | Does not fit comfortably |
| Qwen/Qwen3.8-Flash-Next | 180B / 6B (README) | `qwen4_exp_text` | **Qwen Community License 1.0, not OSI** (display and MaaS conditions) [V] | about 90 GB | No; license and size |
| openai/gpt-oss-20b / 120b | 21B / 117B | `gpt_oss`, native MXFP4 | Apache-2.0 (HF tag) | 13 GB / about 61-65 GB | New arch, MoE, sinks, MXFP4 |
| zai-org/GLM-4.5-Air | 110B | `glm4_moe` | MIT (tag) | about 55 GB | New arch + MoE |
| mistralai Devstral-Small-2507, Mistral-Small-3.2 | 24B dense | `mistral` / `mistral3` | **Apache-2.0 (HF tag)** [V] | about 13 GB | Dense, needs sliding-window mask |
| google/gemma-3-27b-pt | 27B | `gemma3` | Gemma ToU (excluded by policy) | about 14 GB | Large port |

**Reconciliation with the 2026-11-13 doc:** (1) Mistral Small 3.x and Devstral are now Apache-2.0 per HF tags, replacing the earlier "contradictory/unverified" note (policy-compatible; tag only, LICENSE text not read). (2) Qwen3.8-Flash-Next is **not** Apache-2.0: it is the Qwen Community License 1.0 (not OSI). (3) The earlier "1.5 GB per billion params, about 3x nominal" rule is withdrawn: it was extrapolated from a floor, not from the 41 GB run (that run is about 2.7x nominal weights, and the ratio does not transfer to MoE or other recipes). Llama and Gemma stay excluded by licence policy. Not checked: Gemma 4 and GLM-4.7/5 existence [U].

## 5. Quality evidence

- **Verified card numbers** (Qwen3.6-35B-A3B README, Qwen's own scaffold [V]): SWE-bench Verified 73.4 (MoE) vs 75.0 (dense 27B, Qwen3.5-27B column); LiveCodeBench v6 80.4 vs 80.7; SWE-bench Multilingual 67.2 vs 69.3; SWE-bench Pro 49.5 vs 51.2; **Terminal-Bench 2.0 51.5 vs 41.6 is the only MoE win**. With about 3B active parameters, the MoE's advantage is inference speed, not quality. Our checkpoint is Qwen3.8-27B; the card compares Qwen3.5-27B, so confirm same-family scores before relying on the gap.
- Qwen3-Coder-Next's card has no score table [V]; search-summary SWE-bench numbers (about 70.6) are below the 35B-A3B's [U].
- Gemma4-31B scores 52.0 on SWE-bench Verified in the same table [V]: a downgrade for coding. No verified numbers for gpt-oss, GLM-Air, Devstral, Seed-OSS, Nemotron or Granite, so none is ranked on evidence.
- Fine-tuning evidence: MultiPL-T (arXiv 2308.09895) shows test-validated data at tens of thousands of items beats alternatives for low-resource languages [V abstract]; that is 10x or more our 1.9k rows, on small code models. "Large bases forget more on small sets" and "MoE router imbalance on small data" are [U] hypotheses. No controlled study of gain versus base size for a brand-new language was found.
- **Execution-verified v1 checkpoint rescore & saturated-eval diagnosis [V]** (`contracts/reports/mens-execution-eval-rescore-v2.json`): re-scoring all 10 stored checkpoints against real Vox execution harnesses across the 46 runnable items (out of 52 v1 items; 2 v1 reference answers themselves—`char_histogram`, `weighted_average`—did not compile) separates compile-only scores from working code:
  - `0.8B`: v1 `0->38/52` (`73.1%`), **v2 exec `0->10/46` (`21.7%`)** strict / `14/46` (`30.4%`) signature-rescued (`20` logic/runtime failures, `12` compile/stub)
  - `2B`: v1 `0->42/52` (`80.8%`), **v2 exec `0->13/46` (`28.3%`)** strict / `18/46` (`39.1%`) sig-rescued
  - `4B`: v1 `1->43/52` (`82.7%`), **v2 exec `1->22/46` (`47.8%`)** strict / `33/46` (`71.7%`) sig-rescued
  - `9B`: v1 `9->42/52` (`80.8%`), **v2 exec `6->25/46` (`54.3%`)** strict / `32/46` (`69.6%`) sig-rescued (`9B` beats `4B` under real execution!)
  - `27B`: v1 `35->49/52` (`94.2%`), **v2 exec `26->32/46` (`69.6%`)** strict / `43/46` (`93.5%`) sig-rescued (`11` v1 signature-omission misses, plus `2` real logic bugs `067_bubble_sort`/`068_insertion_sort` and `1` compile error `162_from_hex`)
- **Harder eval + verified corpus + fast `0.8B` canary shipped [V]:**
  - `mens/data/heldout_bench_v2/manifest.json`: **166 execution-verified tasks** (`31` heldout `humaneval-vox` + `15` upgraded v1 `function` tasks + `120` net-new across `algorithm`, `data_structures`, `matrix_grid`, `string_parsing`, `adt_match`, `state_machine`, `endpoint_handler`), self-verified `166/166` pass on reference & `166/166` fail on compiling wrong stubs.
  - `mens/data/verified_vox_codegen_v2.jsonl` (`contracts/reports/mens-verified-corpus-v2.json`): **1,012 unique verified rows** (`133/133` `humaneval-vox` execution-verified + `204` `repo_source_pool` + `675` `dogfood_v4` compiler-verified; `895/1,902` = `47.1%` of `dogfood_v4` filtered for compile/stub failures), passing CI `assert_no_text_leakage` against both v1 & v2 heldout sets.
  - **Live `0.8B` canary on `mens/data/heldout_bench_v2_canary` (64 tasks) [V]** (`contracts/reports/mens-canary-0.8b-before-after-v2.json`): Base **3/64 (4.7%)** -> Adapter **9/64 (14.1%, 3.0×)** (`adt_match` `2/6 -> 5/6`), leaving `85.9%` unsaturated headroom for larger rungs and re-training on `verified_vox_codegen_v2.jsonl`.
- **Phase 550 (`CAP-27B-01..03`) chunked `lm_head`, 3.3k verified corpus & `r64` canary ablations [V]** (`contracts/reports/mens-canary-0.8b-r64-chunked-v2.json`):
  - **256-row chunked `lm_head` + CE (`CAP-27B-01`):** Detached-boundary 256-row chunking across `candle-core`/`metal`/`cuda` cut `Qwen3.5-0.8B` `seq_len=1024` Metal peak by **`49.8%`** (`18,241,520,638` B / `18.24 GB` legacy `r16` -> `9,154,657,838` B / `9.15 GB` `r64` with `lm_head` LoRA included) and cleared `sameness_gate` + unit parity tests.
  - **Expanded verified corpus (`CAP-27B-02`):** `mens/data/verified_vox_codegen_v2.jsonl` / `target/v94_canary_data/train.jsonl` reached **`3,338` unique rows** (**`2,340` execution-verified**: `2,109` synthetic + `133` `humaneval-vox` + `98` MIT teacher RFT from `deepseek/deepseek-chat` at `0.1001 USD` total spend in `contracts/reports/mens-mit-teacher-distill-v94.json`, zero overlap against `heldout_bench` and `heldout_bench_v2`). Also added `max_string_bytes = Some(1_048_576)` in `vox-compiler` + `vox-ml-cli` harness eval to abort exponential string-doubling completions (`to_hex`) before OOM.
  - **`0.8B` `r64` rsLoRA canary ablations (`CAP-27B-03`, `64` tasks):** Unstabilized `r64, alpha=16, lr=1.5e-4..2.0e-4` with tied `lm_head` updates (`"tie_word_embeddings": true` on `Qwen3.5-0.8B`) collapsed (`1/64` at `seq=1024`, `0/64` at `seq=512`). Matching legacy AdamW step norm (`scale * sqrt(r) * lr` via `alpha=8` -> `scale=1.0`, `lr=9e-5`) and adding `--lora-exclude-lm-head` for tied-`<= 4B` embeddings (note `Qwen3.8-27B` has `"tie_word_embeddings": false`) restored convergence (`5.93 GB` peak, `final_loss_ema=0.3981`, `0` skips), reaching **`8.0/64` (`12.5%`, `humaneval-vox` `2/16` vs `1/16`)** under a `1800s` (`205`-step = `25.7%` of Epoch 1) time cap versus `3.0/64` (`4.7%`) base and `9.0/64` (`14.1%`) full `437`-step (`12,100.9s`) legacy `r16`.

## 6. Ranked decision table

| # | Option | Effort (LoC [E]) | Memory (est.) | License | Expected benefit | Risk |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Landed (v9.4 Phase 547–549)** | **Expand eval + verified data; recalibrate predictor; fix throughput + arch guard** | Done (`vox-hf-layout`, `telem_helpers`, `memory_model`, `metrics`, `heldout_bench_v2`, `0.8B` canary) | `18.7–60.8 GB` predicted (`1.00–1.17×`); `43.28 GB` exact on 27B | n/a | Unblocked `32B` calibration, fixed `47%` bad dogfood filtering, eliminated eval saturation | Low (verified `[V]`) |
| **b (Phase 550 `CAP-27B-01..03` landed; `04` queued)** | **High-Capacity `Qwen3.8-27B` recipe** (`r64` rsLoRA, chunked `lm_head`, `seq 2048` & verified distillation; supersedes `Qwen3-32B`) | Landed chunked `lm_head` (`CAP-27B-01`), `3,338`-row corpus (`02`), `0.8B` `r64` ablations (`03`) | `0.8B` `seq=1024` Metal peak cut `49.8%` (`18.24 -> 9.15 GB`); `27B` `seq=2048` queued post-`PID 34157` | Apache-2.0 | Fixes `47%` bad dogfood, adds `3,338` verified rows, preserves `256`-row chunked `lm_head` for untied `27B` | Low-medium (`CAP-27B-04` run pending `PID 34157` completion) |
| c (Phase 551 — stopped per user decision) | **Qwen3.6-35B-A3B** via `qwen3_5_moe` | About 1,000-1,800 LoC, 8-10 files (router, shared expert, fused experts, keymap, Mlp enum, merge, tests) | About 20 GB 4-bit weights; peak unmeasured, likely 35-55 GB [E] | Apache-2.0 | About 0 quality on card benchmarks; Terminal-Bench plus inference speed | Deferred/unneeded (`A3`: dense `27B` stays primary) |
| d (Phase 552 — stopped per user decision) | **80B-class (Qwen3-Next / Coder-Next)**, only with raised wired limit and frozen 4-bit experts | Option (c) plus `qwen3_next` layout, streaming per-expert quantize | Weights about 40-46 GB, peak about 65-95 GB [E, +-20%] | Apache-2.0 | Unproven; below the 35B-A3B on search-summary coding scores [U] | Deferred/unneeded (`A3`: high spill risk, low quality upside) |

**Deferred `CAP-27B-04` (`Qwen3.8-27B`) launch command (run once live eval `PID 34157` finishes; rebuild release binary + `mens-candle-metal` plugin first or pass isolated `VOX_PLUGINS_DIR=target/v94_canary_plugins`):**

```text
cargo build --release -p vox-ml-cli --features gpu,execution-api
cargo build --release -p vox-plugin-mens-candle-metal
VOX_PLUGINS_DIR=/Users/brbrainerd/dev/vox/target/v94_canary_plugins ./target/release/vox-ml-cli mens train --model Qwen/Qwen3.8-27B --device metal --backend qlora --tokenizer hf --data-dir target/v94_canary_data --fast-corpus --output-dir mens/runs/qwen3.8-27b-v94-r64 --rank 64 --alpha 8 --use-rslora --seq-len 2048 --batch-size 1 --grad-accum 8 --epochs 2 --lr 2.0e-5 --warmup 25 --seed 42 --min-rating 3 --gradient-checkpointing --sequence-packing --token-normalized-grad-accum --vram-limit-fraction 0.92 --no-auto-heal --no-hf-sync --force-restart --license-class apache-2.0
```

Not recommended now: gpt-oss (1,500-2,500 LoC, new arch), Gemma/Llama (licence), Flash-Next (licence, size), 122B+ (does not fit).

**Architecture guard landed [V]:** `crates/vox-hf-layout/src/lib.rs` (`validate_supported_hf_config` + strict `classify_hf_architecture`) now rejects `qwen3_5_moe`, `qwen3_next`, `gpt_oss`, `glm4_moe`, `gemma3*`/`4*`, and any `num_experts`/`num_local_experts`/`n_routed_experts > 1` or `moe_intermediate_size > 0` early before weight loading (`35` tests in `vox-hf-layout`, `7` in `vox-populi --features mens --test hf_load_config`). Milestones sequenced in `.planning/workstreams/mens-eval-memory-scaling-v94/ROADMAP.md`.

Cheapest dense additions if diversity is wanted: Mistral/Devstral about 150-300 LoC (mostly config and tests; needs a sliding-window mask). The MoE and trainer cores are duplicated in the Metal and CUDA plugins, so each new architecture is about 2x work unless extracted.

## 7. Open questions and verification gaps

- **Independent memory check:** run `mlx_lm.lora --grad-checkpoint` (QLoRA on `mlx-community/Qwen3.5-27B-4bit` or the 35B-A3B 4-bit) as a baseline for peak memory and loss curves. Not run (no GPU jobs allowed here).
- [U] Exact disk sizes of mlx-community repos (HF `safetensors.total` is an element count, not GB).
- [U] `candle` `QMatMul` backward support; Metal quantized kernel coverage.
- [U] MoE overhead model: no MoE measurement exists; the dense 0.95 GB per B-parameter slope in the quantization pass was a single data point from the heavier recipe and is superseded by section 2.
- [U] Active-parameter counts for most MoE rows (names only); gpt-oss and GLM usage-policy text; `iogpu.wired_limit_mb` safe values and persistence mechanics.
- [U] Search-summary Coder-Next benchmark figures; "large bases forget more" claim.
- ~~[U] Reconcile the older "about 6 tok/s" with about 31 tok/s derived from the run artifacts.~~ **Resolved [V]:** `30.77` raw / `28.39` packed / `3.85` supervised tok/s across `108,813.49` s; `telem_helpers.rs` omitted the `grad_accum` multiplier (`8` -> `~4.1` vs `~32.9` tok/s).
- [K] All paper-quality claims for LoftQ, QA-LoRA, DoRA, rsLoRA (arXiv 2310.08659, 2309.14717, 2402.09353, 2312.03732).
- Whether the whole measured peak includes allocator slack versus live residency (`peak_mb` 41,274 versus 43.28 GB Metal allocated suggests a few percent).

## 8. Honest framing

A QLoRA-tuned 27B-36B model is a domain specialist, not a peer of commercial frontier cloud models; on hard general reasoning, cloud models remain the right tool. Moving from 27B to a 35B-A3B MoE or 32B dense buys, on the verified cards, about nothing in code quality. Genuinely larger models (80B-class and up) are possible on a 128 GB Mac only with a raised, user-applied wired limit and substantial trainer work, and nothing here shows they would beat the current model on Vox tasks. Vox's edge is its domain data and eval loop, so improving those is the best use of the next effort.