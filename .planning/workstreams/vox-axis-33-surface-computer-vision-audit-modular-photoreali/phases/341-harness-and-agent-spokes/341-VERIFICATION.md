---
phase: 341-harness-and-agent-spokes
status: passed_with_deferred_gpu_runs
verified: 2026-11-13
---

# Phase 341 Verification: Harness and Agent Spokes

## Automated Checks Run
- `timeout 900s cargo test -p vox-corpus --lib -- tool_selection argument_generation rust_authoring rust_bench vox_reasoning_synth`
  - Result: `22 passed; 0 failed; 2 ignored; 0 measured; 285 filtered out`
  - Includes `corpus::tool_selection_synth::tests::zero_leakage_against_agent_episodes_eval` (`ok`) and `corpus::argument_generation_synth::tests::zero_leakage_against_agent_episodes_eval` (`ok`).
- `timeout 900s cargo test -p vox-populi --features mens-hf-hub --lib -- hub_repo hub_sync domain_router domain_profiles`
  - Result: `70 passed; 0 failed; 1 ignored; 0 measured; 360 filtered out`
  - Includes `harness_profile_exists_without_base` (`ok`) and `all_domain_profiles_are_valid` (`ok`).

## Deferred Manual / GPU Gates
- Live multi-hour GPU training and comparative episode eval between mono `harness` and decomposed `tool-selection` / `argument-generation` QLoRA adapters were not executed in this CI/code pass.
