---
phase: 547-early-moe-arch-rejection-throughput-telemetry-reconciliation
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - crates/vox-hf-layout/src/lib.rs
  - crates/vox-populi/tests/hf_load_config.rs
  - crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/telem_helpers.rs
  - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/mod.rs
  - crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/training_loop/mod.rs
autonomous: true
requirements:
  - ARCH-GUARD-01
  - ARCH-GUARD-02
  - ARCH-GUARD-03
  - TELEM-TOK-01
  - TELEM-TOK-02
  - TELEM-TOK-03
must_haves:
  truths:
    - "HfTransformerLayout::from_config_json_str and detect_hf_architecture fail closed on MoE and unsupported architectures before weight loading."
    - "telem_helpers::build_train_step_payload scales tokens_per_sec by grad_accum and emits supervised_tokens_per_sec."
---

# Phase 547 Plan (Backfilled Record)

Implemented early MoE/unsupported architecture validation in `crates/vox-hf-layout/src/lib.rs` and `grad_accum`-aware packed/supervised token throughput reporting across `telem_helpers.rs` and both Candle training loops.
