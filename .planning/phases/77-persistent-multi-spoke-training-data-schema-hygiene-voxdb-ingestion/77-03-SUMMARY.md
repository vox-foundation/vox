---
phase: 77-persistent-multi-spoke-training-data-schema-hygiene-voxdb-ingestion
plan: 03
subsystem: corpus
tags: [mens, training, voxdb, cas, dataset-snapshot, arca-replay]
requires: [77-01, 77-02]
provides:
  - SpokeMixtureSummary, TrustyDatasetSnapshot, build_trusty_snapshot, persist_snapshot_to_db, load_snapshot_from_db
  - ReplayRow::to_training_record_v2, replay_snapshot_for_spoke
affects: [crates/vox-corpus]
key-files:
  modified:
    - crates/vox-corpus/src/dataset_snapshot.rs
    - crates/vox-corpus/src/arca_replay.rs
key-decisions:
  - "Stored Trusty-URI dataset snapshots in VoxDb's existing content-addressed store (objects + names tables via VoxDb::store and VoxDb::bind_name) without changing VoxDb baseline SQL DDL."
requirements-completed: [CORPUS-06]
completed: 2026-10-16
---

# Phase 77 Plan 03: Content-Addressed VoxDb Dataset Snapshots & Per-Spoke Replay Summary

## Accomplishments
- Implemented `SpokeMixtureSummary`, `TrustyDatasetSnapshot`, `build_trusty_snapshot`, `persist_snapshot_to_db`, and `load_snapshot_from_db` in `crates/vox-corpus/src/dataset_snapshot.rs` (`CORPUS-06`).
- Implemented `ReplayRow::to_training_record_v2` and `replay_snapshot_for_spoke` in `crates/vox-corpus/src/arca_replay.rs` (`CORPUS-06`).
