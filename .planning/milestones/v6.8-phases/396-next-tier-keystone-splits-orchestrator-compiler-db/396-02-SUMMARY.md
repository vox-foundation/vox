---
phase: 396-next-tier-keystone-splits-orchestrator-compiler-db
plan: 02
subsystem: build-performance
tags: [inline-never, vox-populi, vox-publisher, vox-scientia, vox-codegen]
completed: 2026-11-10
requirements:
  - KEYSTONE-V68-01
  - KEYSTONE-V68-02
  - KEYSTONE-V68-03
  - KEYSTONE-V68-04
---

# Phase 396 Plan 02 Summary: Second-Tier Keystone & High-Churn Compile-Time Optimization Across 11 Crates

## Accomplishments

1. **Second-Tier Compilation Firewalls Across 11 Crates:**
   - Added `#[inline(never)]` boundaries across `vox-populi` (`26.9s -> 6.41s`), `vox-publisher` (`19.1s -> 7.33s`), `vox-scientia` (`14.7s -> 7.31s`), `vox-codegen` (`10.9s -> 7.67s`), `vox-corpus` (`8.6s -> 7.57s`), `vox-cli-ci`, `vox-cli-core`, `vox-sql`, `vox-gamify`, `vox-research-shim`, and `vox-search`.
2. **Verification:**
   - Verified clean compilation across all 11 crates via `cargo check` and formatted modified `.rs` files with `rustfmt --edition 2024`.
