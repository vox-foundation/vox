---
phase: 324-repo-naming-and-config
plan: 01
subsystem: mens-hf-hub
tags: [huggingface, mens, size-ladder, config]
provides:
  - Deterministic per-size HF repo naming (`<namespace>/vox-mens-qwen3-<size>`) and index repo (`<namespace>/vox-mens-index`)
  - `VOX_MENS_HF_NAMESPACE` config registry entry
affects: [phase-325, phase-326, phase-327]
key-files:
  created:
    - crates/vox-populi/src/mens/hub_repo.rs
  modified:
    - crates/vox-populi/src/mens/mod.rs
    - crates/vox-config/src/config_registry.rs
key-decisions:
  - "Derive size token from gpu-specs.yaml pinned base (`Qwen/Qwen3-<size>@<sha>`) so repo IDs never drift from the ladder."
  - "Never hardcode operator HF username in source; require `VOX_MENS_HF_NAMESPACE` from config/env."
requirements-completed: [HFNAME-01, HFNAME-02]
completed: 2026-11-01
---

# Phase 324 Plan 01: Repo Naming and Config Summary

## Accomplishments
- Added `vox_populi::mens::hub_repo` (`NAMESPACE_ENV`, `SIZES`, `size_of_pin`, `resolve_namespace`, `repo_id_for_size`, `index_repo_id`).
- Registered `VOX_MENS_HF_NAMESPACE` in `crates/vox-config/src/config_registry.rs`.
- Verified all 5 unit tests in `mens::hub_repo::tests` (`cargo test -p vox-populi --features mens-hf-hub --lib hub_repo` → `5 passed; 0 failed`).
