---
phase: 325-gated-auto-publish
plan: 01
type: summary
status: complete
requirements:
  - HFPUB-01
  - HFPUB-02
  - HFPUB-03
  - HFPUB-04
  - HFPUB-05
---

# Phase 325 Plan 01 Summary — Gated Auto-Publish

## Delivered
- Added `VOX_MENS_HF_SYNC` to `crates/vox-config/src/config_registry.rs` and `--no-hf-sync` flags to `vox mens train` and `vox mens eval-gate` (`HFPUB-01`).
- Implemented `verify_publish_gates` in `crates/vox-populi/src/mens/hub_sync.rs` requiring passing `collateral_damage_report.json` and `gate_receipt.json` unless `--force-ungated` is set (`HFPUB-02`).
- Implemented `stage_publish_bundle` and `render_model_card` producing a complete adapter bundle (`candle_qlora_adapter.safetensors`, `training_manifest.json`, gate receipts, per-slice eval JSONs, `README.md` with YAML frontmatter, and `hf_sync_manifest.json` with BLAKE3 hashes via `vox_crypto::hash_fast_hex`) with host path sanitization (`HFPUB-03`).
- Implemented `manifests_match` and `publish_adapter_run_dir` to skip Hub commits when remote `hf_sync_manifest.json` matches local staged digests (`HFPUB-04`).
- Enforced Clavis `SecretId::HuggingFaceToken` preflight via `require_clavis_hf_token` before reading any bundle files (`HFPUB-05`).
