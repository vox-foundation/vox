---
phase: 326-auto-retrieve
plan: 01
type: summary
status: complete
requirements:
  - HFPULL-01
  - HFPULL-02
  - HFPULL-03
---

# Phase 326 Plan 01 Summary — Auto-Retrieve

## Delivered
- Implemented `resolve_ladder_adapter_target`, `pull_adapter_if_missing`, and `pull_adapter_if_missing_blocking` in `crates/vox-populi/src/mens/hub_sync.rs` (`HFPULL-01`).
- Implemented `verify_pulled_adapter_dir` to validate `training_manifest.json` `base_model` against `mens/config/gpu-specs.yaml`, verify BLAKE3 file hashes against `hf_sync_manifest.json`, verify gate receipts for serve, and delete tampered/mismatched directories immediately (`HFPULL-02`).
- Implemented `ensure_pull_allowed` honoring `VOX_MENS_NO_DOWNLOAD=1` and `HF_HUB_OFFLINE=1` with an explicit error naming the target repo (`HFPULL-03`).
- Wired `pull_adapter_if_missing_blocking` into `resolve_serve_run_dir` (`crates/vox-ml-cli/src/commands/mens/ladder_serve.rs`) and `run_eval_local` (`crates/vox-ml-cli/src/commands/mens/eval_local.rs`).
