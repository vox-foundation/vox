---
phase: 343-ladder-wide-re-gating-and-multi-spoke-promotion
plan: 02
subsystem: mens-hub-promotion
tags: [mens, hub, promotion, multi-spoke]
provides:
  - Phase 343 Plan 02 status record (local multi-spoke staging & CLI flags verified; remote HF upload deferred)
affects: [vox-populi, vox-ml-cli]
key-files:
  modified: []
completed: 2026-11-13
---

# Phase 343 Plan 02 Summary: Full-Ladder Multi-Spoke Promotion

- Multi-spoke Hub staging ([stage_publish_bundle_spoke](file:///Users/brbrainerd/dev/vox/crates/vox-populi/src/mens/hub_sync.rs)), CLI `--spoke` parsing ([hub.rs](file:///Users/brbrainerd/dev/vox/crates/vox-ml-cli/src/commands/mens/hub.rs)), and [mens-models.md](file:///Users/brbrainerd/dev/vox/docs/src/reference/mens-models.md) card-generation foundations pass automated checks.
- **Deferred operator gate**: Remote HF Hub `--push` promotion across the trained ladder requires `HF_TOKEN` and operator execution after live collateral receipts pass.
