---
phase: 405
name: Structural Dependency DAG Decoupling, Keystone Zombie-Dep Pruning, Apple Silicon Linker Optimization & Performant TOESTUB/CI Enforcement
created: 2026-11-12
status: locked
---

# Phase 405 Context — Architectural Decisions

## Locked Decisions

- **D-405-01 (Keystone `vox-repository` Edge Severing via `AGENTS.md` Rule 3 Defactoring):**
  - Instead of adding a new crate (which adds workspace metadata overhead and cannot expand `contracts/ci/crate-edges.allow.v1.json` without `USER-AUTHORIZED-ONLY` exceptions), apply `AGENTS.md §Dependency Discipline` Rule 3 (`// vox:defactored-from vox-repository 2026-11-12`) to inline the `< 45`-line path/capability helpers used by `vox-compiler`, `vox-db`, `vox-doc-inventory`, `vox-corpus`, and `vox-orchestrator-driver`, and remove the dead `vox-repository` dependency from `crates/vox-codegen/Cargo.toml`.
  - Result: Zero new crate edges and 6 high-blast keystones completely insulated from edits to `vox-repository` (`repo_catalog`, `federation`, `skill_scaffold`).

- **D-405-02 (Prune Zombie & Test-Only `[dependencies]` Across `vox-cli` and Keystones):**
  - Audit revealed 29 completely unused `[dependencies]` and 5 test-only `[dependencies]` in `crates/vox-cli/Cargo.toml`, plus 16 unused/test-only dependencies across `vox-compiler`, `vox-codegen`, `vox-db`, `vox-orchestrator`, `vox-orchestrator-mcp`, and `vox-populi`.
  - Remove all truly dead `[dependencies]` and move test-only dependencies (`sha3`, `vox-workflow-runtime`, `vox-effort-audit`, `vox-effort-route`, `vox-publisher`, `tiktoken-rs`, `sysinfo`) into `[dev-dependencies]` so default/release `cargo build` skips linking unused subtrees (`vox-sql` `postgres`/`mysql`, `vox-scientia`, `vox-research-shim`, `vox-oauth-pkce`, `flate2`/`bzip2`/`zstd` on compiler, etc.). Note: `vox-deploy-codegen` is retained in `crates/vox-codegen/Cargo.toml` because `crates/vox-codegen-ts/src/emitter.rs` invokes `vox_deploy_codegen::generate` via `#[path]`.

- **D-405-03 (Apple Silicon Linker Unwind Optimization Without Breaking `release_build.rs`):**
  - `crates/vox-cli/src/commands/ci/release_build.rs` asserts that `.cargo/config.toml` never sets `panic=abort` globally.
  - Configure `[target.aarch64-apple-darwin]` in `.cargo/config.toml` with `rustflags = ["-C", "link-arg=-Wl,-dead_strip"]` so macOS `ld` strips unreachable functions while preserving unwinding. (Note: unoptimized debug `vox` monolith linking still emits the `ld: __eh_frame section too large` compact-unwind warning until further binary splitting / release stripping.)

- **D-405-04 (Performant `toestub` + `vox-arch-check` + `dev-loop-guard` Enforcement):**
  - `crates/vox-code-audit/src/scanner.rs` maps `Cargo.toml` to `Language::Unknown`. Any Cargo-level detector MUST include `Language::Unknown` in `languages()`.
  - Add `CargoDepHygieneDetector` (`vox/cargo/dep-hygiene`) in `vox-code-audit` (`languages = &[Language::Unknown]` so it skips 100% of `.rs`/`.ts`/`.vox` files in O(1)), add `Language::Unknown` to `WorkspaceDriftDetector::languages()`, wire `vox/cargo/dep-hygiene` into `lefthook.yml` `tdd-guard`, enforce `[[forbidden_deps]]` + `max_workspace_deps = 50` (down from `60`, matching total `[dependencies]` + `[dev-dependencies]` counted by `vox-arch-check`) in `docs/src/architecture/layers.toml`, tighten `contracts/ci/fan-in-snapshot.v1.json`, and guard `[target.aarch64-apple-darwin]` `-Wl,-dead_strip` in `vox-cli-ci::dev_loop_guard`.

- **D-405-05 (Empirical Before/After Benchmark Proof):**
  - Measure warm rebuild time on a 1-line `vox-repository::repo_catalog::federation` edit against our pre-Phase-405 baseline (`89.0s`, `43` crates recompiled, `1,161` units, `__eh_frame` link warning) and record verified numbers in `contracts/reports/v68-structural-dag-speedup.v1.json`.
