# Requirements: Milestone v9.4 — MENS Harder Execution Eval, Verified Corpus Expansion, Multi-Term Memory Predictor, Early MoE Guard & Canary Proof

Workstream: `mens-eval-memory-scaling-v94`
Scope: `crates/vox-hf-layout/`, `crates/vox-plugin-mens-candle-{core,metal,cuda}/`, `crates/vox-populi/src/mens/tensor/`, `crates/vox-ml-cli/src/commands/{mens,corpus}/`, `contracts/mens/`, `contracts/reports/`, `mens/data/`, `docs/src/architecture/`.

## Active

### Early Architecture & Telemetry Hygiene (Phase 547)
- [x] **ARCH-GUARD-01**: `validate_supported_hf_config` in `crates/vox-hf-layout/src/lib.rs` fails closed early at `config.json` parse (`from_value` and `detect_hf_architecture`) for MoE (`qwen3_5_moe`, `qwen3_5_moe_text`, `qwen3_moe`, `qwen2_moe`, `glm4_moe`, `deepseek_v2`, `deepseek_v3`, `mixtral`, `*_moe`, or `num_experts > 1` / `moe_intermediate_size > 0`) and unsupported next-gen/non-Qwen architectures (`qwen3_next`, `qwen4_exp*`, `gpt_oss`, `gemma3*`, `gemma4*`, `mamba`, `rwkv`).
- [x] **ARCH-GUARD-02**: `classify_hf_architecture` removes `model_l == "qwen3_next"` and excludes `"moe"`/`"next"` subtypes from matching dense `HfArchitecture::Qwen35`.
- [x] **ARCH-GUARD-03**: Unit tests in `vox-hf-layout` and `vox-populi` verify early diagnostic rejection for `Qwen3.6-35B-A3B` (`qwen3_5_moe_text`), `Qwen3-Coder-Next` (`qwen3_next`), `gpt_oss`, `glm4_moe`, `qwen4_exp_text`, and `gemma3`/`gemma4`.
- [x] **TELEM-TOK-01**: `telem_helpers::build_train_step_payload` scales `ema_steps_per_sec` (optimizer steps/sec) by `grad_accum * batch_size * seq_len` so `tokens_per_sec` reports packed forward/backward token throughput (~28–33 tok/s on 27B) rather than under-reporting by `grad_accum` (~4–6 tok/s).
- [x] **TELEM-TOK-02**: `telem_helpers::build_train_step_payload` emits `supervised_tokens_per_sec` alongside `tokens_per_sec` so raw, packed, and assistant-only token throughput rates are reconciled in `telemetry.jsonl`.
- [x] **TELEM-TOK-03**: Unit test in `telem_helpers.rs` verifies exact reconciliation against `mens/runs/qwen3.8-27b` run figures (`437` opt steps, `108813.49 s`, `grad_accum=8`, `valid_tokens=418896`, `theoretical_tokens=3089400`).

### Harder Execution Eval, Verified Corpus & Fast Canary (Phase 548)
- [x] **EVAL-HARD-01**: `eval_local` + `metrics.rs` support `test_harness` execution verification via `vox_compiler::eval::Interpreter` (`run_harness_assertions`) AND require `semantic_expected_contains` match in `pass` when `test_harness` is present.
- [x] **EVAL-HARD-02**: Materialize and 100%-self-verify `mens/data/heldout_bench_v2/manifest.json` (`166` tasks: `31` held-out `humaneval-vox` with real `tests.vox` assertions, `15` upgraded v1 function tasks, and `120` net-new algorithmic/ADT/state-machine/endpoint `.vox` tasks) plus stratified `64`-task `mens/data/heldout_bench_v2_canary/manifest.json`.
- [x] **EVAL-HARD-03**: Produce offline CPU execution re-score (`contracts/reports/mens-execution-eval-rescore-v2.json`) across all 5 stored trained checkpoints (`0.8B`, `2B`, `4B`, `9B`, `27B`).
- [x] **EVAL-HARD-04**: Run live `Qwen3.5-0.8B` Base vs Adapter Before/After canary evaluation on `heldout_bench_v2_canary` via `target/debug/vox-ml-cli` (`3/64` -> `9/64`, zero release-binary disturbance) and persist `contracts/reports/mens-canary-0.8b-before-after-v2.json`.
- [x] **DATA-VER-01**: Extend `vox-source-pool.yaml` / `source_pool.rs` leakage guard (`additional_heldout_benches`) so both `heldout_bench` and `heldout_bench_v2` are protected from training leakage.
- [x] **DATA-VER-02**: Harvest all `133` `training_eligible: true` `humaneval-vox` problems (compiler + interpreter verified) plus eligible `vox-source-pool` `.vox` files (`204`) and valid `dogfood_v4` codegen rows (`675`) into `mens/data/verified_vox_codegen_v2.jsonl` (`1,012` unique rows).
- [x] **DATA-VER-03**: Enforce 100% `vox_compiler::pipeline::run_frontend_str` compilation + `anti_stub` + zero held-out overlap across every emitted row in `verified_vox_codegen_v2.jsonl` (`assert_no_text_leakage` passed against v1 and v2).
- [x] **DATA-VER-04**: Write summary provenance report `contracts/reports/mens-verified-corpus-v2.json` comparing before vs after verified `.vox` codegen row counts.

### Multi-Term Memory Predictor Recalibration (Phase 549)
- [x] **MEM-FIT-01**: Implement safetensors JSON-header parameter/dtype reader (`safetensors_header_text_params_and_bytes`) and MoE expert parameter fallback in `memory_model.rs` so MoE models like `Qwen3.6-35B-A3B` (`36B`) never trigger `cache_dequantized=true` via `12 * L * H^2`.
- [x] **MEM-FIT-02**: Record all 11 measured Apple M5 Max `Fits` runs (`2026-11-13` PeakSampler + `Nov 17-19` `0.8B..27B` `run_summary.json`) in `docs/src/architecture/measurements/2026-11-19-candle-metal-multi-term-calibration.json`.
- [x] **MEM-FIT-03**: Add `TrainingRecipe` + `MultiTermMemoryModel` (`predict_bytes_with_recipe`) and `calibration::fit_multi_term_lane` parameterized by `cache_dequantized`, `lora_exclude_lm_head`, `L*H*T`, `V*T`, and resident weight allocator multiplier, persisted in `contracts/mens/memory-model.v1.yaml` while preserving legacy `predict_bytes` contracts.
- [x] **MEM-FIT-04**: Pin predicted-vs-measured accuracy across `0.8B..27B` (`43.28 GB` exact on `27B` `r16 --lora-exclude-lm-head`, `77.99 GB` vs `77.07 GB` on `r32` + `lm_head`, `~43.1 GB` lean projection for `Qwen3-32B` dense) and add `derive_floor_mb`.

### Post-Eval Sequenced Capacity Phase (Phase 550)
- [x] **CAP-27B-01**: Project `hidden[start..end]` through `lm_head` inside the 256-row CE chunk loop (`forward.rs` + Metal/CUDA callers) so `lm_head` LoRA trains on `248,320`-vocab models without materializing full `[seq_len, 248320]` logits, verified by forward+backward gradient parity unit tests and `sameness_gate`.
- [x] **CAP-27B-02**: Expand `mens/data/verified_vox_codegen_v2.jsonl` to `>= 3,000` compiler+interpreter-verified rows (`0` overlap against `heldout_bench` and `heldout_bench_v2`) using MIT frontier teacher distillation (`deepseek/deepseek-chat` via `vox_actor_runtime::llm`, `$0.1001` total spend) gated 100% by `verify_completion_with_harness` (`3,338` emitted rows, `2,340` execution-verified).
- [ ] **CAP-27B-03**: Validate `r64` rsLoRA + chunked `lm_head` + expanded corpus on the `Qwen/Qwen3.5-0.8B` `heldout_bench_v2_canary` (`64` tasks) before launching `27B`, beating the `14.1%` (`9/64`) canary baseline *(partial / step-capped at `1800s` / `205` opt steps = `25.7%` of Epoch 1: reached `8.0/64` [`12.5%`, `humaneval-vox` `2/16`] vs `3.0/64` [`4.7%`] base; `-49.8%` Metal memory reduction `18.24 GB -> 9.15 GB` at `seq=1024`)*.
- [ ] **CAP-27B-04**: Post-`PID 34157`, train `Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0` with `rank=64`, `use_rslora=true`, `seq_len=2048`, and `lm_head` LoRA within the calibrated Metal working-set budget and beat baseline on `heldout_bench_v2` (`166` tasks) with a Before/After report in `contracts/reports/` *(deferred awaiting live `PID 34157` completion; ready launch command recorded in `STATE.md` and `550-SUMMARY.md`)*.

### Removed / Out-of-Scope (Phases 551–552 removed via `gsd-phase --remove`)
- `MOE-35B-01..04`: *Removed with Phase `551`* — `Qwen3.6-35B-A3B` MoE trainer support (`qwen3_5_moe`) is unnecessary while dense `Qwen3.8-27B` leads coding benchmarks and `vox-hf-layout` already rejects MoE configs early.
- `NEXT-80B-01..03`: *Removed with Phase `552`* — `80B`-class `qwen3_next` streaming expert support deferred out of `v9.4`.

## Traceability
| REQ | Phase |
|-----|-------|
| ARCH-GUARD-01..03, TELEM-TOK-01..03 | 547 |
| EVAL-HARD-01..04, DATA-VER-01..04 | 548 |
| MEM-FIT-01..04 | 549 |
| CAP-27B-01..04 | 550 |
| MOE-35B-01..04 | Removed (ex-551) |
| NEXT-80B-01..03 | Removed (ex-552) |
