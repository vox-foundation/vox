# `mens/data/rust_bench/` — Held-Out Rust Compile-and-Test Benchmark

Canonical held-out compile-and-test benchmark for the Vox MENS Rust spoke ([SPOKE-07](../../../docs/src/architecture/agentos-ssot-2026.md), D-09, D-22).

## What the Bench Measures

This benchmark evaluates a candidate Rust model or adapter on its ability to produce syntactically valid, type-safe, and functionally correct Rust code for pure algorithm and data structure problems.

- **Scoring:** A candidate solution is scored strictly by compiling it in a fresh, isolated `std`-only crate and executing its hidden unit tests via `cargo test --offline --quiet`.
- **Pass condition:** An item passes if and only if the candidate compiles without error and **every** hidden unit test passes. There is no partial credit and no AST or string comparison.

## Item Format (`items.v1.jsonl`)

Each row in `items.v1.jsonl` is a single JSON object with the following fields:

- `id` (string): Unique identifier (e.g. `rust_01_clamp_range`).
- `prompt` (string): Natural language description of the problem specification without leaking test cases.
- `signature` (string): Required `pub fn` signature.
- `hidden_tests` (string): Fenced Rust unit tests (`#[test] fn ...`) exercising standard cases, edge cases, and boundary values.
- `reference` (string): Canonical reference implementation containing the signature that passes all hidden tests.
- `training_eligible` (boolean): **MUST be `false`**.

## Non-Contamination Rule

All items in this directory carry `"training_eligible": false`.
They are **strictly held out** and must never be ingested by any MENS training pipeline, fine-tuning dataset, or pre-training corpus. The corpus loader in `crates/vox-corpus/src/corpus/rust_bench.rs` fails closed if any item has `training_eligible: true`.

## How to Add an Item

1. Define a pure function signature over `std` types (no external dependencies, no I/O).
2. Write a clear one-paragraph specification prompt.
3. Write 3–6 hidden unit tests testing edge cases and normal behavior.
4. Write a reference implementation.
5. Verify controls:
   - Oracle (reference) must pass.
   - `todo_stub` (signature with `todo!()` body) must fail.
   - `shuffled_reference` (reference from any other problem) must fail to compile or fail tests.
   - Set `"training_eligible": false`.

## Controls Matrix

The test suite evaluates 5 control policies against the bench:
1. **Oracle:** Reference solutions (must achieve 12/12 Passed).
2. **Todo stub:** Signature with `todo!()` body (must achieve 0/12 Passed, failing hidden tests).
3. **Empty:** Empty string (must achieve 0/12 Passed, failing compilation).
4. **Shuffled reference:** Cyclic shift of reference solutions (must achieve 0/12 Passed).
5. **Forbidden API:** Reference with injected process/network calls (must be 12/12 Refused before execution).

## Isolation and Security Limits

- **Pre-execution static scan:** Candidate source code is statically scanned for banned token patterns before execution (`std::process`, `std::net`, `std::fs`, `std::env`, `std::os`, `Command`, `unsafe`, `extern crate`, `include!`, `env!`, `#[path]`). Any match causes immediate refusal.
- **Environment clearing:** Process environment is cleared with an allowlist restricted to minimal build vars (`PATH`, `HOME`, `CARGO_HOME`, `RUSTUP_HOME`, `TMPDIR`).
- **Offline compilation:** Executed with `--offline` and `CARGO_NET_OFFLINE=true`.
- **Note on sandboxing:** This is an execution guard, not a full hypervisor sandbox. Tests run as the current user under a strict timeout with process tree termination.

## Running the Matrix

To run the full 60-run control matrix:

```bash
cargo test -p vox-corpus --lib rust_bench_control_matrix -- --ignored
```

Wall-clock time: ~3–6 minutes across 12 items × 5 control policies.
