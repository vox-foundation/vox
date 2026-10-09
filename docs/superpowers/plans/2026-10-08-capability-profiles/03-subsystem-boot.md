---
title: "Capability Profiles plan 03 — subsystem boot gating"
description: "Turns vox-orchestrator-d's linear boot into a profile-gated step table that boots only the active profile's subsystem closure, degrades on failure, publishes a BootReport to ServerState, and exits non-zero only when every required subsystem failed."
category: "architecture"
status: "proposed"
date: 2026-10-08
---

# Subsystem Boot Gating Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `vox-orchestrator-d` boots only the subsystems in the active profile's closure, records what came up in a `BootReport`, and gives that report to `ServerState`. Then `authorize_tool` returns `SubsystemNotBooted` for tools whose subsystem is down, on stdio MCP, the WS/HTTP gateway, and `orch.tool_call` alike. A failed subsystem degrades boot instead of aborting it. The daemon exits non-zero only when every subsystem the profile `requires` failed. `dev` keeps today's exact step order.

**Architecture:** Today `async_main` (`crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` L179–398) is one straight-line function. This plan moves its body into an ordered table of `BootStep<C>` values, each tagged with an `Option<Subsystem>` (`None` = core step that always runs). A generic runner in a new library target (`crates/vox-orchestrator-d/src/boot_plan.rs`) walks the table. It skips steps outside the closure, records results in a `BootReport`, propagates dependency failures (strict for named profiles, lenient for `dev`), and publishes the report after every step. The daemon-specific steps and their shared context live in `crates/vox-orchestrator-d/src/daemon_steps.rs`. `main` becomes: resolve profile → build context → run plan → exit decision → hand over the final report → serve. Subsystems that are lazy today (model registry, LLM facade, search index, memory, compiler, browser, speech) get explicit steps. These are cheap probe or marker steps: each gives the subsystem a slot in the closure and the report, without adding eager IO.

**Tech Stack:** Rust 2024, tokio, `vox-mcp-registry` types re-exported as `vox_orchestrator_mcp::profiles::*` (plans 01/02), `vox-orchestrator`, `vox-orchestrator-mcp`, `vox-foundation` protocol constants, `cargo nextest`, `tempfile` (dev-dependency).

## Global Constraints

Copied verbatim from `00-contracts.md` §Global constraints:

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

Plan-03-specific constraints:

- Work only in `/Users/brbrainerd/dev/vox/.worktrees/capability-profiles` (branch `feat/capability-profiles`).
- Boot tests live in `crates/vox-orchestrator-d/tests/`, **not** `vox-test-harness`. Spec §9.3 names `vox-test-harness`, but `vox-orchestrator-d → vox-test-harness` is not in `contracts/ci/crate-edges.allow.v1.json`, and adding it would break the no-new-edges rule. `tempfile` is an external dev-dependency, so it is not a workspace edge.
- Every new file stays under 500 non-blank lines (`arch/god_object`). `server_state.rs` grows by ≤ 12 lines; everything else goes in the new `server_state/boot_gate.rs`.

---

## File Structure

| Path | Action | Responsibility |
|---|---|---|
| `crates/vox-orchestrator-d/Cargo.toml` | modify | add `[lib]`, `doc = false` on `[[bin]]`, `[dev-dependencies] tempfile` |
| `crates/vox-orchestrator-d/src/lib.rs` | create | `pub mod boot_plan; pub mod daemon_steps;` |
| `crates/vox-orchestrator-d/src/boot_plan.rs` | create | generic `BootStep<C>`, `run_boot_plan`, `DependencyMode`, `profile_roots`, `every_required_failed`, `planned_step_names` + unit tests |
| `crates/vox-orchestrator-d/src/daemon_steps.rs` | create | `DaemonBootCtx`, the daemon step fns, `DAEMON_STEPS` table (20 rows), `plugins_install_dir` + unit tests (dev order) |
| `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` | modify | `async_main` L211–384 replaced by the boot plan; imports trimmed |
| `crates/vox-orchestrator-d/tests/boot_profiles.rs` | create | real-table boot tests: `models` (no store, no Populi), `knowledge` (no agent fleet, store opened) |
| `crates/vox-orchestrator-mcp/src/server_state.rs` | modify | `mod boot_gate;`, `new_for_daemon` delegates to `new_for_daemon_gated` |
| `crates/vox-orchestrator-mcp/src/server_state/boot_gate.rs` | create | `set_boot_report`, `boot_report_snapshot`, `subsystem_up`, `subsystem_not_booted_json` + tests |
| `crates/vox-orchestrator-mcp/src/daemon_extra.rs` | modify | serve `orch.boot_report`; gate `research.run` / `research.persist_claims` |
| `crates/vox-orchestrator-mcp/src/daemon_route.rs` | modify | `fetch_daemon_boot_report()` (no-spawn) |
| `crates/vox-orchestrator-mcp/src/server.rs` | modify | `list_tools` refreshes the local `BootReport` from an already-running daemon |
| `crates/vox-orchestrator-mcp/tests/daemon_extra_tests.rs` | modify | boot-report RPC + research gate + `orch.tool_call` denial tests |
| `crates/vox-orchestrator-mcp/tests/boot_requires_guard.rs` | create | guards that the db/session tools found in the audit declare the right `requires` |
| `crates/vox-foundation/src/protocol.rs` | modify | `orch_daemon_method::BOOT_REPORT` |
| `contracts/orchestration/orch-daemon-rpc-methods.schema.json` | modify | add `"orch.boot_report"` to the method enum |
| `docs/src/reference/capability-profiles.md` | modify (created by plan 02) | "Boot and degraded mode" section |
| `docs/src/reference/orchestration-unified.md` | modify | "Profile-gated daemon boot" section (the daemon's reference doc) |

### Today's boot (source of truth for the `dev` order)

`crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs`:

| Lines | Today | New step name | Subsystem |
|---|---|---|---|
| 212–216 | `build_repo_scoped_orchestrator(cfg, None)` | (inside `DaemonBootCtx::new`) | core, pre-plan |
| 218–230 | `connect_workspace_journey_optional` + `orch.init_db` | `db` | `Db` |
| — | (lazy: `orch.models_handle()`) | `model_registry` (new probe) | `ModelRegistry` |
| 232–282 | persisted `routing_priority` / `selection_policy` | `routing_preferences` | `ModelRegistry` |
| — | (lazy: `vox_actor_runtime::llm`) | `llm_facade` (new marker) | `LlmFacade` |
| — | (lazy: `vox-search` per call) | `search_index` (new marker) | `SearchIndex` |
| — | (lazy: `MemoryManager::new` per call in `memory_tools/handlers_memory.rs:30,67,193,217`) | `memory` (new marker) | `Memory` |
| 290–299 | `SessionManager::new` (today a `?` abort) | `sessions` | `Sessions` |
| 301–319 | skill registry + plugin bridge | `skills` | core |
| 321–330 | `ServerState::new_for_daemon` (+ scientia mesh jobs inside it) + `with_db_initialized` | `server_state` | core (scientia jobs gated on `Research`) |
| 336–343 | `spawn_agent_fleet_if_enabled_with_dispatcher` | `agent_fleet` | `AgentFleet` |
| 345–355 | `spawn_populi_federation_poller` | `populi_federation_poller` | `Mesh` |
| 356 | `spawn_populi_remote_result_poller` | `a2a_remote_result_poller` | `Mesh` |
| 357 | `spawn_populi_remote_worker_poller` | `a2a_remote_worker_poller` | `Mesh` |
| 359–365 | `spawn_clarification_db_inbox_poller` (only with db) | `clarification_inbox` | `Clarification` |
| 366 | `spawn_socrates_research_poller` | `socrates_research_poller` | `Research` |
| 368–370 | `FlywheelMonitor::spawn` | `flywheel` | `Flywheel` |
| 372–373 | `spawn_attention_calibration` | `attention_calibration` | `Attention` |
| — | (compiler handles built per call in `compiler_tools.rs`) | `compiler` (new marker) | `Compiler` |
| — | (browser plugin loaded per call) | `browser` (new marker) | `Browser` |
| — | (oratio plugin loaded per call) | `speech` (new marker) | `Speech` |
| 375–384 | `McpExtraDispatch::new` + `spawn_http_gateway_if_enabled` | stays in `main`, after the exit decision | core |

Marker steps succeed without IO. Each one exists so the subsystem shows up in the closure and in `BootReport`, which `authorize_tool` reads. It also gives the subsystem a single place to grow an eager init later. When their dependency failed, strict mode still marks markers failed (`dependency db failed`), so `search_index`/`memory` tools are denied under a named profile that lost its db.

### Dependency policy (decision)

- **Strict** (any profile without `dev`): a step runs only if every direct dependency of its subsystem is already booted. Otherwise the subsystem goes into `failed` as `dependency <x> failed` and its steps are skipped. Dependencies are always earlier in the table, which `steps_are_topologically_ordered` asserts.
- **Lenient** (any profile id list containing `dev`): steps whose dependency failed still run, which is exactly what today's code does. For example, with no project store, `SessionManager` still uses file persistence and the Populi poller still starts with `db = None`. This is what makes "`dev` reproduces today's behaviour exactly" true in the degraded-db case as well as the healthy one.
- In both modes, a later step of an already-failed subsystem is skipped. Core-step failure aborts boot with an error, the same as today's `?`.
- **Exit rule:** after the plan, roots = union of `requires` over `PROFILES` whose `id ∈ profile.ids`. If roots is non-empty and every root is in `report.failed`, `async_main` bails, and `main` returns `Err` → non-zero exit code. Marker steps never fail, so `dev` can never trip this.

---

### Task 1: Reconcile plan-02 `ServerState` fields and add boot-report accessors

**Files:**
- Create: `crates/vox-orchestrator-mcp/src/server_state/boot_gate.rs`
- Modify: `crates/vox-orchestrator-mcp/src/server_state.rs` (add `mod boot_gate;` next to the existing `mod vram_probe;` / `mod webhook_intake;` declarations)
- Possibly modify: `crates/vox-mcp-registry/src/boot.rs` (add `serde::Deserialize` to `BootReport`) and `docs/superpowers/plans/2026-10-08-capability-profiles/00-contracts.md` (record that change in the same commit, as the contracts file requires)

**Interfaces:**
- Consumes (plan 02, assumed names, verified in Step 1): `ServerState.active_profile: Arc<ActiveProfile>`, `ServerState.boot_report: Arc<parking_lot::RwLock<BootReport>>` defaulting to "all booted"; `BootReport::is_up`; `ProfileDenial::SubsystemNotBooted(s).code()`.
- Produces:
  - `impl ServerState { pub fn set_boot_report(&self, report: BootReport); pub fn boot_report_snapshot(&self) -> BootReport; pub fn subsystem_up(&self, s: Subsystem) -> bool }`
  - `pub fn subsystem_not_booted_json(kind: &str, name: &str, s: Subsystem) -> String` (wire shape from `00-contracts.md` §Wire shapes)
  - `BootReport: serde::Deserialize` (contract amendment)

- [ ] **Step 1: Verify what plan 02 actually landed.**

```bash
cd /Users/brbrainerd/dev/vox/.worktrees/capability-profiles
rg -n "active_profile|boot_report" crates/vox-orchestrator-mcp/src/server_state.rs
rg -n "derive\(.*\)" crates/vox-mcp-registry/src/boot.rs
rg -n "fn denial_json|subsystem_not_booted" crates/vox-orchestrator-mcp/src
```

Expected: two fields with the assumed names and types. If plan 02 used different names, use plan 02's names everywhere in this plan; the field *names* are not part of `00-contracts.md`. If `boot_report` is a plain `BootReport` and not shared, change it to `Arc<parking_lot::RwLock<BootReport>>` in this task. The HTTP gateway and `McpExtraDispatch` each hold `ServerState` *clones*, so the report must be shared through an `Arc`. If plan 02 already has a helper that renders the `subsystem_not_booted` wire JSON, delete `subsystem_not_booted_json` from the code below and call that helper instead.

- [ ] **Step 2: If `BootReport` lacks `Deserialize`, add it** (`crates/vox-mcp-registry/src/boot.rs`):

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BootReport {
    pub booted: std::collections::BTreeSet<Subsystem>,
    pub failed: std::collections::BTreeMap<Subsystem, String>,
}
```

Then, in `00-contracts.md` §Types `src/boot.rs`, change `serde::Serialize` → `serde::Serialize, serde::Deserialize` on `BootReport` and add the sentence: "`Deserialize` added by plan 03 so stdio `vox mcp` can read the daemon's report over `orch.boot_report`."

- [ ] **Step 3: Write the failing tests** in the new `boot_gate.rs` (full file):

```rust
//! Boot-report plumbing between `vox-orchestrator-d`'s boot plan and the MCP gate.
//!
//! The daemon publishes its [`BootReport`] here while booting and once more when boot
//! finishes. [`vox_mcp_registry::authorize_tool`] reads it to return
//! `subsystem_not_booted`.

use vox_mcp_registry::{BootReport, ProfileDenial, Subsystem};

use super::ServerState;

impl ServerState {
    /// Replace the shared boot report. Every clone of this `ServerState` (HTTP gateway,
    /// `McpExtraDispatch`, agent-fleet dispatcher) sees the new value.
    pub fn set_boot_report(&self, report: BootReport) {
        *self.boot_report.write() = report;
    }

    pub fn boot_report_snapshot(&self) -> BootReport {
        self.boot_report.read().clone()
    }

    pub fn subsystem_up(&self, s: Subsystem) -> bool {
        self.boot_report.read().is_up(s)
    }
}

/// `{"error":{"code":"subsystem_not_booted",...}}` for a tool or daemon RPC method.
pub fn subsystem_not_booted_json(kind: &str, name: &str, s: Subsystem) -> String {
    serde_json::json!({
        "error": {
            "code": ProfileDenial::SubsystemNotBooted(s).code(),
            "message": format!(
                "{kind} {name} requires subsystem {}, which is not running",
                s.as_str()
            ),
            "subsystem": s.as_str(),
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(booted: &[Subsystem], failed: &[(Subsystem, &str)]) -> BootReport {
        BootReport {
            booted: booted.iter().copied().collect(),
            failed: failed.iter().map(|(s, e)| (*s, (*e).to_string())).collect(),
        }
    }

    #[tokio::test]
    async fn set_boot_report_is_visible_through_clones() {
        let state = ServerState::new_test().await;
        let clone = state.clone();
        let r = report(&[Subsystem::ModelRegistry], &[(Subsystem::Db, "boom")]);
        state.set_boot_report(r.clone());
        assert_eq!(clone.boot_report_snapshot(), r);
    }

    #[tokio::test]
    async fn subsystem_up_reflects_booted_set_only() {
        let state = ServerState::new_test().await;
        state.set_boot_report(report(&[Subsystem::Attention], &[(Subsystem::Db, "x")]));
        assert!(state.subsystem_up(Subsystem::Attention));
        assert!(!state.subsystem_up(Subsystem::Db));
        assert!(!state.subsystem_up(Subsystem::Mesh));
    }

    #[test]
    fn subsystem_not_booted_json_matches_wire_shape() {
        let v: serde_json::Value = serde_json::from_str(&subsystem_not_booted_json(
            "tool",
            "vox_kb_query",
            Subsystem::SearchIndex,
        ))
        .expect("valid json");
        assert_eq!(v["error"]["code"], "subsystem_not_booted");
        assert_eq!(v["error"]["subsystem"], "search_index");
        assert_eq!(
            v["error"]["message"],
            "tool vox_kb_query requires subsystem search_index, which is not running"
        );
    }

    #[test]
    fn boot_report_round_trips_through_json() {
        let r = report(&[Subsystem::LlmFacade], &[(Subsystem::Db, "no store")]);
        let back: BootReport =
            serde_json::from_value(serde_json::to_value(&r).expect("ser")).expect("de");
        assert_eq!(back, r);
    }
}
```

Add to `server_state.rs` beside the other child modules:

```rust
mod boot_gate;
pub use boot_gate::subsystem_not_booted_json;
```

- [ ] **Step 4: Run, expect failure first** (before Step 2 if `Deserialize` was missing, or before the `mod` line):

```bash
cargo nextest run -p vox-orchestrator-mcp boot_gate
```

Expected before: compile error (`cannot find function subsystem_not_booted_json` / `BootReport: Deserialize not satisfied`). Expected after: `4 tests run: 4 passed`.

- [ ] **Step 5: Checks and commit.**

```bash
cargo check -p vox-orchestrator-mcp && cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/server_state.rs crates/vox-orchestrator-mcp/src/server_state/boot_gate.rs crates/vox-mcp-registry/src/boot.rs docs/superpowers/plans/2026-10-08-capability-profiles/00-contracts.md
git commit -m "feat(mcp): shared BootReport accessors on ServerState" -m "Plan 03 task 1: the daemon publishes its boot report through set_boot_report so every ServerState clone (gateway, extra dispatch) gates on the same data."
```

**Parallel-safe with:** Task 3 (different crate; Task 3 only needs the plan-01/02 types).

---

### Task 2: `ServerState::new_for_daemon_gated` — make the scientia mesh jobs skippable

`new_for_daemon` (`server_state.rs:348`) calls `spawn_scientia_research_mesh_background_jobs()` unconditionally, near the end of the constructor, so a `models` daemon would still start Research background work. The fix keeps the exact call position, so `dev` order is unchanged.

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/server_state.rs:347-420`
- Test: `crates/vox-orchestrator-mcp/src/server_state/boot_gate.rs` (tests module)

**Interfaces:**
- Produces: `pub fn new_for_daemon_gated(orchestrator: Arc<Orchestrator>, orchestrator_config: OrchestratorConfig, repository: vox_repository::RepositoryContext, session_manager: Arc<TokMutex<SessionManager>>, skill_registry: Arc<SkillRegistry>, spawn_research_jobs: bool) -> Self`
- Unchanged: `pub fn new_for_daemon(...)` (same 5 params) = `new_for_daemon_gated(..., true)`.

- [ ] **Step 1: Failing test** (append to `boot_gate.rs` tests module). `research_events` is a broadcast sender: the scientia subscriber holds one receiver, so `receiver_count()` shows whether it was spawned.

```rust
    fn daemon_parts() -> (
        std::sync::Arc<vox_orchestrator::Orchestrator>,
        vox_orchestrator::OrchestratorConfig,
        vox_repository::RepositoryContext,
        std::sync::Arc<tokio::sync::Mutex<vox_orchestrator::SessionManager>>,
        std::sync::Arc<vox_skills::SkillRegistry>,
    ) {
        let build = vox_orchestrator::bootstrap::build_repo_scoped_orchestrator(
            vox_orchestrator::OrchestratorConfig::for_testing(),
            None,
        );
        let sm = vox_orchestrator::SessionManager::new(vox_orchestrator::SessionConfig {
            persist: false,
            ..vox_orchestrator::SessionConfig::default()
        })
        .expect("in-memory session manager");
        (
            std::sync::Arc::new(build.orchestrator),
            build.config,
            build.repository,
            std::sync::Arc::new(tokio::sync::Mutex::new(sm)),
            vox_skills::new_registry_arc(),
        )
    }

    #[tokio::test]
    async fn gated_constructor_skips_research_subscriber_when_told() {
        let (o, c, r, s, k) = daemon_parts();
        let state = ServerState::new_for_daemon_gated(o, c, r, s, k, false);
        assert_eq!(state.research_events.receiver_count(), 0);
    }

    #[tokio::test]
    async fn new_for_daemon_still_spawns_research_subscriber() {
        let (o, c, r, s, k) = daemon_parts();
        let state = ServerState::new_for_daemon(o, c, r, s, k);
        assert!(state.research_events.receiver_count() >= 1);
    }
```

Before relying on the second assertion, confirm that `spawn_scientia_mesh_research_event_subscriber` keeps its receiver alive inside the spawned task: `rg -n "pub fn spawn_scientia_mesh_research_event_subscriber" -A15 crates/vox-research-shim/src`.

- [ ] **Step 2: Run, expect compile failure** (`no function new_for_daemon_gated`):

```bash
cargo nextest run -p vox-orchestrator-mcp gated_constructor
```

- [ ] **Step 3: Implement.** In `server_state.rs`, rename the existing `pub fn new_for_daemon(` (L348) to `pub fn new_for_daemon_gated(`, add the parameter `spawn_research_jobs: bool` after `skill_registry`, and replace the line `state.spawn_scientia_research_mesh_background_jobs();` inside it with:

```rust
        if spawn_research_jobs {
            state.spawn_scientia_research_mesh_background_jobs();
        }
```

Then add directly above it:

```rust
    /// Minimal constructor for vox-orchestrator-d daemon that already has an Orchestrator.
    pub fn new_for_daemon(
        orchestrator: Arc<Orchestrator>,
        orchestrator_config: OrchestratorConfig,
        repository: vox_repository::RepositoryContext,
        session_manager: Arc<TokMutex<SessionManager>>,
        skill_registry: Arc<SkillRegistry>,
    ) -> Self {
        Self::new_for_daemon_gated(
            orchestrator,
            orchestrator_config,
            repository,
            session_manager,
            skill_registry,
            true,
        )
    }
```

Change the doc comment on `new_for_daemon_gated` to: ``/// [`Self::new_for_daemon`] with the Research-only scientia mesh jobs made optional (skipped when `Research` is outside the capability-profile closure).``

- [ ] **Step 4: Run, expect pass:** `cargo nextest run -p vox-orchestrator-mcp boot_gate` → `6 tests run: 6 passed`.

- [ ] **Step 5: Checks + commit.**

```bash
cargo check -p vox-orchestrator-mcp && cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/server_state.rs crates/vox-orchestrator-mcp/src/server_state/boot_gate.rs
git commit -m "feat(mcp): gate scientia mesh jobs behind new_for_daemon_gated" -m "Plan 03 task 2: the daemon constructor spawned Research background work unconditionally; profiles without Research must not start it."
```

**Parallel-safe with:** Task 3, Task 9.

---

### Task 3: Generic boot runner (`boot_plan.rs`) and the library target

**Files:**
- Modify: `crates/vox-orchestrator-d/Cargo.toml`
- Create: `crates/vox-orchestrator-d/src/lib.rs`, `crates/vox-orchestrator-d/src/boot_plan.rs`

**Interfaces:**
- Consumes: `vox_orchestrator_mcp::profiles::{ActiveProfile, BootReport, DEV_PROFILE_ID, PROFILES, Subsystem}` (plan 01/02).
- Produces:

```rust
pub type StepFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + 'a>>;
pub type StepFn<C> = for<'a> fn(&'a mut C) -> StepFuture<'a>;
pub struct BootStep<C> { pub name: &'static str, pub subsystem: Option<Subsystem>, pub legacy: bool, pub run: StepFn<C> }
pub trait BootContext { fn publish(&mut self, _report: &BootReport) {} }
pub enum DependencyMode { Strict, Lenient }
impl DependencyMode { pub fn for_profile(profile: &ActiveProfile) -> Self }
pub struct BootOutcome { pub report: BootReport, pub ran: Vec<&'static str>, pub skipped: Vec<&'static str> }
pub fn planned_step_names<C>(steps: &[BootStep<C>], closure: &BTreeSet<Subsystem>) -> Vec<&'static str>;
pub async fn run_boot_plan<C: BootContext>(steps: &[BootStep<C>], ctx: &mut C, closure: &BTreeSet<Subsystem>, mode: DependencyMode) -> anyhow::Result<BootOutcome>;
pub fn profile_roots(profile: &ActiveProfile) -> BTreeSet<Subsystem>;
pub fn every_required_failed(roots: &BTreeSet<Subsystem>, report: &BootReport) -> bool;
```

- [ ] **Step 1: Cargo wiring.** In `crates/vox-orchestrator-d/Cargo.toml`, add above `[[bin]]`:

```toml
[lib]
path = "src/lib.rs"
```

Add `doc = false` inside the existing `[[bin]]` table (it shares the package name with the lib, which would cause a rustdoc output-name collision). Then add at the end:

```toml
[dev-dependencies]
tempfile.workspace = true
```

Create `crates/vox-orchestrator-d/src/lib.rs`:

```rust
//! Profile-gated boot for `vox-orchestrator-d` (capability profiles plan 03).
//!
//! The binary in `src/bin/vox_orchestrator_d.rs` builds a [`daemon_steps::DaemonBootCtx`]
//! and runs [`daemon_steps::DAEMON_STEPS`] through [`boot_plan::run_boot_plan`].

pub mod boot_plan;
pub mod daemon_steps;
```

`daemon_steps` is filled in by Task 4. Until then, create it containing only the line `//! Daemon boot steps (plan 03 task 4).` so this task compiles, and drop the `[daemon_steps::…]` intra-doc links from `lib.rs` until Task 4 lands (rustdoc would flag them as broken).

- [ ] **Step 2: Write the failing tests first.** Create `crates/vox-orchestrator-d/src/boot_plan.rs` with the tests module below, plus a stub `run_boot_plan` whose body is `Ok(BootOutcome::default())`. That makes every behavioural test fail for the right reason. Full final file:

```rust
//! Profile-gated boot runner.
//!
//! A boot plan is an ordered table of [`BootStep`]s. A step with `subsystem: Some(s)` runs
//! only when `s` is in the active profile's closure; its failure *degrades* boot (recorded in
//! the [`BootReport`], boot continues). A step with `subsystem: None` is core: it always runs
//! and its failure aborts boot, like the `?`s in the pre-profile `async_main`.

use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

use vox_orchestrator_mcp::profiles::{
    ActiveProfile, BootReport, DEV_PROFILE_ID, PROFILES, Subsystem,
};

pub type StepFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + 'a>>;
pub type StepFn<C> = for<'a> fn(&'a mut C) -> StepFuture<'a>;

pub struct BootStep<C> {
    pub name: &'static str,
    pub subsystem: Option<Subsystem>,
    /// Existed in `async_main` before capability profiles; `dev` must run these in their
    /// original relative order.
    pub legacy: bool,
    pub run: StepFn<C>,
}

pub trait BootContext {
    /// Called after every step that changed the report, so the live MCP gate never serves
    /// from a stale default while boot is still running.
    fn publish(&mut self, _report: &BootReport) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyMode {
    /// A step runs only if every direct dependency of its subsystem is booted.
    Strict,
    /// Steps run even when a dependency failed (pre-profile behaviour; `dev` only).
    Lenient,
}

impl DependencyMode {
    pub fn for_profile(profile: &ActiveProfile) -> Self {
        if profile.ids.contains(&DEV_PROFILE_ID) {
            Self::Lenient
        } else {
            Self::Strict
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootOutcome {
    pub report: BootReport,
    pub ran: Vec<&'static str>,
    pub skipped: Vec<&'static str>,
}

/// Names of the steps a closure selects, in table order, without running anything.
pub fn planned_step_names<C>(
    steps: &[BootStep<C>],
    closure: &BTreeSet<Subsystem>,
) -> Vec<&'static str> {
    steps
        .iter()
        .filter(|s| s.subsystem.is_none_or(|sub| closure.contains(&sub)))
        .map(|s| s.name)
        .collect()
}

pub async fn run_boot_plan<C: BootContext>(
    steps: &[BootStep<C>],
    ctx: &mut C,
    closure: &BTreeSet<Subsystem>,
    mode: DependencyMode,
) -> anyhow::Result<BootOutcome> {
    let mut out = BootOutcome::default();
    for step in steps {
        let Some(sub) = step.subsystem else {
            (step.run)(ctx)
                .await
                .map_err(|e| anyhow::anyhow!("core boot step `{}` failed: {e}", step.name))?;
            out.ran.push(step.name);
            ctx.publish(&out.report);
            continue;
        };
        if !closure.contains(&sub) {
            out.skipped.push(step.name);
            continue;
        }
        if out.report.failed.contains_key(&sub) {
            out.skipped.push(step.name);
            continue;
        }
        if mode == DependencyMode::Strict {
            if let Some(dep) = sub.deps().iter().copied().find(|d| !out.report.is_up(*d)) {
                tracing::warn!(
                    step = step.name,
                    subsystem = sub.as_str(),
                    dependency = dep.as_str(),
                    "boot step skipped: dependency not booted"
                );
                out.report
                    .failed
                    .insert(sub, format!("dependency {} failed", dep.as_str()));
                out.skipped.push(step.name);
                ctx.publish(&out.report);
                continue;
            }
        }
        match (step.run)(ctx).await {
            Ok(()) => {
                tracing::info!(step = step.name, subsystem = sub.as_str(), "boot step ok");
                out.report.booted.insert(sub);
            }
            Err(e) => {
                tracing::warn!(
                    step = step.name,
                    subsystem = sub.as_str(),
                    error = %e,
                    "boot step failed; continuing degraded"
                );
                out.report.booted.remove(&sub);
                out.report.failed.insert(sub, e);
            }
        }
        out.ran.push(step.name);
        ctx.publish(&out.report);
    }
    Ok(out)
}

/// Union of `requires` over the manifests named by `profile.ids` (the exit-rule roots).
pub fn profile_roots(profile: &ActiveProfile) -> BTreeSet<Subsystem> {
    PROFILES
        .iter()
        .filter(|m| profile.ids.contains(&m.id))
        .flat_map(|m| m.requires.iter().copied())
        .collect()
}

/// True when the profile required something and every required subsystem failed.
pub fn every_required_failed(roots: &BTreeSet<Subsystem>, report: &BootReport) -> bool {
    !roots.is_empty() && roots.iter().all(|s| report.failed.contains_key(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        publishes: usize,
    }
    impl BootContext for Fake {
        fn publish(&mut self, _report: &BootReport) {
            self.publishes += 1;
        }
    }

    fn ok(_: &mut Fake) -> StepFuture<'_> {
        Box::pin(async { Ok(()) })
    }
    fn boom(_: &mut Fake) -> StepFuture<'_> {
        Box::pin(async { Err("boom".to_string()) })
    }
    fn st(name: &'static str, sub: Option<Subsystem>, run: StepFn<Fake>) -> BootStep<Fake> {
        BootStep { name, subsystem: sub, legacy: true, run }
    }
    fn set(xs: &[Subsystem]) -> BTreeSet<Subsystem> {
        xs.iter().copied().collect()
    }

    #[tokio::test]
    async fn skips_steps_outside_closure() {
        let steps = [
            st("db", Some(Subsystem::Db), ok),
            st("models", Some(Subsystem::ModelRegistry), ok),
            st("core", None, ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::ModelRegistry]);
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["models", "core"]);
        assert_eq!(out.skipped, vec!["db"]);
        assert_eq!(out.report.booted, set(&[Subsystem::ModelRegistry]));
        assert!(out.report.failed.is_empty());
    }

    #[tokio::test]
    async fn runs_in_table_order_and_publishes_after_each_executed_step() {
        let steps = [
            st("a", Some(Subsystem::Attention), ok),
            st("core", None, ok),
            st("c", Some(Subsystem::Compiler), ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::Attention, Subsystem::Compiler]);
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["a", "core", "c"]);
        assert_eq!(c.publishes, 3);
    }

    #[tokio::test]
    async fn failed_step_degrades_and_boot_continues() {
        let steps = [
            st("db", Some(Subsystem::Db), boom),
            st("attention", Some(Subsystem::Attention), ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::Db, Subsystem::Attention]);
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["db", "attention"]);
        assert_eq!(out.report.failed.get(&Subsystem::Db).map(String::as_str), Some("boom"));
        assert_eq!(out.report.booted, set(&[Subsystem::Attention]));
    }

    #[tokio::test]
    async fn strict_mode_fails_dependents_with_dependency_message() {
        let steps = [
            st("db", Some(Subsystem::Db), boom),
            st("sessions", Some(Subsystem::Sessions), ok),
            st("agent_fleet", Some(Subsystem::AgentFleet), ok),
        ];
        let closure = set(&[Subsystem::Db, Subsystem::Sessions, Subsystem::AgentFleet]);
        let mut c = Fake::default();
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict).await.unwrap();
        assert_eq!(out.ran, vec!["db"]);
        assert_eq!(out.skipped, vec!["sessions", "agent_fleet"]);
        assert_eq!(out.report.failed[&Subsystem::Sessions], "dependency db failed");
        assert_eq!(out.report.failed[&Subsystem::AgentFleet], "dependency sessions failed");
        assert!(out.report.booted.is_empty());
    }

    #[tokio::test]
    async fn lenient_mode_still_runs_dependents_after_failure() {
        let steps = [
            st("db", Some(Subsystem::Db), boom),
            st("sessions", Some(Subsystem::Sessions), ok),
            st("agent_fleet", Some(Subsystem::AgentFleet), ok),
        ];
        let closure = set(&[Subsystem::Db, Subsystem::Sessions, Subsystem::AgentFleet]);
        let mut c = Fake::default();
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Lenient).await.unwrap();
        assert_eq!(out.ran, vec!["db", "sessions", "agent_fleet"]);
        assert_eq!(out.report.failed.keys().copied().collect::<Vec<_>>(), vec![Subsystem::Db]);
        assert_eq!(out.report.booted, set(&[Subsystem::Sessions, Subsystem::AgentFleet]));
    }

    #[tokio::test]
    async fn later_steps_of_a_failed_subsystem_are_skipped() {
        let steps = [
            st("db", Some(Subsystem::Db), ok),
            st("mesh_a", Some(Subsystem::Mesh), boom),
            st("mesh_b", Some(Subsystem::Mesh), ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::Db, Subsystem::Mesh]);
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Lenient)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["db", "mesh_a"]);
        assert_eq!(out.skipped, vec!["mesh_b"]);
        assert!(!out.report.booted.contains(&Subsystem::Mesh));
    }

    #[tokio::test]
    async fn core_step_failure_aborts_boot() {
        let steps = [
            st("server_state", None, boom),
            st("attention", Some(Subsystem::Attention), ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::Attention]);
        let err = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict)
            .await
            .expect_err("core failure must abort");
        assert!(
            err.to_string().contains("core boot step `server_state` failed: boom"),
            "{err}"
        );
    }

    #[test]
    fn planned_step_names_filters_by_closure_and_keeps_core() {
        let steps = [
            st("db", Some(Subsystem::Db), ok),
            st("core", None, ok),
            st("speech", Some(Subsystem::Speech), ok),
        ];
        assert_eq!(
            planned_step_names(&steps, &set(&[Subsystem::Speech])),
            vec!["core", "speech"]
        );
    }

    #[test]
    fn every_required_failed_only_when_all_roots_failed() {
        let roots = set(&[Subsystem::ModelRegistry, Subsystem::LlmFacade]);
        let mut r = BootReport::default();
        r.failed.insert(Subsystem::ModelRegistry, "x".into());
        assert!(!every_required_failed(&roots, &r));
        r.failed
            .insert(Subsystem::LlmFacade, "dependency model_registry failed".into());
        assert!(every_required_failed(&roots, &r));
        assert!(!every_required_failed(&BTreeSet::new(), &r));
    }

    #[test]
    fn dependency_mode_is_lenient_only_for_dev() {
        assert_eq!(
            DependencyMode::for_profile(&ActiveProfile::dev()),
            DependencyMode::Lenient
        );
        let models = ActiveProfile::resolve(&["models"]).expect("models profile exists");
        assert_eq!(DependencyMode::for_profile(&models), DependencyMode::Strict);
    }

    #[test]
    fn profile_roots_of_dev_is_every_subsystem() {
        let all: BTreeSet<Subsystem> = Subsystem::ALL.iter().copied().collect();
        assert_eq!(profile_roots(&ActiveProfile::dev()), all);
    }

    #[test]
    fn profile_roots_of_models_is_its_manifest_requires() {
        let models = ActiveProfile::resolve(&["models"]).expect("models profile exists");
        let manifest = PROFILES.iter().find(|m| m.id == "models").expect("manifest");
        assert_eq!(
            profile_roots(&models),
            manifest.requires.iter().copied().collect::<BTreeSet<_>>()
        );
    }
}
```

- [ ] **Step 3: Run with the stub `run_boot_plan`, expect failures:**

```bash
cargo nextest run -p vox-orchestrator-d boot_plan
```

Expected: the 7 `run_boot_plan` tests FAIL (e.g. `left: [] right: ["models", "core"]`); the 5 pure-function tests pass.

- [ ] **Step 4: Replace the stub with the full body above; run again.** Expected: `12 tests run: 12 passed`.

- [ ] **Step 5: Checks + commit.**

```bash
cargo check -p vox-orchestrator-d && cargo clippy -p vox-orchestrator-d --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-d/Cargo.toml crates/vox-orchestrator-d/src/lib.rs crates/vox-orchestrator-d/src/boot_plan.rs crates/vox-orchestrator-d/src/daemon_steps.rs Cargo.lock
git commit -m "feat(daemon): generic profile-gated boot runner" -m "Plan 03 task 3: BootStep table + runner that skips steps outside the profile closure, degrades on subsystem failure, and fails dependents in strict mode."
```

**Parallel-safe with:** Task 1, Task 2, Task 9.

---

### Task 4: Daemon step table (`daemon_steps.rs`) with the `dev`-order back-compat test

**Files:**
- Create (replace placeholder): `crates/vox-orchestrator-d/src/daemon_steps.rs`; restore the intra-doc links in `src/lib.rs`

**Interfaces:**
- Consumes: Task 1 (`set_boot_report`), Task 2 (`new_for_daemon_gated`), Task 3 (runner types).
- Produces:

```rust
pub struct DaemonBootCtx { pub orch, pub orch_config, pub repository, pub repository_id, pub profile, pub start_dir, pub plugins_dir, pub db, pub session_manager, pub skill_registry, pub state, pub populi_remote_snapshot, pub populi_poll_join }
impl DaemonBootCtx { pub fn new(cfg: OrchestratorConfig, start_dir: Option<&Path>, profile: ActiveProfile, plugins_dir: PathBuf) -> Self }
impl BootContext for DaemonBootCtx
pub static DAEMON_STEPS: &[BootStep<DaemonBootCtx>];
pub fn plugins_install_dir(env_value: Option<String>) -> PathBuf;
```

- [ ] **Step 1: Write the tests first** (the `tests` module at the bottom of the file below), with `DAEMON_STEPS` temporarily `&[]`. Run `cargo nextest run -p vox-orchestrator-d daemon_steps`: `dev_plan_runs_todays_steps_in_todays_order` and `every_subsystem_has_a_step` FAIL.

- [ ] **Step 2: Full file:**

```rust
//! The daemon's boot steps as a [`BootStep`] table, in pre-profile order.
//!
//! Each step is the code that used to sit inline in `async_main`, moved verbatim apart from
//! reading/writing [`DaemonBootCtx`] instead of locals. Steps for subsystems that were lazy
//! before profiles (`model_registry`, `llm_facade`, `search_index`, `memory`, `compiler`,
//! `browser`, `speech`) are probes or markers: they do no eager IO, they give the subsystem a
//! slot in the closure and in the [`BootReport`] the MCP gate reads.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use vox_orchestrator::{
    Orchestrator, OrchestratorConfig, RemotePopuliSnapshot, SessionConfig, SessionManager, a2a,
    build_repo_scoped_orchestrator, clarification_db_inbox_poll, mesh_federation_poll, runtime,
};
use vox_orchestrator_mcp::profiles::{ActiveProfile, BootReport, Subsystem};
use vox_orchestrator_mcp::server_state::ServerState;

use crate::boot_plan::{BootContext, BootStep, StepFn, StepFuture};

pub struct DaemonBootCtx {
    pub orch: Arc<Orchestrator>,
    pub orch_config: OrchestratorConfig,
    pub repository: vox_repository::RepositoryContext,
    pub repository_id: String,
    pub profile: ActiveProfile,
    /// Repository discovery hint for the store; `None` = process CWD (production).
    pub start_dir: Option<PathBuf>,
    pub plugins_dir: PathBuf,
    pub db: Option<Arc<vox_db::VoxDb>>,
    pub session_manager: Option<SessionManager>,
    pub skill_registry: Option<Arc<vox_skills::SkillRegistry>>,
    pub state: Option<ServerState>,
    pub populi_remote_snapshot: Arc<RwLock<RemotePopuliSnapshot>>,
    pub populi_poll_join: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl DaemonBootCtx {
    pub fn new(
        cfg: OrchestratorConfig,
        start_dir: Option<&Path>,
        profile: ActiveProfile,
        plugins_dir: PathBuf,
    ) -> Self {
        let build = build_repo_scoped_orchestrator(cfg, start_dir);
        let orch_config = build.config.clone();
        let repository_id = build.repository.repository_id.clone();
        Self {
            orch: Arc::new(build.orchestrator),
            orch_config,
            repository: build.repository,
            repository_id,
            profile,
            start_dir: start_dir.map(Path::to_path_buf),
            plugins_dir,
            db: None,
            session_manager: None,
            skill_registry: None,
            state: None,
            populi_remote_snapshot: Arc::new(RwLock::new(RemotePopuliSnapshot::default())),
            populi_poll_join: Arc::new(Mutex::new(None)),
        }
    }
}

impl BootContext for DaemonBootCtx {
    fn publish(&mut self, report: &BootReport) {
        if let Some(state) = self.state.as_ref() {
            state.set_boot_report(report.clone());
        }
    }
}

/// `$VOX_PLUGINS_DIR` if set, else the platform-local default (moved verbatim from `async_main`).
pub fn plugins_install_dir(env_value: Option<String>) -> PathBuf {
    env_value.map(PathBuf::from).unwrap_or_else(|| {
        dirs::data_local_dir()
            .map(|p| p.join("vox").join("plugins"))
            .unwrap_or_else(|| PathBuf::from("./vox-plugins"))
    })
}

fn step_db(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let db = vox_db::connect_workspace_journey_optional_at(
            vox_db::DbConnectSurface::Runtime,
            ctx.start_dir.as_deref(),
            false,
        )
        .await
        .ok_or_else(|| "workspace journey store unavailable".to_string())?;
        let db = Arc::new(db);
        if let Err(e) = ctx.orch.init_db(db.clone()).await {
            tracing::warn!(error = %e, "orchestrator init_db failed; continuing without persisted Codex");
            return Err(format!("orchestrator init_db failed: {e}"));
        }
        tracing::info!("Codex attached and orchestrator schema synced");
        ctx.db = Some(db);
        Ok(())
    })
}

fn step_model_registry(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let handle = ctx.orch.models_handle();
        let n = vox_orchestrator::sync_lock::rw_read(&*handle)
            .list_models()
            .len();
        tracing::info!(models = n, "model registry ready");
        Ok(())
    })
}

fn step_routing_preferences(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        // Apply any GUI-persisted model-emphasis (routing priority) as this daemon's
        // global scorer default. Per-call SelectionAxes still override it via the
        // scorer's thread-local. Done before serving so all scoring sees it.
        let Some(db) = ctx.db.as_ref() else {
            return Ok(());
        };
        if let Ok(Some(csv)) = db
            .get_user_preference("local_user", "routing_priority")
            .await
        {
            let csv = csv.trim();
            if !csv.is_empty() {
                // Install as a thread-safe process-global rather than mutating the process
                // environment (set_var under the multi-threaded runtime is UB). See
                // `install_base_routing_priority`.
                match vox_config::AutoRoutingPriority::try_parse_csv(csv) {
                    Some(axes) => {
                        vox_orchestrator::models::install_base_routing_priority(Some(axes));
                        tracing::info!(priority = %csv, "applied persisted routing-priority emphasis");
                    }
                    None => tracing::warn!(
                        priority = %csv,
                        "routing_priority preference parsed no axes; leaving scorer default unchanged"
                    ),
                }
            }
        }
        // Apply any persisted ordered selection-policy chain; empty / absent leaves the
        // pre-existing selection cascade unchanged.
        if let Ok(Some(json)) = db
            .get_user_preference("local_user", "selection_policy")
            .await
        {
            let json = json.trim();
            if !json.is_empty() {
                match vox_orchestrator::models::SelectionPolicy::from_json(json) {
                    Ok(policy) => {
                        let n = policy.steps.len();
                        vox_orchestrator::models::install_active_policy(Some(policy));
                        tracing::info!(steps = n, "applied persisted selection-policy chain");
                    }
                    Err(e) => tracing::warn!(
                        error = %e,
                        "selection_policy preference is not valid JSON; ignoring"
                    ),
                }
            }
        }
        Ok(())
    })
}

/// Subsystems initialised lazily per call; booting them is declaring them available.
fn step_lazy(_ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async { Ok(()) })
}

fn step_sessions(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let cfg = SessionConfig {
            repository_id: Some(ctx.repository_id.clone()),
            sessions_dir: ctx
                .repository
                .root
                .join(vox_config::mcp_sessions_dir(&ctx.repository_id)),
            ..SessionConfig::default()
        };
        let sm = SessionManager::new(cfg)
            .map_err(|e| format!("session manager initialization failed: {e}"))?;
        ctx.session_manager = Some(sm);
        Ok(())
    })
}

fn step_skills(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        // Skills are discovered from plugins; bridge plugin-host skills into vox-skills.
        let registry = vox_skills::new_registry_arc();
        vox_orchestrator_mcp::plugin_skills_bridge::install_discovered_skills(
            &registry,
            &ctx.plugins_dir,
        )
        .await;
        ctx.skill_registry = Some(registry);
        Ok(())
    })
}

fn step_server_state(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        // `sessions` outside the closure (or failed under `dev`): an in-memory manager keeps
        // ServerState total; the gate denies `vox_session_*` via `requires: [sessions]`.
        let session_manager = match ctx.session_manager.take() {
            Some(sm) => sm,
            None => SessionManager::new(SessionConfig {
                repository_id: Some(ctx.repository_id.clone()),
                persist: false,
                ..SessionConfig::default()
            })
            .map_err(|e| format!("in-memory session manager: {e}"))?,
        };
        let registry = ctx
            .skill_registry
            .clone()
            .unwrap_or_else(vox_skills::new_registry_arc);
        let mut state = ServerState::new_for_daemon_gated(
            ctx.orch.clone(),
            ctx.orch_config.clone(),
            ctx.repository.clone(),
            Arc::new(tokio::sync::Mutex::new(session_manager)),
            registry,
            ctx.profile.subsystems.contains(&Subsystem::Research),
        );
        state.active_profile = Arc::new(ctx.profile.clone());
        if let Some(db) = ctx.db.clone() {
            state = state.with_db_initialized(db).await;
        }
        ctx.state = Some(state);
        Ok(())
    })
}

fn step_agent_fleet(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let state = ctx
            .state
            .as_ref()
            .ok_or_else(|| "server state not built".to_string())?;
        // Wire the AgentFleet's AiTaskProcessor with a real ToolDispatcher backed by this
        // same ServerState, so autonomous `@tool` intents go through the MCP approval gate.
        runtime::spawn_agent_fleet_if_enabled_with_dispatcher(
            ctx.orch.clone(),
            Some(
                vox_orchestrator_mcp::autonomous_tool_dispatch::McpToolDispatcher::new_arc(
                    state.clone(),
                ),
            ),
        );
        Ok(())
    })
}

fn step_populi_federation(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        mesh_federation_poll::spawn_populi_federation_poller(
            &ctx.orch_config,
            ctx.repository_id.clone(),
            ctx.db.clone(),
            ctx.orch.clone(),
            Arc::clone(&ctx.populi_remote_snapshot),
            Arc::clone(&ctx.populi_poll_join),
        );
        Ok(())
    })
}

fn step_a2a_result(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        a2a::spawn_populi_remote_result_poller(ctx.orch.clone(), Arc::new(Mutex::new(None)));
        Ok(())
    })
}

fn step_a2a_worker(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        a2a::spawn_populi_remote_worker_poller(ctx.orch.clone(), Arc::new(Mutex::new(None)));
        Ok(())
    })
}

fn step_clarification(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        let db = ctx
            .db
            .clone()
            .ok_or_else(|| "workspace journey store not attached".to_string())?;
        clarification_db_inbox_poll::spawn_clarification_db_inbox_poller(
            db,
            ctx.repository_id.clone(),
            Arc::new(Mutex::new(None)),
        );
        Ok(())
    })
}

fn step_socrates(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        vox_orchestrator::socrates::spawn_socrates_research_poller(ctx.orch.clone());
        Ok(())
    })
}

fn step_flywheel(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        vox_orchestrator::services::flywheel::FlywheelMonitor::new(ctx.orch.clone())
            .spawn()
            .await;
        Ok(())
    })
}

fn step_attention(ctx: &mut DaemonBootCtx) -> StepFuture<'_> {
    Box::pin(async move {
        vox_orchestrator::services::attention_calibration::spawn_attention_calibration(
            ctx.orch.clone(),
        );
        Ok(())
    })
}

const fn s(
    name: &'static str,
    subsystem: Option<Subsystem>,
    legacy: bool,
    run: StepFn<DaemonBootCtx>,
) -> BootStep<DaemonBootCtx> {
    BootStep {
        name,
        subsystem,
        legacy,
        run,
    }
}

/// Pre-profile order for `legacy` steps; new steps slot in topologically.
/// The HTTP gateway stays in `main`, after the exit decision.
pub static DAEMON_STEPS: &[BootStep<DaemonBootCtx>] = &[
    s("db", Some(Subsystem::Db), true, step_db),
    s("model_registry", Some(Subsystem::ModelRegistry), false, step_model_registry),
    s("routing_preferences", Some(Subsystem::ModelRegistry), true, step_routing_preferences),
    s("llm_facade", Some(Subsystem::LlmFacade), false, step_lazy),
    s("search_index", Some(Subsystem::SearchIndex), false, step_lazy),
    s("memory", Some(Subsystem::Memory), false, step_lazy),
    s("sessions", Some(Subsystem::Sessions), true, step_sessions),
    s("skills", None, true, step_skills),
    s("server_state", None, true, step_server_state),
    s("agent_fleet", Some(Subsystem::AgentFleet), true, step_agent_fleet),
    s("populi_federation_poller", Some(Subsystem::Mesh), true, step_populi_federation),
    s("a2a_remote_result_poller", Some(Subsystem::Mesh), true, step_a2a_result),
    s("a2a_remote_worker_poller", Some(Subsystem::Mesh), true, step_a2a_worker),
    s("clarification_inbox", Some(Subsystem::Clarification), true, step_clarification),
    s("socrates_research_poller", Some(Subsystem::Research), true, step_socrates),
    s("flywheel", Some(Subsystem::Flywheel), true, step_flywheel),
    s("attention_calibration", Some(Subsystem::Attention), true, step_attention),
    s("compiler", Some(Subsystem::Compiler), false, step_lazy),
    s("browser", Some(Subsystem::Browser), false, step_lazy),
    s("speech", Some(Subsystem::Speech), false, step_lazy),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot_plan::planned_step_names;

    /// `async_main` before capability profiles, L218–373, in order.
    const TODAYS_ORDER: &[&str] = &[
        "db",
        "routing_preferences",
        "sessions",
        "skills",
        "server_state",
        "agent_fleet",
        "populi_federation_poller",
        "a2a_remote_result_poller",
        "a2a_remote_worker_poller",
        "clarification_inbox",
        "socrates_research_poller",
        "flywheel",
        "attention_calibration",
    ];

    #[test]
    fn dev_plan_runs_todays_steps_in_todays_order() {
        let planned = planned_step_names(DAEMON_STEPS, &ActiveProfile::dev().subsystems);
        let legacy: Vec<&str> = planned
            .iter()
            .copied()
            .filter(|n| DAEMON_STEPS.iter().any(|s| s.name == *n && s.legacy))
            .collect();
        assert_eq!(legacy, TODAYS_ORDER);
    }

    #[test]
    fn dev_plan_runs_every_step() {
        let planned = planned_step_names(DAEMON_STEPS, &ActiveProfile::dev().subsystems);
        let all: Vec<&str> = DAEMON_STEPS.iter().map(|s| s.name).collect();
        assert_eq!(planned, all);
    }

    #[test]
    fn every_subsystem_has_a_step() {
        for sub in Subsystem::ALL {
            assert!(
                DAEMON_STEPS.iter().any(|s| s.subsystem == Some(*sub)),
                "no boot step for {}",
                sub.as_str()
            );
        }
    }

    #[test]
    fn steps_are_topologically_ordered() {
        for (i, step) in DAEMON_STEPS.iter().enumerate() {
            let Some(sub) = step.subsystem else { continue };
            for dep in sub.deps() {
                assert!(
                    DAEMON_STEPS[..i].iter().any(|p| p.subsystem == Some(*dep)),
                    "step {} ({}) runs before its dependency {}",
                    step.name,
                    sub.as_str(),
                    dep.as_str()
                );
            }
        }
    }

    #[test]
    fn models_plan_has_no_db_session_or_mesh_steps() {
        let models = ActiveProfile::resolve(&["models"]).expect("models profile exists");
        let planned = planned_step_names(DAEMON_STEPS, &models.subsystems);
        for banned in [
            "db",
            "sessions",
            "agent_fleet",
            "populi_federation_poller",
            "a2a_remote_result_poller",
            "a2a_remote_worker_poller",
            "clarification_inbox",
            "socrates_research_poller",
            "flywheel",
        ] {
            assert!(!planned.contains(&banned), "{banned} planned for models: {planned:?}");
        }
        assert!(planned.contains(&"skills") && planned.contains(&"server_state"));
    }

    #[test]
    fn plugins_install_dir_prefers_env_value() {
        assert_eq!(
            plugins_install_dir(Some("/opt/vox-plugins".into())),
            PathBuf::from("/opt/vox-plugins")
        );
        let fallback = plugins_install_dir(None);
        assert!(
            fallback.ends_with(Path::new("vox").join("plugins"))
                || fallback == Path::new("./vox-plugins"),
            "{}",
            fallback.display()
        );
    }

    #[test]
    fn ctx_new_builds_orchestrator_without_side_effects() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ctx = DaemonBootCtx::new(
            OrchestratorConfig::for_testing(),
            Some(tmp.path()),
            ActiveProfile::dev(),
            tmp.path().join("plugins"),
        );
        assert!(ctx.db.is_none() && ctx.state.is_none() && ctx.session_manager.is_none());
        assert_eq!(ctx.start_dir.as_deref(), Some(tmp.path()));
        assert!(ctx.populi_poll_join.lock().unwrap().is_none());
    }
}
```

If plan 02 attaches the profile some other way than the `active_profile` field (Task 1 Step 1), replace the line `state.active_profile = Arc::new(ctx.profile.clone());` with plan 02's mechanism. Note that `db` uses `connect_workspace_journey_optional_at(.., ctx.start_dir.as_deref(), ..)`. With `start_dir = None` (production) this is identical to today's `connect_workspace_journey_optional`, which passes the current dir into the same function.

- [ ] **Step 3: Run:**

```bash
cargo nextest run -p vox-orchestrator-d daemon_steps
```

Expected: `7 tests run: 7 passed`.

- [ ] **Step 4: Checks + commit.**

```bash
cargo check -p vox-orchestrator-d && cargo clippy -p vox-orchestrator-d --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-d/src/daemon_steps.rs crates/vox-orchestrator-d/src/lib.rs
git commit -m "feat(daemon): boot step table with dev-order back-compat test" -m "Plan 03 task 4: today's async_main body as BootSteps tagged by Subsystem; lazy subsystems get probe/marker steps so they appear in the BootReport."
```

**Parallel-safe with:** Task 9.

---

### Task 5: Rewire `async_main` onto the boot plan

**Files:**
- Modify: `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` L5–13 (imports) and L211–384 (body)

**Interfaces:**
- Consumes: `ActiveProfile::from_env` (plan 02), `boot_plan::*`, `daemon_steps::*`, `ServerState::set_boot_report`.
- Produces: no new API; process exits non-zero when `every_required_failed`.

- [ ] **Step 1: Failing test first** (bin `tests` module). It pins the exit message so the bail text stays stable for operators and `vox doctor`.

```rust
    #[test]
    fn all_required_failed_message_names_profile_and_failures() {
        let mut report = vox_orchestrator_mcp::profiles::BootReport::default();
        report.failed.insert(
            vox_orchestrator_mcp::profiles::Subsystem::Db,
            "workspace journey store unavailable".into(),
        );
        let msg = all_required_failed_message("knowledge", &report);
        assert!(msg.contains("profile `knowledge`"), "{msg}");
        assert!(msg.contains("db: workspace journey store unavailable"), "{msg}");
    }
```

Run `cargo nextest run -p vox-orchestrator-d all_required_failed_message` → compile error (fn missing).

- [ ] **Step 2: Implement.** Replace the imports at L9–13 with:

```rust
use vox_orchestrator::{OrchestratorConfig, orch_daemon};
use vox_orchestrator_d::boot_plan::{self, DependencyMode};
use vox_orchestrator_d::daemon_steps::{self, DaemonBootCtx};
use vox_orchestrator_mcp::profiles::{ActiveProfile, BootReport};
```

and change `use std::sync::{Arc, Mutex, RwLock};` to `use std::sync::Arc;`.

Add next to `wants_help`:

```rust
fn all_required_failed_message(profile_id: &str, report: &BootReport) -> String {
    let failures: Vec<String> = report
        .failed
        .iter()
        .map(|(s, e)| format!("{}: {e}", s.as_str()))
        .collect();
    format!(
        "every subsystem required by profile `{profile_id}` failed to boot ({})",
        failures.join("; ")
    )
}
```

Replace L211 (`let cfg = load_config();`) through L384 (end of the `spawn_http_gateway_if_enabled` block) with the code below. If plan 02 already resolves `ActiveProfile` earlier in `async_main`, reuse that binding and drop the first statement:

```rust
    let profile = ActiveProfile::from_env().map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut ctx = DaemonBootCtx::new(
        load_config(),
        None,
        profile.clone(),
        daemon_steps::plugins_install_dir(std::env::var("VOX_PLUGINS_DIR").ok()),
    );
    let outcome = boot_plan::run_boot_plan(
        daemon_steps::DAEMON_STEPS,
        &mut ctx,
        &profile.subsystems,
        DependencyMode::for_profile(&profile),
    )
    .await?;
    tracing::info!(
        profile = %profile.id_string(),
        booted = ?outcome.report.booted,
        failed = ?outcome.report.failed,
        skipped = ?outcome.skipped,
        "daemon boot plan finished"
    );
    if boot_plan::every_required_failed(&boot_plan::profile_roots(&profile), &outcome.report) {
        anyhow::bail!(
            "{}",
            all_required_failed_message(&profile.id_string(), &outcome.report)
        );
    }
    let state = ctx
        .state
        .take()
        .context("server_state boot step did not run")?;
    state.set_boot_report(outcome.report);
    let orch = ctx.orch.clone();
    let repository_id = ctx.repository_id.clone();

    // Serve orch.tool_call / orch.resolve_approval / orch.list_pending_approvals
    // against this same ServerState so the GUI runs tools + resolves HITL
    // approvals through the one shared orchestrator (B5 path-c, B3 cross-process).
    let extra: Option<Arc<dyn orch_daemon::ExtraDispatch>> = Some(Arc::new(
        vox_orchestrator_mcp::daemon_extra::McpExtraDispatch::new(state.clone()),
    ));

    if let Err(e) = vox_orchestrator_mcp::http_gateway::spawn_http_gateway_if_enabled(state) {
        tracing::error!(error = %e, "Failed to spawn HTTP gateway");
    }
```

L386–397 (stdio / TCP serve) stay unchanged; they use `orch` and `repository_id`. `ctx` stays alive until `async_main` returns, so its join slots and snapshot `Arc`s are not dropped early. `VOX_PLUGINS_DIR` is a non-secret path read exactly as before (moved from L304).

- [ ] **Step 3: Run:**

```bash
cargo nextest run -p vox-orchestrator-d
```

Expected: 8 existing bin tests + 1 new + 12 (`boot_plan`) + 7 (`daemon_steps`): `28 tests run: 28 passed`.

- [ ] **Step 4: Smoke the real binary under `dev` and `models`** (throwaway dir, stdio transport so no token file is written):

```bash
cd "$(mktemp -d)" && printf '[package]\nname = "smoke"\nversion = "0.0.0"\n' > Vox.toml
echo '{"id":"1","method":"orch.ping","params":{}}' | RUST_LOG=info VOX_ORCHESTRATOR_DAEMON_SOCKET=stdio cargo run -q --manifest-path /Users/brbrainerd/dev/vox/.worktrees/capability-profiles/Cargo.toml -p vox-orchestrator-d 2>dev.log | head -1
rm -rf .vox
echo '{"id":"1","method":"orch.ping","params":{}}' | RUST_LOG=info VOX_PROFILE=models VOX_ORCHESTRATOR_DAEMON_SOCKET=stdio cargo run -q --manifest-path /Users/brbrainerd/dev/vox/.worktrees/capability-profiles/Cargo.toml -p vox-orchestrator-d 2>models.log | head -1
rg -c "boot step ok" dev.log models.log; rg "daemon boot plan finished" models.log; ls .vox 2>&1
```

Expected: both runs print a `{"id":"1",...,"ok":true,...}` line. The runner logs `boot step ok` only for subsystem steps (core steps don't log it). `dev.log` therefore has 20 such lines, because the temp dir is a real project and the store opens. `models.log` has 3 (`model_registry`, `routing_preferences`, `llm_facade`), and its summary line shows `booted={ModelRegistry, LlmFacade}` and `failed={}`. The final `ls .vox` reports "No such file or directory": the models run opened no store.

- [ ] **Step 5: Checks + commit.**

```bash
cargo check -p vox-orchestrator-d && cargo clippy -p vox-orchestrator-d --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs
git commit -m "feat(daemon): boot through the profile-gated plan" -m "Plan 03 task 5: async_main resolves VOX_PROFILE, runs DAEMON_STEPS, exits non-zero only if every required subsystem failed, and publishes the final BootReport before serving."
```

---

### Task 6: Real-table boot tests (`models`, `knowledge`)

**Files:**
- Create: `crates/vox-orchestrator-d/tests/boot_profiles.rs`

**Interfaces:**
- Consumes: `DaemonBootCtx`, `DAEMON_STEPS`, `run_boot_plan`, `ActiveProfile::resolve`, `vox_db::store::DEFAULT_PROJECT_STORE_PATH`.

Seams. `BootOutcome.ran` records step execution, as produced by the runner. Whether the project store file (`vox_db::store::DEFAULT_PROJECT_STORE_PATH` under a temp project) exists is the real-IO observation that a turso connection was opened. `knowledge` is the positive control that proves this observation is live. Note: the Populi pollers compile to no-ops in this binary because `vox-orchestrator`'s `populi-transport` feature is not enabled for `vox-orchestrator-d`. So the ran-list is their only observable, which is why the mutation in Task 10 targets the runner.

- [ ] **Step 1: Write the tests:**

```rust
//! Profile-gated boot against the real daemon step table (capability profiles plan 03).

use std::path::{Path, PathBuf};

use vox_orchestrator::OrchestratorConfig;
use vox_orchestrator_d::boot_plan::{BootOutcome, DependencyMode, run_boot_plan};
use vox_orchestrator_d::daemon_steps::{DAEMON_STEPS, DaemonBootCtx};
use vox_orchestrator_mcp::profiles::{ActiveProfile, Subsystem};

const POPULI_STEPS: [&str; 3] = [
    "populi_federation_poller",
    "a2a_remote_result_poller",
    "a2a_remote_worker_poller",
];

fn project_dir() -> tempfile::TempDir {
    assert!(
        !std::env::var("VOX_WORKSPACE_JOURNEY_STORE")
            .is_ok_and(|v| v.trim().eq_ignore_ascii_case("canonical")),
        "unset VOX_WORKSPACE_JOURNEY_STORE=canonical: these tests observe the project-local store"
    );
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        tmp.path().join("Vox.toml"),
        "[package]\nname = \"boot-test\"\nversion = \"0.0.0\"\n",
    )
    .expect("write Vox.toml");
    tmp
}

fn store_db(dir: &Path) -> PathBuf {
    dir.join(vox_db::store::DEFAULT_PROJECT_STORE_PATH)
}

async fn boot(ids: &[&str], dir: &Path) -> (DaemonBootCtx, BootOutcome) {
    let profile = ActiveProfile::resolve(ids).expect("profile resolves");
    let mut ctx = DaemonBootCtx::new(
        OrchestratorConfig::for_testing(),
        Some(dir),
        profile.clone(),
        dir.join("plugins"),
    );
    let out = run_boot_plan(
        DAEMON_STEPS,
        &mut ctx,
        &profile.subsystems,
        DependencyMode::for_profile(&profile),
    )
    .await
    .expect("boot plan completes");
    (ctx, out)
}

#[tokio::test(flavor = "multi_thread")]
async fn models_profile_opens_no_store_and_starts_no_populi_pollers() {
    let dir = project_dir();
    let (ctx, out) = boot(&["models"], dir.path()).await;

    assert!(!out.ran.contains(&"db"), "db step ran: {:?}", out.ran);
    for step in POPULI_STEPS {
        assert!(!out.ran.contains(&step), "{step} ran: {:?}", out.ran);
    }
    assert!(ctx.db.is_none());
    assert!(
        !store_db(dir.path()).exists(),
        "models boot opened {}",
        store_db(dir.path()).display()
    );
    assert!(ctx.populi_poll_join.lock().unwrap().is_none());

    let state = ctx.state.as_ref().expect("server_state core step ran");
    assert!(state.db.is_none());
    assert_eq!(out.report.booted, ctx.profile.subsystems);
    assert!(out.report.failed.is_empty(), "{:?}", out.report.failed);
    assert_eq!(state.boot_report_snapshot(), out.report);
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_profile_opens_store_but_not_agent_fleet() {
    let dir = project_dir();
    let (ctx, out) = boot(&["knowledge"], dir.path()).await;

    // Positive control: the store observation above is live.
    assert!(out.ran.contains(&"db"), "{:?}", out.ran);
    assert!(store_db(dir.path()).exists());
    assert!(ctx.state.as_ref().expect("state").db.is_some());

    assert!(!out.ran.contains(&"agent_fleet"), "{:?}", out.ran);
    assert!(!out.report.booted.contains(&Subsystem::AgentFleet));
    assert!(!out.report.failed.contains_key(&Subsystem::AgentFleet));
    assert!(out.report.booted.contains(&Subsystem::Sessions));
}
```

- [ ] **Step 2: Run:**

```bash
cargo nextest run -p vox-orchestrator-d --test boot_profiles
```

Expected: `2 tests run: 2 passed`. If plan 02's `knowledge` manifest does not put `Sessions` in the closure, change that last assertion to a subsystem the manifest's `requires` does list. Read it with `rg -n "requires" contracts/profiles/knowledge.v1.yaml`. Task 10 then proves these tests can fail.

- [ ] **Step 3: Checks + commit.**

```bash
cargo clippy -p vox-orchestrator-d --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-d/tests/boot_profiles.rs
git commit -m "test(daemon): profile boot tests for models and knowledge" -m "Plan 03 task 6: models opens no project store and plans no Populi pollers; knowledge opens the store (positive control) and never starts the agent fleet."
```

---

### Task 7: `orch.boot_report` RPC and gating the non-tool research methods

`McpExtraDispatch` serves two RPC methods that bypass `authorize_tool` because they are not tool calls. `research.run` (`daemon_extra.rs:59`) needs Research. `research.persist_claims` (`daemon_extra.rs:95-98`, which today answers "VoxDb not attached to daemon") needs Db.

**Files:**
- Modify: `crates/vox-foundation/src/protocol.rs` (after `ATTENTION_SNAPSHOT`, ~L133)
- Modify: `contracts/orchestration/orch-daemon-rpc-methods.schema.json` (method enum, after `"orch.attention_snapshot"`)
- Modify: `crates/vox-orchestrator-mcp/src/daemon_extra.rs` L17 (imports), L51–100
- Test: `crates/vox-orchestrator-mcp/tests/daemon_extra_tests.rs`

**Interfaces:**
- Produces: `orch_daemon_method::BOOT_REPORT = "orch.boot_report"` — params `{}` → `BootReport` JSON (`{"booted":[..],"failed":{"db":"..."}}`).

- [ ] **Step 1: Failing tests** (append to `daemon_extra_tests.rs`):

```rust
use vox_foundation::protocol::{DispatchPayload, DispatchRequest, dei_method, orch_daemon_method};
use vox_orchestrator_mcp::profiles::{BootReport, Subsystem};

fn req(method: &str, params: serde_json::Value) -> DispatchRequest {
    serde_json::from_value(serde_json::json!({ "id": "t1", "method": method, "params": params }))
        .expect("dispatch request")
}

fn all_but(down: Subsystem) -> BootReport {
    let mut r = BootReport::default();
    r.booted = Subsystem::ALL.iter().copied().filter(|s| *s != down).collect();
    r.failed.insert(down, "test: down".into());
    r
}

#[tokio::test]
async fn boot_report_method_returns_the_daemon_report() {
    let state = ServerState::new_full(load_config());
    let report = all_but(Subsystem::Db);
    state.set_boot_report(report.clone());
    let extra = McpExtraDispatch::new(state);
    let resp = extra
        .try_handle(&req(orch_daemon_method::BOOT_REPORT, serde_json::json!({})))
        .await
        .expect("orch.boot_report handled");
    let DispatchPayload::Result { value } = resp.payload else {
        panic!("orch.boot_report returned an error payload");
    };
    let got: BootReport = serde_json::from_value(value).expect("BootReport json");
    assert_eq!(got, report);
}

#[tokio::test]
async fn research_run_refused_when_research_not_booted() {
    let state = ServerState::new_full(load_config());
    state.set_boot_report(all_but(Subsystem::Research));
    let extra = McpExtraDispatch::new(state);
    let resp = extra
        .try_handle(&req(dei_method::RESEARCH_RUN, serde_json::json!({ "query": "q" })))
        .await
        .expect("research.run handled");
    let DispatchPayload::Error { message, .. } = resp.payload else {
        panic!("research.run must be refused when Research is down");
    };
    assert!(
        message.contains("subsystem_not_booted") && message.contains("\"research\""),
        "{message}"
    );
}

#[tokio::test]
async fn persist_claims_refused_when_db_not_booted() {
    let state = ServerState::new_full(load_config());
    state.set_boot_report(all_but(Subsystem::Db));
    let extra = McpExtraDispatch::new(state);
    let resp = extra
        .try_handle(&req(
            dei_method::RESEARCH_PERSIST_CLAIMS,
            serde_json::json!({ "session_id": 1, "claims": [] }),
        ))
        .await
        .expect("research.persist_claims handled");
    let DispatchPayload::Error { message, .. } = resp.payload else {
        panic!("persist_claims must be refused when Db is down");
    };
    assert!(
        message.contains("subsystem_not_booted") && message.contains("\"db\""),
        "{message}"
    );
}

#[tokio::test]
async fn tool_call_for_db_tool_refused_when_db_not_booted() {
    let entry = vox_mcp_registry::registry_entry("vox_kb_list").expect("vox_kb_list registered");
    assert!(
        vox_mcp_registry::subsystem_closure(entry.requires).contains(&Subsystem::Db),
        "vox_kb_list must require db (directly or transitively)"
    );
    let state = ServerState::new_full(load_config());
    state.set_boot_report(all_but(Subsystem::Db));
    let extra = McpExtraDispatch::new(state);
    let resp = extra
        .try_handle(&req(
            orch_daemon_method::TOOL_CALL,
            serde_json::json!({ "name": "vox_kb_list", "args": {} }),
        ))
        .await
        .expect("orch.tool_call handled");
    let text = match resp.payload {
        DispatchPayload::Result { value } => value.to_string(),
        DispatchPayload::Error { message, .. } => message,
    };
    assert!(text.contains("subsystem_not_booted"), "{text}");
}
```

The last test proves the end-to-end path a stdio `vox mcp` client goes through: `daemon_route::call_tool_via_daemon` → `orch.tool_call` → `handle_tool_call_with_mode` → plan 02's `authorize_tool` reading the daemon's report. If `DispatchPayload` has more variants than `Result`/`Error`, add `_ => panic!("unexpected payload")` to the match.

Run `cargo nextest run -p vox-orchestrator-mcp --test daemon_extra_tests` → compile error (`BOOT_REPORT` missing).

- [ ] **Step 2: Protocol constant** (`crates/vox-foundation/src/protocol.rs`, after `ATTENTION_SNAPSHOT`):

```rust
    /// Params: `{}` → the daemon's `BootReport` (`{"booted": [subsystem ids], "failed":
    /// {subsystem id: error}}`) — which capability-profile subsystems this daemon booted.
    /// Read by stdio `vox mcp` to filter `tools/list` (capability profiles plan 03).
    pub const BOOT_REPORT: &str = "orch.boot_report";
```

Schema enum: change `"orch.attention_snapshot"` to `"orch.attention_snapshot",` and add `"orch.boot_report"` on the next line.

```bash
cargo nextest run -p vox-foundation orch_daemon_method_schema_parity
```

Expected: `1 test run: 1 passed` (it fails if only one of the two edits is made).

- [ ] **Step 3: `McpExtraDispatch`.** In `daemon_extra.rs`, add these crate-local imports below the existing `use crate::server_state::ServerState;`:

```rust
use crate::profiles::Subsystem;
use crate::server_state::subsystem_not_booted_json;
```

As the first statement inside the `dei_method::RESEARCH_RUN => {` arm (before params parsing), add:

```rust
                if !self.state.subsystem_up(Subsystem::Research) {
                    return Some(error(
                        &req.id,
                        subsystem_not_booted_json(
                            "method",
                            dei_method::RESEARCH_RUN,
                            Subsystem::Research,
                        ),
                    ));
                }
```

As the first statement inside `dei_method::RESEARCH_PERSIST_CLAIMS => {`, add:

```rust
                if !self.state.subsystem_up(Subsystem::Db) {
                    return Some(error(
                        &req.id,
                        subsystem_not_booted_json(
                            "method",
                            dei_method::RESEARCH_PERSIST_CLAIMS,
                            Subsystem::Db,
                        ),
                    ));
                }
```

Add a new arm before `orch_daemon_method::TOOL_CALL =>`:

```rust
            orch_daemon_method::BOOT_REPORT => Some(
                match serde_json::to_value(self.state.boot_report_snapshot()) {
                    Ok(value) => result(&req.id, value),
                    Err(e) => error(&req.id, format!("serialize boot report: {e}")),
                },
            ),
```

- [ ] **Step 4: Run:**

```bash
cargo nextest run -p vox-orchestrator-mcp --test daemon_extra_tests
```

Expected: `8 tests run: 8 passed` (4 existing + 4 new). The existing `daemon_research_run_enqueues_session_via_extra_dispatch` still passes because plan 02's default report is "all booted".

- [ ] **Step 5: Checks + commit.**

```bash
cargo check -p vox-foundation -p vox-orchestrator-mcp
cargo clippy -p vox-foundation -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-foundation/src/protocol.rs contracts/orchestration/orch-daemon-rpc-methods.schema.json crates/vox-orchestrator-mcp/src/daemon_extra.rs crates/vox-orchestrator-mcp/tests/daemon_extra_tests.rs
git commit -m "feat(daemon): orch.boot_report RPC and subsystem gate on research methods" -m "Plan 03 task 7: research.run / research.persist_claims bypassed authorize_tool; they now answer subsystem_not_booted. orch.boot_report lets stdio vox mcp see the daemon's boot state."
```

**Parallel-safe with:** Tasks 3, 4, 6 (different crate), after Task 1.

---

### Task 8: stdio `vox mcp` — `tools/list` reflects the daemon's boot state

Stdio tool *calls* already go through the daemon (`server.rs:381` → `daemon_route::call_tool_via_daemon`), so they get the daemon's `subsystem_not_booted` (Task 7, last test). `tools/list`, however, is computed from the stdio process's own local `ServerState` (`server.rs:204`), whose report is plan 02's default "all booted". It must pull the daemon's report, without spawning a daemon just to list tools.

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/daemon_route.rs`
- Modify: `crates/vox-orchestrator-mcp/src/server.rs` L204–210 (start of `list_tools`)

**Interfaces:**
- Produces: `pub async fn fetch_daemon_boot_report() -> Option<BootReport>`. It uses `vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::ping_existing_daemon` and never spawns a daemon.

- [ ] **Step 1: Failing test** (new `#[cfg(test)] mod tests` at the bottom of `daemon_route.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// No daemon listening ⇒ `None`, and nothing is spawned (fails fast on a port nothing binds).
    #[tokio::test]
    #[serial_test::serial(orchestrator_daemon_socket_env)]
    async fn fetch_daemon_boot_report_is_none_without_a_running_daemon() {
        // SAFETY: serialized via `#[serial_test::serial]` on every test that mutates this var.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_ORCHESTRATOR_DAEMON_SOCKET", "127.0.0.1:1");
        }
        assert!(fetch_daemon_boot_report().await.is_none());
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_ORCHESTRATOR_DAEMON_SOCKET");
        }
    }
}
```

Run `cargo nextest run -p vox-orchestrator-mcp fetch_daemon_boot_report` → compile error.

- [ ] **Step 2: Implement** (append to `daemon_route.rs` above the tests):

```rust
/// The boot report of an **already-running** daemon, for filtering `tools/list`.
/// `None` when no daemon answers; listing tools must never spawn one.
pub async fn fetch_daemon_boot_report() -> Option<crate::profiles::BootReport> {
    let client =
        vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::ping_existing_daemon().await?;
    let value = client
        .call(orch_daemon_method::BOOT_REPORT, serde_json::json!({}))
        .await
        .ok()?;
    serde_json::from_value(value).ok()
}
```

In `server.rs` `list_tools`, insert as the first statement of the body:

```rust
        if let Some(report) = crate::daemon_route::fetch_daemon_boot_report().await {
            self.state.set_boot_report(report);
        }
```

Plan 02's filter then runs on the refreshed report.

- [ ] **Step 3: Run:** `cargo nextest run -p vox-orchestrator-mcp fetch_daemon_boot_report` → `1 test run: 1 passed`. Then `cargo nextest run -p vox-orchestrator-mcp list_tools`. The existing list tests run without a daemon, so the refresh is a no-op and they still pass.

- [ ] **Step 4: Manual end-to-end check** (record the output in the PR description):

```bash
cd "$(mktemp -d)" && printf '[package]\nname = "e2e"\nversion = "0.0.0"\n' > Vox.toml
VOX_PROFILE=knowledge VOX_ORCHESTRATOR_DAEMON_SOCKET=127.0.0.1:9799 cargo run -q --manifest-path /Users/brbrainerd/dev/vox/.worktrees/capability-profiles/Cargo.toml -p vox-orchestrator-d &
sleep 5
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized"}' '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' | VOX_PROFILE=knowledge VOX_ORCHESTRATOR_DAEMON_SOCKET=127.0.0.1:9799 cargo run -q --manifest-path /Users/brbrainerd/dev/vox/.worktrees/capability-profiles/Cargo.toml -p vox-cli -- mcp | rg -o '"name":"vox_[a-z_]+"' | sort -u | head -50
kill %1
```

Expected: only `knowledge` tools are listed. Then make the store unopenable (`chmod 000 .vox` after the first daemon run), restart the daemon, and repeat the listing. The `knowledge` tools from Task 9's `DB_TOOLS` list (e.g. `vox_kb_list`) must disappear. Restore with `chmod 755 .vox`.

- [ ] **Step 5: Checks + commit.**

```bash
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/daemon_route.rs crates/vox-orchestrator-mcp/src/server.rs
git commit -m "feat(mcp): stdio tools/list follows the daemon's boot report" -m "Plan 03 task 8: stdio tool calls already hit the daemon gate; tools/list was computed from a local all-booted default."
```

---

### Task 9: Degraded-mode audit — no crash when a subsystem is skipped

**Audit result** (worktree at `8519ca7a1`). No production code panics on a missing db or session manager: `ServerState.db` is `Option`, every reader branches on it, and `session_manager` is always present. Under a skipped `sessions` step it is the in-memory manager from Task 4's `step_server_state`. The risk is different: a tool reaches its graceful "VoxDb is not connected" branch *instead of* returning `subsystem_not_booted`, because its catalog `requires` omits `db`. Every site found:

| Site | Behaviour today | Handling in this plan |
|---|---|---|
| `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:232-282` | prefs read only `if let Some(db)` | `routing_preferences` step no-ops without db (Task 4) |
| `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:290-299` | `SessionManager::new(..)?` aborts the daemon | `sessions` step degrades; `server_state` falls back to a `persist: false` manager (Task 4) |
| `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs:359-365` | clarification poller only with db | `clarification_inbox` step errors without db → `failed` (Task 4) |
| `crates/vox-orchestrator-mcp/src/server_state.rs:348` (`new_for_daemon`) | always spawns scientia mesh jobs | `new_for_daemon_gated` (Task 2) |
| `crates/vox-orchestrator-mcp/src/server_state.rs:646` (`with_db_initialized`) | only called with `Some(db)` | unchanged; `step_server_state` calls it only when `ctx.db` is set |
| `crates/vox-orchestrator-mcp/src/daemon_extra.rs:59` (`research.run`) | no Research/db check; not a tool, bypasses `authorize_tool` | gated on `Research` (Task 7) |
| `crates/vox-orchestrator-mcp/src/daemon_extra.rs:96` (`research.persist_claims`) | "VoxDb not attached to daemon" | gated on `Db` (Task 7) |
| `crates/vox-orchestrator-mcp/src/memory_tools/handlers_session.rs:12,23,34,49,64,77` | `session_manager.lock()` — no panic | gate via `requires ⊇ sessions` (guard test below) |
| `crates/vox-orchestrator-mcp/src/compiler_tools.rs:152` | `session_manager.try_lock()` — no panic | none needed |
| `crates/vox-orchestrator-mcp/src/benchmark_tools.rs:45,149` | `let Some(db) = state.db … else` → error envelope | guard test: `vox_benchmark_list`, `vox_benchmark_record` |
| `crates/vox-orchestrator-mcp/src/questioning_tools.rs:48,102,123` | same | guard test: `vox_questioning_sync_ssot`, `vox_questioning_pending`, `vox_questioning_submit_answer` |
| `crates/vox-orchestrator-mcp/src/memory_tools/handlers_memory.rs:356,454,493` | same | guard test: `vox_research_start`, `vox_research_status`, `vox_research_get` |
| `crates/vox-orchestrator-mcp/src/kb_tools/handlers.rs:17` (`require_db`) | same | guard test: `vox_kb_create`, `vox_kb_list`, `vox_kb_delete` |
| `crates/vox-orchestrator-mcp/src/codex_tools.rs:15` (`require_db`) | same | guard test: `vox_db_research_session_upsert` |
| `crates/vox-orchestrator-mcp/src/trust_tools.rs:13` (`require_db`) | same | guard test: `vox_db_trust_rollups`, `vox_db_trust_summary`, `vox_db_trust_drift`, `vox_db_trust_propagate` |
| `crates/vox-orchestrator-mcp/src/reliability_tools.rs:25,44` | same | no direct `dispatch.rs` arm found for these helpers; covered by whichever tool calls them (no change) |
| `crates/vox-orchestrator-mcp/src/attention_policy.rs:84`, `dei_tools/orchestrator_snapshot.rs:335`, `chat_socrates_meta.rs:156,273,751` | background/helper paths that skip work without db | no change (not tool entry points) |
| `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs:1772` | `.expect("db attached")` | inside `#[tokio::test]` (L1724); not production |

**Files:**
- Create: `crates/vox-orchestrator-mcp/tests/boot_requires_guard.rs`

- [ ] **Step 1: Write the guard test:**

```rust
//! Tools whose handlers need a db / session manager must declare it in `requires`, so the
//! profile gate answers `subsystem_not_booted` instead of the handler's own fallback.
//! The tool lists come from the plan-03 degraded-mode audit.

use vox_mcp_registry::{Subsystem, registry_entry, subsystem_closure};

const DB_TOOLS: &[&str] = &[
    "vox_benchmark_list",
    "vox_benchmark_record",
    "vox_questioning_sync_ssot",
    "vox_questioning_pending",
    "vox_questioning_submit_answer",
    "vox_research_start",
    "vox_research_status",
    "vox_research_get",
    "vox_kb_create",
    "vox_kb_list",
    "vox_kb_delete",
    "vox_db_research_session_upsert",
    "vox_db_trust_rollups",
    "vox_db_trust_summary",
    "vox_db_trust_drift",
    "vox_db_trust_propagate",
];

const SESSION_TOOLS: &[&str] = &[
    "vox_session_create",
    "vox_session_list",
    "vox_session_reset",
    "vox_session_compact",
    "vox_session_info",
    "vox_session_cleanup",
];

fn assert_requires(tools: &[&str], needed: Subsystem) {
    let mut missing = Vec::new();
    for name in tools {
        let entry =
            registry_entry(name).unwrap_or_else(|| panic!("{name} is not in the MCP registry"));
        if !subsystem_closure(entry.requires).contains(&needed) {
            missing.push(*name);
        }
    }
    assert!(
        missing.is_empty(),
        "these tools need `{}` in catalog requires (contracts/operations/catalog.v1.yaml), then `vox ci operations-sync --target mcp --write`: {missing:?}",
        needed.as_str()
    );
}

#[test]
fn db_backed_tools_require_db() {
    assert_requires(DB_TOOLS, Subsystem::Db);
}

#[test]
fn session_tools_require_sessions() {
    assert_requires(SESSION_TOOLS, Subsystem::Sessions);
}
```

- [ ] **Step 2: Run:**

```bash
cargo nextest run -p vox-orchestrator-mcp --test boot_requires_guard
```

Expected: pass if plan 01 classified them correctly. If it fails, the message names the tools. Fix the `requires` in `contracts/operations/catalog.v1.yaml` for exactly those ops (add `db` or `sessions`), regenerate, and re-run until `2 tests run: 2 passed`:

```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp --write
cargo run -q -p vox-cli -- ci operations-sync
```

If a name is not in the registry at all (`… is not in the MCP registry`), confirm with `rg -n '"<name>"' crates/vox-orchestrator-mcp/src/dispatch.rs` that it is dispatch-only (no catalog op), and drop it from the list with a comment saying so.

- [ ] **Step 3: Commit.**

```bash
git add crates/vox-orchestrator-mcp/tests/boot_requires_guard.rs contracts/operations/catalog.v1.yaml contracts/mcp/tool-registry.canonical.yaml
git commit -m "test(mcp): guard db/session tools declare subsystem requires" -m "Plan 03 task 9: degraded boot relies on requires to answer subsystem_not_booted instead of each handler's own db-missing fallback."
```

**Parallel-safe with:** Tasks 2–6.

---

### Task 10: Mutation-verify the closure guard

**Files:** `crates/vox-orchestrator-d/src/boot_plan.rs` (temporary edit, restored).

- [ ] **Step 1: Before-presence check.**

```bash
rg -c 'if !closure.contains\(&sub\) \{' crates/vox-orchestrator-d/src/boot_plan.rs
git diff --quiet -- crates/vox-orchestrator-d/src/boot_plan.rs && echo CLEAN
```

Expected: `1` and `CLEAN`.

- [ ] **Step 2: Break the guard** (the runner ignores the closure and runs everything). In `run_boot_plan`, change `if !closure.contains(&sub) {` to `if false && !closure.contains(&sub) {`. Confirm:

```bash
rg -c 'if false && !closure.contains\(&sub\) \{' crates/vox-orchestrator-d/src/boot_plan.rs
```

Expected: `1`.

- [ ] **Step 3: Tests must fail:**

```bash
cargo nextest run -p vox-orchestrator-d --test boot_profiles models_profile_opens_no_store_and_starts_no_populi_pollers
cargo nextest run -p vox-orchestrator-d skips_steps_outside_closure
```

Expected: both FAIL. The first fails with `db step ran: [...]`; the second with `left: ["db", "models", "core"]`.

- [ ] **Step 4: Restore and verify the restore.**

```bash
git checkout -- crates/vox-orchestrator-d/src/boot_plan.rs
rg -c 'if !closure.contains\(&sub\) \{' crates/vox-orchestrator-d/src/boot_plan.rs
rg -c 'if false' crates/vox-orchestrator-d/src/boot_plan.rs || echo "0 (restored)"
cargo nextest run -p vox-orchestrator-d
```

Expected: `1`, `0 (restored)`, all tests pass. Note in the PR description that the mutation was run, with the failing assertion text.

- [ ] **Step 5: Second mutation (strict dependency rule).** Change `if mode == DependencyMode::Strict {` to `if false {`. Run `cargo nextest run -p vox-orchestrator-d strict_mode_fails_dependents` and expect FAIL. Restore with `git checkout -- crates/vox-orchestrator-d/src/boot_plan.rs`, then re-check: `rg -c 'if mode == DependencyMode::Strict' crates/vox-orchestrator-d/src/boot_plan.rs` → `1`. No commit (the tree is unchanged).

---

### Task 11: Docs — "Boot and degraded mode"

**Files:**
- Modify: `docs/src/reference/capability-profiles.md` (created by plan 02; append a section)
- Modify: `docs/src/reference/orchestration-unified.md` (add a section after "Workspace journey store (Codex)")

- [ ] **Step 1: Append to `capability-profiles.md`:**

```markdown
## Boot and degraded mode

`vox-orchestrator-d` boots only the subsystems in the active profile's closure (the profile's
`requires` plus their dependencies). Steps run in a fixed order; `dev` runs every step in the
same order as before profiles existed.

| Subsystem | Boot step(s) | Notes |
|---|---|---|
| `db` | `db` | Opens the workspace journey store and syncs the orchestrator schema. |
| `model_registry` | `model_registry`, `routing_preferences` | Persisted routing preferences need `db`; without it the scorer defaults stand. |
| `llm_facade`, `search_index`, `memory`, `compiler`, `browser`, `speech` | one step each | Initialised lazily per call; the step declares them available. |
| `sessions` | `sessions` | |
| `agent_fleet` | `agent_fleet` | Still honours `VOX_MCP_AGENT_FLEET`. |
| `mesh` | `populi_federation_poller`, `a2a_remote_result_poller`, `a2a_remote_worker_poller` | |
| `clarification` | `clarification_inbox` | |
| `research` | `socrates_research_poller` (+ scientia mesh jobs) | |
| `flywheel` | `flywheel` | |
| `attention` | `attention_calibration` | |

**Failure degrades, it does not abort.** A failed step puts its subsystem in the boot report's
`failed` map with the error, and boot continues. Under any named profile, subsystems that depend
on a failed one are not started and are recorded as `dependency <x> failed`. Under `dev`, they
still start, matching pre-profile behaviour (for example, sessions fall back to file persistence
when the store is unavailable). The daemon exits non-zero only when **every** subsystem the
profile `requires` failed.

**What clients see.** Tools whose `requires` include a subsystem that is not up are dropped from
`tools/list`. Calling one anyway returns `subsystem_not_booted` (HTTP 503) with the subsystem
name. Stdio `vox mcp` reads the daemon's report through the `orch.boot_report` RPC, and the
daemon-only RPCs `research.run` and `research.persist_claims` apply the same check.
```

- [ ] **Step 2: Add to `orchestration-unified.md`** after the "Workspace journey store (Codex)" section:

```markdown
## Profile-gated daemon boot

`vox-orchestrator-d` reads `VOX_PROFILE` (unset ⇒ `dev`) and runs its boot step table
(`crates/vox-orchestrator-d/src/daemon_steps.rs`) through the runner in
`crates/vox-orchestrator-d/src/boot_plan.rs`. Only steps whose subsystem is in the profile's
closure run. The resulting boot report is attached to the daemon's MCP `ServerState` and served
by the `orch.boot_report` JSON-line method. Without a project store, `dev` behaves as before:
Codex is not attached and dependent loops still start. See
[capability profiles — boot and degraded mode](capability-profiles.md#boot-and-degraded-mode).
```

- [ ] **Step 3: Lint:**

```bash
cargo run -q -p vox-doc-pipeline -- --lint-only --paths reference/capability-profiles.md reference/orchestration-unified.md
```

Expected: no errors.

- [ ] **Step 4: Commit.**

```bash
git add docs/src/reference/capability-profiles.md docs/src/reference/orchestration-unified.md
git commit -m "docs: capability profiles boot and degraded mode" -m "Plan 03 task 11."
```

**Parallel-safe with:** Task 9, Task 10 (docs only), once Tasks 5 and 7 have fixed the names.

---

### Task 12: Final verification

- [ ] **Step 1:**

```bash
cargo check -p vox-orchestrator-d -p vox-orchestrator-mcp -p vox-foundation -p vox-mcp-registry
cargo clippy -p vox-orchestrator-d -p vox-orchestrator-mcp -p vox-foundation -p vox-mcp-registry --all-targets -- -D warnings
cargo nextest run -p vox-orchestrator-d
cargo nextest run -p vox-orchestrator-mcp boot_gate daemon_extra boot_requires_guard fetch_daemon_boot_report
cargo nextest run -p vox-foundation orch_daemon_method_schema_parity
cargo run -q -p vox-arch-check
cargo run -q -p vox-cli -- ci crate-edges
VOX_FMT_CHECK=1 vox run scripts/fmt.vox
vox ci line-endings
vox ci pre-push --complete
```

Expected: all green. `crate-edges` must report no new edge, because `tempfile` is external. If `vox-arch-check` reports `arch/god_object` on `server_state.rs`, move the `new_for_daemon` delegate into `boot_gate.rs` (a child module can call the parent's private helpers). Do not add a suppression.

- [ ] **Step 2: Self-review the full branch range** (`git diff origin/main...HEAD -- . ':(exclude)Cargo.lock'`), reading every hunk.

---

## Self-Review

- **Spec coverage.** §5 "runs each existing boot step only if its subsystem is in [the closure]": Tasks 3–5. "`dev` … reproduces today's order exactly": the Task 4 order tests plus lenient mode. §8 "degrades rather than aborts … dependent tools drop from `tools/list` … exits non-zero if every required subsystem fails": Tasks 3, 5, 8. §9.3 boot tests (`models` opens no turso connection, starts no Populi pollers): Task 6, in the crate's `tests/` dir rather than `vox-test-harness` (justified above). §9.2 mutation-verified guards: Task 10.
- **Contract names used verbatim:** `Subsystem`, `Subsystem::ALL`, `deps()`, `as_str()`, `subsystem_closure`, `BootReport{booted,failed}`, `is_up`, `ActiveProfile{ids,subsystems}`, `ActiveProfile::{dev,resolve,from_env,id_string}`, `PROFILES`, `ProfileManifest.requires`, `DEV_PROFILE_ID`, `ProfileDenial::SubsystemNotBooted(s).code()`, `registry_entry`, `McpToolRegistryEntry.requires`, wire shape `{"error":{"code","message","subsystem"}}`.
- **Every new `pub fn` has a same-file test:** `planned_step_names`, `run_boot_plan`, `profile_roots`, `every_required_failed`, `DependencyMode::for_profile` (boot_plan.rs); `DaemonBootCtx::new`, `plugins_install_dir` (daemon_steps.rs); `set_boot_report`, `boot_report_snapshot`, `subsystem_up`, `subsystem_not_booted_json`, `new_for_daemon_gated` (boot_gate.rs); `fetch_daemon_boot_report` (daemon_route.rs).
- **No placeholders.** The only conditional instructions are the reconciliation points with plans 01/02, which are being written in parallel: field names on `ServerState`, an existing denial-JSON helper, and the `knowledge` manifest's `requires`. Each has an exact grep and an exact fallback.
- **Ordering.** The table is topologically sorted (tested). The HTTP gateway moved after the exit decision, but it was already the last effect before serving, so no observable order changes for `dev`.

## Risks

- **Plan 01/02 drift.** This plan assumes `ServerState.active_profile` / `ServerState.boot_report: Arc<parking_lot::RwLock<BootReport>>` and plan 02's gate inside `handle_tool_call_with_mode`. Task 1 Step 1 checks this. If plan 02 put the gate only in `server.rs`/HTTP and not in dispatch, `orch.tool_call` (GUI, stdio-via-daemon) would bypass it. Task 7's last test catches that, and the fix belongs in plan 02's gate location.
- **Contract amendment needed.** `BootReport` must derive `serde::Deserialize` so stdio can read `orch.boot_report`. The new RPC method `orch.boot_report` is also not in `00-contracts.md`. Record both there: the derive in Task 1 Step 2, and one line under §Wire shapes for the method when Task 7 lands.
- **Hard-to-gate subsystems.** `model_registry` is built inside `Orchestrator::new`, so it cannot be skipped, only reported; plan 05 (`vox-models` extraction) is what makes it optional. `llm_facade`, `search_index`, `memory`, `compiler`, `browser` and `speech` have no boot-time work today. Their steps are markers, so "booted" means "allowed", not "warmed up". A provider outage surfaces per call, not in the report. Research also has hidden work in `ServerState::new_for_daemon`: the scientia mesh jobs and the `vox-publisher` intake (feature `news-publish`) inside the same call. Both are now gated (Task 2). Skill hydration, webhook intake and the NVML VRAM probe inside the constructor stay unconditional core work.
- **Populi observability.** The pollers are no-ops in the daemon build (`populi-transport` off), so tests can observe "not started" only through the step ran-list. If the feature is enabled later, add a join-slot assertion with `populi_control_url` set.
- **Behaviour change outside `dev`.** Named profiles lose persisted routing preferences when `db` is outside the closure (`models` scores with defaults). This is intended, but users switching from `dev` to `models` will notice; it is documented in Task 11.
- **`dev` exactness under failure.** Lenient mode keeps the step order and side effects. But two things now differ in reporting: tools needing a failed `db` answer `subsystem_not_booted` instead of the handler's own "VoxDb is not connected" envelope, and `SessionManager::new` failure no longer aborts the daemon. Both are improvements the spec asks for, so call them out in the PR.
- **Env-sensitive tests.** Task 6 assumes `VOX_WORKSPACE_JOURNEY_STORE` is not `canonical`; the test asserts this up front. It opens a real project store in a tempdir, as `hitl_rehydrate_on_restart_tests.rs` already does elsewhere.
- **`arch/god_object` on `server_state.rs`.** It is already 827 lines and grows by ~12. If the detector fires, move the delegate into `boot_gate.rs` (Task 12) instead of suppressing it.
