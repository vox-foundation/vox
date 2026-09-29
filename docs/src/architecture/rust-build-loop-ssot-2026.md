---
title: "Rust build loop: where the time goes and what is allowed to change it"
description: "Measured sinks in the edit, test and commit loop for the orchestrator crates, the levers that keep coverage, the ones that do not, and the gates a crate split must pass first."
category: "Architecture SSOTs"
status: "roadmap"
sort_order: 50
---

# Rust build loop SSOT

An agent or contributor edits a file under `crates/vox-orchestrator/src/models/`, runs the filtered tests, runs the
crate's tests and clippy, and commits. This page records where the wall-clock goes, which changes reduce it without
reducing what is checked, and what a crate split would have to prove before anyone starts it. The tasks that act on it
are in [`2026-09-29-rust-build-loop-and-models-crate.md`](../../superpowers/plans/2026-09-29-rust-build-loop-and-models-crate.md).

## Measured on 2026-09-29 (macOS arm64, load average about 40 from other processes)

| Sink | Evidence | Status |
|---|---|---|
| Pre-commit `fmt-fix` rebuilt `vox-cli` | `cargo run -p vox-cli -- run scripts/fmt.vox` recompiles and relinks a ~275 MB binary whenever a crate `vox-cli` depends on changed: 128 s, 199 s and 1936 s in three consecutive commits | **Fixed** (`ff25d0aef`): `rustfmt --config-path . {staged_files}`, 0.05 s. Pre-push and CI still `rustfmt --check` every target root |
| Two agents in one crate tree | A half-written file from one lane fails the other's `cargo test`; one agy run STOPPED on it | Process rule: pair a Rust task with a TypeScript task, or keep two Rust tasks in disjoint crates |
| Agent and reviewer each run the full gates | The agent's own full `--lib` and clippy runs duplicated the reviewer's | Process rule: agents run only the filtered tests; the reviewer runs the crate suite and clippy once |
| Mutation proofs, one rebuild each | 3 to 5 minutes per rebuild of the orchestrator test binary | Batch mutants that are expected to fail different named tests in one build |
| Feature-set variants of one crate | `target/debug/deps` holds 18 MB, 20 MB and 32 MB `libvox_orchestrator` metadata files and 122 MB rlibs from different feature sets | Unmeasured: see plan Task A2 |
| Machine load | Load average about 40; two `godot` processes near 100% CPU and several `node` runs from other sessions | Outside this repo |

`contracts/ci/crate-build-map.v1.json` (Windows `cargo check`) lists `vox-orchestrator-mcp` (158 s) and
`vox-orchestrator` (119 s) as the two slowest first-party crates; `vox-orchestrator` has `blast_s` 373.
`layers.toml` records that per-crate LoC is not a clean-build lever (91% of workspace compile time is third-party).
That statement is about clean builds; the edit and retest loop is a different cost and has not been measured.

## What a `models` crate split would have to solve

`vox-orchestrator` is 94k lines in 335 files; `models/` is 12.3k. `models/` reaches outward through a small surface
(counted on committed files):

| Needs from the rest of the crate | Where it lives today | Movable? |
|---|---|---|
| `route_policy::{set_test_privacy_override, privacy_allows_model_for_mode, inference_privacy_local_only_from_env, route_policy_exclusion_reason, route_policy_allows_model, is_local_http_provider}` | `route_policy.rs`, 149 lines, itself uses `models::{ModelSpec, ProviderType}` and `vox_actor_runtime` | Yes, into the new crate |
| `usage::{RemainingBudget, LlmUsageKey}` | `usage.rs` 756 lines + `usage_policy.rs` 146 lines; only `serde`, `serde_json`, `vox_secrets` | Yes, into the new crate |
| `catalog::{LiteLLMPricingEntry, ModelCatalog, OpenRouterCatalog, MensCatalog, LiteLLMCatalog, AnthropicDirectCatalog}` | `catalog.rs`, 1660 lines, uses `models::ModelSpec` | Yes, into the new crate |
| `calibration::ContextualBandit`, `catalog_classifier::classify_models` | 406 and 110 lines, leaf modules | Yes, into the new crate |
| `config::CostPreference` (13 uses) | `config/enums.rs`, serde only | Yes, lowered to `vox-orchestrator-types` and re-exported |
| `mode::ClutchProfile` (`mode_select.rs`) | `mode.rs`, 920 lines; the enum is small, `resolve()` returns types from `attention` and `config` | Enum lowers; `resolve()`/`from_label` (15 call sites) need an extension trait or stay via a re-export |
| `types::AgentTask` and `types::TaskId`/`TaskPriority` | `types/tasks.rs`, 1515 lines; its fields reference `reconstruction`, `socrates`, `attachment_manifest`, `observer`, `contract` and `planning` | **No.** This is the real blocker: `AgentTask` cannot leave the crate, so `models` must take a narrow routing view instead |
| `types::FreeRoutingProfile` | `types/routing_profile.rs`, 196 lines; `config_to_routing_profile` uses `OrchestratorConfig` | The enum lowers; the function stays |

Every inherent `impl ModelRegistry` block must live in the crate that defines `ModelRegistry`, so `registry.rs`,
`select.rs`, `mode_select.rs`, `family.rs` and `ranking.rs` move together. External crates that import
`vox_orchestrator::models::…` (about 24 files in `vox-orchestrator-mcp`, 15 in `vox-cli`, 13 in `vox-research-shim`, 3 in
`vox-gui`) keep working through `pub use vox_orchestrator_models as models;`.

## Rules for this work

1. **No behaviour change from a move.** Every move task ends with the same test count passing and the same public paths
   resolving. A moved test keeps its name.
2. **The split is gated by measurement.** Nothing in Part B of the plan starts until the plan's Task A1 records that the
   edit-and-retest loop for `models/` is dominated by the `vox-orchestrator` unit, and that the extracted set is small
   enough to matter (Task A4).
3. **New crate edges are user-authorized.** `crates/vox-orchestrator-models` needs a layer in `layers.toml` and new rows in
   `contracts/ci/crate-edges.allow.v1.json` and the fan-in snapshot. Agents never regenerate those baselines; the plan
   stops and asks (Task B0).
4. **Build-profile changes are adopted only with a before and after.** A change ships if it cuts CPU-seconds in the
   measured loop by at least 20% and loses no test, panic location or lint. Nightly-only options (Cranelift, the
   parallel front end) are out: the toolchain is pinned stable.
5. **Measure CPU-seconds, not wall-clock,** while other processes load the machine.

## Deferred

- Splitting `vox-orchestrator-mcp` (158 s, the slowest first-party crate).
- Moving tests out of `models/` files into `tests/`: the Test-First Policy requires in-file tests for new public functions.
