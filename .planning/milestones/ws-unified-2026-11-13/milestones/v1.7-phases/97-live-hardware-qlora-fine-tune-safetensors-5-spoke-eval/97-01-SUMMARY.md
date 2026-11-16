---
phase: 97-live-hardware-qlora-fine-tune-safetensors-5-spoke-eval
plan: 01
subsystem: vox-corpus, vox-ml-cli
tags: [mens, qlora, safetensors, checkpoint-manifest, verification]
provides:
  - "`MensCheckpointManifestV1::emit_verified_safetensors_checkpoint`"
  - "`MensCheckpointManifestV1::verify_adapter_file_blake3`"
affects: [vox-corpus, vox-ml-cli]
key-files:
  modified:
    - crates/vox-corpus/src/training/adapter_registry.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
requirements-completed: [LIVE-QLORA-01, LIVE-QLORA-02]
completed: 2026-10-19
---

# Phase 97 Plan 01: Live Safetensors Checkpoint Emission & Integrity Verification Summary

- Implemented `MensCheckpointManifestV1::emit_verified_safetensors_checkpoint` and `MensCheckpointManifestV1::verify_adapter_file_blake3` in `crates/vox-corpus/src/training/adapter_registry.rs` to write and verify `adapter.safetensors`, `checkpoint-manifest.v1.json`, and `mens_checkpoint_manifest.json` with SFT/DPO loss trajectory and per-spoke curriculum provenance.
- Wired `vox mens eval-gate --five-spokes` in `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` to verify `adapter.safetensors` and `checkpoint-manifest.v1.json` in `run_dir`.
- Verified with `cargo test -p vox-corpus --lib -- adapter_registry`.
