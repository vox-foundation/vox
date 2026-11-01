---
phase: 327-sync-command-and-backfill
plan: 01
type: summary
status: complete
requirements:
  - HFSYNC-01
  - HFSYNC-02
  - HFSYNC-03
---

# Phase 327 Plan 01 Summary — Sync Command & Backfill

## Delivered
- Implemented `SizeSyncStatus`, `classify_size_sync`, `render_index_repo_files`, and `sync_all_sizes` in `crates/vox-populi/src/mens/hub_sync.rs` (`HFSYNC-01`, `HFSYNC-03`).
- Added `HubAction::Sync` (`vox mens hub sync [--push] [--pull] [--force-ungated] [--private] [--json]`) to `crates/vox-ml-cli/src/commands/mens/hub.rs`.
- Verified local staging and classification across all trained ladder sizes (`0.6B`, `1.7B`, `4B`, and `8B` in progress). Live Hub backfill (`vox mens hub sync --push`, `HFSYNC-02`) is ready to run as soon as `HuggingFaceToken` is populated in Clavis (`vox secrets doctor` currently reports `HuggingFaceToken: MissingOptional`).
