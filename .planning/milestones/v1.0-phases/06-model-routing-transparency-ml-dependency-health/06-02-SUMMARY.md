---
phase: 06-model-routing-transparency-ml-dependency-health
plan: 02
subsystem: cli
status: implementation-complete-verification-blocked
requirements: [ML-01]

provides:
  - "Tested cuda-features command specs for vox-plugin-speech and vox-ml-cli"
  - "Tested cuda-release-build command spec for the vox-ml-cli release binary"
  - "Locked cargo-metadata contract for the unified Candle, peft-rs, and qlora-rs versions"

key-files:
  created:
    - "crates/vox-cli/tests/ml_dependency_health.rs"
  modified:
    - "crates/vox-cli/src/commands/ci/run_body_helpers/cuda.rs"
    - "crates/vox-cli/src/commands/ci/run_body_helpers/cuda_release_build.rs"

commits: []
completed: 2026-09-30
---

# Phase 6 Plan 02: ML Dependency Health Summary

Implemented the current CUDA ownership contracts without changing manifests, the lockfile, or patches.

## Changes

- `cuda-features` now derives both subprocess invocations from a pure command-spec helper:
  - `cargo check -p vox-plugin-speech --features cuda`
  - `cargo check -p vox-ml-cli --features gpu,mens-candle-cuda`
- CUDA feature failures now report the exact failed Cargo argument sequence, including the owning package.
- `cuda-release-build` now runs `cargo build -p vox-ml-cli --bin vox-ml-cli --release --features gpu,mens-candle-cuda`; its start, spawn, failure, and success text identifies `vox-ml-cli`.
- Added unit contracts for both command specs.
- Added `ml_dependency_health.rs`, which runs brokered `cargo metadata --format-version 1 --locked`, parses the JSON, and requires singleton versions: Candle crates `0.10.2`, `peft-rs` `1.0.3`, and `qlora-rs` `1.0.5`.

## Verification

Passed:

- `rustfmt --edition 2024 --check` on all three authorized Rust files.
- `git diff --check` on all three authorized Rust files.
- `cargo metadata --format-version 1 --locked` resolved exactly:
  - `candle-core`, `candle-nn`, `candle-transformers`: `0.10.2`
  - `peft-rs`: `1.0.3`
  - `qlora-rs`: `1.0.5`
- `git diff --exit-code -- Cargo.toml Cargo.lock patches/` passed; no dependency files changed.
- Target search found only `vox-plugin-speech` and `vox-ml-cli` in the production command specs; no stale `vox-speech` target remains.

Blocked by an unrelated existing compile failure before the requested tests could execute:

- `cargo test -p vox-cli --lib commands::ci::run_body::run_body_helpers::cuda::tests`
- `cargo test -p vox-cli --lib commands::ci::run_body::run_body_helpers::cuda_release_build::tests`
- `cargo test -p vox-cli --test ml_dependency_health`
- `cargo check -p vox-cli`

All four fail while compiling `vox-orchestrator-mcp`, before the requested tests can execute or the package check can complete:

- `crates/vox-orchestrator-mcp/src/visus_review/mod.rs:385`: `CacheEntry` initializer missing `defects` and `ux_report` (`E0063`).
- `crates/vox-orchestrator-mcp/src/visus_review/mod.rs:984`: use of partially moved `ux` after moving `ux.verdict` (`E0382`).

No files outside the four authorized plan files were edited, and nothing was staged or committed.
