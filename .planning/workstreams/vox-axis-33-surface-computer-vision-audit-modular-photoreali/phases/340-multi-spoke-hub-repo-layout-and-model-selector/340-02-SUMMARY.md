---
phase: 340-multi-spoke-hub-repo-layout-and-model-selector
plan: 02
subsystem: mens-routing-and-selector
tags: [mens, routing, auto_select, orchestrator, embedder]
provides:
  - `SpokeGateChecker`, `JsonSpokeGateChecker`, `resolve_task_spoke`, `route_task_to_spoke` in `crates/vox-populi/src/mens/tensor/domain_router.rs`
  - `select_optimal_spoke_and_size` in `crates/vox-orchestrator/src/models/auto_select.rs`
  - `B3_RETRIEVAL_EMBEDDER_PIN`, `route_retrieval_lane_to_embedder`, `research_expert_reward_hook` re-exported on `vox_orchestrator::models`
affects: [vox-populi, vox-orchestrator]
key-files:
  modified:
    - crates/vox-populi/src/mens/tensor/domain_router.rs
    - crates/vox-orchestrator/src/models/auto_select.rs
    - crates/vox-orchestrator/src/models/registry.rs
    - crates/vox-orchestrator/src/models/registry/mens_spoke_routing.rs
    - crates/vox-orchestrator/src/models/mod.rs
    - crates/vox-orchestrator/src/planning/gsd/scheduler.rs
completed: 2026-11-13
---

# Phase 340 Plan 02 Summary: Spoke-Aware Model Selector & Retrieval Routing

- Added `SpokeGateChecker`, `JsonSpokeGateChecker`, `resolve_task_spoke`, and `route_task_to_spoke` + unit test `route_task_to_spoke_checks_gate_and_maps_harness` in [domain_router.rs](file:///Users/brbrainerd/dev/vox/crates/vox-populi/src/mens/tensor/domain_router.rs).
- Added [select_optimal_spoke_and_size](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/models/auto_select.rs) (combining live memory selection with `resolve_task_spoke`) and unit test `select_optimal_spoke_and_size_maps_lanes_and_size`.
- Exported `B3_RETRIEVAL_EMBEDDER_PIN`, `route_retrieval_lane_to_embedder`, and `research_expert_reward_hook` in [mens_spoke_routing.rs](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/models/registry/mens_spoke_routing.rs), [registry.rs](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/models/registry.rs), and [mod.rs](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/models/mod.rs), with unit test `retrieval_lanes_and_research_expert_hooks_resolve_deterministically`.
- Aligned `vox_orchestrator_types::FileAffinity` import in [scheduler.rs](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/planning/gsd/scheduler.rs) with `crate::types::FileAffinity` so `vox-orchestrator` lib tests compile cleanly.
