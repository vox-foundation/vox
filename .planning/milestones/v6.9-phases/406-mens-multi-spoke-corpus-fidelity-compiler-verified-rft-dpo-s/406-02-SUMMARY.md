---
phase: 406-mens-multi-spoke-corpus-fidelity-compiler-verified-rft-dpo-s
plan: "02"
subsystem: mens-trainer
tags: [qlora, sequence-packing, rslora, lm-head, token-normalized-grad-accum, relative-paths]
requires:
  - phase: 405
    provides: structural dependency DAG decoupling and clean workspace baseline
provides:
  - "Workspace-relative `train_path` and `training_contract.yaml` resolution in `check_corpus_threshold` (`run_train.rs`)"
  - "Token-normalized gradient accumulation (`token_norm_window_scale`) and cross-document boundary masking (`build_packed_mask_and_doc_ids`) in `training_loop/forward.rs`"
  - "Document-masked causal attention + local per-document RoPE positions (`forward_packed` in `model.rs`) verified bitwise against separate forwards on `Device::Cpu`"
  - "`rsLoRA` (`alpha / sqrt(rank)`) and explicit `lora_exclude_lm_head` config/CLI/runtime wiring"
affects: [vox-ml-cli, vox-populi, vox-plugin-mens-candle-core, vox-plugin-mens-candle-metal]
coverage:
  - id: D-406-04
    description: "Token-normalized `grad_accum` loss (`token_norm_window_scale`), weighted cross-document boundary zeroing (`build_packed_mask_and_doc_ids`), epoch-end `flush_pending_grads`, shared `PackedAttnContext` with contiguous `doc_id` validation, and `forward_checkpointed_packed` + packed bin ETA/LR/reward calibration."
    requirement: MENS-CORPUS-406-04
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-mens-candle-metal --lib token_norm_window_scale (1/1); packed_mask (2/2); flush_pending_grads (1/1); packed_forward (2/2); checkpointed_packed (1/1)"
        status: pass
    human_judgment: false
  - id: D-406-05
    description: "`--lora-exclude-lm-head` (`from_weight_frozen` keeping `gpu_nf4: None` so input grads propagate for all `seq_len`) and `--use-rslora` (`PopuliAdapterManifestV3::effective_alpha_f64` preserving non-square rank precision) across config, CLI, merge, and `candle_qlora_train`."
    requirement: MENS-CORPUS-406-05
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-mens-candle-core --lib effective_alpha (1/1); cargo test -p vox-plugin-mens-candle-metal --lib effective_lora_alpha (1/1); cargo test -p vox-plugin-mens-candle-metal --lib from_weight_frozen (1/1)"
        status: pass
    human_judgment: false
  - id: D-406-06
    description: "Relative corpus/manifest path resolution with streaming `BufReader` line threshold check in `check_corpus_threshold` (`run_train.rs`)."
    requirement: MENS-CORPUS-406-06
    verification:
      - kind: unit
        ref: "cargo test -p vox-ml-cli --features gpu --lib corpus_threshold (5 passed; 0 failed incl. direct_relative_jsonl_file_path_passes, relative_training_contract_override_passes)"
        status: pass
    human_judgment: false
key-files:
  modified:
    - patches/qlora-rs-1.0.5/src/qlora.rs
    - patches/qlora-rs-1.0.5/src/training.rs
    - crates/vox-ml-cli/src/commands/schola/train/run_train.rs
    - crates/vox-ml-cli/src/commands/schola/train/gpu.rs
    - crates/vox-ml-cli/src/commands/schola/merge_qlora.rs
    - crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
    - crates/vox-populi/src/mens/tensor/training_config.rs
    - crates/vox-plugin-mens-candle-core/src/config.rs
    - crates/vox-plugin-mens-candle-core/src/adapter_schema_v3.rs
    - crates/vox-plugin-mens-candle-core/src/merge.rs
    - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs
    - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/finalize.rs
    - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/forward.rs
    - crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/mod.rs
    - crates/vox-plugin-mens-candle-metal/src/model.rs
requirements-completed:
  - MENS-CORPUS-406-04
  - MENS-CORPUS-406-05
  - MENS-CORPUS-406-06
completed: 2026-11-12
---

# Phase 406 Plan 02: Token-Normalized Grad Accum, Document-Masked Packed QLoRA, rsLoRA / lm_head Guard & Portable Relative Preflight Summary

## What Was Built & Verified
- Relative `train_path` and `training_contract.yaml` resolution with streaming `BufReader` early stop in `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`.
- `lora_exclude_lm_head` (`QuantizedLinear::from_weight_frozen` in `patches/qlora-rs-1.0.5/src/qlora.rs`, `model.rs`, `candle_qlora_train/mod.rs`) and `use_rslora` (`PopuliAdapterManifestV3::effective_alpha_f64` in `adapter_schema_v3.rs`, `merge.rs`, `merge_qlora.rs`, `finalize.rs`).
- Token-normalized gradient accumulation (`token_norm_window_scale`), epoch-end `flush_pending_grads` (`patches/qlora-rs-1.0.5/src/training.rs` + `training_loop/mod.rs`), heterogeneous weighted cross-document masking (`build_packed_mask_and_doc_ids` in `training_loop/forward.rs`), and checkpointed/eager `PackedAttnContext` (`forward_packed` / `forward_checkpointed_packed` in `model.rs`) verified on `Device::Cpu`.

## Verification Results
- `cargo test -p vox-ml-cli --features gpu --lib corpus_threshold`: `5 passed; 0 failed` (`0.01s`).
- `cargo test -p vox-plugin-mens-candle-core --lib effective_alpha`: `1 passed; 0 failed` (`0.00s`).
- `cargo test -p vox-plugin-mens-candle-metal --lib effective_lora_alpha`: `1 passed; 0 failed` (`0.00s`).
- `cargo test -p vox-plugin-mens-candle-metal --lib token_norm_window_scale`: `1 passed; 0 failed` (`0.00s`).
- `cargo test -p vox-plugin-mens-candle-metal --lib packed_mask`: `2 passed; 0 failed` (`0.00s`).
- `cargo test -p vox-plugin-mens-candle-metal --lib flush_pending_grads`: `1 passed; 0 failed` (`0.00s`).
- `cargo test -p vox-plugin-mens-candle-metal --lib packed_forward`: `2 passed; 0 failed` (`0.00s`).
- `cargo test -p vox-plugin-mens-candle-metal --lib checkpointed_packed`: `1 passed; 0 failed` (`0.00s`).
- `cargo test -p vox-plugin-mens-candle-metal --lib from_weight_frozen`: `1 passed; 0 failed` (`0.00s`).
