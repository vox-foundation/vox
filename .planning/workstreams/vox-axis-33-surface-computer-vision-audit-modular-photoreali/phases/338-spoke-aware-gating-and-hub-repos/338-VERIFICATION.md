---
phase: 338-spoke-aware-gating-and-hub-repos
verified: 2026-11-13
status: passed
---

# Phase 338 Verification: Spoke-Aware Gating & Hub Repos

## Verification Results

| Check | Command | Result |
|---|---|---|
| MCP tool catalog & salvage parity | `timeout 900s cargo test -p vox-orchestrator-mcp --lib -- tool_catalog salvage` | **Passed** (`6 passed; 0 failed; 1006 filtered out`) |
| `vox-eval` episode controls, suite report & gate state | `timeout 900s cargo test -p vox-eval --lib` | **Passed** (`238 passed; 0 failed; 4 ignored`) |
| `vox-populi` hub sync, gate policy, domain profiles, card facts | `timeout 900s cargo test -p vox-populi --features mens-hf-hub --lib` | **Passed** (`424 passed; 0 failed; 3 ignored`) |
| `vox-ml-cli` spoke eval, gate checks, hub CLI & train arm | `timeout 900s cargo test -p vox-ml-cli --features gpu --lib -- eval_gate eval_spoke hub train_arm` | **Passed** (`99 passed; 0 failed; 0 ignored; 393 filtered out`) |

## Notes on Full Unfiltered `vox-ml-cli --features gpu --lib` Run
An earlier unfiltered run of `cargo test -p vox-ml-cli --features gpu --lib` passed 486 tests and surfaced 1 change-related failure (`eval_gate::tests::gate_keys_inventory`, subsequently fixed and verified above) plus 3 pre-existing unrelated test failures outside Phase 338's modified code paths:
1. `commands::mens::eval_collateral::tests::the_ladder_policy_states_alpha_and_a_policy_without_it_is_refused` (pre-existing synthetic fixture uses unknown field `version` rejected by `#[serde(deny_unknown_fields)]`).
2. `commands::mens::populi::dispatch::serve_preconditions_tests::a_below_floor_throughput_run_is_refused_by_the_real_policy` (pre-existing test checks retired `throughput` gate key).
3. `commands::schola::train::run_train::tests::cpu_explicit_size_is_kept_unchecked` (pre-existing expectation of `"qwen3_dev_cpu"` preset vs `"qwen3_0_6b"` in `gpu-specs.yaml`).
