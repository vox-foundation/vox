---
phase: 550-high-capacity-qwen3-8-27b-recipe-r64-rslora-chunked-lm-head
plan: 01
type: execute
wave: 1
depends_on: [548, 549]
files_modified:
  - crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/types.rs
  - crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/forward.rs
  - crates/vox-plugin-mens-candle-metal/src/model.rs
  - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs
  - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/mod.rs
  - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/logic.rs
  - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/validation.rs
  - crates/vox-plugin-mens-candle-cuda/src/model.rs
  - crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/mod.rs
  - crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/training_loop/mod.rs
  - crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/training_loop/logic.rs
  - crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/training_loop/validation.rs
  - crates/vox-populi/src/mens/tensor/calibration.rs
  - crates/vox-populi/src/mens/tensor/memory_model.rs
  - crates/vox-ml-cli/src/commands/mens/v94_synth_exec.rs
  - crates/vox-ml-cli/src/commands/mens/v94_teacher_distill.rs
  - crates/vox-ml-cli/src/commands/mens/v94_eval_corpus.rs
  - mens/data/verified_vox_codegen_v2.jsonl
  - contracts/reports/mens-verified-corpus-v2.json
  - contracts/reports/mens-mit-teacher-distill-v94.json
  - contracts/reports/mens-canary-0.8b-r64-chunked-v2.json
  - docs/src/architecture/larger-local-training-candidates-research-2026-11.md
autonomous: true
requirements:
  - CAP-27B-01
  - CAP-27B-02
  - CAP-27B-03
must_haves:
  truths:
    - "Checkpointed forward+backward projects hidden_2d[start..end] through lm_head per 256-row chunk with exact F32 gradient parity against eager backward."
    - "verified_vox_codegen_v2.jsonl contains >= 3,000 compiler-clean rows (0 overlap with heldout_bench and heldout_bench_v2), with OpenRouter distillation spend <= $3.00."
    - "Qwen3.5-0.8B r64 rsLoRA (alpha 16) + chunked lm_head canary completes in isolated target/v94_canary_plugins without disturbing PID 34157 and reports Before/After metrics on heldout_bench_v2_canary."
---

# Phase 550 Plan (`CAP-27B-01..03` + `CAP-27B-04` Pre-Gate)

1. Implement detached-boundary 256-row chunked `lm_head` + CE across `candle-core`, `candle-metal`, and `candle-cuda`, plus `TrainingRecipe::chunked_lm_head_rows` in `vox-populi` and exact forward/backward gradient parity tests (`CAP-27B-01`).
2. Add deterministic execution-verified Vox corpus synthesis (`v94_synth_exec.rs`), local clean pool harvesting, and `$3.00`-capped MIT teacher distillation (`v94_teacher_distill.rs` via `vox_actor_runtime::llm`) to expand `verified_vox_codegen_v2.jsonl` to `>= 3,000` unique verified rows (`CAP-27B-02`).
3. Build `mens-candle-metal` into `target/v94_canary_rel` + `VOX_PLUGINS_DIR=target/v94_canary_plugins`, run `Qwen3.5-0.8B` `r64` `alpha=16` `--use-rslora` with `lm_head` LoRA enabled, and evaluate against `heldout_bench_v2_canary` (`CAP-27B-03`).
4. Gate `CAP-27B-04` (`Qwen3.8-27B` `r64` rsLoRA `seq 2048` + chunked `lm_head`) on `PID 34157` completion; if still running, record deferred execution state cleanly.
