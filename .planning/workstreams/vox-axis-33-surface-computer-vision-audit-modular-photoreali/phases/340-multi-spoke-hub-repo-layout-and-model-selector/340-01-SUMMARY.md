---
phase: 340-multi-spoke-hub-repo-layout-and-model-selector
plan: 01
subsystem: mens-hub-layout
tags: [mens, hub, spoke, staging, cli]
provides:
  - Spoke gate contract schema `contracts/mens/spoke_gates.v1.schema.json`
  - `KNOWN_SPOKES` and `spoke_rel_dir` helper in `crates/vox-populi/src/mens/hub_repo.rs`
  - `stage_publish_bundle_spoke` staging under `spokes/<spoke>/` with `hf_sync_manifest.json` updates in `crates/vox-populi/src/mens/hub_sync.rs`
  - `--spoke` CLI flag validation and clap tests on `HubAction::Pull` and `HubAction::Sync` in `crates/vox-ml-cli/src/commands/mens/hub.rs`
affects: [vox-populi, vox-ml-cli, contracts/mens]
key-files:
  created:
    - contracts/mens/spoke_gates.v1.schema.json
  modified:
    - crates/vox-populi/src/mens/hub_repo.rs
    - crates/vox-populi/src/mens/hub_sync.rs
    - crates/vox-ml-cli/src/commands/mens/hub.rs
completed: 2026-11-13
---

# Phase 340 Plan 01 Summary: Multi-Spoke Hub Repository Layout

- Created [spoke_gates.v1.schema.json](file:///Users/brbrainerd/dev/vox/contracts/mens/spoke_gates.v1.schema.json) defining `vox_mens_spoke_gates_v1`.
- Added `KNOWN_SPOKES` and `spoke_rel_dir` + unit test `spoke_rel_dir_maps_known_spokes_and_rejects_unknown` in [hub_repo.rs](file:///Users/brbrainerd/dev/vox/crates/vox-populi/src/mens/hub_repo.rs).
- Added [stage_publish_bundle_spoke](file:///Users/brbrainerd/dev/vox/crates/vox-populi/src/mens/hub_sync.rs) + unit test `stage_publish_bundle_spoke_places_files_under_spokes_subdir` duplicating spoke artifacts under `spokes/<spoke>/`, recording BLAKE3 file hashes in `manifest.files`, and updating `hf_sync_manifest.json` while keeping root compatibility aliases intact.
- Added `--spoke` to `HubAction::Pull` and `HubAction::Sync` with `spoke_rel_dir` validation and clap parse tests in [hub.rs](file:///Users/brbrainerd/dev/vox/crates/vox-ml-cli/src/commands/mens/hub.rs).
