---
phase: 333
status: all_fixed
findings_in_scope: 1
fixed: 1
skipped: 0
iteration: 1
fixed_at: 2026-11-13T08:19:00Z
---

# Phase 333 Code Review Fix Report

## Fixed

### CR-333-01: Feature-gated `spoke_route` re-export in `vox-populi`
- **File:** `crates/vox-populi/src/mens/serving/vllm_lora.rs:17-18`
- **Change:** Added `#[cfg(feature = "mens-hf-hub")]` to `pub use super::spoke_route::resolve_spoke_adapter;` to match `crates/vox-populi/src/mens/serving/mod.rs:6-11`, unblocking compilation of downstream crates (`vox-cli-research`) when `mens-hf-hub` is not enabled.

## Verification
- `timeout 900s cargo test -p vox-cli-research --lib -- py::env::tests`: `8 passed; 0 failed; 660 filtered out` (`0.13s`).
