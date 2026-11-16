---
phase: 405-structural-dependency-dag-decoupling-and-cli-binary-split
verified: 2026-11-13T06:50:00Z
status: passed
score: 5/5 must-haves verified
---

# Phase 405 Verification Report

**Phase:** `405-structural-dependency-dag-decoupling-and-cli-binary-split`
**Requirement Set:** `STRUCT-DAG-SPEED-01..05`
**Status:** `passed`

## Verified Truths

| # | Requirement | Status | Evidence |
|---|---|---|---|
| 1 | `STRUCT-DAG-SPEED-01` | passed | Zero `vox-repository` edges in `vox-compiler`, `vox-db`, `vox-codegen`, `vox-doc-inventory`, `vox-corpus`, `vox-orchestrator-driver`; `cargo test -p vox-compiler --lib` (`642` passed, `0` failed, `1` pre-existing ignored) and `cargo test -p vox-db --lib` (`309` passed, `0` failed). |
| 2 | `STRUCT-DAG-SPEED-02` | passed | Zombie/test-only deps pruned across `vox-cli` (`max_workspace_deps` `60 -> 50`) and 6 keystones; `cargo check -p vox-cli --tests` (`1,797` units), `vox-populi --tests`, `vox-orchestrator --tests`, `vox-orchestrator-mcp --tests`, and `vox-codegen`/`vox-doc-inventory`/`vox-corpus`/`vox-orchestrator-driver` all exit `0`. |
| 3 | `STRUCT-DAG-SPEED-03` | passed | `.cargo/config.toml` sets `-Wl,-dead_strip` for `aarch64-apple-darwin` without global `panic=abort`; `cargo test -p vox-cli-ci --lib -- --test-threads=1` passes `557/557` (incl. `flags_missing_macos_dead_strip_or_global_panic_abort`, `the_repository_passes_the_dev_loop_guard`, `crate_api_docs::tests::test_run_in_valid_repo`). |
| 4 | `STRUCT-DAG-SPEED-04` | passed | `cargo test -p vox-code-audit --lib` passes `485` (`1` pre-existing ignored); `./target/debug/toestub ... --rules vox/cargo/dep-hygiene --mode enforce-strict` exits `0` (`0` findings); `vox ci fan-in-budget` prints `ok` for `vox-compiler: 20`, `vox-config: 40`, `vox-db: 27`, `vox-llm-egress: 4`, `vox-repository: 13`; `vox-arch-check` emits `0` `forbidden_deps` violations and `0` `workspace-dep budget` warnings. |
| 5 | `STRUCT-DAG-SPEED-05` | passed | Live 1-line `crates/vox-repository/src/repo_catalog/federation.rs` probe measured `cargo check -p vox-compiler -p vox-db` at `4.10s` (`0` crates recompiled) and `cargo build -p vox-cli` at `74s` probe / `66s` revert (`16` crates / `20` units vs `43` crates / `89.0s` before), recorded in [v68-structural-dag-speedup.v1.json](file:///Users/brbrainerd/dev/vox/contracts/reports/v68-structural-dag-speedup.v1.json). |

## Pre-Existing Unrelated Baseline Findings Observed (Outside Phase 405 Scope)
- `./target/debug/vox-arch-check` exited `1` on pre-existing `[no-hardcoded-abs-path]` in `crates/vox-code-audit/src/detectors/hardcoded_abs_dir.rs:164` (`0` Phase 405 `forbidden_deps` or budget warnings).
- `./target/debug/vox ci crate-edges --tighten` exited `1` because `55` pre-existing unrecorded edges exist in workspace manifests (`vox-cli-research`, `vox-cli-ci`, `vox-orchestrator-models`, `vox-package`, `vox-plugin-mens-candle-core`, `vox-test-harness`); `contracts/ci/crate-edges.allow.v1.json` was left untouched (`USER-AUTHORIZED-ONLY`).
- `./target/debug/vox ci fan-in-budget` exited `1` solely due to `2` pre-existing regressions (`vox-ast: 3 -> 4`, `vox-bounded-fs: 23 -> 24`).
- `cargo build -p vox-cli` still emits macOS `ld: __eh_frame section too large` on unoptimized debug `vox` monolith linking despite `-Wl,-dead_strip`.
