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
| Feature-set variants of one crate | `target/debug/deps` holds 18 MB, 20 MB and 32 MB `libvox_orchestrator` metadata files and 122 MB rlibs from different feature sets | **Measured 2026-10-03: no steady-state saving** (below); a one-time cold-cache cost only |
| Machine load | Load average about 40; two `godot` processes near 100% CPU and several `node` runs from other sessions | Outside this repo |

`contracts/ci/crate-build-map.v1.json` (Windows `cargo check`) lists `vox-orchestrator-mcp` (158 s) and
`vox-orchestrator` (119 s) as the two slowest first-party crates; `vox-orchestrator` has `blast_s` 373.
`layers.toml` records that per-crate LoC is not a clean-build lever (91% of workspace compile time is third-party).
That statement is about clean builds; the edit and retest loop is measured next.

## Measured on 2026-10-03 (macOS arm64, load average about 3, a worktree on `main`)

Method: `resource.getrusage(RUSAGE_CHILDREN)` around the whole command tree (user + sys CPU-seconds), because `time` is not
in the contributor shell allowlist. The method was checked against a known workload: `openssl speed md5`, a CPU-bound
grandchild, reported 18.0 CPU-s for 18.1 s wall. Every measured build is incremental: a comment is appended to one file, the
command is timed, and the file is restored from `HEAD`.

**Baseline** (`cargo test --package vox-orchestrator --lib --no-run`, three touches each):

| Touched file | CPU-seconds (user + sys) | Median | Of which sys |
|---|---|---|---|
| `models/tiering.rs` | 35.6, 40.0, 37.1 | **37.1** (`T_models`) | about 30 |
| `circuit_breaker.rs` (outside `models/`) | 32.0, 31.6, 29.8 | **31.6** (`T_other`) | about 25 |

Touching `models/` costs 5.5 CPU-s (15%) more than touching any other file, so a `models` crate could remove at most that
much from a `models/` edit. The loop is fixed cost: relinking the test binary, not recompiling the 12k lines.

**One feature set per verification run** (after one touch of `models/tiering.rs`; both forms warmed first):

| Form | Commands | CPU-seconds |
|---|---|---|
| Separate (`U_sep`) | `test` orchestrator 19.5, `test` mcp 40.6, `clippy` both 817.4 | **877.5** |
| One invocation (`U_one`) | `test` both 82.8, `clippy` both 833.0 | **915.8** |

`U_one / U_sep` = 1.04, against the rule's 0.8: **not adopted as a saving**. Once both forms are warm, both artifact
variants coexist and neither pays again; the combined `test` is one larger compile, and `clippy` (identical in both) is
about 93% of either total. The unified form still avoids the one-time `vox-corpus` feature-variant rebuild on a cold
cache, which is why the driver notes keep recommending it, but it is not a steady-state CPU saving.

**Test-profile debug level** (`CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0`, three touches of `models/tiering.rs`
after one 578 CPU-s first-party rebuild): 36.5, 40.4, 31.7, median **36.5** against 37.1, a 1.6% change. The rule needs
20%: **`[profile.test] debug = 0` is not adopted** and nothing in `Cargo.toml` changes.

**Decision (Task A4): NO-GO.** `T_models` is 37.1 CPU-s against the 150 CPU-s the rule requires, and a file outside
`models/` costs 31.6. Extracting `models/` into `vox-orchestrator-models` would save about 5.5 CPU-s per `models/` edit and
add a crate, a layer assignment and new edges that need the owner's authorization. Part B is not started.

**What the numbers do show: system time dominates everything.** 80 to 92% of every measured CPU-second is kernel time,
not compiler work: a touch is about 30 of 37 sys, a cold `debug = 0` rebuild 454 of 578, the incremental two-crate
`clippy` 750 of 817, and the first `clippy` of those crates 1422 of 1637 (user 215). Rust builds on this machine are
normally nearer 0.2 to 0.5 sys per user second; here it is 5 to 7. The cause is not established. The leading suspect is
macOS assessing every freshly written executable or dylib (each build script and proc-macro `dylib` is new after a
rebuild), with `syspolicyd` the top CPU process on the machine at about 38% averaged over two days. If that is right,
the persistent lever is outside the repository: allow the app that launches `cargo` (the terminal, or the Claude app
for agent runs) under System Settings, Privacy and Security, Developer Tools. That is a security setting, so it was not
changed here; a before and after with the Part A loop above would confirm or refute it in about ten minutes.

### Part B authorization (Task B0)

**B0 approved 2026-10-03 by the owner, in chat ("part b build loop authorized"), overriding the NO-GO above.** The
approval covers exactly: a `layers.toml` row `vox-orchestrator-models = { layer = 3 }`; the new edge
`vox-orchestrator -> vox-orchestrator-models`; the edges `vox-orchestrator-models -> {vox-orchestrator-types, vox-config,
vox-db, vox-secrets, vox-actor-runtime, vox-telemetry, vox-mesh-transport, vox-mesh-policy, vox-mesh-types, vox-bounded-fs,
vox-repository}` as Task B4's `cargo check` confirms them (an edge outside that list stops the work); the matching
fan-in baseline rows; and the `where-things-live.md` row. The measurement still predicts a small saving, so Task B6
records `T_models_after` and `T_other_after` as evidence rather than treating a miss as a revert order: the extraction is
kept unless it makes `T_other` worse than 1.2x.

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
   enough to matter (Task A4). **Recorded 2026-10-03: NO-GO** (`T_models` 37.1 CPU-s against 150), so Part B has not
   started and the gate stays closed until a re-measurement says otherwise.
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
