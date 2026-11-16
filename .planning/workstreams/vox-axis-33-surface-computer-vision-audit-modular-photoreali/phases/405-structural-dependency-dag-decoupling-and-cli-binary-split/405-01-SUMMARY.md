---
phase: 405-structural-dependency-dag-decoupling-and-cli-binary-split
plan: 01
subsystem: build-dag
tags: [vox-repository, compilation-firewalls, apple-silicon, linker, dev-loop-guard, cargo-dep-hygiene, toestub]
requires:
  - phase: 404
    provides: federated mirror parity and standalone satellite builds
provides:
  - "`vox_repository::core` leaf facade + Rule-3 inline defactoring severing `vox-repository` from `vox-compiler`, `vox-db`, `vox-codegen`, `vox-doc-inventory`, `vox-corpus`, and `vox-orchestrator-driver`"
  - "Pruned dead & test-only `[dependencies]` across `vox-cli` (`max_workspace_deps` lowered `60 -> 50`) and 6 keystones"
  - "`CargoDepHygieneDetector` (`vox/cargo/dep-hygiene`) + `WorkspaceDriftDetector` `Language::Unknown` fix + `lefthook.yml` `tdd-guard` manifest globbing"
  - "Tightened `contracts/ci/fan-in-snapshot.v1.json` (`vox-compiler: 20`, `vox-config: 40`, `vox-db: 27`, `vox-llm-egress: 4`, `vox-repository: 13`), `layers.toml` `[[forbidden_deps]]`, and `dev_loop_guard` `-Wl,-dead_strip` enforcement"
  - "Live before/after incremental rebuild proof in `contracts/reports/v68-structural-dag-speedup.v1.json`"
affects:
  - vox-repository
  - vox-compiler
  - vox-db
  - vox-codegen
  - vox-doc-inventory
  - vox-corpus
  - vox-orchestrator-driver
  - vox-orchestrator
  - vox-orchestrator-mcp
  - vox-populi
  - vox-cli
  - vox-cli-ci
  - vox-code-audit
coverage:
  - id: D-405-01
    description: "Severed `vox-repository` from 6 keystones (`vox-compiler`, `vox-db`, `vox-codegen`, `vox-doc-inventory`, `vox-corpus`, `vox-orchestrator-driver`) via `vox_repository::core` + Rule-3 inline helpers."
    requirement: STRUCT-DAG-SPEED-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-compiler --lib (642 passed, 1 pre-existing ignored); cargo test -p vox-db --lib (309 passed)"
        status: pass
    human_judgment: false
  - id: D-405-02
    description: "Pruned zombie and test-only `[dependencies]` across `crates/vox-cli/Cargo.toml` and 6 keystones; verified full test compilation across all 1,797 `vox-cli --tests` units."
    requirement: STRUCT-DAG-SPEED-02
    verification:
      - kind: build
        ref: "cargo check -p vox-cli --tests (exit 0, 1797 units); cargo check -p vox-populi --tests; cargo check -p vox-orchestrator --tests; cargo check -p vox-orchestrator-mcp --tests"
        status: pass
    human_judgment: false
  - id: D-405-03
    description: "Apple Silicon `[target.aarch64-apple-darwin] -Wl,-dead_strip` configured in `.cargo/config.toml` and guarded by `vox-cli-ci::dev_loop_guard` without global `panic=abort`."
    requirement: STRUCT-DAG-SPEED-03
    verification:
      - kind: unit
        ref: "cargo test -p vox-cli-ci --lib -- --test-threads=1 (557 passed, 0 failed, incl. flags_missing_macos_dead_strip_or_global_panic_abort and the_repository_passes_the_dev_loop_guard)"
        status: pass
    human_judgment: false
  - id: D-405-04
    description: "Added O(1) `CargoDepHygieneDetector` (`vox/cargo/dep-hygiene`), fixed `WorkspaceDriftDetector` `Language::Unknown`, wired `lefthook.yml`, enforced `layers.toml` `[[forbidden_deps]]`, and ratcheted `fan-in-snapshot.v1.json`."
    requirement: STRUCT-DAG-SPEED-04
    verification:
      - kind: unit
        ref: "cargo test -p vox-code-audit --lib (485 passed, 1 pre-existing ignored); ./target/debug/toestub ... --rules vox/cargo/dep-hygiene --mode enforce-strict (0 findings)"
        status: pass
    human_judgment: false
  - id: D-405-05
    description: "Live before/after incremental benchmark on `crates/vox-repository/src/repo_catalog/federation.rs` recorded in `contracts/reports/v68-structural-dag-speedup.v1.json`."
    requirement: STRUCT-DAG-SPEED-05
    verification:
      - kind: benchmark
        ref: "cargo check -p vox-compiler -p vox-db (4.10s, 0 crates recompiled); cargo build -p vox-cli (74s probe / 66s revert, 16 crates / 20 units vs 43 crates / 89.0s before)"
        status: pass
    human_judgment: false
requirements-completed:
  - STRUCT-DAG-SPEED-01
  - STRUCT-DAG-SPEED-02
  - STRUCT-DAG-SPEED-03
  - STRUCT-DAG-SPEED-04
  - STRUCT-DAG-SPEED-05
completed: 2026-11-13
---

# Phase 405 Plan 01: Structural Dependency DAG Decoupling, Zombie-Dep Pruning & Enforceable Guards Summary

## What Was Built
- Severed `vox-repository` completely from `vox-compiler`, `vox-db`, `vox-codegen`, `vox-doc-inventory`, `vox-corpus`, and `vox-orchestrator-driver` using `AGENTS.md §Dependency Discipline` Rule 3 (`// vox:defactored-from vox-repository 2026-11-12` inline helpers) alongside `vox_repository::core` and `21` `#[inline(never)]` compilation firewalls.
- Pruned unused `[dependencies]` and relocated test-only dependencies to `[dev-dependencies]` across [crates/vox-cli/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-cli/Cargo.toml) (`max_workspace_deps` lowered `60 -> 50`), [crates/vox-compiler/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-compiler/Cargo.toml), [crates/vox-codegen/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-codegen/Cargo.toml) (retaining `vox-deploy-codegen` for `crates/vox-codegen-ts/src/emitter.rs`), [crates/vox-db/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-db/Cargo.toml), [crates/vox-orchestrator/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/Cargo.toml), [crates/vox-orchestrator-mcp/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/Cargo.toml), and [crates/vox-populi/Cargo.toml](file:///Users/brbrainerd/dev/vox/crates/vox-populi/Cargo.toml). Synced [docs/src/api/vox-cli.generated.md](file:///Users/brbrainerd/dev/vox/docs/src/api/vox-cli.generated.md).
- Added [cargo_dep_hygiene.rs](file:///Users/brbrainerd/dev/vox/crates/vox-code-audit/src/detectors/cargo_dep_hygiene.rs) (`vox/cargo/dep-hygiene`, `Language::Unknown`), fixed [workspace_drift.rs](file:///Users/brbrainerd/dev/vox/crates/vox-code-audit/src/detectors/workspace_drift.rs) (`Language::Unknown`), wired manifest globs into [lefthook.yml](file:///Users/brbrainerd/dev/vox/lefthook.yml), added `[[forbidden_deps]]` + `max_workspace_deps = 50` in [layers.toml](file:///Users/brbrainerd/dev/vox/docs/src/architecture/layers.toml), tightened [fan-in-snapshot.v1.json](file:///Users/brbrainerd/dev/vox/contracts/ci/fan-in-snapshot.v1.json) (`vox-compiler: 20`, `vox-config: 40`, `vox-db: 27`, `vox-llm-egress: 4`, `vox-repository: 13`), and guarded `-Wl,-dead_strip` in [dev_loop_guard.rs](file:///Users/brbrainerd/dev/vox/crates/vox-cli-ci/src/dev_loop_guard.rs).

## Verification & Before/After Benchmark
- **Incremental `vox-repository/src/repo_catalog/federation.rs` 1-line edit:**
  - Before: `43` crates / `1,161` units recompiled + `89.0s` (`1m 29s`) `cargo build -p vox-cli` (recompiling `vox-compiler` + `vox-db`), emitting `ld: __eh_frame section too large`.
  - After: `cargo check -p vox-compiler -p vox-db` finished in **4.10s** with **0 crates recompiled**; `cargo build -p vox-cli` rebuilt **16 crates / 20 units** in **74s** (probe) / **66s** (revert) (note: `ld: __eh_frame section too large` still emits on unoptimized debug `vox` monolith linking).
- `cargo test -p vox-compiler --lib`: **642 passed; 0 failed; 1 pre-existing ignored** (`3.02s`).
- `cargo test -p vox-db --lib`: **309 passed; 0 failed** (`70.29s`).
- `cargo test -p vox-code-audit --lib`: **485 passed; 0 failed; 1 pre-existing ignored** (`2.54s`).
- `cargo test -p vox-cli-ci --lib -- --test-threads=1`: **557 passed; 0 failed** (`25.72s`).
- `cargo check` with `--tests` exited `0` across `vox-cli` (`1,797` units), `vox-populi`, `vox-orchestrator`, `vox-orchestrator-mcp`, and all decoupled keystones.
- `./target/debug/toestub ... --rules vox/cargo/dep-hygiene --mode enforce-strict`: exited `0` with `0` findings (`~3s` wall).
- Pre-existing unrelated workspace gate findings noted in [405-VERIFICATION.md](file:///Users/brbrainerd/dev/vox/.planning/phases/405-structural-dependency-dag-decoupling-and-cli-binary-split/405-VERIFICATION.md): `vox-arch-check` exits `1` on pre-existing `hardcoded_abs_dir.rs:164`; `vox ci crate-edges --tighten` refuses due to `55` pre-existing unrecorded edges (`vox-cli-research`, `vox-cli-ci`, etc.); `vox ci fan-in-budget` exits `1` on `2` pre-existing regressions (`vox-ast: 3->4`, `vox-bounded-fs: 23->24`).
