# Roadmap: Workstream `mens-eval-memory-scaling-v94` (`v9.4`)

**Workstream:** `.planning/workstreams/mens-eval-memory-scaling-v94/`
**Primary Research SSOT:** [`docs/src/architecture/larger-local-training-candidates-research-2026-11.md`](../../../docs/src/architecture/larger-local-training-candidates-research-2026-11.md)
**Requirements:** [`REQUIREMENTS.md`](REQUIREMENTS.md) (`22` active requirements across `Phases 547–550`; Phases `551–552` removed)
**Numbering:** Continues sequentially after `v9.3` (`Phases 541–546`).

## Phase Checklist

- [x] **Phase 547: Early MoE/Arch Rejection & Throughput Telemetry Reconciliation** (`ARCH-GUARD-01..03`, `TELEM-TOK-01..03`) — *Completed 2026-11-19 (`vox-hf-layout` 35/35 + `hf_load_config` 7/7 + `candle-core` 92/92 + `sameness_gate` 1/1)*
- [x] **Phase 548: Harder Execution-Verified Eval (`heldout_bench_v2`), Verified Corpus Expansion & `0.8B` Canary Proof** (`EVAL-HARD-01..04`, `DATA-VER-01..04`) — *Completed 2026-11-19 (`166` self-verified tasks, `1,012` verified corpus rows, 0 leakage, 5-rung rescore, live `0.8B` canary `3/64 -> 9/64`)*
- [x] **Phase 549: Safetensors Header Sizing & Multi-Term Recipe-Aware Memory Predictor Recalibration** (`MEM-FIT-01..04`) — *Completed 2026-11-19 (`multi_term` fitted on 11 Metal runs; `mens::tensor` 212 passed, 1 ignored)*
- [ ] **Phase 550: High-Capacity `Qwen3.8-27B` Recipe (`r64` rsLoRA, Chunked `lm_head`, `seq 2048` & Verified Distillation)** (`CAP-27B-01..04`) — *Post-`PID 34157`; supersedes `Qwen3-32B`*

---

### Phase 547: Early MoE/Arch Rejection & Throughput Telemetry Reconciliation

**Goal:** Stop unsupported MoE (`qwen3_5_moe`, `qwen3_next`, `glm4_moe`) and next-gen/non-Qwen checkpoints (`qwen4_exp*`, `gpt_oss`, `gemma3`, `gemma4`) cleanly at `config.json` validation instead of failing late at shard load, and fix `grad_accum` omission in `telem_helpers.rs` reconciling `30.77 raw tok/s` / `28.39 packed tok/s` vs `3.85 supervised tok/s`.
**Success Criteria:**

1. `HfTransformerLayout::from_config_json_str` and `detect_hf_architecture` reject `qwen3_5_moe_text`, `qwen3_next`, `gpt_oss`, `glm4_moe`, `qwen4_exp_text`, and `gemma3/4` with explicit diagnostics naming `model_type`.
2. `telem_helpers::build_train_step_payload` multiplies `ema_steps_per_sec * grad_accum * batch_size * seq_len` and emits `supervised_tokens_per_sec`.
3. `cargo test -p vox-hf-layout`, `cargo test -p vox-populi --test hf_load_config`, and `cargo test -p vox-plugin-mens-candle-core` pass.

### Phase 548: Harder Execution-Verified Eval (`heldout_bench_v2`), Verified Corpus Expansion & `0.8B` Canary Proof

**Goal:** Break through the saturated `94.2%` compile-only ceiling on `52` tasks by adding interpreter test-harness execution scoring (`test_harness`), expanding to `>= 160` uncontaminated execution-verified tasks (`heldout_bench_v2`), expanding the compiler+test-verified training corpus (`verified_vox_codegen_v2.jsonl`), re-scoring all 5 stored ladder runs in seconds on CPU, and measuring a fast live `Qwen3.5-0.8B` canary Before/After run.
**Success Criteria:**

1. `verify_completion_with_harness` rejects wrong-algorithm completions that merely compile (`pass_compile = true, pass_exec = Some(false), pass = false`).
2. `mens/data/heldout_bench_v2/manifest.json` (`>= 160` tasks) and `mens/data/heldout_bench_v2_canary/manifest.json` (`64` tasks) pass 100% reference self-verification and fail wrong-return stubs.
3. `mens/data/verified_vox_codegen_v2.jsonl` compiles 100% cleanly with `0` overlap against `heldout_bench` and `heldout_bench_v2`.
4. `contracts/reports/mens-execution-eval-rescore-v2.json` and `contracts/reports/mens-canary-0.8b-before-after-v2.json` are generated without disturbing `PID 34157`.

### Phase 549: Safetensors Header Sizing & Multi-Term Recipe-Aware Memory Predictor Recalibration

**Goal:** Replace the single-slope `2B`-only predictor (`1.79x` conservative on `27B` `lm_head`-excluded, `45–48%` under on `2B`/`4B` cached BF16) with a safetensors-header-backed, recipe-aware multi-term Metal model fitted across all 11 measured Apple M5 Max runs.
**Success Criteria:**

1. `project_qlora_shape` reads safetensors header parameter counts (with MoE config fallback) so `Qwen3.6-35B-A3B` routes to `cache_dequantized = false`.
2. `predict_bytes_with_recipe` predicts `27B r16 --lora-exclude-lm-head` within `[0%, +10%]` of measured `43.28 GB`, covers `r32 lm_head` `77.07 GB`, covers all `0.8B..9B` BF16-cached steady-state peaks, and estimates `Qwen3-32B` dense at `~42–48 GB`.

### Phase 550: High-Capacity `Qwen3.8-27B` Recipe (`r64` rsLoRA, Chunked `lm_head`, `seq 2048` & Verified Distillation)

**Depends on:** Phase 548, Phase 549
**Requirements:** `CAP-27B-01..04`
**Goal:** Outperform the `r16` `dogfood_v4` `Qwen/Qwen3.8-27B` checkpoint locally on 128 GB Apple Silicon by moving `lm_head` projection inside the 256-row cross-entropy chunk loop (eliminating the `+34 GB` `[seq, 248320]` logits spike), scaling `Qwen3.8-27B` to `rank=64` with `use_rslora=true` and `seq_len=2048` inside the calibrated Metal working-set budget, expanding `verified_vox_codegen_v2.jsonl` with compiler+interpreter-verified MIT frontier teacher solutions (`DeepSeek-V4.1-Flash-1017` / `GLM-5.3-Flash` via `vox_actor_runtime::llm`), and proving Before/After execution gain on `heldout_bench_v2` (`166` tasks).
**Success Criteria:**

1. `crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/forward.rs` (+ Metal/CUDA callers) projects `hidden[start..end]` through `lm_head` inside the 256-row CE chunk loop so `lm_head` LoRA trains on `248,320`-vocab models without materializing full `[seq_len, 248320]` logits, verified by forward+backward gradient parity unit tests and `sameness_gate`.
2. `mens/data/verified_vox_codegen_v2.jsonl` expands (`>= 3,000` compiler+interpreter-verified rows, `0` held-out overlap against `heldout_bench` and `heldout_bench_v2`) via MIT-licensed teacher synthesis routed strictly through `vox_actor_runtime::llm` and gated 100% by `verify_completion_with_harness`.
3. Fast canary validation on `Qwen/Qwen3.5-0.8B` (`heldout_bench_v2_canary`, `64` tasks) confirms `r64` rsLoRA + chunked `lm_head` + expanded corpus beats the `14.1%` (`9/64`) canary baseline before launching the `27B` run.
4. Post-`PID 34157`, `Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0` trains with `rank=64`, `use_rslora=true`, `seq_len=2048`, and `lm_head` LoRA enabled within the calibrated Metal working-set budget and measurably improves strict execution pass rate on `mens/data/heldout_bench_v2/manifest.json` (`166` tasks) with a paired Before/After report in `contracts/reports/`.
