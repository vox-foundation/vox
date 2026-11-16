---
phase: 338-spoke-aware-gating-and-hub-repos
plan: "08"
subsystem: mens-rust-bench
tags: [rust-bench, eval-gate, suite-report]
requires:
  - phase: 338-07
    provides: "SuiteGate schema and eval_spoke runner harness"
provides:
  - "Paired Rust compile-and-test runner run_rust_bench_suite refusing training_eligible items"
  - "rust_bench suite threshold block in mens/config/eval-gates-rust.yaml"
affects: [vox-ml-cli, mens/config/eval-gates-rust.yaml]
tech-stack:
  added: []
  patterns: [sandboxed cargo test suite report]
key-files:
  modified:
    - crates/vox-ml-cli/src/commands/mens/eval_spoke.rs
    - mens/config/eval-gates-rust.yaml
key-decisions:
  - "Score rust_bench items via vox_corpus::corpus::rust_bench::{load_items, run_item} into paired SuiteReport items"
requirements-completed: [SPOKE-08]
completed: 2026-11-13
---

# Phase 338 Plan 08: Rust Functional Suite Integration & Gate Configuration Summary

**Connected `vox_corpus::corpus::rust_bench` into paired `SuiteReport` generation and configured `eval-gates-rust.yaml` thresholds.**

## Key Changes
- Implemented `run_rust_bench_suite` in `crates/vox-ml-cli/src/commands/mens/eval_spoke.rs` producing paired `(base, candidate)` `SuiteReport` objects from `rust_bench::load_items` and `rust_bench::run_item`.
- Configured `suites.rust_bench` thresholds in `mens/config/eval-gates-rust.yaml`.

## Verification
- `timeout 900s cargo test -p vox-ml-cli --features gpu --lib -- eval_gate eval_spoke`: passed within the `99 passed; 0 failed` run.
