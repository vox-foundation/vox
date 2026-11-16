---
phase: 338-spoke-aware-gating-and-hub-repos
plan: "09"
subsystem: mens-hub-datasets
tags: [hf-hub, dataset-repos, card-facts, leakage-guard]
requires:
  - phase: 338-06
    provides: "Spoke metadata on AdapterCard and CardFacts"
provides:
  - "hub_repo::{eval_dataset_repo_id, data_dataset_repo_id}"
  - "hub_dataset::{DatasetKind, DatasetSource, DatasetBundleSpec, StagedDatasetBundle, stage_dataset_bundle, publish_dataset_bundle}"
  - "CLI HubAction::Dataset command in vox mens hub dataset"
  - "CardFacts::{train_lanes, spokes, datasets} and cite_datasets helper"
affects: [vox-populi, vox-ml-cli]
tech-stack:
  added: []
  patterns: [secret and held-out leakage blocking prior to dataset staging]
key-files:
  created:
    - crates/vox-populi/src/mens/hub_dataset.rs
  modified:
    - crates/vox-populi/src/mens/mod.rs
    - crates/vox-populi/src/mens/hub_repo.rs
    - crates/vox-populi/src/mens/hub_card/card_facts.rs
    - crates/vox-ml-cli/src/commands/mens/hub.rs
key-decisions:
  - "Preserve all existing golden model-card snapshots verbatim while adding CardFacts dataset citation metadata"
  - "Block staging of dataset bundles containing secret patterns or held-out training-eligible leaks"
requirements-completed: [SPOKE-09, SPOKE-10]
completed: 2026-11-13
---

# Phase 338 Plan 09: Hub Dataset Repos & Model Card Citations Summary

**Implemented Hub dataset repo naming, leak-checked dataset staging, `vox mens hub dataset`, and `CardFacts::cite_datasets`.**

## Key Changes
- Added `eval_dataset_repo_id` and `data_dataset_repo_id` in `crates/vox-populi/src/mens/hub_repo.rs`.
- Created `crates/vox-populi/src/mens/hub_dataset.rs` with `DatasetKind`, `DatasetSource`, `DatasetBundleSpec`, `StagedDatasetBundle`, `stage_dataset_bundle`, and `publish_dataset_bundle`, guarded by secret/held-out leakage unit tests, and exposed `HubAction::Dataset` in `crates/vox-ml-cli/src/commands/mens/hub.rs`.
- Added `train_lanes`, `spokes`, `datasets`, `cite_datasets`, and `cite_datasets_prefers_spoke_and_eval_suite` in `crates/vox-populi/src/mens/hub_card/card_facts.rs`.

## Verification
- `timeout 900s cargo test -p vox-populi --features mens-hf-hub --lib`: `424 passed; 0 failed; 3 ignored`.
- `timeout 900s cargo test -p vox-ml-cli --features gpu --lib -- eval_gate eval_spoke hub train_arm`: `99 passed; 0 failed; 0 ignored`.
