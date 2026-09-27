---
title: "Workspace test inventory (2026)"
description: "Regenerable counts of Rust tests, ignores, and related harness patterns across the workspace (fully regenerated; refresh dates via git history)."
category: "Architecture SSOTs"
status: "current"
training_eligible: false
---

# Workspace test inventory

Regenerate this page with:

`cargo run -p vox-cli -- ci test-inventory --markdown docs/src/architecture/test-inventory-2026.md`

Machine-readable JSON:

`cargo run -p vox-cli -- ci test-inventory --json`

## Summary counts

| Metric | Value |
| --- | ---: |
| Workspace crates (`crates/*/Cargo.toml`) | 129 |
| Rust files under `crates/**/*.rs` | 4067 |
| Cargo unit tests (`#[test]` / `tokio::test` / `rstest` / `proptest` in `src/`) | 10507 |
| Cargo integration tests (`crates/.../tests/`) | 3044 |
| Cargo bench fns (`#[bench]` in scanned paths) | 0 |
| Ignored test functions (best-effort parse) | 169 |
| Golden `.vox` files (`examples/golden/**/*.vox`) | 87 |
| `@test` lines in golden Vox | 87 |
| App E2E-style files (`apps/**/*.test.*` / `*.spec.*`) | 692 |
| Doctest candidate src files (rust/no_run doc fences) | 11 |
| Doctest fence lines counted | 17 |

### Test harness patterns (Rust files in unit/integration/bench paths)

| Pattern | Count |
| --- | ---: |
| `sleep` sites | 207 |
| Env reads (`env::var` / `std::env::var`) | 776 |
| Env mutations (`set_var` / `remove_var`) | 1016 |
| `Command::new` | 571 |
| `serial_test` | 92 |
| `proptest::` | 9 |
| `quickcheck::` | 2 |
| `insta::` | 109 |

## Caveats

- **WebIR / internal pipelines:** Ignored tests that mention WebIR are treated as active internal pipeline tests unless the ignore reason clearly indicates tombstone, retired, or dropped parity.
- **Nextest vs doctests:** `cargo nextest` runs compiled test binaries (unit/integration in crates) but does not execute `cargo test` doctests; this inventory tracks doctest candidates separately via ```rust / ```no_run fences in crate src files.
- **WebIR-related ignored tests (active heuristic):** 3
- **WebIR ignores with retired/tombstone-style reasons:** 0

## Zero-test crates

- `vox-cli-contracts`
- `workspace-hack`

## Top ignored files

| File | Ignored tests |
| --- | ---: |
| `crates/vox-mesh-transport/tests/interp_executor.rs` | 12 |
| `crates/vox-compiler/tests/golden_dashboard_composites_test.rs` | 11 |
| `crates/vox-compiler/tests/golden_dashboard_surfaces_test.rs` | 9 |
| `crates/vox-compiler/tests/state_machine_integration_test.rs` | 9 |
| `crates/vox-compiler/tests/golden_runs_surface_test.rs` | 6 |
| `crates/vox-codegen/src/codegen_rust/mod.rs` | 5 |
| `crates/vox-compiler/tests/golden_dashboard_chrome_test.rs` | 5 |
| `crates/vox-compiler/tests/golden_mesh_surface_test.rs` | 5 |
| `crates/vox-integration-tests/tests/orchestrator_e2e_test.rs` | 5 |
| `crates/vox-integration-tests/tests/pipeline/includes/include_01.rs` | 5 |
| `crates/vox-compiler/tests/golden_for_loop_test.rs` | 4 |
| `crates/vox-compiler/tests/golden_svg_vuv_test.rs` | 4 |
| `crates/vox-eval/src/lib.rs` | 4 |
| `crates/vox-integration-tests/tests/parity_contracts_test.rs` | 4 |
| `crates/vox-cli/tests/run_benchmark.rs` | 3 |
| `crates/vox-cli/tests/run_mode_dispatch.rs` | 3 |
| `crates/vox-compiler/tests/bug_handler_lambda_repro.rs` | 3 |
| `crates/vox-compiler/tests/golden_svg_snake_case_test.rs` | 3 |
| `crates/vox-integration-tests/tests/codegen_rust_test.rs` | 3 |
| `crates/vox-plugin-mens-candle-metal/src/inference.rs` | 3 |
| `crates/vox-codegen/tests/generated_project_builds_outside_repo.rs` | 2 |
| `crates/vox-compiler/tests/tombstone_test.rs` | 2 |
| `crates/vox-compiler/tests/web_ir_environment_gates_test.rs` | 2 |
| `crates/vox-integration-tests/tests/cli_test.rs` | 2 |
| `crates/vox-integration-tests/tests/ts_emit_typecheck_test.rs` | 2 |

## Rust files by kind

- `benches`: 3
- `integration_tests`: 755
- `other`: 24
- `unit_src`: 3285

