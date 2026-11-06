---
phase: 321-harness-before-after-harn-01-bench-04
plan: 01
subsystem: ml-cli
tags: [mens, harness, before-after, serve-gate, harn-01, bench-04]
provides:
  - resolve_harness_serve_target for chosen Qwen3 size base or gated adapter serve
  - write_harness_before_after_report across all qwen3_code QLoRA sizes
  - vox mens harness-before-after CLI subcommand
affects:
  - crates/vox-ml-cli/src/commands/mens/harness_before_after.rs
  - crates/vox-ml-cli/src/commands/mens/mod.rs
  - crates/vox-ml-cli/src/commands/mens/populi/mod.rs
  - crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs
  - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
tech-stack:
  added: []
  patterns:
    - Chosen-size serve resolution behind assert_collateral_gate + assert_serve_preconditions
    - Consolidated multi-size harness_before_after.v1 JSON report with reasoning + intelligence scores
key-files:
  created:
    - crates/vox-ml-cli/src/commands/mens/harness_before_after.rs
  modified:
    - crates/vox-ml-cli/src/commands/mens/mod.rs
    - crates/vox-ml-cli/src/commands/mens/populi/mod.rs
    - crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
key-decisions:
  - "HARN-01: resolve_harness_serve_target resolves any Qwen3 size spelling through detect::resolve_base_pin, verifies snapshot_complete, and when an adapter is provided enforces adapter_manifest_names_snapshot + assert_collateral_gate + assert_serve_preconditions."
  - "BENCH-04: write_harness_before_after_report emits harness_before_after.v1 with base and adapter scores across all 3 reasoning slices (n=32) plus intelligence (pass_rate_at_1, n) for trained sizes, while keeping untrained sizes free of numeric score keys."
patterns-established:
  - "milestone_ready is computed directly from trained_scored_count >= 2."
requirements-completed:
  - HARN-01
  - BENCH-04
duration: 10min
completed: 2026-11-01
---

# Phase 321 Plan 01: Harness Before/After (`HARN-01`, `BENCH-04`) Summary

Implemented `resolve_harness_serve_target` (`HARN-01`) and `write_harness_before_after_report` (`BENCH-04`) in `crates/vox-ml-cli/src/commands/mens/harness_before_after.rs` and wired `vox mens harness-before-after` on `PopuliMensTail`.

## Verification

- `CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --features gpu --lib harness_before_after -- --test-threads=8` (`2 passed; 0 failed`)
