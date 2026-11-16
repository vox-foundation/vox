---
phase: 333
plan: 01
subsystem: mens
tags: [mens, qlora, hub-cards, release-gate, checkpoint]
requires: []
provides:
  - "qwen3_0_6b shared QLoRA preset (rank 16, alpha 32, grad_accum 8)"
  - "Cumulative total_tokens and micro_steps_executed persisted across CheckpointState resumes"
  - "Gated complete merged checkpoint detection and bundle staging in hub_sync"
  - "Plain-language Hugging Face card & index templates without 'ungated' or '--allow-ungated'"
affects:
  - crates/vox-populi
  - crates/vox-plugin-mens-candle-core
  - crates/vox-plugin-mens-candle-metal
  - crates/vox-plugin-mens-candle-cuda
  - contracts/mens/training-presets.v1.yaml
requirements-completed: [REL-01, REL-02, REL-03, REL-04]
completed: 2026-11-13
---

# Phase 333 Summary

Implemented `REL-01..04`:

1. **Shared Recipe for `0.6B` & Cumulative Resume Telemetry (`REL-01`):**
   - Registered `qwen3_0_6b` in `contracts/mens/training-presets.v1.yaml`, `spoke_base_resolver.rs`, `preset_schema.rs`, and `size_select.rs` using the shared rank 16 / alpha 32 / grad_accum 8 profile (keeping `qwen3_dev_cpu` strictly as the smoke preset).
   - Added `total_tokens: usize` and `micro_steps_executed: u32` (`#[serde(default)]`) to `CheckpointState` and `QloraTrainingResume` across `vox-populi`, `vox-plugin-mens-candle-core`, `vox-plugin-mens-candle-metal`, and `vox-plugin-mens-candle-cuda`, preserving full-run counters across pause, OOM, mid-epoch, and epoch-boundary checkpoints.

2. **Gated Complete-Model Staging & Card Convergence (`REL-02..04`):**
   - Added `detect_merged_checkpoint_files` (`recombined_full/` or merged `.safetensors` + `config.json`/`tokenizer.json`) in `crates/vox-populi/src/mens/hub_card/facts.rs` and exposed `is_complete_model()` / `merged_files_text()` on `CardFacts` plus `ready_merged_rows()` on `IndexFacts`.
   - Updated `stage_publish_bundle` in `crates/vox-populi/src/mens/hub_sync.rs` so passing runs with merged checkpoints stage the complete merged checkpoint rather than `candle_qlora_adapter.safetensors`.
   - Updated `templates/hub/{partials/frontmatter.md,partials/use_it.md,model.md,index.md}` and refreshed golden cards so non-passed sizes describe the trained add-on and unfinished review (`vox mens hub download --repo ...` + Files tab) with zero occurrences of `"ungated"` or `--allow-ungated`, while ready merged sizes switch `base_model_relation: finetune` and document the complete merged checkpoint.
