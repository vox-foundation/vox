---
phase: 97-live-hardware-qlora-fine-tune-safetensors-5-spoke-eval
plan: 02
subsystem: vox-cli, vox-orchestrator
tags: [mens, eval, model-registry, safetensors, five-spokes]
provides:
  - "`execute_mens_five_spoke_evaluation_in_dir` with `--run-dir` live `.safetensors` verification"
  - "End-to-end 25-problem 5-spoke evaluation and `ModelRegistry` registration"
affects: [vox-cli, vox-orchestrator]
key-files:
  modified:
    - crates/vox-cli/src/commands/model/eval.rs
    - crates/vox-orchestrator/src/models/registry.rs
requirements-completed: [LIVE-QLORA-03, LIVE-QLORA-04]
completed: 2026-10-19
---

# Phase 97 Plan 02: Live 5-Spoke Inference Evaluation & ModelRegistry Wiring Summary

- Extended `EvalArgs` (`--run-dir`) and implemented `execute_mens_five_spoke_evaluation_in_dir` in `crates/vox-cli/src/commands/model/eval.rs` to emit/verify `adapter.safetensors` and `checkpoint-manifest.v1.json`, write `mens_spoke_scorecard.json`, and calibrate `ModelRegistry::register_calibrated_mens_adapter` across all 5 spokes.
- Verified with `cargo test -p vox-cli --lib -- mens_five_spoke_evaluation_and_registry_wiring_succeeds`.
