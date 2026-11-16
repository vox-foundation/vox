---
phase: 338-spoke-aware-gating-and-hub-repos
plan: "06"
subsystem: mens-gating
tags: [spoke-derivation, adapter-card, vllm-lora, hub-sync]
requires:
  - phase: 338-02..04
    provides: "SpokeGateRow, gate_policy, and eval-gate CLI"
provides:
  - "DomainProfilesFile::spokes_for_lanes mapping training lanes to declared spokes while ignoring review lanes"
  - "AdapterCard train_lanes and spokes provenance metadata fields"
  - "gate_policy::applying_rows filtering spoke gate checks to claimed spokes"
  - "eval_gate::spokes_of_run derivation from adapter card and training manifest"
  - "serving::spoke_route::resolve_spoke_adapter for per-spoke adapter weights"
affects: [vox-populi, vox-ml-cli]
tech-stack:
  added: []
  patterns: [lane-to-spoke derivation, applying-row gate filter]
key-files:
  created:
    - crates/vox-populi/src/mens/serving/spoke_route.rs
  modified:
    - crates/vox-populi/src/mens/tensor/domain_profiles.rs
    - crates/vox-populi/src/mens/tensor/adapter_card.rs
    - crates/vox-populi/src/mens/gate_policy.rs
    - crates/vox-populi/src/mens/hub_sync.rs
    - crates/vox-populi/src/mens/serving/mod.rs
    - crates/vox-populi/src/mens/serving/vllm_lora.rs
    - crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs
    - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
    - crates/vox-ml-cli/src/commands/mens/eval_gate/mod.rs
    - crates/vox-ml-cli/src/commands/mens/eval_gate/run_gate.rs
key-decisions:
  - "Derive spokes from adapter_card.json train_lanes / spokes via DomainProfilesFile::spokes_for_lanes rather than trusting static defaults"
  - "Restrict verify_publish_gates_with and eval-gate reporting to applying_rows so uninvolved spokes do not falsely pass or block unrelated runs"
requirements-completed: [SPOKE-06, SPOKE-09]
completed: 2026-11-13
---

# Phase 338 Plan 06: Lane-Derived Spokes, Applying Gate Filter & Spoke Adapter Routing Summary

**Replaced hardcoded default-spoke assumptions with training-lane derivation and gated publish/serving on applying spoke receipts.**

## Key Changes
- Added `DomainProfilesFile::spokes_for_lanes` in `crates/vox-populi/src/mens/tensor/domain_profiles.rs` and `train_lanes` / `spokes` fields on `AdapterCard` (`adapter_card.rs`, `train_arm.rs`, `pipeline_dispatch.rs`).
- Added `gate_policy::applying_rows` and enforced it in `hub_sync::verify_publish_gates_with` / `stage_publish_bundle` and `eval_gate::{mod,run_gate}.rs` (`SpokeDerivation`, `spokes_of_run`).
- Created `crates/vox-populi/src/mens/serving/spoke_route.rs` (`resolve_spoke_adapter`) and re-exported it from `serving/mod.rs` and `serving/vllm_lora.rs`.

## Verification
- `timeout 900s cargo test -p vox-populi --features mens-hf-hub --lib`: `424 passed; 0 failed; 3 ignored`.
- `timeout 900s cargo test -p vox-ml-cli --features gpu --lib -- eval_gate eval_spoke hub train_arm`: `99 passed; 0 failed; 0 ignored`.
