---
phase: 340-multi-spoke-hub-repo-layout-and-model-selector
status: passed
verified: 2026-11-13
---

# Phase 340 Verification: Multi-Spoke Hub Layout & Model Selector

## Automated Checks Run
- `timeout 900s cargo test -p vox-populi --features mens-hf-hub --lib -- hub_repo hub_sync domain_router domain_profiles`
  - Result: `70 passed; 0 failed; 1 ignored; 0 measured; 360 filtered out`
  - Includes `mens::hub_repo::tests::spoke_rel_dir_maps_known_spokes_and_rejects_unknown`, `mens::hub_sync::tests::stage_publish_bundle_spoke_places_files_under_spokes_subdir`, and `mens::tensor::domain_router::b6_champion_challenger_tests::route_task_to_spoke_checks_gate_and_maps_harness`.
- `timeout 900s cargo test -p vox-ml-cli --features gpu --lib -- commands::mens::hub`
  - Result: `7 passed; 0 failed; 0 ignored; 0 measured; 485 filtered out`
  - Includes `--spoke rust` parse checks on `sync` and `pull` inside `commands::mens::hub::tests::cards_only_flag_parses`.
- `timeout 900s cargo test -p vox-orchestrator --lib -- auto_select mens_spoke_routing`
  - Result: `11 passed; 0 failed; 0 ignored; 0 measured; 1443 filtered out`
  - Includes `models::auto_select::tests::select_optimal_spoke_and_size_maps_lanes_and_size` and `models::registry::mens_spoke_routing::tests::retrieval_lanes_and_research_expert_hooks_resolve_deterministically`.
