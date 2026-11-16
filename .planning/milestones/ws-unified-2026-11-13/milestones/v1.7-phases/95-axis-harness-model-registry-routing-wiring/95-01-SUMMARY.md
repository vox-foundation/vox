---
phase: 95-axis-harness-model-registry-routing-wiring
plan: 01
subsystem: vox-orchestrator
tags: [mens, model-registry, axis-harness, routing, bench-evidence]
provides:
  - "`ModelRegistry::register_calibrated_mens_adapter` and `ModelRegistry::route_spoke_task`"
  - "`Confirmed` confidence calibration via `BenchEvidence` from 5-spoke scorecard"
  - "Checkpoint manifest discovery in `vox-orchestrator::catalog::refresh_local_mens_Registry`"
affects: [vox-orchestrator]
key-files:
  modified:
    - crates/vox-orchestrator/src/models/registry.rs
    - crates/vox-orchestrator/src/catalog.rs
requirements-completed: [MENS-HARNESS-01, MENS-HARNESS-02, MENS-HARNESS-03, MENS-HARNESS-04]
completed: 2026-10-19
---

# Phase 95 Plan 01: Axis Harness & ModelRegistry Routing Wiring Summary

- Implemented `ModelRegistry::register_calibrated_mens_adapter` and `ModelRegistry::route_spoke_task` in `crates/vox-orchestrator/src/models/registry.rs`, attaching `BenchEvidence` (`Confidence::Confirmed`) across all 5 spokes when `overall_pass_at_1 >= 0.80`.
- Wired `refresh_local_mens_Registry` in `crates/vox-orchestrator/src/catalog.rs` to inspect `checkpoint-manifest.v1.json`, verify its BLAKE3 integrity, and register `mens-qwen35-4b-5spoke-r1` automatically.
- Verified with `cargo test -p vox-orchestrator --lib -- calibrated_mens_adapter_routes_all_five_spokes_with_confirmed_confidence` (1 passed, 0 failed).
