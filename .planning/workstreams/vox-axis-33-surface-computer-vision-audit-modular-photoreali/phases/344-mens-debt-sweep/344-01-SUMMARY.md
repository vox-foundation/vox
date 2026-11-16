---
phase: 344-mens-debt-sweep
plan: 01
subsystem: mens-domain-profiles
tags: [mens, debt-sweep, domain_profiles, validation]
provides:
  - `DomainProfilesFile::validate_all_profiles` invariant check across hub embedder, 4 QLoRA spokes, 4 non-base profiles, and `research-expert` reward hook
  - Regression test `all_domain_profiles_are_valid` in `crates/vox-populi/src/mens/tensor/domain_profiles.rs`
affects: [vox-populi]
key-files:
  modified:
    - crates/vox-populi/src/mens/tensor/domain_profiles.rs
completed: 2026-11-13
---

# Phase 344 Plan 01 Summary: MENS Debt Sweep & Domain Profile Invariants

- Added [DomainProfilesFile::validate_all_profiles](file:///Users/brbrainerd/dev/vox/crates/vox-populi/src/mens/tensor/domain_profiles.rs) enforcing:
  1. Presence and `@`-pinned revision validity of `hub.embedder`.
  2. All four v1 QLoRA spokes (`vox-lang`, `rust`, `tool-selection`, `argument-generation`) specify `base` with `TrainMethod::Qlora` and a non-empty `eval_gate`.
  3. Non-adapter profiles (`harness`, `rocks`, `research`, `populi-meta`) omit `base`.
  4. `research-expert` specifies `reward_hook = "research_synthesis_eval"`.
- Added unit test `spoke_base_tests::all_domain_profiles_are_valid` against workspace [domain-profiles.yaml](file:///Users/brbrainerd/dev/vox/mens/config/domain-profiles.yaml).
- **Verification**: `cargo test -p vox-populi --features mens-hf-hub --lib -- hub_repo hub_sync domain_router domain_profiles` (`70 passed; 0 failed; 1 ignored; 360 filtered out`).
