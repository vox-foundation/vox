---
title: "Capability Profiles 05 — extract vox-attention and vox-models from vox-orchestrator"
description: "Behaviour-preserving extraction of the attention/budget logic (vox-attention, L2) and the model registry/selection/catalogs (vox-models, L3) out of vox-orchestrator, with one-step re-export shims, user-authorized crate edges, and full move maps."
category: "architecture"
status: "proposed"
date: 2026-10-08
---

# Model and Attention Crate Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `crates/vox-orchestrator/src/{attention,budget}/`, `budget_gate.rs` and `fatigue_monitor.rs` into a new
L2 crate `vox-attention`, and `crates/vox-orchestrator/src/models/` plus `route_policy.rs`, `catalog.rs`,
`catalog_classifier.rs`, `catalog_live.rs` and `build.rs` into a new L3 crate `vox-models`. Behaviour does not change.
This lets plan 03 boot the `attention` and `model_registry` subsystems without the whole orchestrator.

**Architecture:** Pure-data enums that both new crates need, and that live in `vox-orchestrator` today, move down
first into `vox-orchestrator-types` (L0). These are `TaskPriority`, `PricingSource`, and the clutch primitives from
`mode.rs`. That step adds no edges. Each crate is then extracted with `git mv`, and `vox-orchestrator` keeps a
`pub use` shim for exactly one task. The next task rewrites every call site to the new crate path, deletes the shim,
and proves with `rg` that no old path remains. `vox-attention` stays free of any database: `BudgetManager` reaches
persisted state through a new `BudgetSource` trait, which `vox-orchestrator` implements over VoxDB as
`VoxDbBudgetSource`. Every new crate edge goes to the user for authorization (Task 3) before the first commit that
would fail `vox ci crate-edges`.

**Tech Stack:** Rust 2024 workspace, Cargo features, `async-trait`, `cargo nextest`, `vox ci crate-edges` /
`fan-in-budget` / `affected-crates` / `ssot-drift`, `cargo run -p vox-arch-check`, `vox graph crate-map`.

## Global Constraints

- TDD: failing test first; every new `pub fn` has a test in the same file (`skeleton/untested-pub-api`).
- Security guards are mutation-verified: break the guard, see the test fail, restore, with a
  before/after presence check (AGENTS.md §PR & Review Discipline).
- No new workspace crate edges in plans 01–04, 06–07. Plan 05's new edges are **proposed for user
  authorization** and are not added to `contracts/ci/crate-edges.allow.v1.json` by an agent.
- Secrets via `vox_secrets::resolve_secret`; no `std::env::var` for tokens. Non-secret `VOX_*`
  config via existing `vox-config` patterns.
- Never `cargo fmt --all`; format dirty files with `vox run scripts/fmt.vox`.
- Generated files (`tool-registry.canonical.yaml`, `*.generated.md`) are regenerated, never hand-edited.
- New `.md` under `docs/src/` has frontmatter; lint with `cargo run -p vox-doc-pipeline -- --lint-only --paths <path relative to docs/src>`.
- Per-crate checks before each commit: `cargo check -p <crate>`, `cargo clippy -p <crate> --all-targets -- -D warnings`, targeted `cargo nextest run -p <crate> <filter>`.
- `dev` profile must reproduce today's behaviour exactly (back-compat tests in plans 02 and 03).

Plan-specific constraints:

- Work only in `/Users/brbrainerd/dev/vox/.worktrees/capability-profiles` (branch `feat/capability-profiles`). Never
  edit `/Users/brbrainerd/dev/vox` (the main checkout).
- `contracts/ci/crate-edges.allow.v1.json` and `contracts/ci/fan-in-snapshot.v1.json` are **user-edited only**.
  Never regenerate either to admit an edge this plan introduces. `vox ci crate-edges --tighten` (removal-only) is allowed.
- Moves use `git mv`. After every move, check that each moved item is defined exactly once (AGENTS.md §Perennial Bug
  Patterns, "Move + reformat = a duplicate definition").
- `tracing` targets such as `target: "vox_orchestrator::model_route"` are log-filter names, not Rust paths. Never rewrite them.
- Every vestigial shim carries `// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="capability-profiles-05" canonical="<new crate>"`.

---

## Measured inventory (recorded 2026-10-08 on `8519ca7a1`, before the model-bench WIP lands)

Task 0 recomputes all of this with commands. If the recomputed numbers differ, update the move lists and tables
below before you continue.

| | `vox-attention` | `vox-models` |
|---|---|---|
| Files moved | 11 (`attention/` 5, `budget/` 4, `budget_gate.rs`, `fatigue_monitor.rs`) | 31 `.rs` (`models/` 27, `route_policy.rs`, `catalog.rs`, `catalog_classifier.rs`, `catalog_live.rs`) + `build.rs` |
| Lines moved | 3,768 (attention 1,970 · budget 1,353 · budget_gate 349 · fatigue_monitor 96) | 15,494 `.rs` (models 13,367 · catalogs+route_policy 2,127) + 574 `build.rs` |
| Unit tests moving | 67 | 322 |
| Inbound files (outside the moved set) | `vox-orchestrator` 25 · `vox-orchestrator-mcp` 17 (src + tests) · `vox-cli` 1 | `vox-orchestrator` 21 · `vox-orchestrator-mcp` 37 · `vox-cli` 16 · `vox-research-shim` 13 · `vox-gui` 5 · `vox-orchestrator-d` 1 · `vox-integration-tests` 1 (dev) |
| Model-bench WIP adds | none | `adaptation.rs` 176, `bench_evidence.rs` 148, `leaderboard.rs` 162 (new), ~+400 net in 9 modified `models/` files |

### Outbound couplings and their resolution

`vox-attention` (target L2: may depend only on L0–L2):

| Coupling (today) | Used by | Resolution |
|---|---|---|
| `crate::types::{AgentId, TaskId, now_unix_ms}` | attention/, budget/ | Already `vox_orchestrator_types` (re-exported by `types/ids.rs`); rewrite the path |
| `crate::types::TaskPriority` | `attention/budget.rs` (`ActionDescriptor.priority`), tests | **Lower** to `vox-orchestrator-types` (Task 2) |
| `crate::models::spec::PricingSource` | `budget/mod.rs` `record_cost` | **Lower** to `vox-orchestrator-types` (Task 2); `vox_models::spec` re-exports it |
| `crate::mode::BudgetAggressiveness` | `budget_gate.rs` | **Lower** clutch primitives to `vox_orchestrator_types::clutch` (Task 2) |
| `crate::fatigue_monitor` (96 lines, std only, used only by `budget/`) | budget/ | **Move** into `vox-attention` |
| `crate::sync_lock` (= `vox_orchestrator_queue::sync_lock`, L3, 20 lines) | budget/ | **Defactor** a private copy (`// vox:defactored-from vox-orchestrator-queue 2026-10-08`) |
| `crate::services::persistence_obs::log_persistence_failure` (7 lines) | `budget/persistence.rs` | **Defactor** a private fn with the same tracing target |
| `vox_db::VoxDb` (`get/set_user_preference`, `query_tool_latency`, `record_exec_timeout`, `record_exec_time`) | `budget/mod.rs`, `budget/persistence.rs` | **Invert** through the new `BudgetSource` trait; `vox-orchestrator` implements `VoxDbBudgetSource` |
| `vox_db::research_metrics_contract::METRIC_TYPE_BUDGET_DECISION` | `budget_gate.rs` | Same constant is `vox_telemetry::METRIC_TYPE_BUDGET_DECISION` (vox-db re-exports it); rewrite the path |
| `vox_crypto::fast_hash`, `chrono`, `vox_telemetry::record_default_decision!` | budget/ | Direct deps (L1) |
| Intra-doc links to `crate::{tier_cascade, gate, attention_tracker, mode::ResolvedRisk}` | doc comments | Rewrite to plain code spans (rustdoc `-D warnings`) |

`vox-models` (target L3):

| Coupling (today) | Used by | Resolution |
|---|---|---|
| `crate::config::CostPreference` (16) | models/ | Already `vox_orchestrator_types::CostPreference`; rewrite the path |
| `crate::types::FreeRoutingProfile` (7) | `policy.rs` | Already `vox_orchestrator_types::RoutingProfile`; rewrite the path |
| `crate::types::TaskCategory` (8) | spec, registry, tests | It *is* `crate::models::generated::TaskCategory` re-exported; rewrite to `crate::TaskCategory`. **Does not move to vox-orchestrator-types** (see Deviations) |
| `crate::usage::{RemainingBudget, LlmUsageKey}`, `crate::calibration::ContextualBandit` | scoring, spec, registry | Already in `vox-orchestrator-models`; rewrite to `vox_orchestrator_models::…` |
| `crate::route_policy::*` (149 lines; uses `models` + `vox_actor_runtime::route_capability_policy`) | select, ranking, registry, health, reference | **Move** into `vox-models` |
| `crate::catalog::*`, `catalog_classifier`, `catalog_live` (depend on `models`) | registry; outside: `orchestrator/catalog_refresh.rs`, `vox-gui` | **Move** into `vox-models` |
| `crate::mode::ClutchProfile` (+`resolve()`) | `health.rs`, `mode_select.rs`, WIP `leaderboard.rs` | **Lower** clutch primitives (Task 2), not the `exclude_elite: bool` rewrite the build-loop plan proposed. Health and the WIP leaderboard iterate detents and call `resolve()`, so a bool does not cover them |
| `crate::types::{AgentTask, TaskId, TaskPriority}` in **non-test** code: `health.rs:72` probe; WIP `leaderboard.rs:64` probe | | Build the `RoutingTask` literal directly. Task 1 pins `RoutingTask::from(&probe) == literal` |
| Same, in tests: `routing_task.rs` (2), `ranking.rs:601`, `mode_select.rs:193` | | Rewrite the fixtures to build a `RoutingTask`. The conversion test moves to `types/tasks.rs` |
| `impl From<&AgentTask> for RoutingTask` | `types/tasks.rs:1128` | Stays in `vox-orchestrator`. Legal under the orphan rule: `&AgentTask` is local |
| `vox_populi::mens::hardware::macos_metal::probe_metal` (1 call, `auto_select.rs:122`, `cfg(feature = "runtime")`) | | **Option A (default):** optional edge `vox-models → vox-populi` under `runtime`, mirroring today. **Option B:** inject the probe and drop the edge. Task 3 asks the user to choose |
| `vox_http_client` (14 uses, catalogs) | | Direct dep (L1). `vox-orchestrator`'s own edge has 0 remaining uses and is removed in the same commit |
| `vox_mesh_policy` (4) | | Direct dep. `vox-orchestrator`'s edge has 0 remaining uses and is removed |
| `vox_mesh_transport`, `iroh` (`cfg(feature = "populi-transport")`) | mesh_directory, registry, scoring | Optional deps under the `populi-transport` feature |
| `pub(crate) fn mesh_directory::vox_dir` | used by `a2a/mesh_relay.rs` | Widen to `pub` (+ test) |
| `include_str!` relative paths (`pareto.rs:408-409`, `tests.rs:193`, `spec.rs:352`) | | One directory shallower after flattening: drop one `../` |
| Path readers: `vox-cli-ci/src/model_routing_check.rs:76` (`scoring.rs`), `vox-drift-check/src/rules/timeout_literal.rs:79` (`catalog.rs`), `vox-cli/src/commands/ci/command_compliance/validators.rs:504` (`spec.rs`) | CI gates | Repoint to new paths in the same commit |
| Path-keyed baselines: `contracts/toestub/weak-test-baseline.v1.json` (6), `silent-drop-baseline.v1.json` (4), `contracts/config/config-hygiene-baseline.txt` (3), `contracts/telemetry/emit-site-inventory.csv` (4), `default-decision-sites.csv` (budget rows) | | Rename keys only (no new entries) |

### Deviations from the spec (§5) — Task 8 records them in `00-contracts.md`

1. **`TaskCategory` stays in `vox-models`.** `vox-orchestrator-types/build.rs` already generates an enum with the same
   name, but it is a different wire type. That one is `#[serde(rename_all = "snake_case")]`, has lowercase `Display`
   and no `Ord`, and puts `General` first. The orchestrator's enum serializes PascalCase (`"CodeGen"`), has PascalCase
   `Display`, derives `PartialOrd, Ord`, and puts `General` 8th. Unifying them would change persisted JSON and ordering.
   `vox-models/tests/public_surface.rs` pins the current wire format.
2. **`ModelSpec` stays in `vox-models`.** It drags `ModelCapabilities`, `ProviderType`, `StrengthTag`, `ModelTier`,
   `QualityPrior` and `LlmUsageKey` with it, and no consumer below L3 needs it.
3. **`CostPreference` and `RemainingBudget` already moved** (build-loop B1/B3, PR #598). Nothing to do.
4. **Lowered instead:** `TaskPriority`, `PricingSource`, and the clutch primitives (`Modalities`, `QualityLevel`,
   `TierProfile`, `InferenceConfig`, `BudgetAggressiveness`, `ResolvedClutch`, `ClutchProfile`).
5. **`vox-models` dependencies** are the measured list in Task 3, not the spec's guess. `vox-llm-config` and
   `vox-llm-egress` are unused.
6. **`vox-research-shim` keeps `vox-orchestrator`.** `selection/` moves to `vox-models` and
   `vox_orchestrator_types::clutch`. These remain: `registry_model_resolve::*` (re-exported by `selection/mod.rs`;
   uses `routing::ModelSelectionEngine` and `privacy_router`), `types::RoutingProfile` (the 7-variant orchestrator
   intent enum), `services::embeddings::EmbeddingService`, and `MemoryConfig`.
7. This plan supersedes build-loop Task B4 (`docs/superpowers/plans/2026-09-29-rust-build-loop-and-models-crate.md`).
   That task targeted `vox-orchestrator-models`; the spec names a new crate. Task 8 updates the build-loop SSOT's
   "Part B status".

---

## File Structure

### Lowered into `vox-orchestrator-types` (Task 2, no new edges)

| From | To |
|---|---|
| `crates/vox-orchestrator/src/types/tasks.rs` — `enum TaskPriority` + `impl TaskPriority` + `impl fmt::Display for TaskPriority` (lines 42–75) | `crates/vox-orchestrator-types/src/agent_types/task_priority.rs` (new) |
| `crates/vox-orchestrator/src/models/spec.rs` — `enum PricingSource` (lines 12–33) | `crates/vox-orchestrator-types/src/pricing.rs` (new) |
| `crates/vox-orchestrator/src/mode.rs` — `Modalities` … `impl ClutchProfile` (lines 8–177) + `mod semcov_behavior_tests` + `mod clutch_tests` | `crates/vox-orchestrator-types/src/clutch.rs` (new) |

### `vox-attention` (Task 4)

| Old path | New path |
|---|---|
| `crates/vox-orchestrator/src/attention/mod.rs` | `crates/vox-attention/src/attention/mod.rs` |
| `crates/vox-orchestrator/src/attention/budget.rs` | `crates/vox-attention/src/attention/budget.rs` |
| `crates/vox-orchestrator/src/attention/calibrator.rs` | `crates/vox-attention/src/attention/calibrator.rs` |
| `crates/vox-orchestrator/src/attention/interruption_policy.rs` | `crates/vox-attention/src/attention/interruption_policy.rs` |
| `crates/vox-orchestrator/src/attention/routing.rs` | `crates/vox-attention/src/attention/routing.rs` |
| `crates/vox-orchestrator/src/budget/mod.rs` | `crates/vox-attention/src/budget/mod.rs` |
| `crates/vox-orchestrator/src/budget/persistence.rs` | `crates/vox-attention/src/budget/persistence.rs` (rewritten onto `BudgetSource`) |
| `crates/vox-orchestrator/src/budget/session.rs` | `crates/vox-attention/src/budget/session.rs` |
| `crates/vox-orchestrator/src/budget/tracker.rs` | `crates/vox-attention/src/budget/tracker.rs` |
| `crates/vox-orchestrator/src/budget_gate.rs` | `crates/vox-attention/src/budget_gate.rs` |
| `crates/vox-orchestrator/src/fatigue_monitor.rs` | `crates/vox-attention/src/fatigue_monitor.rs` |
| — | `crates/vox-attention/{Cargo.toml, src/lib.rs, src/source.rs, src/sync_lock.rs, tests/budget_source.rs}` (new) |
| — | `crates/vox-orchestrator/src/budget_source.rs` (new: `VoxDbBudgetSource`) |

`crates/vox-orchestrator/src/attention_tracker.rs` and `services/attention_calibration.rs` stay. They are VoxDB
persistence and services that consume `vox-attention`.

### `vox-models` (Task 6) — `models/` is flattened to the crate root

| Old path | New path |
|---|---|
| `crates/vox-orchestrator/src/models/mod.rs` | `crates/vox-models/src/lib.rs` |
| `crates/vox-orchestrator/src/models/<f>.rs` for each `<f>` in: `admission auto_select autonomic cost_tier discovery_pipeline family generated health key_guard mesh_directory mode_select pareto policy prompt_profiles provenance ranking reference registry routing_table routing_task scoring select spec tests tiering vram` (+ WIP `adaptation bench_evidence leaderboard`) | `crates/vox-models/src/<f>.rs` |
| `crates/vox-orchestrator/src/route_policy.rs` | `crates/vox-models/src/route_policy.rs` |
| `crates/vox-orchestrator/src/catalog.rs` | `crates/vox-models/src/catalog.rs` |
| `crates/vox-orchestrator/src/catalog_classifier.rs` | `crates/vox-models/src/catalog_classifier.rs` |
| `crates/vox-orchestrator/src/catalog_live.rs` | `crates/vox-models/src/catalog_live.rs` |
| `crates/vox-orchestrator/build.rs` (generates only `models/generated.rs`) | `crates/vox-models/build.rs` |
| — | `crates/vox-models/{Cargo.toml, tests/public_surface.rs}` (new) |

Module path mapping for call sites: `vox_orchestrator::models::X` becomes `vox_models::X`.
`vox_orchestrator::{route_policy, catalog, catalog_classifier, catalog_live}::X` becomes `vox_models::{…}::X`.
Test names lose their `models::` prefix (for example `models::select::tests::foo` becomes `select::tests::foo`).

### Registration and references

- `Cargo.toml` (root) `[workspace.dependencies]`: `vox-attention`, `vox-models`. Members are a `crates/*` glob, so
  they need no entry.
- `docs/src/architecture/layers.toml`: `vox-attention = { layer = 2 }`, `vox-models = { layer = 3 }`.
- `docs/src/architecture/where-things-live.md`: two crate rows and repointed concept rows.
- `contracts/ci/crate-graph.v1.json`: regenerated mirror (`vox ci affected-crates --regen`).

---

### Task 0: Gate — model-bench WIP merged; inventory re-run; baselines captured

**Files:** none modified except `target/cp05/*` (scratch, git-ignored).

**Interfaces:** Produces `target/cp05/{attention,models}-move-list.txt`, `*-outbound.txt`, `*-inbound.txt`,
`baseline-*.txt` and `crate-map-before.txt`, all consumed by later tasks.

- [ ] **Step 1: WIP gate.** The other session's model-bench work must be committed and merged into this branch. Check:

```bash
cd /Users/brbrainerd/dev/vox/.worktrees/capability-profiles
git -C /Users/brbrainerd/dev/vox status --porcelain -- crates/vox-orchestrator/src/models crates/vox-cli/src/commands/model crates/vox-db/src/store/ops_model_bench.rs crates/vox-gui/src/commands
git log --all --oneline -- crates/vox-orchestrator/src/models/leaderboard.rs | head -3
```

  Expected: the first command prints nothing, and the second prints at least one commit. If either fails, **STOP**:
  the WIP has not landed. Otherwise merge the branch that contains it:

```bash
WIP_SHA=$(git log --all --format=%H -1 -- crates/vox-orchestrator/src/models/leaderboard.rs)
git branch -a --contains "$WIP_SHA"
git merge --no-ff <branch printed above>   # or rebase onto it if the branch is linear with ours
git ls-files crates/vox-orchestrator/src/models/leaderboard.rs   # must print the path
```

- [ ] **Step 2: B2 regression check.** The WIP branch forked at `b120286c5`, 21 commits before `origin/main`, so it
  predates PR #598. That PR made `models/` route by `RoutingTask` instead of `AgentTask`. On that branch,
  `mode_select.rs` takes `task: &AgentTask` (lines 29, 44, 98). After the merge, run:

```bash
rg -n 'task: &AgentTask|Option<&AgentTask>|&crate::types::AgentTask' crates/vox-orchestrator/src/models
rg -n 'AgentTask' crates/vox-orchestrator/src/models | rg -v '#\[cfg\(test\)\]|mod tests|^\S+:\s*//'
```

  Expected for the first command: no output. Expected for the second: only `health.rs` (probe, ~line 72),
  `leaderboard.rs` (probe, ~line 64) and test modules (`routing_task.rs`, `ranking.rs`, `mode_select.rs`). If any
  public signature takes `AgentTask`, **STOP** and report. The merge reverted B2, and it must be re-resolved to
  `RoutingTask` (callers pass `&RoutingTask::from(&task)`) before this plan proceeds.

- [ ] **Step 3: Recompute the move lists.**

```bash
mkdir -p target/cp05
git ls-files crates/vox-orchestrator/src/attention crates/vox-orchestrator/src/budget \
  crates/vox-orchestrator/src/budget_gate.rs crates/vox-orchestrator/src/fatigue_monitor.rs > target/cp05/attention-move-list.txt
git ls-files crates/vox-orchestrator/src/models crates/vox-orchestrator/src/route_policy.rs \
  crates/vox-orchestrator/src/catalog.rs crates/vox-orchestrator/src/catalog_classifier.rs \
  crates/vox-orchestrator/src/catalog_live.rs > target/cp05/models-move-list.txt
wc -l < target/cp05/attention-move-list.txt; wc -l < target/cp05/models-move-list.txt
xargs wc -l < target/cp05/attention-move-list.txt | tail -1; xargs wc -l < target/cp05/models-move-list.txt | tail -1
```

  Expected: 11 attention files (~3,768 lines). Expected for models: 34 files (31 + 3 WIP), ~16.4k lines. Any other
  count means the File Structure tables need updating first.

- [ ] **Step 4: Recompute outbound couplings.**

```bash
xargs rg -o --no-filename 'crate::[a-z_]+' < target/cp05/attention-move-list.txt | sort | uniq -c \
  | rg -v 'crate::(attention|budget|budget_gate|fatigue_monitor)$' > target/cp05/attention-outbound.txt
xargs rg -o --no-filename 'crate::[a-z_]+' < target/cp05/models-move-list.txt | sort | uniq -c \
  | rg -v 'crate::(models|route_policy|catalog|catalog_classifier|catalog_live)$' > target/cp05/models-outbound.txt
cat target/cp05/attention-outbound.txt target/cp05/models-outbound.txt
```

  Expected attention modules: `crate::{fatigue_monitor? (internal), mode, models, services, sync_lock, types, gate,
  tier_cascade, attention_tracker}`. The last three appear in doc comments only. Expected models modules:
  `crate::{calibration, catalog*, config, mode, route_policy, types, usage}`. **Any other module name is an unplanned
  coupling: STOP** and add a row to the Outbound table with a resolution.

- [ ] **Step 5: Recompute inbound references and external crate usage.**

```bash
rg -l '\bvox_orchestrator::(attention|budget|budget_gate|fatigue_monitor)\b|\bvox_orchestrator::\{[^}]*\b(BudgetManager|AgentBudgetAllocation|BudgetSignal|ApprovalTier|TrustTier|AttentionBudget|AttentionEvent|AttentionEventType|AgentTrustScore|ActionDescriptor|NasaTlxWeights|TierGateConfig|FocusDepth|ApprovalOutcome|InterruptionChannel|InterruptionDecision|InterruptionSignals|classify_tier|compute_attention_cost_ms|decision_entropy_bits|evaluate_interruption|scaled_interrupt_cost_ms)\b' crates --glob '*.rs' > target/cp05/attention-inbound.txt
rg -l '\bvox_orchestrator::(models|route_policy|catalog|catalog_classifier|catalog_live)\b' crates --glob '*.rs' > target/cp05/models-inbound.txt
cut -d/ -f2 target/cp05/attention-inbound.txt | sort | uniq -c
cut -d/ -f2 target/cp05/models-inbound.txt | sort | uniq -c
xargs rg -o --no-filename '\bvox_[a-z_]+::' < target/cp05/models-move-list.txt | sort | uniq -c
```

  Expected attention consumers: `vox-orchestrator-mcp` and `vox-cli`. Expected models consumers: `vox-cli`,
  `vox-gui`, `vox-integration-tests`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, `vox-research-shim`. Doc-comment
  mentions in `vox-actor-runtime`, `vox-db`, `vox-populi` and `vox-orchestrator-models` are not dependents. Expected
  external `vox_*` crates: exactly the 14 in Task 3's list. A new consumer crate or a new `vox_*` crate means Task 3's
  edge list must be updated before you present it.

- [ ] **Step 6: Baselines.** Record the current pass/fail state of the behaviour-preservation matrix. Some tests may
  already be quarantined on `main` (nextest profile `ci-gate`); the after-run is compared against this, not against
  "all green".

```bash
cargo nextest run -p vox-orchestrator 2>&1 | tee target/cp05/baseline-orchestrator.txt | tail -3
cargo nextest run -p vox-orchestrator-mcp -E 'test(/llm_bridge|model_route_policy|chat_model_resolve|visus_review|vram_probe|mcp_context|attention_policy|pending_approvals|budget/) | binary(pending_approvals_tests) | binary(hitl_rehydrate_on_restart_tests) | binary(daemon_extra_tests) | binary(autonomous_tool_dispatch_tests)' 2>&1 | tee target/cp05/baseline-mcp.txt | tail -3
cargo nextest run -p vox-research-shim 2>&1 | tee target/cp05/baseline-shim.txt | tail -3
cargo nextest run -p vox-cli -E 'test(/commands::model::|commands::safety|commands::chat|orchestration_audit|audit_route|model_catalog/)' 2>&1 | tee target/cp05/baseline-cli.txt | tail -3
cargo nextest run -p vox-integration-tests -E 'binary(chat_harness_regression_test)' 2>&1 | tee target/cp05/baseline-integration.txt | tail -3
cargo nextest run -p vox-cli-ci -E 'test(/model_routing_check|crate_edges/)' 2>&1 | tee target/cp05/baseline-cli-ci.txt | tail -3
cargo nextest list -p vox-orchestrator --message-format oneline 2>/dev/null | rg -c ' (models|route_policy|catalog|catalog_classifier|catalog_live)::' > target/cp05/count-models-tests-before.txt
cargo nextest list -p vox-orchestrator --message-format oneline 2>/dev/null | rg -c ' (attention|budget|budget_gate|fatigue_monitor)::' > target/cp05/count-attention-tests-before.txt
cat target/cp05/count-*-before.txt
```

  Expected counts: ~322 models-side and ~67 attention-side, plus whatever tests the WIP adds. `vox-gui` is checked
  separately in Task 7. A fresh worktree needs `vox run scripts/gui-build.vox` and `pnpm --dir crates/vox-gui/ui
  install && pnpm --dir crates/vox-gui/ui build` first (AGENTS.md "vox-gui sidecar missing in a fresh worktree").

- [ ] **Step 7: Blast-radius baseline.**

```bash
vox graph status
vox graph refresh --auto
vox graph crate-map > target/cp05/crate-map-before.txt
cargo run -q -p vox-arch-check 2>&1 | tail -5 | tee target/cp05/arch-before.txt
vox ci crate-edges 2>&1 | tail -3
```

  Expected: `vox ci crate-edges` passes on the untouched branch. If it does not, fix or report the pre-existing
  failure first. A later failure must be attributable to this plan.

- [ ] **Step 8:** No commit. If Steps 3–5 changed any table above, rewrite this plan file and commit only that:
  `git add docs/superpowers/plans/2026-10-08-capability-profiles/05-crate-extraction.md && git commit -m "docs(plan): refresh plan 05 inventory after model-bench merge"`.

---

### Task 1: Pin current behaviour before anything moves

**Files:** Modify `crates/vox-orchestrator/src/types/tasks.rs` (tests), `crates/vox-orchestrator/src/budget/persistence.rs`
(tests), `crates/vox-orchestrator/src/lib.rs` (tests).

**Interfaces:** Consumes nothing. Produces pinning tests that must pass before and after every later task (adjusted
only for path).

- [ ] **Step 1: RoutingTask-equivalence pins.** Add to the existing `mod tests` in `types/tasks.rs`. This proves
  that the four `AgentTask` probes and fixtures in `models/` equal plain `RoutingTask` literals, which is what Task 6
  rewrites them to.

```rust
    #[test]
    fn routing_probe_tasks_equal_plain_routing_task_literals() {
        for (desc, category, complexity) in [
            ("routing health probe", TaskCategory::CodeGen, 10u8),
            ("mode routing table", TaskCategory::Research, 5),
            ("hard codegen", TaskCategory::CodeGen, 10),
            ("test task", TaskCategory::CodeGen, 8),
        ] {
            let mut task = AgentTask::new(TaskId(0), desc, TaskPriority::Normal, vec![]);
            task.task_category = category;
            task.estimated_complexity = complexity;
            assert_eq!(
                crate::models::RoutingTask::from(&task),
                crate::models::RoutingTask {
                    category,
                    complexity,
                    ..Default::default()
                },
                "{desc}"
            );
        }
    }

    #[test]
    fn budgeted_task_converts_to_routing_task_with_max_cost() {
        let mut task = AgentTask::new(TaskId(1), "budgeted", TaskPriority::Normal, vec![]);
        task.budget = Some(Budget {
            max_cost_usd: Some(0.001),
            max_latency_ms: None,
        });
        let rt = crate::models::RoutingTask::from(&task);
        assert_eq!(rt.max_cost_usd, Some(0.001));
        assert_eq!(rt.category, TaskCategory::default());
        assert_eq!(rt.complexity, 5);
        assert!(rt.research_hints.is_empty() && rt.tool_hints.is_empty());
    }

    #[test]
    fn task_priority_wire_format_is_stable() {
        assert_eq!(serde_json::to_string(&TaskPriority::Urgent).unwrap(), "\"Urgent\"");
        assert_eq!(TaskPriority::Background.to_string(), "background");
        assert_eq!(TaskPriority::from_u8(2), TaskPriority::Urgent);
        assert_eq!(TaskPriority::from_u8(9), TaskPriority::Normal);
    }
```

- [ ] **Step 2: Budget persistence pins over a real VoxDB.** Add a test module at the end of `budget/persistence.rs`.
  Task 4 moves it, unchanged except for construction, into `budget_source.rs`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::BudgetSignal;

    #[tokio::test]
    async fn allocation_persists_through_voxdb() {
        let db = Arc::new(vox_db::VoxDb::open_memory().await.expect("mem db"));
        let writer = BudgetManager::new(Some(db.clone()));
        writer
            .set_and_persist_allocation(AgentId(7), AgentBudgetAllocation::new(1234, 2.5))
            .await;
        let reader = BudgetManager::new(Some(db));
        reader.load_user_configured_budget(AgentId(7)).await;
        let alloc = reader
            .check_budget(AgentId(7))
            .and_then(|b| b.allocation)
            .expect("allocation restored from VoxDB");
        assert_eq!(alloc.max_tokens, 1234);
        assert_eq!(alloc.max_cost_usd, 2.5);
    }

    #[tokio::test]
    async fn tool_latency_without_samples_is_unknown() {
        let db = Arc::new(vox_db::VoxDb::open_memory().await.expect("mem db"));
        let bm = BudgetManager::new(Some(db));
        let sig = bm
            .query_tool_latency_signal("mcp:vox_build_crate", "repo", 0.2, 7, 1.5, 30_000)
            .await;
        assert_eq!(
            sig,
            BudgetSignal::ToolLatencyUnknown {
                tool_key: "mcp:vox_build_crate".into(),
                default_budget_ms: 30_000,
            }
        );
    }

    #[tokio::test]
    async fn tool_latency_without_source_is_unknown() {
        let bm = BudgetManager::new(None);
        let sig = bm.query_tool_latency_signal("t", "r", 0.2, 7, 1.5, 9).await;
        assert_eq!(
            sig,
            BudgetSignal::ToolLatencyUnknown {
                tool_key: "t".into(),
                default_budget_ms: 9,
            }
        );
    }
}
```

- [ ] **Step 3: Old-path compile pins.** Add to the existing `#[cfg(test)] mod tests` in `lib.rs` (next to
  `reexport_paths_resolve`). Tasks 4/6 must keep these green through the shims. Tasks 5/7 delete them together with
  the shims, and the new crates' own tests replace them.

```rust
    #[test]
    fn extraction_old_paths_resolve() {
        let _ = crate::models::ModelRegistry::default();
        let _ = crate::route_policy::is_local_http_provider(&crate::models::ProviderType::Ollama);
        let _: Option<crate::catalog::OpenRouterCatalog> = None;
        let _: Option<crate::attention::ApprovalTier> = None;
        let _ = crate::budget::BudgetManager::new(None);
        let _: Option<crate::budget_gate::OrchestratorBudgetGate> = None;
        let _: Option<crate::fatigue_monitor::FatigueMonitor> = None;
        let _: Option<crate::BudgetManager> = None;
    }
```

- [ ] **Step 4: Run. These pin today's behaviour, so they pass immediately.** A failure means the pin is wrong.
  Fix the pin, never the code.

```bash
cargo nextest run -p vox-orchestrator -E 'test(routing_probe_tasks_equal_plain_routing_task_literals) | test(budgeted_task_converts_to_routing_task_with_max_cost) | test(task_priority_wire_format_is_stable) | test(/budget::persistence::tests/) | test(extraction_old_paths_resolve)'
```

  Expected: `7 tests run: 7 passed`. If `VoxDb::open_memory()` lacks the user-preference schema, switch to
  `VoxDb::in_memory()` (`crates/vox-db/src/lib.rs:409`) and re-run.

- [ ] **Step 5: Mutation check of the equivalence pin.** Temporarily change `routing health probe` to
  `routing health probe [[tool:x]]` in the test table. The test must fail, because `parse_description_hints` fills
  `tool_hints`. Restore it, and confirm with `rg -c '"routing health probe",' crates/vox-orchestrator/src/types/tasks.rs`
  that it prints `1`.

- [ ] **Step 6: Commit.**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-orchestrator --all-targets -- -D warnings
git add crates/vox-orchestrator/src/types/tasks.rs crates/vox-orchestrator/src/budget/persistence.rs crates/vox-orchestrator/src/lib.rs
git commit -m "test(orchestrator): pin routing-probe, budget persistence and old paths before extraction"
```

---

### Task 2: Lower `TaskPriority`, `PricingSource` and the clutch primitives into `vox-orchestrator-types`

**Files:** Create `crates/vox-orchestrator-types/src/agent_types/task_priority.rs`, `src/pricing.rs`,
`src/clutch.rs`. Modify `crates/vox-orchestrator-types/src/{lib.rs, agent_types/mod.rs}`,
`crates/vox-orchestrator/src/types/tasks.rs`, `src/models/spec.rs`, `src/mode.rs`.

**Interfaces:** Produces `vox_orchestrator_types::{TaskPriority, PricingSource}` and
`vox_orchestrator_types::clutch::{Modalities, QualityLevel, TierProfile, InferenceConfig, BudgetAggressiveness,
ResolvedClutch, ClutchProfile}`. Derives, serde names and defaults are identical. Old paths
(`crate::types::TaskPriority`, `crate::models::spec::PricingSource`, `crate::mode::ClutchProfile`, …) keep working
through permanent re-exports, because these modules stay in `vox-orchestrator`. Consumed by Tasks 4 and 6. No new
crate edges: `vox-orchestrator-types` keeps only `serde` + `thiserror`.

- [ ] **Step 1: Write the failing tests.** Create `crates/vox-orchestrator-types/src/pricing.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pricing_source_wire_names_and_default_are_unchanged() {
        for (p, key) in [
            (PricingSource::Bootstrap, "bootstrap"),
            (PricingSource::Unknown, "unknown"),
            (PricingSource::OpenRouter, "open_router"),
            (PricingSource::AnthropicDirect, "anthropic_direct"),
            (PricingSource::LiteLLM, "lite_l_l_m"),
            (PricingSource::UserConfig, "user_config"),
            (PricingSource::Telemetry, "telemetry"),
        ] {
            assert_eq!(serde_json::to_string(&p).unwrap(), format!("\"{key}\""));
        }
        assert_eq!(PricingSource::default(), PricingSource::Bootstrap);
    }
}
```

  Create `src/agent_types/task_priority.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_priority_wire_format_and_order_are_unchanged() {
        assert!(TaskPriority::Urgent > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Background);
        assert_eq!(serde_json::to_string(&TaskPriority::Normal).unwrap(), "\"Normal\"");
        assert_eq!(TaskPriority::Urgent.to_string(), "urgent");
        assert_eq!(TaskPriority::from_u8(0), TaskPriority::Background);
        assert_eq!(TaskPriority::from_u8(7), TaskPriority::Normal);
    }
}
```

  Create `src/clutch.rs` with only:

```rust
#[cfg(test)]
mod wire_tests {
    use super::*;

    #[test]
    fn clutch_profile_wire_names_and_labels_are_unchanged() {
        for (c, key) in [
            (ClutchProfile::Free, "free"),
            (ClutchProfile::Efficiency, "efficiency"),
            (ClutchProfile::Balanced, "balanced"),
            (ClutchProfile::Genius, "genius"),
        ] {
            assert_eq!(serde_json::to_string(&c).unwrap(), format!("\"{key}\""));
            assert_eq!(ClutchProfile::from_label(key), Some(c));
        }
        assert_eq!(ClutchProfile::default(), ClutchProfile::Efficiency);
        assert_eq!(BudgetAggressiveness::Aggressive.thresholds(), (0.70, 0.90));
    }
}
```

  Add `pub mod clutch; pub mod pricing;` and `pub use pricing::PricingSource;` to `lib.rs`. Add
  `pub mod task_priority;` and `pub use task_priority::TaskPriority;` to `agent_types/mod.rs`, and `TaskPriority`
  to the `pub use agent_types::{…}` list in `lib.rs`. Before writing the expected value `"lite_l_l_m"`, check it:
  run `rg -n 'LiteLLM' crates/vox-orchestrator/src/models/spec.rs crates/vox-gui/ui/src -g '*.rs' -g '*.ts'`. If any
  JSON fixture or TS type shows a different spelling, use the spelling serde produces today. The test pins today's
  output, whatever it is.

- [ ] **Step 2: Run to verify failure.**

```bash
cargo test -p vox-orchestrator-types --lib 2>&1 | tail -15
```

  Expected: `error[E0412]`/`E0433` for `PricingSource`, `TaskPriority`, `ClutchProfile`, `BudgetAggressiveness`.

- [ ] **Step 3: Move the definitions.**
  1. `pricing.rs`: above the test module, add `use serde::{Deserialize, Serialize};` and paste the doc comment,
     derives and `pub enum PricingSource { … }` cut verbatim from `models/spec.rs` (today lines 12–33). In
     `spec.rs`, put `pub use vox_orchestrator_types::PricingSource;` where the enum was.
  2. `task_priority.rs`: above the test module, paste:

```rust
//! Task priority (dequeue order). Moved from `vox-orchestrator/src/types/tasks.rs`.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Priority level for a task. Higher priority tasks are dequeued first.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    /// Background work — lowest priority.
    Background = 0,
    /// Normal priority — default.
    Normal = 1,
    /// Urgent — preempts normal tasks.
    Urgent = 2,
}

impl TaskPriority {
    /// Map a wire `u8` (e.g. the `priority` field of a `HopperSync` op) to a
    /// `TaskPriority`. Matches the enum's `repr` (0/1/2); any out-of-range value
    /// falls back to `Normal` rather than failing — replication should degrade
    /// gracefully, not drop an admission.
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Background,
            2 => Self::Urgent,
            _ => Self::Normal,
        }
    }
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Background => write!(f, "background"),
            Self::Normal => write!(f, "normal"),
            Self::Urgent => write!(f, "urgent"),
        }
    }
}
```

     In `types/tasks.rs`, delete the same three items (today lines 41–75: doc comment through the `Display` impl)
     and put `pub use vox_orchestrator_types::TaskPriority;` in their place. `types/mod.rs`'s
     `pub use tasks::{…, TaskPriority, …}` keeps working unchanged.
  3. `clutch.rs`: above the test module, add:

```rust
//! Clutch primitives: inference quality/tier config, budget-gate aggressiveness and the
//! four-detent `ClutchProfile`. Pure data, shared by model selection (`vox-models`), the
//! budget gate (`vox-attention`) and `vox-orchestrator::mode`.

use serde::{Deserialize, Serialize};

use crate::CostPreference;
```

     Then cut verbatim from `mode.rs`: everything from `#[derive(Debug, Clone, Default, Serialize, Deserialize)]`
     / `pub struct Modalities` (today line 8) through the closing `}` of `impl ClutchProfile` (the line before
     `// ── Task 2: RiskPosture`, today line 177). Also cut the whole `mod semcov_behavior_tests` and
     `mod clutch_tests` blocks with their `#[cfg(test)]`. In the moved `clutch_tests`, delete
     `use crate::config::CostPreference;`, because `use super::*;` already brings `CostPreference`. Locate the anchors
     with `rg -n 'pub struct Modalities|// ── Task 2: RiskPosture|mod semcov_behavior_tests|mod clutch_tests|mod risk_tests' crates/vox-orchestrator/src/mode.rs`
     before cutting.
  4. In `mode.rs`, under the remaining `use` lines, add:

```rust
pub use vox_orchestrator_types::clutch::{
    BudgetAggressiveness, ClutchProfile, InferenceConfig, Modalities, QualityLevel, ResolvedClutch,
    TierProfile,
};
```

     Keep `use crate::attention::ApprovalTier;` and `use crate::config::CostPreference;` only if clippy reports them
     used. `ApprovalLean`, `ResolvedRisk` and `RiskPosture` still use `ApprovalTier`.

- [ ] **Step 4: Define-once check.**

```bash
for n in 'enum PricingSource' 'enum TaskPriority' 'enum ClutchProfile' 'struct ResolvedClutch' 'enum BudgetAggressiveness' 'enum QualityLevel' 'struct InferenceConfig' 'enum TierProfile' 'struct Modalities' 'fn clutch_resolves_each_detent' 'fn to_cost_preference_balanced_and_flash_are_economy_premium_is_performance'; do
  printf '%s: ' "$n"; rg -c "$n\b" crates/vox-orchestrator/src crates/vox-orchestrator-types/src | awk -F: '{s+=$2} END {print s}'; done
```

  Expected: every line prints `1`.

- [ ] **Step 5: Run to verify pass.**

```bash
cargo nextest run -p vox-orchestrator-types
cargo check -p vox-orchestrator --all-targets --features populi-transport,test-support
cargo nextest run -p vox-orchestrator -E 'test(/^mode::|types::tasks|models::spec/)'
cargo check -p vox-orchestrator-mcp -p vox-research-shim -p vox-cli
```

  Expected: all pass. The moved `semcov_behavior_tests`/`clutch_tests` now run under `vox-orchestrator-types`.

- [ ] **Step 6: Mutation proofs.** (a) Swap `#[default]` from `Bootstrap` to `Unknown` in `pricing.rs`: the pricing
  test fails. (b) Change `"urgent"` to `"high"` in `task_priority.rs`: the priority test fails. (c) Change
  `Aggressive => (0.70, 0.90)` to `(0.75, 0.90)`: the clutch test fails. Restore each one and re-run Step 5's
  first command to green.

- [ ] **Step 7: Commit.**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-orchestrator-types -p vox-orchestrator --all-targets -- -D warnings
vox ci crate-edges   # expected: pass (no new edges)
git add crates/vox-orchestrator-types crates/vox-orchestrator/src/types/tasks.rs crates/vox-orchestrator/src/models/spec.rs crates/vox-orchestrator/src/mode.rs
git commit -m "refactor(types): lower TaskPriority, PricingSource and clutch primitives into vox-orchestrator-types"
```

---

### Task 3: STOP — present the crate-edge and fan-in changes for user authorization

**Files:** none edited by the agent. The user edits `contracts/ci/crate-edges.allow.v1.json` and
`contracts/ci/fan-in-snapshot.v1.json`.

**Interfaces:** Produces a recorded user decision. Tasks 4–7 may not commit until it exists. Task 2 may already be
committed while waiting.

- [ ] **Step 1: Re-derive the list** from Task 0 Step 5's external-crate output and `cargo metadata`. The table below
  must match it exactly; update the table if it does not.

**New edges** (normal deps only; dev-deps are not counted by `vox ci crate-edges`):

| # | From → To | Introduced in | Why |
|---|---|---|---|
| 1 | `vox-attention` → `vox-orchestrator-types` | Task 4 | `AgentId`, `TaskId`, `TaskPriority`, `PricingSource`, `clutch::BudgetAggressiveness`, `socrates_policy` consts |
| 2 | `vox-attention` → `vox-telemetry` | Task 4 | `record_default_decision!`, `METRIC_TYPE_BUDGET_DECISION` |
| 3 | `vox-attention` → `vox-crypto` | Task 4 | `fast_hash` drift fingerprint (edge already exists from `vox-orchestrator`) |
| 4 | `vox-orchestrator` → `vox-attention` | Task 4 | consumes the logic; implements `BudgetSource` |
| 5 | `vox-orchestrator-mcp` → `vox-attention` | Task 5 | shim deletion: 17 files name attention/budget types |
| 6 | `vox-cli` → `vox-attention` | Task 5 | shim deletion: `commands/safety.rs` |
| 7–20 | `vox-models` → `vox-actor-runtime`, `vox-bounded-fs`, `vox-config`, `vox-db`, `vox-http-client`, `vox-mesh-policy`, `vox-mesh-transport` (opt), `vox-mesh-types`, `vox-orchestrator-models`, `vox-orchestrator-types`, `vox-populi` (opt; Option A), `vox-repository`, `vox-secrets`, `vox-telemetry` | Task 6 | each is used by moved code today (Task 0 Step 5 counts) |
| 21 | `vox-orchestrator` → `vox-models` | Task 6 | consumes the registry |
| 22 | `vox-orchestrator-mcp` → `vox-models` | Task 7 | shim deletion (37 files) |
| 23 | `vox-cli` → `vox-models` | Task 7 | shim deletion (16 files) |
| 24 | `vox-gui` → `vox-models` | Task 7 | shim deletion (5 files) |
| 25 | `vox-orchestrator-d` → `vox-models` | Task 7 | `install_base_routing_priority`, `SelectionPolicy`, `install_active_policy` |
| 26 | `vox-research-shim` → `vox-models` | Task 7 | `selection/` + research model selection |

**Removed edges** (Task 6, same commit as #7–21): `vox-orchestrator` → `vox-http-client`, and `vox-orchestrator` →
`vox-mesh-policy`. Both have 0 remaining uses after the move (measured 2026-10-08).

**Layer check:** L2 → {L0, L1} for #1–3; L3 → ≤L3 for #7–21; L5/L3 consumers → L2/L3. None is upward.

**Option B for #17 (`vox-models` → `vox-populi`):** in `auto_select.rs`, replace the `vox_populi::…::probe_metal()`
call with a registered probe, following the existing `vram::register_vram_probe` pattern. The orchestrator would
register it at startup. Trade-off: `vox-cli chat` and `vox-gui` call `select_optimal_local_model*` without that
registration, so on macOS they would fall back to the `sysctl` estimate unless they register too. That is a
behaviour change; Option A has none. The build-loop SSOT preferred injection to avoid a 20k-line dependency.

**Fan-in snapshot** (`vox ci fan-in-budget` fails when a listed crate gains dependents). Expected bumps, to be
re-verified by running the gate after Task 6/7's dry build:

```diff
-    "vox-bounded-fs":               23,
+    "vox-bounded-fs":               24,
-    "vox-config":                   41,
+    "vox-config":                   42,
-    "vox-db":                       28,
+    "vox-db":                       29,
-    "vox-secrets":                  35,
+    "vox-secrets":                  36,
```

`vox-http-client` stays at 25 (+`vox-models`, −`vox-orchestrator`). `vox-orchestrator` stays at 12, because no
consumer drops it (`vox-research-shim` keeps it; see Deviation 6).

**Proposed `exceptions` entries** for `contracts/ci/crate-edges.allow.v1.json`. The user appends these, or tells the
agent which to drop. The `date`/`authorized_by` values follow the existing entries' format; set them to the actual
approval day.

```json
{"from":"vox-attention","to":"vox-orchestrator-types","reason":"capability-profiles plan 05: attention/budget logic names AgentId/TaskId/TaskPriority/PricingSource/BudgetAggressiveness from the L0 types crate","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-attention","to":"vox-telemetry","reason":"capability-profiles plan 05: record_default_decision! sites and METRIC_TYPE_BUDGET_DECISION moved with budget/","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-attention","to":"vox-crypto","reason":"capability-profiles plan 05: budget drift fingerprint (vox_crypto::fast_hash) moved with budget/","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-orchestrator","to":"vox-attention","reason":"capability-profiles plan 05: orchestrator consumes extracted attention/budget logic and implements BudgetSource over VoxDB","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-orchestrator-mcp","to":"vox-attention","reason":"capability-profiles plan 05: vox_orchestrator::{attention,budget} re-export shim deleted; MCP names the types directly","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-cli","to":"vox-attention","reason":"capability-profiles plan 05: re-export shim deleted; commands/safety.rs names attention types directly","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-actor-runtime","reason":"capability-profiles plan 05: route_policy uses route_capability_policy; registry builds llm::LlmConfig (runtime feature)","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-bounded-fs","reason":"capability-profiles plan 05: bounded reads moved with models/","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-config","reason":"capability-profiles plan 05: registry/catalog configuration moved with models/","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-db","reason":"capability-profiles plan 05: model scoreboard/bench evidence persistence moved with models/","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-http-client","reason":"capability-profiles plan 05: provider catalogs build HTTP clients; vox-orchestrator's own edge is removed in the same commit","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-mesh-policy","reason":"capability-profiles plan 05: mesh model policy moved with models/; vox-orchestrator's own edge is removed","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-mesh-transport","reason":"capability-profiles plan 05: mesh_directory peers (populi-transport feature, optional)","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-mesh-types","reason":"capability-profiles plan 05: mesh model types moved with models/","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-orchestrator-models","reason":"capability-profiles plan 05: scoring/spec/registry use usage::{RemainingBudget,LlmUsageKey} and calibration::ContextualBandit","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-orchestrator-types","reason":"capability-profiles plan 05: CostPreference, RoutingProfile, PricingSource, clutch primitives","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-populi","reason":"capability-profiles plan 05 (Option A): auto_select macOS Metal VRAM probe, runtime feature, optional — mirrors vox-orchestrator today","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-repository","reason":"capability-profiles plan 05: repo-root discovery for MensCatalog moved with models/","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-secrets","reason":"capability-profiles plan 05: key_guard provider-key availability via resolve_secret","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-models","to":"vox-telemetry","reason":"capability-profiles plan 05: routing telemetry moved with models/","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-orchestrator","to":"vox-models","reason":"capability-profiles plan 05: orchestrator consumes the extracted model registry","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-orchestrator-mcp","to":"vox-models","reason":"capability-profiles plan 05: vox_orchestrator::models shim deleted","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-cli","to":"vox-models","reason":"capability-profiles plan 05: vox_orchestrator::models shim deleted (vox model *, chat, audit, doctor)","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-gui","to":"vox-models","reason":"capability-profiles plan 05: vox_orchestrator::models shim deleted (models, routing_explain, llm_settings)","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-orchestrator-d","to":"vox-models","reason":"capability-profiles plan 05: daemon installs routing priority and selection policy from vox-models","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"},
{"from":"vox-research-shim","to":"vox-models","reason":"capability-profiles plan 05: selection/ and research model selection depend on vox-models instead of vox-orchestrator::models","date":"2026-10-08","authorized_by":"user (explicit chat approval, 2026-10-08)"}
```

- [ ] **Step 2: STOP.** Present Step 1 (table, Option A/B for #17, fan-in diff, exceptions JSON) to the user in chat
  or in the PR description, then wait. **Do not edit `crate-edges.allow.v1.json` or `fan-in-snapshot.v1.json`.** Do
  not run `vox ci crate-edges --tighten` to "absorb" anything.
- [ ] **Step 3: Record the decision.** Once the user has applied the files, run
  `git log -1 --format='%H %an %s' -- contracts/ci/crate-edges.allow.v1.json` and copy the line into the PR
  description. If the user refused some edges, re-plan the affected task (Option B, defactoring, or keeping a
  facade) before continuing.

---

### Task 4: Extract `vox-attention` (with a one-task re-export shim)

**Files:** Create `crates/vox-attention/{Cargo.toml, src/lib.rs, src/source.rs, src/sync_lock.rs,
tests/budget_source.rs}`, `crates/vox-orchestrator/src/budget_source.rs`. Move the 11 attention files (table
above). Modify the root `Cargo.toml`, `crates/vox-orchestrator/{Cargo.toml, src/lib.rs}`,
`crates/vox-orchestrator-mcp/src/server_state.rs`, `docs/src/architecture/{layers.toml, where-things-live.md}`,
and the path-keyed contracts listed in Step 10.

**Interfaces:** Produces the crate `vox_attention` with modules `attention`, `budget`, `budget_gate`,
`fatigue_monitor`, `source`. The root re-exports exactly the list that `vox-orchestrator`'s `lib.rs` re-exports
today, plus `BudgetSource`, `BudgetSourceError`, `ToolLatency`. `BudgetManager::new(Option<Arc<dyn BudgetSource>>)`
replaces `new(Option<Arc<VoxDb>>)`, and every `new(None)` call compiles unchanged. `attach_source(Arc<dyn
BudgetSource>)` replaces `attach_db(Arc<VoxDb>)` (one call site). `source()` replaces `db()` (no external callers).
`vox_orchestrator::budget_source::VoxDbBudgetSource` is new. Shim: `vox_orchestrator::{attention, budget,
budget_gate, fatigue_monitor}` and the top-level names keep resolving until Task 5. Consumes Task 2 (lowered types)
and Task 3 (authorization for edges #1–4).

- [ ] **Step 1: Scaffold the crate and write the failing contract test.**

  Root `Cargo.toml`, `[workspace.dependencies]`, next to `vox-orchestrator-models`:

```toml
vox-attention             = { path = "crates/vox-attention" }
```

  `crates/vox-attention/Cargo.toml`:

```toml
[package]
name = "vox-attention"
description = "Developer-attention budget, per-agent token/cost budgets and the orchestrator budget gate. Pure logic; live state via BudgetSource. Extracted from vox-orchestrator."
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[dependencies]
async-trait = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
vox-crypto.workspace = true
vox-orchestrator-types.workspace = true
vox-telemetry.workspace = true
workspace-hack = { version = "0.6", path = "../workspace-hack" }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt"] }

[lints]
workspace = true
```

  `crates/vox-attention/src/lib.rs` (crate docs only for now):

```rust
//! Developer attention as a first-class budget resource, per-agent token/cost budgets and
//! the orchestrator budget gate.
//!
//! Pure logic: no database and no async runtime. Live state (persisted allocations,
//! tool-latency history) is reached through [`BudgetSource`]; `vox-orchestrator` implements
//! it over VoxDB. With no source attached, [`BudgetManager`] works purely in memory.
```

  `crates/vox-attention/tests/budget_source.rs`:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use vox_attention::{
    AgentBudgetAllocation, BudgetManager, BudgetSignal, BudgetSource, BudgetSourceError,
    ToolLatency,
};
use vox_orchestrator_types::AgentId;

#[derive(Default)]
struct FakeSource {
    prefs: Mutex<HashMap<(String, String), String>>,
    latency: Mutex<Option<ToolLatency>>,
    calls: Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl BudgetSource for FakeSource {
    async fn get_user_preference(
        &self,
        user_id: &str,
        key: &str,
    ) -> Result<Option<String>, BudgetSourceError> {
        Ok(self.prefs.lock().unwrap().get(&(user_id.into(), key.into())).cloned())
    }
    async fn set_user_preference(
        &self,
        user_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), BudgetSourceError> {
        self.prefs
            .lock()
            .unwrap()
            .insert((user_id.into(), key.into()), value.into());
        Ok(())
    }
    async fn query_tool_latency(
        &self,
        _tool_key: &str,
        _repository_id: &str,
        _window_days: u32,
        _safety_multiplier: f64,
    ) -> Result<Option<ToolLatency>, BudgetSourceError> {
        Ok(*self.latency.lock().unwrap())
    }
    async fn record_exec_timeout(
        &self,
        tool_key: &str,
        repository_id: &str,
        duration_ms: u64,
    ) -> Result<(), BudgetSourceError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("timeout:{tool_key}:{repository_id}:{duration_ms}"));
        Ok(())
    }
    async fn record_exec_success(
        &self,
        tool_key: &str,
        repository_id: &str,
        duration_ms: u64,
        timeout_budget_ms: Option<u64>,
    ) -> Result<(), BudgetSourceError> {
        self.calls.lock().unwrap().push(format!(
            "success:{tool_key}:{repository_id}:{duration_ms}:{timeout_budget_ms:?}"
        ));
        Ok(())
    }
}

#[tokio::test]
async fn allocation_round_trips_through_source_under_local_user_key() {
    let src = Arc::new(FakeSource::default());
    let writer = BudgetManager::new(Some(src.clone()));
    writer
        .set_and_persist_allocation(AgentId(7), AgentBudgetAllocation::new(1000, 2.0))
        .await;
    assert!(
        src.prefs
            .lock()
            .unwrap()
            .contains_key(&("local_user".into(), "agent_budget.7".into()))
    );
    let reader = BudgetManager::new(Some(src));
    reader.load_user_configured_budget(AgentId(7)).await;
    let alloc = reader.check_budget(AgentId(7)).and_then(|b| b.allocation).unwrap();
    assert_eq!(alloc.max_tokens, 1000);
    assert_eq!(alloc.max_cost_usd, 2.0);
}

#[tokio::test]
async fn tool_latency_signal_maps_source_profile() {
    let src = Arc::new(FakeSource::default());
    let bm = BudgetManager::new(Some(src.clone()));
    assert_eq!(
        bm.query_tool_latency_signal("t", "r", 0.2, 7, 1.5, 9).await,
        BudgetSignal::ToolLatencyUnknown { tool_key: "t".into(), default_budget_ms: 9 }
    );
    *src.latency.lock().unwrap() =
        Some(ToolLatency { timeout_rate: 0.5, recommended_budget_ms: 4000, p90_ms: 2500.0 });
    assert_eq!(
        bm.query_tool_latency_signal("t", "r", 0.2, 7, 1.5, 9).await,
        BudgetSignal::ToolLatencyHigh {
            tool_key: "t".into(),
            recommended_budget_ms: 4000,
            p90_ms: 2500.0,
            timeout_rate: 0.5,
        }
    );
    *src.latency.lock().unwrap() =
        Some(ToolLatency { timeout_rate: 0.1, recommended_budget_ms: 4000, p90_ms: 2500.0 });
    assert_eq!(
        bm.query_tool_latency_signal("t", "r", 0.2, 7, 1.5, 9).await,
        BudgetSignal::Normal { usage_ratio: 0.1 }
    );
}

#[tokio::test]
async fn execution_outcomes_route_to_distinct_source_calls() {
    let src = Arc::new(FakeSource::default());
    let bm = BudgetManager::new(None);
    bm.record_tool_execution_outcome("t", "r", 5, true, None).await;
    bm.attach_source(src.clone()).await;
    bm.record_tool_execution_outcome("t", "r", 1500, true, Some(1000)).await;
    bm.record_tool_execution_outcome("t", "r", 900, false, Some(1000)).await;
    assert_eq!(
        *src.calls.lock().unwrap(),
        vec!["timeout:t:r:1500".to_string(), "success:t:r:900:Some(1000)".to_string()]
    );
}
```

- [ ] **Step 2: Run to verify failure.**

```bash
cargo test -p vox-attention --test budget_source 2>&1 | tail -10
```

  Expected: `error[E0432]: unresolved imports vox_attention::AgentBudgetAllocation, …`.

- [ ] **Step 3: Move with history.** Record the item inventory first, then move:

```bash
xargs rg -o --no-filename '^\s*(pub(\([a-z]+\))? )?(async )?(fn|struct|enum|trait|type|const|static) [A-Za-z_][A-Za-z0-9_]*' \
  < target/cp05/attention-move-list.txt | sed -E 's/.* //' | sort | uniq -c > target/cp05/attention-items-before.txt
mkdir -p crates/vox-attention/src
git mv crates/vox-orchestrator/src/attention crates/vox-attention/src/attention
git mv crates/vox-orchestrator/src/budget crates/vox-attention/src/budget
git mv crates/vox-orchestrator/src/budget_gate.rs crates/vox-attention/src/budget_gate.rs
git mv crates/vox-orchestrator/src/fatigue_monitor.rs crates/vox-attention/src/fatigue_monitor.rs
git status --short | rg '^R' | wc -l     # expected: 11
```

- [ ] **Step 4: Mechanical path rewrites inside the moved files** (`crate::attention`, `crate::budget`,
  `crate::budget_gate` and `crate::fatigue_monitor` stay valid, because the module names are unchanged):

```bash
F=$(git ls-files crates/vox-attention/src)
perl -pi -e 's/\bcrate::types::(AgentId|TaskId|now_unix_ms)\b/vox_orchestrator_types::$1/g; s/\bcrate::types::TaskPriority\b/vox_orchestrator_types::TaskPriority/g; s/\bcrate::models::spec::PricingSource\b/vox_orchestrator_types::PricingSource/g; s/\bcrate::mode::BudgetAggressiveness\b/vox_orchestrator_types::clutch::BudgetAggressiveness/g; s/\bvox_db::research_metrics_contract::METRIC_TYPE_BUDGET_DECISION\b/vox_telemetry::METRIC_TYPE_BUDGET_DECISION/g' $F
perl -pi -e 's/use crate::types::\{AgentId, TaskId, TaskPriority\};/use vox_orchestrator_types::{AgentId, TaskId, TaskPriority};/; s/use crate::types::\{AgentId, TaskPriority\};/use vox_orchestrator_types::{AgentId, TaskPriority};/' $F
rg -n 'crate::(types|models|mode|services|tier_cascade|gate|attention_tracker)\b|vox_db' crates/vox-attention/src
```

  Fix the remaining hits by hand:
  - Doc comments that link `[`crate::tier_cascade::CompositeSignal`]`, `[`crate::gate::BudgetGate`]`,
    `[`crate::attention_tracker`]`, `[`crate::mode::ResolvedRisk::safety_token_multiplier`]` and
    `[`crate::mode::BudgetAggressiveness`]` become plain code spans naming the item and its crate, for example
    `` `vox_orchestrator::tier_cascade::CompositeSignal` ``. Leave the text otherwise unchanged.
  - `crate::sync_lock` stays as written and now resolves to the local module (Step 6).
  - `crate::services::persistence_obs` and every `vox_db` use disappear in Step 5.

- [ ] **Step 5: Invert persistence onto `BudgetSource`.**

  `crates/vox-attention/src/source.rs`:

```rust
//! Live state the budget logic reads and writes, abstracted so this crate stays free of any
//! database. With no source attached, [`crate::BudgetManager`] is purely in-memory.

/// Error returned by a [`BudgetSource`]; its `Display` text is what gets logged.
pub type BudgetSourceError = Box<dyn std::error::Error + Send + Sync>;

/// Tool-latency profile over a query window (the fields the budget signal reads).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolLatency {
    /// Fraction of observations that ended in timeout.
    pub timeout_rate: f64,
    /// Suggested agent wait in milliseconds.
    pub recommended_budget_ms: u64,
    /// Approximate 90th-percentile latency in milliseconds.
    pub p90_ms: f64,
}

/// Persistence the budget manager needs; `vox-orchestrator` implements it over VoxDB.
#[async_trait::async_trait]
pub trait BudgetSource: Send + Sync {
    /// Read a stored user preference (`None` when unset).
    async fn get_user_preference(
        &self,
        user_id: &str,
        key: &str,
    ) -> Result<Option<String>, BudgetSourceError>;
    /// Store a user preference.
    async fn set_user_preference(
        &self,
        user_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), BudgetSourceError>;
    /// Latency profile for a tool in a repository, or `None` without enough samples.
    async fn query_tool_latency(
        &self,
        tool_key: &str,
        repository_id: &str,
        window_days: u32,
        safety_multiplier: f64,
    ) -> Result<Option<ToolLatency>, BudgetSourceError>;
    /// Record a timed-out tool execution.
    async fn record_exec_timeout(
        &self,
        tool_key: &str,
        repository_id: &str,
        duration_ms: u64,
    ) -> Result<(), BudgetSourceError>;
    /// Record a successful tool execution.
    async fn record_exec_success(
        &self,
        tool_key: &str,
        repository_id: &str,
        duration_ms: u64,
        timeout_budget_ms: Option<u64>,
    ) -> Result<(), BudgetSourceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_source_is_object_safe() {
        fn takes(_: Option<std::sync::Arc<dyn BudgetSource>>) {}
        takes(None);
    }
}
```

  In `crates/vox-attention/src/budget/mod.rs`, make exactly these replacements:

```rust
// field (was: pub db: Arc<std::sync::RwLock<Option<Arc<vox_db::VoxDb>>>>,)
    pub source: Arc<std::sync::RwLock<Option<Arc<dyn crate::source::BudgetSource>>>>,

// constructor signature (was: pub fn new(db: Option<Arc<vox_db::VoxDb>>) -> Self {)
    pub fn new(source: Option<Arc<dyn crate::source::BudgetSource>>) -> Self {

// constructor field init (was: db: Arc::new(std::sync::RwLock::new(db)),)
            source: Arc::new(std::sync::RwLock::new(source)),

// accessor (was: pub fn db(&self) -> Option<Arc<vox_db::VoxDb>> { sync_lock::rw_read(&*self.db).clone() })
    pub fn source(&self) -> Option<Arc<dyn crate::source::BudgetSource>> {
        sync_lock::rw_read(&*self.source).clone()
    }
```

  Replace `crates/vox-attention/src/budget/persistence.rs` wholesale. Delete the Task 1 test module; it moves to
  the orchestrator in Step 8.

```rust
use super::{AgentBudgetAllocation, BudgetManager};
use crate::source::BudgetSource;
use std::sync::Arc;
use vox_orchestrator_types::AgentId;

// vox:defactored-from vox-orchestrator 2026-10-08
fn log_persistence_failure(op_id: &'static str, err: impl std::fmt::Display) {
    tracing::error!(
        target: "vox.orchestrator.persistence",
        op = op_id,
        error = %err,
        "persistence write failed; data not recorded"
    );
}

impl BudgetManager {
    /// Load the custom dollar caps or budgets from the attached [`BudgetSource`].
    pub async fn load_user_configured_budget(&self, agent_id: AgentId) {
        if let Some(source) = self.source() {
            let key = format!("agent_budget.{}", agent_id.0);
            if let Ok(Some(val)) = source.get_user_preference("local_user", &key).await {
                if let Ok(alloc) = serde_json::from_str::<AgentBudgetAllocation>(&val) {
                    self.set_allocation(agent_id, alloc);
                }
            }
        }
    }

    /// Save the custom dollar caps or budgets through the attached [`BudgetSource`].
    pub async fn set_and_persist_allocation(
        &self,
        agent_id: AgentId,
        allocation: AgentBudgetAllocation,
    ) {
        self.set_allocation(agent_id, allocation.clone());
        if let Some(source) = self.source() {
            let key = format!("agent_budget.{}", agent_id.0);
            if let Ok(val) = serde_json::to_string(&allocation) {
                if let Err(e) = source.set_user_preference("local_user", &key, &val).await {
                    // In-memory allocation above already succeeded; the DB write is a
                    // durability-only side effect. Swallowing the Result here means the
                    // allocation looks persisted but is silently lost on restart.
                    // Refs: docs/src/architecture/semantic-gap-audit-2026.md F2-F6;
                    // vox-axis-harness-reliability-spec-plan-2026-07-02.md T5.1.
                    log_persistence_failure("budget.set_and_persist_allocation", e);
                }
            }
        }
    }

    /// Attach a budget source late (replaces any earlier one).
    pub async fn attach_source(&self, source: Arc<dyn BudgetSource>) {
        crate::sync_lock::rw_write(&*self.source).replace(source);
    }

    pub async fn query_tool_latency_signal(
        &self,
        tool_key: &str,
        repository_id: &str,
        timeout_rate_alert_threshold: f64,
        window_days: u32,
        safety_multiplier: f64,
        default_budget_ms: u64,
    ) -> super::BudgetSignal {
        let Some(source) = self.source() else {
            return super::BudgetSignal::ToolLatencyUnknown {
                tool_key: tool_key.to_string(),
                default_budget_ms,
            };
        };

        match source
            .query_tool_latency(tool_key, repository_id, window_days, safety_multiplier)
            .await
        {
            Ok(Some(profile)) => {
                if profile.timeout_rate > timeout_rate_alert_threshold {
                    super::BudgetSignal::ToolLatencyHigh {
                        tool_key: tool_key.to_string(),
                        recommended_budget_ms: profile.recommended_budget_ms,
                        p90_ms: profile.p90_ms,
                        timeout_rate: profile.timeout_rate,
                    }
                } else {
                    super::BudgetSignal::Normal {
                        usage_ratio: profile.timeout_rate,
                    }
                }
            }
            _ => super::BudgetSignal::ToolLatencyUnknown {
                tool_key: tool_key.to_string(),
                default_budget_ms,
            },
        }
    }

    pub async fn record_tool_execution_outcome(
        &self,
        tool_key: &str,
        repository_id: &str,
        duration_ms: u64,
        timed_out: bool,
        attempted_budget_ms: Option<u64>,
    ) {
        if let Some(source) = self.source() {
            if timed_out {
                if let Err(e) = source
                    .record_exec_timeout(tool_key, repository_id, duration_ms)
                    .await
                {
                    // Lost exec-timeout rows under-count agent usage against budgets.
                    // Refs: docs/src/architecture/semantic-gap-audit-2026.md F6.
                    log_persistence_failure("budget.exec_timeout", e);
                }
            } else if let Err(e) = source
                .record_exec_success(tool_key, repository_id, duration_ms, attempted_budget_ms)
                .await
            {
                // Lost exec-time rows under-count agent usage against budgets;
                // budget gate cannot enforce limits it doesn't see.
                // Refs: docs/src/architecture/semantic-gap-audit-2026.md F6.
                log_persistence_failure("budget.exec_time", e);
            }
        }
    }
}
```

- [ ] **Step 6: `sync_lock.rs` and `lib.rs`.**

  `crates/vox-attention/src/sync_lock.rs`:

```rust
// vox:defactored-from vox-orchestrator-queue 2026-10-08
//! Recover from poisoned [`std::sync::RwLock`] guards: `PoisonError::into_inner` yields the
//! guard so long-lived budget maps keep working after a panicking writer.

use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub(crate) fn rw_read<T>(lock: &RwLock<T>) -> RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

pub(crate) fn rw_write<T>(lock: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    lock.write().unwrap_or_else(|e| e.into_inner())
}
```

  `crates/vox-attention/src/lib.rs` (complete):

```rust
//! Developer attention as a first-class budget resource, per-agent token/cost budgets and
//! the orchestrator budget gate.
//!
//! Pure logic: no database and no async runtime. Live state (persisted allocations,
//! tool-latency history) is reached through [`BudgetSource`]; `vox-orchestrator` implements
//! it over VoxDB. With no source attached, [`BudgetManager`] works purely in memory.

pub mod attention;
pub mod budget;
pub mod budget_gate;
pub mod fatigue_monitor;
pub mod source;
mod sync_lock;

pub use attention::{
    ActionDescriptor, AgentTrustScore, ApprovalOutcome, ApprovalTier, AttentionBudget,
    AttentionEvent, AttentionEventType, DEFAULT_ATTENTION_BUDGET_MS, DEFAULT_INTERRUPT_COST_MS,
    FocusDepth, InterruptionChannel, InterruptionDecision, InterruptionSignals, NasaTlxWeights,
    TierGateConfig, TrustTier, classify_tier, compute_attention_cost_ms, decision_entropy_bits,
    evaluate_interruption, scaled_interrupt_cost_ms,
};
pub use budget::{AgentBudgetAllocation, BudgetManager, BudgetSignal};
pub use source::{BudgetSource, BudgetSourceError, ToolLatency};

#[cfg(test)]
mod tests {
    #[test]
    fn root_reexports_resolve() {
        let _ = crate::BudgetManager::new(None);
        let _: Option<crate::ApprovalTier> = None;
        let _: Option<crate::budget_gate::OrchestratorBudgetGate> = None;
        let _: Option<crate::fatigue_monitor::FatigueMonitor> = None;
    }
}
```

  Moved test modules that used `use crate::types::{AgentId, TaskPriority};` were rewritten by Step 4. Check with
  `cargo test -p vox-attention --no-run`.

- [ ] **Step 7: Run the crate.**

```bash
cargo check -p vox-attention --all-targets
cargo nextest run -p vox-attention
cargo nextest list -p vox-attention --message-format oneline 2>/dev/null | wc -l
```

  Expected: all pass. The count is `count-attention-tests-before` (~67) + 3 contract tests + 1 object-safety test +
  1 root test. Task 1's three persistence pins move to the orchestrator, so they don't count here.

- [ ] **Step 8: Orchestrator side — dependency, `VoxDbBudgetSource`, shim.**

  `crates/vox-orchestrator/Cargo.toml` `[dependencies]` (next to `vox-orchestrator-models`):

```toml
vox-attention.workspace = true
```

  `crates/vox-orchestrator/src/budget_source.rs`:

```rust
//! [`vox_attention::BudgetSource`] backed by the workspace VoxDB.

use std::sync::Arc;

use vox_attention::{BudgetSource, BudgetSourceError, ToolLatency};

/// Budget allocations and tool-latency history stored in VoxDB.
pub struct VoxDbBudgetSource {
    db: Arc<vox_db::VoxDb>,
}

impl VoxDbBudgetSource {
    pub fn new(db: Arc<vox_db::VoxDb>) -> Self {
        Self { db }
    }
}

fn boxed(e: impl std::fmt::Display) -> BudgetSourceError {
    e.to_string().into()
}

#[async_trait::async_trait]
impl BudgetSource for VoxDbBudgetSource {
    async fn get_user_preference(
        &self,
        user_id: &str,
        key: &str,
    ) -> Result<Option<String>, BudgetSourceError> {
        self.db.get_user_preference(user_id, key).await.map_err(boxed)
    }

    async fn set_user_preference(
        &self,
        user_id: &str,
        key: &str,
        value: &str,
    ) -> Result<(), BudgetSourceError> {
        self.db.set_user_preference(user_id, key, value).await.map_err(boxed)
    }

    async fn query_tool_latency(
        &self,
        tool_key: &str,
        repository_id: &str,
        window_days: u32,
        safety_multiplier: f64,
    ) -> Result<Option<ToolLatency>, BudgetSourceError> {
        let profile = self
            .db
            .query_tool_latency(tool_key, repository_id, window_days, safety_multiplier)
            .await
            .map_err(boxed)?;
        Ok(profile.map(|p| ToolLatency {
            timeout_rate: p.timeout_rate,
            recommended_budget_ms: p.recommended_budget_ms,
            p90_ms: p.p90_ms,
        }))
    }

    async fn record_exec_timeout(
        &self,
        tool_key: &str,
        repository_id: &str,
        duration_ms: u64,
    ) -> Result<(), BudgetSourceError> {
        self.db
            .record_exec_timeout(tool_key, repository_id, duration_ms)
            .await
            .map_err(boxed)
    }

    async fn record_exec_success(
        &self,
        tool_key: &str,
        repository_id: &str,
        duration_ms: u64,
        timeout_budget_ms: Option<u64>,
    ) -> Result<(), BudgetSourceError> {
        let record = vox_db::ExecTimeRecord {
            tool_key,
            repository_id,
            duration_ms,
            timeout_budget_ms,
            compute_tokens_used: None,
            vendor_cost_usd_micros: None,
            attention_cost_ms: None,
            outcome: vox_db::ExecOutcome::Success,
        };
        self.db.record_exec_time(&record).await.map_err(boxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_attention::{AgentBudgetAllocation, BudgetManager, BudgetSignal};
    use vox_orchestrator_types::AgentId;

    fn source(db: &Arc<vox_db::VoxDb>) -> Option<Arc<dyn BudgetSource>> {
        Some(Arc::new(VoxDbBudgetSource::new(db.clone())))
    }

    #[tokio::test]
    async fn allocation_persists_through_voxdb() {
        let db = Arc::new(vox_db::VoxDb::open_memory().await.expect("mem db"));
        let writer = BudgetManager::new(source(&db));
        writer
            .set_and_persist_allocation(AgentId(7), AgentBudgetAllocation::new(1234, 2.5))
            .await;
        let reader = BudgetManager::new(source(&db));
        reader.load_user_configured_budget(AgentId(7)).await;
        let alloc = reader
            .check_budget(AgentId(7))
            .and_then(|b| b.allocation)
            .expect("allocation restored from VoxDB");
        assert_eq!(alloc.max_tokens, 1234);
        assert_eq!(alloc.max_cost_usd, 2.5);
    }

    #[tokio::test]
    async fn tool_latency_without_samples_is_unknown() {
        let db = Arc::new(vox_db::VoxDb::open_memory().await.expect("mem db"));
        let bm = BudgetManager::new(source(&db));
        let sig = bm
            .query_tool_latency_signal("mcp:vox_build_crate", "repo", 0.2, 7, 1.5, 30_000)
            .await;
        assert_eq!(
            sig,
            BudgetSignal::ToolLatencyUnknown {
                tool_key: "mcp:vox_build_crate".into(),
                default_budget_ms: 30_000,
            }
        );
    }

    #[tokio::test]
    async fn tool_latency_without_source_is_unknown() {
        let bm = BudgetManager::new(None);
        let sig = bm.query_tool_latency_signal("t", "r", 0.2, 7, 1.5, 9).await;
        assert_eq!(
            sig,
            BudgetSignal::ToolLatencyUnknown {
                tool_key: "t".into(),
                default_budget_ms: 9,
            }
        );
    }
}
```

  In `crates/vox-orchestrator/src/lib.rs`:
  - Delete these declarations with their doc comments: `pub mod attention;`, `pub mod budget;`,
    `pub mod budget_gate;`, `pub mod fatigue_monitor;`. Keep `pub mod attention_tracker;`.
  - Add `/// VoxDB-backed [`vox_attention::BudgetSource`].` and `pub mod budget_source;`.
  - Replace the `pub use attention::{…};` block (today lines 282–288) and `pub use budget::{AgentBudgetAllocation,
    BudgetManager, BudgetSignal};` (today line 294) with:

```rust
// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="capability-profiles-05" canonical="vox_attention"
pub use vox_attention::{attention, budget, budget_gate, fatigue_monitor};
// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="capability-profiles-05" canonical="vox_attention"
pub use vox_attention::{
    ActionDescriptor, AgentBudgetAllocation, AgentTrustScore, ApprovalOutcome, ApprovalTier,
    AttentionBudget, AttentionEvent, AttentionEventType, BudgetManager, BudgetSignal,
    DEFAULT_ATTENTION_BUDGET_MS, DEFAULT_INTERRUPT_COST_MS, FocusDepth, InterruptionChannel,
    InterruptionDecision, InterruptionSignals, NasaTlxWeights, TierGateConfig, TrustTier,
    classify_tier, compute_attention_cost_ms, decision_entropy_bits, evaluate_interruption,
    scaled_interrupt_cost_ms,
};
```

  In `crates/vox-orchestrator-mcp/src/server_state.rs` (today line 651), replace
  `self.budget_manager.attach_db(db.clone()).await;` with:

```rust
        self.budget_manager
            .attach_source(std::sync::Arc::new(
                vox_orchestrator::budget_source::VoxDbBudgetSource::new(db.clone()),
            ))
            .await;
```

  This adds no MCP edge in this task: both names reach MCP through `vox_orchestrator`.

- [ ] **Step 9: Register the crate.**

  `docs/src/architecture/layers.toml`, in the L2 block after the `vox-journal` line:

```toml
vox-attention           = { layer = 2 }                          # attention + token/cost budget logic; live state via BudgetSource (capability-profiles plan 05)
```

  `docs/src/architecture/where-things-live.md`: add this row to the crate roster after `vox-orchestrator-driver`:

```markdown
| [`vox-attention`](../../../crates/vox-attention/) | Developer-attention budget (NASA-TLX cost, trust EWMA, interruption policy, calibrator), per-agent token/cost `BudgetManager`, orchestrator budget gate, fatigue monitor. Pure logic; persisted state goes through the `BudgetSource` trait, implemented over VoxDB by `vox_orchestrator::budget_source::VoxDbBudgetSource`. |
```

  and repoint the concept rows:

```markdown
| Budget gate — token/cost limits (D7) | `crates/vox-attention/src/budget_gate.rs` |
| Attention interruption calibrator (learns from logged outcomes) | `crates/vox-attention/src/attention/calibrator.rs` |
```

- [ ] **Step 10: Path-reference sweep (attention paths).**

```bash
P='vox-orchestrator(/src/|,src/)(attention/|budget/|budget_gate\.rs|fatigue_monitor\.rs)'
rg -n "$P" crates contracts docs/src --glob '!docs/src/archive/**' --glob '!**/*.generated.md'
```

  Rewrite each hit to the new location:
  - `crates/vox-orchestrator/src/X` becomes `crates/vox-attention/src/X`.
  - CSV rows `vox-orchestrator,src/budget/mod.rs:<line>` become `vox-attention,src/budget/mod.rs:<line>`. Keep the
    line number and re-check it with `rg -n` on the record_default_decision site.
  - `path_glob` values in `contracts/toestub/{weak-test,silent-drop}-baseline.v1.json` become
    `**/crates/vox-attention/src/…`.
  - `contracts/orchestration/permission-modes.v1.yaml` `source:` becomes `crates/vox-attention/src/attention/routing.rs`.

  Rename keys only. Add no new baseline entries.

- [ ] **Step 11: Verify.**

```bash
git ls-files crates/vox-attention/src | xargs rg -o --no-filename '^\s*(pub(\([a-z]+\))? )?(async )?(fn|struct|enum|trait|type|const|static) [A-Za-z_][A-Za-z0-9_]*' \
  | sed -E 's/.* //' | sort | uniq -c > target/cp05/attention-items-after.txt
diff target/cp05/attention-items-before.txt target/cp05/attention-items-after.txt
for d in attention budget budget_gate.rs fatigue_monitor.rs; do test ! -e crates/vox-orchestrator/src/$d || echo "LEFTOVER $d"; done
cargo check -p vox-orchestrator --all-targets --features populi-transport,test-support
cargo check -p vox-orchestrator-mcp --all-targets -p vox-cli
cargo nextest run -p vox-attention
cargo nextest run -p vox-orchestrator -E 'test(/budget_source::tests|extraction_old_paths_resolve|mode::|gate::|rebalance|scaling|attention_tracker|attention_calibration/)'
cargo clippy -p vox-attention -p vox-orchestrator --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p vox-attention --no-deps
cargo run -q -p vox-arch-check
vox ci crate-edges
vox ci fan-in-budget
vox ci affected-crates --regen --out contracts/ci/crate-graph.v1.json
vox ci ssot-drift
```

  Expected:
  - The `diff` shows only additions: `BudgetSource`, `BudgetSourceError`, `ToolLatency`, `rw_read`, `rw_write`,
    `log_persistence_failure`, `attach_source`, `source`, the new test fns. It also shows the removals `attach_db`
    and `db`, plus the three persistence pin test names (moved to the orchestrator). No name has a count higher
    than before unless it was already duplicated before (for example `new`, `default`).
  - No `LEFTOVER` lines.
  - `vox ci crate-edges` passes if Task 3's edits are in. Otherwise it fails listing exactly edges #1–4. **If it
    lists anything else, STOP.**
  - `fan-in-budget` passes (no snapshot crate gains a dependent in this task).

- [ ] **Step 12: Mutation proof of the inversion.** In `budget/persistence.rs`, swap the two branches (call
  `record_exec_success` when `timed_out`). `execution_outcomes_route_to_distinct_source_calls` must fail. Restore,
  confirm with `rg -c 'record_exec_timeout\(tool_key, repository_id, duration_ms\)' crates/vox-attention/src/budget/persistence.rs`
  (prints `1`), and re-run `cargo nextest run -p vox-attention --test budget_source`.

- [ ] **Step 13: Commit.**

```bash
vox run scripts/fmt.vox
git add Cargo.toml Cargo.lock crates/vox-attention crates/vox-orchestrator crates/vox-orchestrator-mcp/src/server_state.rs \
  docs/src/architecture/layers.toml docs/src/architecture/where-things-live.md contracts/ci/crate-graph.v1.json \
  contracts/toestub contracts/telemetry contracts/orchestration/permission-modes.v1.yaml
git commit -m "refactor(attention): extract vox-attention from vox-orchestrator behind BudgetSource"
```

---

### Task 5: Rewrite attention call sites; delete the attention shim

**Files:** Modify `crates/vox-orchestrator/src/**` (25 files from `target/cp05/attention-inbound.txt` plus
`lib.rs`), `crates/vox-orchestrator-mcp/{Cargo.toml, src/**, tests/**}` (17 files), `crates/vox-cli/{Cargo.toml,
src/commands/safety.rs}`, and `crates/vox-orchestrator/tests/**` where `rg` finds old paths.

**Interfaces:** After this task there is no `vox_orchestrator::{attention, budget, budget_gate, fatigue_monitor}`
module and no top-level `vox_orchestrator::{BudgetManager, ApprovalTier, …}` re-export. Every consumer uses
`vox_attention::…`. Consumes Task 4 and Task 3 (edges #5–6). Not parallel-safe with Task 6, which edits the same
Cargo.toml and lib.rs files.

- [ ] **Step 1: Red — the zero-old-path check fails today.**

```bash
ATT_RE='\bvox_orchestrator::(attention|budget|budget_gate|fatigue_monitor)\b|\bvox_orchestrator::(ActionDescriptor|AgentBudgetAllocation|AgentTrustScore|ApprovalOutcome|ApprovalTier|AttentionBudget|AttentionEvent|AttentionEventType|BudgetManager|BudgetSignal|DEFAULT_ATTENTION_BUDGET_MS|DEFAULT_INTERRUPT_COST_MS|FocusDepth|InterruptionChannel|InterruptionDecision|InterruptionSignals|NasaTlxWeights|TierGateConfig|TrustTier|classify_tier|compute_attention_cost_ms|decision_entropy_bits|evaluate_interruption|scaled_interrupt_cost_ms)\b'
rg -n "$ATT_RE" crates --glob '*.rs' | rg -v 'target:\s*"' | wc -l
rg -n '\bcrate::(attention|budget|budget_gate|fatigue_monitor)\b' crates/vox-orchestrator/src crates/vox-orchestrator/tests | wc -l
```

  Expected: both counts are non-zero (red).

- [ ] **Step 2: Delete the shim** (the two `vox-deprecated-since … canonical="vox_attention"` blocks in
  `crates/vox-orchestrator/src/lib.rs`). Also delete the attention/budget lines of `extraction_old_paths_resolve`
  (keep the models/catalog lines for Task 7).

- [ ] **Step 3: Mechanical first pass.**

```bash
perl -pi -e 's/\bcrate::(attention|budget|budget_gate|fatigue_monitor)::/vox_attention::$1::/g' $(rg -l '\bcrate::(attention|budget|budget_gate|fatigue_monitor)::' crates/vox-orchestrator/src crates/vox-orchestrator/tests)
perl -pi -e 's/\bvox_orchestrator::(attention|budget|budget_gate|fatigue_monitor)::/vox_attention::$1::/g' $(rg -l '\bvox_orchestrator::(attention|budget|budget_gate|fatigue_monitor)::' crates --glob '*.rs')
perl -pi -e 's/\bvox_orchestrator::(ActionDescriptor|AgentBudgetAllocation|AgentTrustScore|ApprovalOutcome|ApprovalTier|AttentionBudget|AttentionEvent|AttentionEventType|BudgetManager|BudgetSignal|FocusDepth|InterruptionChannel|InterruptionDecision|InterruptionSignals|NasaTlxWeights|TierGateConfig|TrustTier|classify_tier|compute_attention_cost_ms|decision_entropy_bits|evaluate_interruption|scaled_interrupt_cost_ms)\b/vox_attention::$1/g' $(rg -l "$ATT_RE" crates --glob '*.rs')
```

- [ ] **Step 4: Dependencies.** In `crates/vox-orchestrator-mcp/Cargo.toml` `[dependencies]` and
  `crates/vox-cli/Cargo.toml` `[dependencies]`, add `vox-attention.workspace = true`. If
  `crates/vox-orchestrator/tests` now names `vox_attention`, nothing more is needed: integration tests can use the
  package's normal dependencies.

- [ ] **Step 5: Compiler-guided remainder.** Grouped imports such as `use vox_orchestrator::{BudgetManager,
  Orchestrator};` and `use crate::{budget::X, …}` are not caught by Step 3. Iterate until clean:

```bash
cargo check -p vox-orchestrator --all-targets --features populi-transport,test-support 2>&1 | rg -A3 '^error' | head -60
cargo check -p vox-orchestrator-mcp --all-targets --all-features 2>&1 | rg -A3 '^error' | head -60
cargo check -p vox-cli --all-targets 2>&1 | rg -A3 '^error' | head -60
```

  Split each grouped import. The attention names come from `vox_attention::…`; the rest stay on `vox_orchestrator::…`.

- [ ] **Step 6: Green — the zero-old-path check.**

```bash
rg -n "$ATT_RE" crates --glob '*.rs' | rg -v 'target:\s*"'
rg -n '\bcrate::(attention|budget|budget_gate|fatigue_monitor)\b' crates/vox-orchestrator/src crates/vox-orchestrator/tests
rg -n 'vox-deprecated-since=.*canonical="vox_attention"' crates
```

  Expected: all three print nothing.

- [ ] **Step 7: Verify.**

```bash
cargo nextest run -p vox-attention
cargo nextest run -p vox-orchestrator
cargo nextest run -p vox-orchestrator-mcp -E 'test(/mcp_context|attention_policy|pending_approvals|budget/) | binary(pending_approvals_tests) | binary(hitl_rehydrate_on_restart_tests) | binary(daemon_extra_tests) | binary(autonomous_tool_dispatch_tests)'
cargo nextest run -p vox-cli -E 'test(/commands::safety/)'
cargo clippy -p vox-orchestrator -p vox-orchestrator-mcp -p vox-cli --all-targets -- -D warnings
vox ci crate-edges
vox ci fan-in-budget
cargo run -q -p vox-arch-check
```

  Expected: the pass/fail sets match `target/cp05/baseline-{orchestrator,mcp,cli}.txt`. `crate-edges` passes with
  the authorized edges #5–6, or fails listing exactly those two if the user has not applied them yet.

- [ ] **Step 8: Commit.**

```bash
vox run scripts/fmt.vox
git add -A crates/vox-orchestrator crates/vox-orchestrator-mcp crates/vox-cli Cargo.lock
git commit -m "refactor(attention): call vox-attention directly; drop vox_orchestrator attention re-exports"
```

---

### Task 6: Extract `vox-models` (with a one-task re-export shim)

**Files:** Create `crates/vox-models/{Cargo.toml, tests/public_surface.rs}`. Move every file in
`target/cp05/models-move-list.txt` plus `crates/vox-orchestrator/build.rs`. Modify the root `Cargo.toml`,
`crates/vox-orchestrator/{Cargo.toml, src/lib.rs}`, the files listed in Steps 5 and 9, and
`docs/src/architecture/{layers.toml, where-things-live.md}`.

**Interfaces:** Produces the crate `vox_models`. Its root is the old `models` module (same `pub use` surface), plus
`pub mod route_policy, catalog, catalog_classifier, catalog_live`. Features: `runtime`, `populi-transport`,
`test-support`, each forwarded from the same-named `vox-orchestrator` feature. Shim: `vox_orchestrator::models` (an
alias of the crate) and `vox_orchestrator::{route_policy, catalog, catalog_classifier, catalog_live}` keep resolving
until Task 7. `mesh_directory::vox_dir` becomes `pub`. Consumes Tasks 1–3 (edges #7–21, the removal of two
orchestrator edges, Option A/B).

- [ ] **Step 1: Scaffold and write the failing surface test.**

  Root `Cargo.toml` `[workspace.dependencies]`:

```toml
vox-models                = { path = "crates/vox-models" }
```

  `crates/vox-models/Cargo.toml` (Option A; for Option B delete the `vox-populi` line and `dep:vox-populi` from
  `runtime`):

```toml
[package]
name = "vox-models"
description = "Model registry, selection, scoring, routing policy and provider catalogs. Extracted from vox-orchestrator."
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true

[features]
default = []
# `llm_config_for_spec` (vox-actor-runtime LlmConfig) and the macOS Metal VRAM probe.
runtime = ["dep:vox-populi", "vox-populi/mens"]
# Populi mesh peers in the registry and scoring.
populi-transport = ["dep:vox-mesh-transport", "dep:iroh"]
# `key_guard::set_test_key_availability` and `route_policy::set_test_privacy_override` for other crates' tests.
test-support = []

[dependencies]
anyhow.workspace = true
async-trait = { workspace = true }
dirs = { workspace = true }
reqwest = { workspace = true, features = ["json"] }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }
vox-actor-runtime = { workspace = true }
vox-bounded-fs.workspace = true
vox-config.workspace = true
vox-db.workspace = true
vox-http-client.workspace = true
vox-mesh-policy = { workspace = true }
vox-mesh-types.workspace = true
vox-orchestrator-models.workspace = true
vox-orchestrator-types.workspace = true
vox-repository.workspace = true
vox-secrets.workspace = true
vox-telemetry.workspace = true
vox-mesh-transport = { workspace = true, optional = true }
iroh = { workspace = true, optional = true }
vox-populi = { path = "../vox-populi", optional = true, default-features = false }
workspace-hack = { version = "0.6", path = "../workspace-hack" }

[build-dependencies]
serde_yaml = { workspace = true }
serde = { workspace = true, features = ["derive"] }

[dev-dependencies]
serial_test = { workspace = true }
tempfile = { workspace = true }
tokio = { workspace = true, features = ["full", "test-util"] }
vox-test-harness = { path = "../vox-test-harness" }

[lints]
workspace = true
```

  `vox-actor-runtime` is non-optional because `route_policy.rs` uses it unconditionally today. In
  `vox-orchestrator` it is "optional" but always on via `runtime` (default) and `vox-orchestrator-mcp`'s explicit
  `features = ["runtime"]`.

  `crates/vox-models/tests/public_surface.rs`:

```rust
use vox_models::{ModelRegistry, ProviderType, RoutingTask, TaskCategory};

#[test]
fn task_category_wire_format_is_pascal_case_and_ordered() {
    assert_eq!(serde_json::to_string(&TaskCategory::CodeGen).unwrap(), "\"CodeGen\"");
    assert_eq!(TaskCategory::CodeGen.to_string(), "CodeGen");
    assert_eq!("codegen".parse::<TaskCategory>(), Ok(TaskCategory::CodeGen));
    assert_eq!(TaskCategory::default(), TaskCategory::General);
    assert!(TaskCategory::CodeGen < TaskCategory::General);
}

#[test]
fn moved_modules_are_reachable() {
    let _ = ModelRegistry::default();
    assert!(vox_models::route_policy::is_local_http_provider(&ProviderType::Ollama));
    let _: Option<vox_models::catalog::OpenRouterCatalog> = None;
    let _ = RoutingTask::default().estimated_token_count();
}
```

  Create `crates/vox-models/src/lib.rs` containing only `//! placeholder until Step 3 moves models/mod.rs here.`.
  Step 3's `git mv` overwrites it; delete the file first with `rm crates/vox-models/src/lib.rs` right before the
  move.

- [ ] **Step 2: Run to verify failure.**

```bash
cargo test -p vox-models --test public_surface 2>&1 | tail -8
```

  Expected: `error[E0432]: unresolved imports vox_models::ModelRegistry …`.

- [ ] **Step 3: Move with history (list computed at execution time).**

```bash
xargs rg -o --no-filename '^\s*(pub(\([a-z]+\))? )?(async )?(fn|struct|enum|trait|type|const|static) [A-Za-z_][A-Za-z0-9_]*' \
  < target/cp05/models-move-list.txt | sed -E 's/.* //' | sort | uniq -c > target/cp05/models-items-before.txt
rm crates/vox-models/src/lib.rs
git mv crates/vox-orchestrator/src/models/mod.rs crates/vox-models/src/lib.rs
for f in $(git ls-files crates/vox-orchestrator/src/models); do git mv "$f" "crates/vox-models/src/${f#crates/vox-orchestrator/src/models/}"; done
for f in route_policy catalog catalog_classifier catalog_live; do git mv crates/vox-orchestrator/src/$f.rs crates/vox-models/src/$f.rs; done
git mv crates/vox-orchestrator/build.rs crates/vox-models/build.rs
test -z "$(git ls-files crates/vox-orchestrator/src/models)" && echo MODELS_DIR_EMPTY
git status --short | rg -c '^R'   # expected: models-move-list count + 1 (build.rs)
```

- [ ] **Step 4: Flatten the paths inside the moved files.**

```bash
F=$(git ls-files crates/vox-models/src)
perl -pi -e 's/\bcrate::models::/crate::/g' $F
perl -pi -e 's/\bcrate::config::CostPreference\b/vox_orchestrator_types::CostPreference/g; s/\bcrate::types::FreeRoutingProfile\b/vox_orchestrator_types::RoutingProfile/g; s/\bcrate::types::TaskCategory\b/crate::TaskCategory/g; s/\bcrate::mode::ClutchProfile\b/vox_orchestrator_types::clutch::ClutchProfile/g; s/\bcrate::usage::/vox_orchestrator_models::usage::/g; s/\bcrate::calibration::/vox_orchestrator_models::calibration::/g' $F
perl -pi -e 's{include_str!\("\.\./\.\./\.\./vox-cli/}{include_str!("../../vox-cli/}g; s{include_str!\("\.\./\.\./\.\./\.\./contracts/}{include_str!("../../../contracts/}g' $F
rg -n 'crate::(types|mode|config|usage|calibration|models)\b' crates/vox-models/src
```

  Fix the remaining hits by hand:
  - `use crate::types::{AgentTask, …}` in test modules is handled in Step 5.
  - `use crate::types::FreeRoutingProfile as P;` (`policy.rs`) becomes `use vox_orchestrator_types::RoutingProfile as P;`.
  - The `use crate::config::CostPreference;` lines in tests become `use vox_orchestrator_types::CostPreference;`.
  - `build.rs` needs no edit: `../../contracts/orchestration/model-routing.v1.yaml` resolves from `crates/vox-models`.

- [ ] **Step 5: Replace the `AgentTask` uses (all pinned by Task 1).**
  - `health.rs`, the non-test probe (today ~line 72): replace the five lines that build `task` and the line
    `let routing_task = super::RoutingTask::from(&task);` with:

```rust
    let routing_task = super::RoutingTask {
        category: crate::TaskCategory::CodeGen,
        complexity: 10,
        ..Default::default()
    };
```

  - `leaderboard.rs` (WIP, ~line 64): replace the `probe` construction with
    `let probe = super::RoutingTask { category, complexity: complexity.clamp(1, 10), ..Default::default() };`. After
    Task 0 Step 2, `best_for_task_under_gate` takes `&RoutingTask`. If it still takes `&AgentTask`, Task 0 Step 2
    should have stopped.
  - `mode_select.rs` tests: `hard_task()` becomes
    `RoutingTask { category: TaskCategory::CodeGen, complexity: 10, ..Default::default() }`. Delete
    `use crate::types::{AgentTask, TaskCategory, TaskId, TaskPriority};` and import `crate::TaskCategory`.
  - `ranking.rs` test case 3: replace the `AgentTask` + `Budget` lines with
    `let rt = RoutingTask { max_cost_usd: Some(0.001), ..Default::default() };`. Delete the
    `use crate::types::{AgentTask, Budget, TaskId, TaskPriority};` line.
  - `routing_task.rs` tests: delete `routing_task_from_agent_task_carries_category_and_complexity`. Task 1's
    `routing_probe_tasks_equal_plain_routing_task_literals` in `types/tasks.rs` covers it. In
    `best_for_task_with_routing_task_matches_agent_task`, replace the `AgentTask` setup and
    `let rt: RoutingTask = (&task).into();` with
    `let rt = RoutingTask { category: TaskCategory::CodeGen, complexity: 10, ..Default::default() };`. Keep the
    test name and assertion. Fix the imports to `use crate::{…, RoutingTask, TaskCategory}` and
    `use vox_orchestrator_types::CostPreference;`.
  - Confirm: `rg -n 'AgentTask|TaskPriority|TaskId\b' crates/vox-models/src` prints only doc comments, or nothing.

- [ ] **Step 6: `lib.rs` crate docs, the four new modules, `vox_dir` visibility.**

  `crates/vox-models/src/lib.rs` becomes (the three WIP lines `adaptation`, `bench_evidence`, `leaderboard` come
  from the model-bench merge; keep every `pub use` exactly as the merged `models/mod.rs` has it):

```rust
//! Model registry, selection, scoring, routing policy and provider catalogs.
//!
//! Every LLM call picks its model here (AGENTS.md §Model-Agnostic LLM Boundary): `select`
//! and `decide` rank [`ModelRegistry`] entries on the same axes for every provider, MENS
//! revisions included. `ModelTier`, `StrengthTag`, `TaskCategory`, `Capability` and
//! `PromptIntent` are generated by `build.rs` from
//! `contracts/orchestration/model-routing.v1.yaml`. Usage accounting and calibration live
//! in `vox-orchestrator-models`.

pub mod adaptation;
pub mod admission;
pub mod auto_select;
pub mod autonomic;
pub mod bench_evidence;
pub mod catalog;
pub mod catalog_classifier;
pub mod catalog_live;
pub mod cost_tier;
pub mod discovery_pipeline;
pub mod family;
pub mod generated;
pub mod health;
pub mod key_guard;
pub mod leaderboard;
#[cfg(feature = "populi-transport")]
pub mod mesh_directory;
pub mod mode_select;
mod pareto;
pub mod policy;
pub mod prompt_profiles;
pub mod provenance;
pub mod ranking;
pub mod reference;
mod registry;
pub mod route_policy;
pub mod routing_table;
pub mod routing_task;
pub mod scoring;
pub mod select;
pub mod spec;
#[cfg(test)]
mod tests;
pub mod tiering;
pub mod vram;

pub use auto_select::{
    AutoModelSelection, detect_hardware_vram_gb, is_host_apple_silicon, select_optimal_local_model,
    select_optimal_local_model_for_host, select_tier_for_vram,
};
pub use cost_tier::{
    CHEAP_COST_PER_1K_USD, CostTier, blended_cost_per_1k, cost_tier_for, cost_tier_for_blended,
};
pub use generated::{
    Capability, CapabilityFlags, ModelTier, PromptIntent, StrengthTag, TaskCategory,
    infer_capabilities, infer_prompt_intents, intent_required_capabilities,
};
pub use pareto::{ParetoPoint, is_observed, pareto_frontier, pareto_point_for};
pub use policy::{
    FallbackCondition, PolicyContext, SelectionAxisKind, SelectionPolicy, SelectionStep,
    active_policy, install_active_policy, policy_for_profile, resolve_policy,
};
#[cfg(feature = "runtime")]
pub use registry::llm_config_for_spec;
pub use registry::{
    MIN_CALLS_FOR_CONFIDENT_RANK, ModelRegistry, ModelScore, wilson_score_interval,
};
pub use routing_task::RoutingTask;
pub use scoring::install_base_routing_priority;
pub use select::{
    CandidateScope, ModelSelectionDecision, ModelSelectionRequest, ScoreBreakdown, SelectionAxes,
    SelectionIntent, SelectionOutcome, SelectionReason, decide, select,
    select_with_default_registry, select_with_policy,
};
pub use spec::{
    ModelCapabilities, ModelConfig, ModelRouteBackend, ModelSpec, PricingSource, ProviderType,
    route_backend_for_model, task_category_premium_key, task_category_strength,
};
pub use vram::{
    VramFit, VramProbe, estimate_vram_fit, free_vram_mb_hint, refresh_free_vram_hint_from_nvml,
    register_vram_probe, registered_vram_probe,
};
```

  `mesh_directory.rs`: change `pub(crate) fn vox_dir()` to `pub fn vox_dir()` and add the doc line
  `/// User-level `.vox` directory holding `mesh.key`, `mesh_outbox` and `mesh_trust.json`.`. Add this test to its
  existing `mod tests` (today line 77):

```rust
    #[test]
    fn vox_dir_is_the_configured_user_dot_vox_dir() {
        assert_eq!(super::vox_dir(), vox_config::paths::dot_vox_user_dir());
    }
```

- [ ] **Step 7: Run the crate.**

```bash
cargo check -p vox-models --all-targets
cargo check -p vox-models --all-targets --features runtime,populi-transport,test-support
cargo nextest run -p vox-models --features runtime,test-support
cargo nextest list -p vox-models --features runtime,test-support --message-format oneline 2>/dev/null | wc -l
```

  Expected: all pass. The count equals `count-models-tests-before` minus 1 (the deleted conversion test), plus 2
  (`public_surface`), plus 1 (`vox_dir` test). Compare it with `target/cp05/count-models-tests-before.txt`. A test
  name gated by `#[cfg(all(test, feature = "populi-transport"))]` (`scoring.rs:1230`) also needs
  `--features populi-transport` to be counted.

- [ ] **Step 8: Orchestrator side — dependency, features, shim, dropped edges.**

  `crates/vox-orchestrator/Cargo.toml`:
  - `[dependencies]`: add `vox-models = { workspace = true }`.
  - Delete the whole `[build-dependencies]` table; `build.rs` moved.
  - Features: append `"vox-models/runtime"` to `runtime`, `"vox-models/populi-transport"` to `populi-transport`,
    and set `test-support = ["vox-models/test-support"]`.
  - Run `rg -n '\bvox_http_client::|\bvox_mesh_policy::' crates/vox-orchestrator/src crates/vox-orchestrator/tests`.
    If it prints nothing, delete the `vox-http-client.workspace = true` and `vox-mesh-policy = { workspace = true }`
    lines (the removed edges in Task 3). If it prints anything, keep the line and remove that edge from Task 3's
    "removed" list.
  - `[dev-dependencies]`: add `vox-models = { workspace = true, features = ["test-support"] }`.

  `crates/vox-orchestrator/src/lib.rs`:
  - Delete `pub mod route_policy;`, `pub mod catalog;`, `pub mod catalog_classifier;`, `pub mod catalog_live;` and
    `pub mod models;` with their doc comments.
  - Add:

```rust
// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="capability-profiles-05" canonical="vox_models"
pub use vox_models as models;
// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="capability-profiles-05" canonical="vox_models"
pub use vox_models::{catalog, catalog_classifier, catalog_live, route_policy};
```

  `types/tasks.rs:208` `pub use crate::models::generated::TaskCategory;` keeps compiling through the alias, and so
  do `impl From<&AgentTask> for crate::models::RoutingTask` and `a2a/mesh_relay.rs`'s
  `crate::models::mesh_directory::{endpoint, vox_dir}`.

- [ ] **Step 9: Path readers and path-keyed baselines (gates break without this).**

```bash
P='vox-orchestrator(/src/|,src/)(models/|route_policy\.rs|catalog(_classifier|_live)?\.rs)|vox-orchestrator/build\.rs'
rg -n "$P" crates contracts --glob '*.{rs,json,yaml,txt,csv,toml}'
```

  Rewrite each hit (keys only, no new entries):
  - `crates/vox-cli-ci/src/model_routing_check.rs:76`: `"crates/vox-orchestrator/src/models/scoring.rs"` becomes
    `"crates/vox-models/src/scoring.rs"`.
  - `crates/vox-drift-check/src/rules/timeout_literal.rs:79`: `"crates/vox-orchestrator/src/catalog.rs"` becomes
    `"crates/vox-models/src/catalog.rs"`.
  - `crates/vox-cli/src/commands/ci/command_compliance/validators.rs:504`: `"crates/vox-orchestrator/src/models/spec.rs"`
    becomes `"crates/vox-models/src/spec.rs"`.
  - `contracts/toestub/weak-test-baseline.v1.json` (5 models entries) and `silent-drop-baseline.v1.json` (1):
    `**/crates/vox-orchestrator/src/models/<f>` becomes `**/crates/vox-models/src/<f>`.
  - `contracts/config/config-hygiene-baseline.txt` (3 lines): `crates/vox-orchestrator/src/models/select.rs`
    becomes `crates/vox-models/src/select.rs`.
  - `contracts/telemetry/emit-site-inventory.csv` / `default-decision-sites.csv`: `vox-orchestrator,src/models/<f>`
    becomes `vox-models,src/<f>`, and `vox-orchestrator,src/catalog.rs` becomes `vox-models,src/catalog.rs`.
  - `contracts/orchestration/tenant-budgets.v1.yaml:28` (prose) and the doc-comment mentions in
    `vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:57` and
    `vox-integration-tests/tests/chat_harness_regression_test.rs:182,195,260`: repoint to `crates/vox-models/src/…`.

- [ ] **Step 10: Register the crate.**

  `docs/src/architecture/layers.toml`, after `vox-orchestrator-models`:

```toml
vox-models              = { layer = 3 }                          # model registry/selection/catalogs extracted from vox-orchestrator (capability-profiles plan 05)
```

  `docs/src/architecture/where-things-live.md`: add the crate row after `vox-attention`:

```markdown
| [`vox-models`](../../../crates/vox-models/) | Model registry, selection (`select`, `decide`, selection policies), scoring and ranking, routing health, provider catalogs (`catalog`, `catalog_live`, `catalog_classifier`), `route_policy`, and the `ModelTier` / `StrengthTag` / `TaskCategory` enums generated from `contracts/orchestration/model-routing.v1.yaml`. Every LLM call site selects its model here. |
```

  Repoint the concept rows to `crates/vox-models/src/reference.rs`, `crates/vox-models/src/ranking.rs` and
  `crates/vox-models/src/health.rs`. Change "Inference tier/modality config for routing (`InferenceConfig`,
  `TierProfile`)" to `crates/vox-orchestrator-types/src/clutch.rs`. In the `vox-orchestrator-models` row, replace
  "`models/` and the provider catalogs follow in B4" with "`models/` and the provider catalogs moved to
  `vox-models`". In the `vox-orchestrator` row, replace the `ModelTier … generated from …` sentence with "model
  tiers and routing enums: see `vox-models`".

- [ ] **Step 11: Verify.**

```bash
git ls-files crates/vox-models/src | xargs rg -o --no-filename '^\s*(pub(\([a-z]+\))? )?(async )?(fn|struct|enum|trait|type|const|static) [A-Za-z_][A-Za-z0-9_]*' \
  | sed -E 's/.* //' | sort | uniq -c > target/cp05/models-items-after.txt
diff target/cp05/models-items-before.txt target/cp05/models-items-after.txt
for f in route_policy.rs catalog.rs catalog_classifier.rs catalog_live.rs build.rs; do test ! -e crates/vox-orchestrator/src/$f -a ! -e crates/vox-orchestrator/$f || echo "LEFTOVER $f"; done
cargo check -p vox-orchestrator --all-targets --features populi-transport,test-support
cargo check -p vox-orchestrator-mcp --all-targets --all-features
cargo check -p vox-research-shim -p vox-cli -p vox-orchestrator-d --all-targets
cargo check -p vox-gui --all-targets   # see AGENTS.md fresh-worktree note if tauri-build fails
cargo nextest run -p vox-models --features runtime,populi-transport,test-support
cargo nextest run -p vox-orchestrator
cargo nextest run -p vox-cli-ci -E 'test(/model_routing_check/)'
cargo nextest run -p vox-drift-check -E 'test(/timeout_literal/)'
cargo clippy -p vox-models -p vox-orchestrator --all-targets --all-features -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc -p vox-models --no-deps
cargo run -q -p vox-arch-check
vox ci crate-edges
vox ci fan-in-budget
vox ci affected-crates --regen --out contracts/ci/crate-graph.v1.json
vox ci ssot-drift
vox ci crate-build-map-parity
```

  Expected:
  - The `diff` shows only `vox_dir`'s unchanged name, the new test fns, and the deleted
    `routing_task_from_agent_task_carries_category_and_complexity`. **Any name whose count rose is a duplicate:
    STOP.**
  - No `LEFTOVER`.
  - The orchestrator suite matches `baseline-orchestrator.txt`. The moved tests now run under `vox-models`, so the
    orchestrator test count drops by `count-models-tests-before` and rises by Task 1's pins (already counted).
  - `crate-edges` passes with edges #7–21 authorized. Otherwise it fails listing exactly those and reports the two
    removed edges as `stale baseline edge` warnings, which Task 8 tightens. **Anything else: STOP.**
  - `fan-in-budget` passes once the user has applied the snapshot bumps. Otherwise it fails naming exactly
    `vox-bounded-fs`, `vox-config`, `vox-db`, `vox-secrets`.
  - If `crate-build-map-parity` fails only because the two new crates have no measured `compile_s`, **STOP and
    report**. That SSOT is produced by its measurement workflow; never write invented numbers into
    `crate-build-map.v1.json`.

- [ ] **Step 12: Mutation proof of feature forwarding.** Remove `"vox-models/test-support"` from
  `vox-orchestrator`'s `test-support`. `cargo check -p vox-orchestrator-mcp --tests` must fail on
  `set_test_privacy_override` (`llm_bridge/local_health.rs:87`). Restore, confirm with
  `rg -c 'vox-models/test-support' crates/vox-orchestrator/Cargo.toml` (prints `1`), and re-check.

- [ ] **Step 13: Commit.**

```bash
vox run scripts/fmt.vox
git add -A Cargo.toml Cargo.lock crates/vox-models crates/vox-orchestrator crates/vox-cli-ci/src/model_routing_check.rs \
  crates/vox-drift-check/src/rules/timeout_literal.rs crates/vox-cli/src/commands/ci/command_compliance/validators.rs \
  crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs crates/vox-integration-tests/tests/chat_harness_regression_test.rs \
  contracts/toestub contracts/config/config-hygiene-baseline.txt contracts/telemetry contracts/orchestration/tenant-budgets.v1.yaml \
  contracts/ci/crate-graph.v1.json docs/src/architecture/layers.toml docs/src/architecture/where-things-live.md
git commit -m "refactor(models): extract vox-models (registry, selection, catalogs, route_policy) from vox-orchestrator"
```

---

### Task 7: Rewrite model call sites; switch `vox-research-shim`; delete the models shim

**Files:** Modify `crates/vox-orchestrator/src/**` (21 files + `types/tasks.rs` + `lib.rs` +
`orchestrator/catalog_refresh.rs` + `a2a/mesh_relay.rs`), `crates/vox-orchestrator/tests/**`,
`crates/vox-orchestrator-mcp/{Cargo.toml, src/**}` (37 files), `crates/vox-cli/{Cargo.toml, src/**}` (16),
`crates/vox-gui/{Cargo.toml, src/commands/{models,routing_explain,llm_settings,harness_issues}.rs}`,
`crates/vox-orchestrator-d/{Cargo.toml, src/bin/vox_orchestrator_d.rs}`,
`crates/vox-research-shim/{Cargo.toml, src/**}` (13), `crates/vox-integration-tests/{Cargo.toml,
tests/chat_harness_regression_test.rs}`.

**Interfaces:** After this task there is no `vox_orchestrator::{models, route_policy, catalog,
catalog_classifier, catalog_live}`. `vox-research-shim::selection` depends on `vox_models` and
`vox_orchestrator_types::clutch`. Consumes Task 6 and Task 3 (edges #22–26).

- [ ] **Step 1: Red — the zero-old-path check fails today.**

```bash
MOD_RE='\bvox_orchestrator::(models|route_policy|catalog|catalog_classifier|catalog_live)\b'
rg -n "$MOD_RE" crates --glob '*.rs' | rg -v 'target:\s*"' | wc -l
rg -n '\bcrate::(models|route_policy|catalog|catalog_classifier|catalog_live)\b' crates/vox-orchestrator/src crates/vox-orchestrator/tests | wc -l
```

  Expected: non-zero.

- [ ] **Step 2: Delete the shim** (both `canonical="vox_models"` lines in `lib.rs`) and the `extraction_old_paths_resolve`
  test. `vox-models/tests/public_surface.rs` replaces it.

- [ ] **Step 3: Mechanical first pass.**

```bash
perl -pi -e 's/\bcrate::models::/vox_models::/g; s/\bcrate::(route_policy|catalog|catalog_classifier|catalog_live)::/vox_models::$1::/g' $(rg -l '\bcrate::(models|route_policy|catalog|catalog_classifier|catalog_live)::' crates/vox-orchestrator/src crates/vox-orchestrator/tests)
perl -pi -e 's/\bvox_orchestrator::models::/vox_models::/g; s/\bvox_orchestrator::(route_policy|catalog|catalog_classifier|catalog_live)::/vox_models::$1::/g' $(rg -l "$MOD_RE" crates --glob '*.rs')
```

  Then fix these by hand:
  - `crates/vox-orchestrator/src/types/tasks.rs:208` becomes `pub use vox_models::TaskCategory;`. The `From` impls
    become `impl From<&AgentTask> for vox_models::RoutingTask` / `impl From<AgentTask> for vox_models::RoutingTask`.
  - `use crate::models;`, `use crate::{models::X, …}` and `use vox_orchestrator::{models::X, …}` groups are found by
    the compiler in Step 5.

- [ ] **Step 4: `vox-research-shim` selection switch.** In `src/selection/{free_tier,scorer,task_routing,
  virtual_models,tests}.rs`:
  - `use vox_orchestrator::config::CostPreference;` becomes `use vox_orchestrator_types::CostPreference;`.
  - `use vox_orchestrator::mode::{ClutchProfile, InferenceConfig};`, `vox_orchestrator::mode::ResolvedClutch`,
    `QualityLevel` and `TierProfile` become `vox_orchestrator_types::clutch::…`.
  - `use vox_orchestrator::types::TaskCategory;` becomes `use vox_models::TaskCategory;`.
  - `use vox_orchestrator::types::{RoutingProfile, TaskCategory};` becomes
    `use vox_orchestrator::types::RoutingProfile; use vox_models::TaskCategory;`.

  Leave unchanged (Deviation 6): `selection/mod.rs` `pub use vox_orchestrator::registry_model_resolve::{…}`,
  `research/orchestrator/config.rs` `vox_orchestrator::services::embeddings::EmbeddingService`, `pipeline.rs`
  `vox_orchestrator::MemoryConfig`, and every `target: "vox_orchestrator::…"` tracing string.

- [ ] **Step 5: Dependencies, then compiler-guided remainder.**
  - `[dependencies]`: add `vox-models = { workspace = true }` to `vox-orchestrator-mcp`, `vox-cli`, `vox-gui`,
    `vox-orchestrator-d` and `vox-research-shim`.
  - `[dev-dependencies]`: add `vox-models = { workspace = true, features = ["test-support"] }` to
    `vox-orchestrator-mcp` (`local_health.rs` seam), `vox-gui` (`routing_explain.rs:479` seam) and
    `vox-integration-tests`.

```bash
cargo check -p vox-orchestrator --all-targets --features populi-transport,test-support 2>&1 | rg -A3 '^error' | head -60
cargo check -p vox-orchestrator-mcp --all-targets --all-features 2>&1 | rg -A3 '^error' | head -60
cargo check -p vox-cli -p vox-orchestrator-d -p vox-research-shim --all-targets 2>&1 | rg -A3 '^error' | head -60
cargo check -p vox-research-shim --all-targets --all-features 2>&1 | rg -A3 '^error' | head -40
cargo check -p vox-gui --all-targets 2>&1 | rg -A3 '^error' | head -40
cargo check -p vox-integration-tests --tests 2>&1 | rg -A3 '^error' | head -40
```

  Iterate until clean. Split grouped imports. Doc-comment links like
  `` [`vox_orchestrator::models::ModelRegistry`] `` in `vox-research-shim/src/research/model_select.rs:1-3` become
  `` [`vox_models::ModelRegistry`] ``.

- [ ] **Step 6: Green — the zero-old-path check.**

```bash
rg -n "$MOD_RE" crates --glob '*.rs' | rg -v 'target:\s*"'
rg -n '\bcrate::(models|route_policy|catalog|catalog_classifier|catalog_live)\b' crates/vox-orchestrator/src crates/vox-orchestrator/tests
rg -n 'vox-deprecated-since=.*capability-profiles-05' crates
rg -n 'vox_orchestrator::(config::CostPreference|mode::|models::|types::TaskCategory)' crates/vox-research-shim/src/selection
```

  Expected: all four print nothing. Doc-only mentions in `vox-actor-runtime/src/model_resolution.rs` and
  `vox-orchestrator-models/src/calibration.rs` match the first regex too. Rewrite them to `vox_models::…`; they are
  prose, not edges.

- [ ] **Step 7: Verify (behaviour preservation).**

```bash
cargo nextest run -p vox-models --features runtime,populi-transport,test-support
cargo nextest run -p vox-orchestrator
cargo nextest run -p vox-orchestrator-mcp -E 'test(/llm_bridge|model_route_policy|chat_model_resolve|visus_review|vram_probe|mcp_context|attention_policy|pending_approvals|budget/) | binary(pending_approvals_tests) | binary(hitl_rehydrate_on_restart_tests) | binary(daemon_extra_tests) | binary(autonomous_tool_dispatch_tests)'
cargo nextest run -p vox-research-shim
cargo nextest run -p vox-cli -E 'test(/commands::model::|commands::safety|commands::chat|orchestration_audit|audit_route|model_catalog/)'
cargo nextest run -p vox-gui -E 'test(/routing_explain|llm_settings|commands::models/)'
cargo nextest run -p vox-integration-tests -E 'binary(chat_harness_regression_test)'
cargo check -p vox-orchestrator-d
cargo clippy -p vox-models -p vox-orchestrator -p vox-orchestrator-mcp -p vox-research-shim -p vox-cli -p vox-orchestrator-d --all-targets -- -D warnings
vox ci crate-edges
vox ci fan-in-budget
cargo run -q -p vox-arch-check
```

  Expected: each pass/fail set matches its `target/cp05/baseline-*.txt` (with the `models::` test-name prefix
  dropped). `crate-edges` passes with #22–26 authorized, or lists exactly those.

- [ ] **Step 8: Commit.**

```bash
vox run scripts/fmt.vox
git add -A crates Cargo.lock
git commit -m "refactor(models): call vox-models directly; research-shim selection uses vox-models; drop vox_orchestrator::models re-exports"
```

---

### Task 8: Tighten, record deviations, sweep docs, final verification and blast-radius report

**Files:** Modify `contracts/ci/crate-edges.allow.v1.json` (only through `--tighten`, removal-only),
`docs/superpowers/plans/2026-10-08-capability-profiles/00-contracts.md`,
`docs/superpowers/specs/2026-10-08-capability-profiles-design.md` (§5 note),
`docs/src/architecture/rust-build-loop-ssot-2026.md` ("Part B status"), and `docs/src/**` files linking moved paths.

**Interfaces:** Consumes Tasks 4–7. Produces the final state and `target/cp05/crate-map-after.txt`.

- [ ] **Step 1: Tighten removed edges** (allowed without authorization, removal-only):

```bash
vox ci crate-edges --tighten
git diff --stat contracts/ci/crate-edges.allow.v1.json
git diff contracts/ci/crate-edges.allow.v1.json | rg '^[-+]\s+"' | head
```

  Expected: only `-` lines, for `["vox-orchestrator","vox-http-client"]` and `["vox-orchestrator","vox-mesh-policy"]`.
  **If any `+` line appears, `git checkout -- contracts/ci/crate-edges.allow.v1.json` and STOP.**

- [ ] **Step 2: Record the deviations.** In `00-contracts.md`, under the existing "Deviation from spec §5" paragraph,
  add a "Plan 05 deviations" bullet list that copies Deviations 1–7 from this plan verbatim. In the spec's §5
  "Extraction forced by boot gating" list, append one sentence: "As executed (plan 05): `TaskCategory` and
  `ModelSpec` stay in `vox-models`; `TaskPriority`, `PricingSource` and the clutch primitives moved to
  `vox-orchestrator-types`; `vox-research-shim` keeps `vox-orchestrator` for `registry_model_resolve`, embeddings
  and memory config." In `rust-build-loop-ssot-2026.md` "Part B status", add: "2026-10-08: B4 superseded by
  capability-profiles plan 05, which extracts `models/` into the new `vox-models` crate."

- [ ] **Step 3: Docs path sweep.**

```bash
rg -l 'crates/vox-orchestrator/src/(models/|attention/|budget/|budget_gate\.rs|route_policy\.rs|catalog(_classifier|_live)?\.rs|fatigue_monitor\.rs)' docs/src --glob '!archive/**' --glob '!**/*.generated.md' > target/cp05/doc-refs.txt
wc -l < target/cp05/doc-refs.txt
perl -pi -e 's{crates/vox-orchestrator/src/models/}{crates/vox-models/src/}g; s{crates/vox-orchestrator/src/(route_policy|catalog|catalog_classifier|catalog_live)\.rs}{crates/vox-models/src/$1.rs}g; s{crates/vox-orchestrator/src/(attention|budget)/}{crates/vox-attention/src/$1/}g; s{crates/vox-orchestrator/src/(budget_gate|fatigue_monitor)\.rs}{crates/vox-attention/src/$1.rs}g' $(cat target/cp05/doc-refs.txt)
rg -n 'crates/vox-orchestrator/src/(models/|attention/|budget/|budget_gate\.rs|route_policy\.rs|catalog(_classifier|_live)?\.rs|fatigue_monitor\.rs)' docs/src --glob '!archive/**'
cargo run -q -p vox-cli -- ci check-links
cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md
```

  Expected: about 80 files rewritten; the second `rg` prints nothing; `check-links` and the doc lint pass. Do not edit
  `docs/src/archive/**` or any `*.generated.md`. If a generated reference names a moved path, regenerate it with
  `cargo run -p vox-cli -- ci command-sync`.

- [ ] **Step 4: Full verification matrix.**

```bash
cargo check --workspace
cargo nextest run -p vox-orchestrator-types -p vox-attention
cargo nextest run -p vox-models --features runtime,populi-transport,test-support
cargo nextest run -p vox-orchestrator
cargo nextest run -p vox-orchestrator-mcp -E 'test(/llm_bridge|model_route_policy|chat_model_resolve|visus_review|vram_probe|mcp_context|attention_policy|pending_approvals|budget/) | binary(pending_approvals_tests) | binary(hitl_rehydrate_on_restart_tests) | binary(daemon_extra_tests) | binary(autonomous_tool_dispatch_tests)'
cargo nextest run -p vox-research-shim
cargo nextest run -p vox-cli -E 'test(/commands::model::|commands::safety|commands::chat|orchestration_audit|audit_route|model_catalog/)'
cargo nextest run -p vox-integration-tests -E 'binary(chat_harness_regression_test)'
cargo nextest run -p vox-cli-ci -E 'test(/model_routing_check|crate_edges/)'
cargo run -q -p vox-arch-check
vox ci crate-edges
vox ci fan-in-budget
vox ci ssot-drift
vox ci pre-push --complete
```

  Expected: every pass/fail set equals its Task 0 baseline. `--complete` runs clippy on affected crates. If it fails
  only inside `tauri-build` for `vox-gui`, follow the AGENTS.md fresh-worktree note; nothing is committed for that.

- [ ] **Step 5: Blast-radius report.**

```bash
vox graph refresh --auto
vox graph crate-map > target/cp05/crate-map-after.txt
diff target/cp05/crate-map-before.txt target/cp05/crate-map-after.txt | head -80
cargo run -q -p vox-arch-check 2>&1 | tail -5 > target/cp05/arch-after.txt; diff target/cp05/arch-before.txt target/cp05/arch-after.txt
```

  Put this in the PR description:
  - The blast radius of `vox-orchestrator` before and after.
  - The blast radius of `vox-attention` and `vox-models`.
  - The community moves of the two new crates.
  - The count of crates that no longer rebuild when `vox-models` changes alone. Use the crates that depend on
    `vox-orchestrator` but not on `vox-models`, from `cargo tree -i vox-models --workspace --edges normal`.

- [ ] **Step 6: Commit.**

```bash
git add contracts/ci/crate-edges.allow.v1.json docs/superpowers/plans/2026-10-08-capability-profiles/00-contracts.md \
  docs/superpowers/specs/2026-10-08-capability-profiles-design.md docs/src
git commit -m "docs(arch): record plan 05 deviations, repoint moved paths, tighten removed orchestrator edges"
```

---

## Task order and parallel safety

| Task | Depends on | Parallel-safe with |
|---|---|---|
| 0 | the model-bench WIP landing | none |
| 1 | 0 | none (it adds tests to files Task 2 also edits) |
| 2 | 1 | Task 3 (waiting on the user) |
| 3 | 0 (the list); it blocks commits of 4–7 | 2 |
| 4 | 2, 3 | none |
| 5 | 4 | none: it edits `vox-orchestrator/{Cargo.toml,lib.rs}`, `vox-orchestrator-mcp/Cargo.toml` and `vox-cli/Cargo.toml`, as does 6/7 |
| 6 | 5 (the attention shim is gone first, so only one shim exists at a time) | none |
| 7 | 6 | none |
| 8 | 7 | none |

## Self-Review

- **Spec coverage.** The spec says `vox-models` is L3 from `models/`, and that its deps are named in the PR: Task 6
  does this, and Task 3 lists the measured deps. "`ModelSpec`, `TaskCategory`, `CostPreference`,
  `RemainingBudget` move to `vox-orchestrator-types`" is satisfied only partly, and on purpose. Two were already
  done; two would change wire formats (Deviations 1–3) and are recorded in `00-contracts.md` (Task 8). The spec says
  `vox-research-shim::selection` depends on `vox-models`: done in Task 7, and the shim keeps `vox-orchestrator`
  (Deviation 6). The spec says `vox-attention` is L2, covers attention/budget/budget_gate, and uses `BudgetSource`:
  Tasks 4–5 do this, and `fatigue_monitor` moves along because only `budget/` uses it. `layers.toml` and
  `where-things-live.md` are updated in the same commit as each crate. The edges are proposed, not self-added
  (Task 3).
- **Shim lifetime.** Each shim lives for exactly one task (4→5, 6→7), carries the deprecation marker, and is proven
  gone by `rg` (Steps 5.6 and 7.6). The `mode.rs` re-export of the clutch primitives is not a shim. `mode` stays in
  `vox-orchestrator` and owns task policy; the re-export is permanent.
- **Behaviour preservation.** Before anything moves, Task 1 pins the four `AgentTask` → `RoutingTask` rewrites
  and the VoxDB persistence path. `TaskCategory`, `PricingSource`, `TaskPriority` and `ClutchProfile` wire formats
  are pinned at their new homes. The `BudgetSource` inversion keeps the log target, op ids, the `"local_user"` key,
  the `agent_budget.<id>` key and the timeout `duration_ms` argument identical. A mutation run proves the
  timeout/success routing.
- **Placeholder scan.** The new files are shown in full: both `Cargo.toml`s, `vox-attention/src/lib.rs`,
  `source.rs`, `sync_lock.rs`, `persistence.rs`, `budget_source.rs`, `vox-models/src/lib.rs`, and the two
  integration tests. The two verbatim moves of existing code (`clutch.rs` body, `PricingSource`) give exact
  anchors and an `rg` to locate them. The `exceptions` JSON uses the plan date; the user sets the real approval
  date.
- **Names consistent with `00-contracts.md`.** This plan introduces no contract type. `Subsystem::{Attention,
  ModelRegistry}` stay in `vox-mcp-registry` (plan 01).

## Risks

1. **The WIP predates PR #598 (B1–B3).** The model-bench branch forked at `b120286c5`, 21 commits behind
   `origin/main`. Its `mode_select.rs` takes `&AgentTask`, and `routing_task.rs` does not exist there. Merging it
   will conflict in `models/{mode_select,registry,select}.rs`, `runtime.rs` and `vox-gui/src/commands/routing_explain.rs`.
   This is the classic "move + edit" duplicate risk. Task 0 Step 2 stops if the merge regresses B2. Whoever merges
   must re-resolve to `RoutingTask` and run the define-once check before this plan touches anything.
2. **Edge count.** 26 new edges, 7 of them only from deleting the shims (#5–6, #22–26). If the user refuses those,
   the alternative is to keep `vox_orchestrator::{models, attention, …}` as a permanent facade. That would
   contradict the user's "delete the shims" requirement, so it is not planned. It is noted here so Task 3 can offer it.
3. **`vox-populi` edge (#17).** Option A mirrors today's build graph exactly but pulls a 20k-line L3 crate into
   `vox-models` under `runtime`. Option B removes it, but changes macOS VRAM detection for `vox chat`/`vox-gui`
   unless they also register the probe.
4. **Fan-in and build-map SSOTs.** `fan-in-snapshot.v1.json` needs 4 user-applied bumps. `crate-build-map.v1.json`
   has measured `compile_s` values that an agent cannot produce. If `crate-build-map-parity` requires entries for
   new crates, the plan stops (Task 6 Step 11) instead of fabricating them.
5. **`--no-default-features` builds.** `vox-models` makes `vox-actor-runtime` non-optional, because
   `route_policy.rs` already needed it unconditionally. A `vox-orchestrator --no-default-features` build that
   somehow compiled today would now link `vox-actor-runtime`. No shipping consumer builds that way.
6. **Path-keyed baselines and readers.** Toestub, config-hygiene and telemetry CSV baselines are keyed by file
   path. A missed rename silently drops a suppression and surfaces old findings as new ones. Steps 4.10, 6.9 and
   8.3 sweep them with `rg`, and `vox ci pre-push --complete` catches the rest.
7. **`cfg(test)` seams across the crate boundary.** `key_guard`, `route_policy` and `vram` behave differently
   under `cfg(test)`. Consumers outside `vox-models` already see the non-test behaviour (they were never compiled
   with `cfg(test)` for these modules), and `test-support` forwarding preserves the two explicit seams (mutation
   check, Task 6 Step 12).
8. **Tracing targets.** `target: "vox_orchestrator::model_route"` and similar strings stay unchanged, so
   `RUST_LOG` filters keep working. The `rg` checks exclude them explicitly, so a blind rewrite would show up as a
   spurious diff in review.
