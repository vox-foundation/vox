---
phase: 342-rust-spoke
plan: 01
subsystem: mens-corpus-rust
tags: [mens, rust_authoring, rust_bench, leakage]
provides:
  - Deterministic zero-leakage test `zero_leakage_against_rust_bench` in `crates/vox-corpus/src/corpus/rust_authoring.rs`
affects: [vox-corpus]
key-files:
  modified:
    - crates/vox-corpus/src/corpus/rust_authoring.rs
completed: 2026-11-13
---

# Phase 342 Plan 01 Summary: Rust Spoke Corpus Zero-Leakage Guard

- Added [rust_authoring::tests::zero_leakage_against_rust_bench](file:///Users/brbrainerd/dev/vox/crates/vox-corpus/src/corpus/rust_authoring.rs) validating that `corpus_from_workspace` pairs have zero prompt (`user`) or signature (`assistant`) overlap against held-out benchmarks in [items.v1.jsonl](file:///Users/brbrainerd/dev/vox/mens/data/rust_bench/items.v1.jsonl).
- **Verification**: `cargo test -p vox-corpus --lib -- tool_selection argument_generation rust_authoring rust_bench vox_reasoning_synth` (`22 passed; 0 failed; 2 ignored; 285 filtered out`).
