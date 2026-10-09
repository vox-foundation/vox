---
title: "Capability Profiles — Plan 02: profile manifests, ActiveProfile, filtering, --profile, vox serve, bundle profiles"
description: "Task-by-task TDD plan that adds contracts/profiles/*.v1.yaml, the generated PROFILES table and authorize_tool in vox-mcp-registry, profile filtering of stdio MCP and HTTP tool advertisement and dispatch, vox mcp --profile, vox serve, VOX_PROFILE, profile_mismatch on daemon adopt, and bundle default profiles."
category: "architecture"
status: "proposed"
date: 2026-10-08
---

# Capability Profiles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Vox server can be started with `--profile <ids>` (or `VOX_PROFILE`) so that MCP `tools/list`, HTTP `GET /v1/tools`, WebSocket `list_tools`, and every dispatch path (stdio → daemon, daemon `orch.tool_call`, HTTP, WS, autonomous agent dispatch) expose only the tools in the composed profile, with stable typed errors; with no profile the behaviour is byte-for-byte today's (`dev`).

**Architecture:** Profile manifests live in `contracts/profiles/<id>.v1.yaml`. `crates/vox-mcp-registry/build.rs` (already generating `TOOL_REGISTRY`) also reads them, expands globs/lanes, clips to `max_exposure`, validates tool names, subsystem names and subsystem coverage, and emits `PROFILES` (plus a synthesized `dev`). `vox-mcp-registry` owns the single decision function `authorize_tool` (5 ordered checks). `vox-orchestrator-mcp` holds an `ActiveProfile` + `BootReport` on `ServerState` (the `BootReport` is "everything booted" until plan 03) and calls one guard (`profiles::guard_tool`) from the central dispatcher, the stdio server, the HTTP gateway, and the WS handler. Spawners pass `VOX_PROFILE` to `vox-orchestrator-d`, whose ping reports `"profile"`; adopting a daemon with a different profile fails with `profile_mismatch`.

**Tech Stack:** Rust 2024, `serde`/`serde_yaml` (build script), `thiserror`, `rmcp`, `axum`, `clap`, `toml`; `cargo nextest`; Vox CI subcommands (`vox ci operations-sync`, `vox ci command-sync`, `vox ci generate-plugin-catalog-docs`).

## Global Constraints

Copied from [`00-contracts.md`](00-contracts.md) §Global constraints — apply to every task:

- TDD: failing test first; every new `pub fn` has a test in the same file (`skeleton/untested-pub-api`).
- Security guards are mutation-verified: break the guard, see the test fail, restore, with a before/after presence check (AGENTS.md §PR & Review Discipline).
- No new workspace crate edges in plans 01–04, 06–07. Plan 05's new edges are **proposed for user authorization** and are not added to `contracts/ci/crate-edges.allow.v1.json` by an agent.
- Secrets via `vox_secrets::resolve_secret`; no `std::env::var` for tokens. Non-secret `VOX_*` config via existing `vox-config` patterns.
- Never `cargo fmt --all`; format dirty files with `vox run scripts/fmt.vox`.
- Generated files (`tool-registry.canonical.yaml`, `*.generated.md`) are regenerated, never hand-edited.
- New `.md` under `docs/src/` has frontmatter; lint with `cargo run -p vox-doc-pipeline -- --lint-only --paths <path relative to docs/src>`.
- Per-crate checks before each commit: `cargo check -p <crate>`, `cargo clippy -p <crate> --all-targets -- -D warnings`, targeted `cargo nextest run -p <crate> <filter>`.
- `dev` profile must reproduce today's behaviour exactly (back-compat tests in plans 02 and 03).

Plan-02-specific constraints:

- Work only in `/Users/brbrainerd/dev/vox/.worktrees/capability-profiles` (branch `feat/capability-profiles`). Never touch `/Users/brbrainerd/dev/vox` outside `.worktrees/`.
- Plan 01 has landed: `vox-mcp-registry` exports `Exposure`, `SideEffectClass`, `Subsystem`, `subsystem_closure`, `registry_entry`, and `McpToolRegistryEntry { name, description, product_lane, http_read_role_eligible, exposure, side_effect_class, requires }`; its `build.rs` `ToolEntry` has `exposure: String`, `side_effect_class: String`, `requires: Vec<String>`; `tier`/`VOX_MCP_TIERS` filtering is gone from `server.rs::list_tools`. Line numbers below are from `origin/main` before plan 01 and may have shifted by a few lines — locate by the quoted code, not the number.
- Plan 02 does **not** change caller-role/off-loopback semantics: every host call site passes `CallContext::LOCAL_WRITE` to the guard, and the HTTP gateway keeps its existing `AccessRole` allowlist logic. Plan 04 switches the gateway to the real `CallContext`.

---

## File Structure

| File | Status | Responsibility |
|---|---|---|
| `contracts/profiles/models.v1.yaml` … `speech.v1.yaml` (12 files) | create | First-wave profile manifests (SSOT). |
| `crates/vox-mcp-registry/Cargo.toml` | modify | Runtime deps `serde`, `thiserror`; dev-dep `serde_json`. |
| `crates/vox-mcp-registry/build.rs` | modify | Read/expand/clip/validate manifests; emit `profiles.rs` with `PROFILES` (incl. synthesized `dev`). |
| `crates/vox-mcp-registry/src/boot.rs` | create | `BootReport` (data type + `is_up`/`missing_for`), `ProfileDenial` (`code`/`http_status`). |
| `crates/vox-mcp-registry/src/profile.rs` | create | `ProfileManifest`, `DEV_PROFILE_ID`, `PROFILES`, `ActiveProfile`, `ProfileError`, `CallerRole`, `CallContext`, `authorize_tool`. |
| `crates/vox-mcp-registry/src/lib.rs` | modify | `mod boot; mod profile;` + re-exports. |
| `crates/vox-orchestrator-mcp/src/profiles.rs` | create | Re-export of registry profile types (`vox_orchestrator_mcp::profiles::*`), `is_dev`, `full_boot_report`, `guard_tool`, `denial_error_json`, `is_profile_denial_json`. |
| `crates/vox-orchestrator-mcp/src/lib.rs` | modify | `pub mod profiles;`. |
| `crates/vox-orchestrator-mcp/src/server_state.rs` | modify | `active_profile` + `boot_report` fields, `with_profile`, envelope error detection. |
| `crates/vox-orchestrator-mcp/src/dispatch.rs` | modify | Central dispatch guard. |
| `crates/vox-orchestrator-mcp/src/registry.rs` | modify | `tool_registry_for(state)`, profile-filtered `merged_tool_registry`, back-compat snapshot tests. |
| `crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt` | create | Snapshot of today's default-feature `tools/list`. |
| `crates/vox-orchestrator-mcp/src/server.rs` | modify | `list_tools` skill-tool filtering, `call_tool` local guard, daemon ensure built `for_profile`, envelope error detection. |
| `crates/vox-orchestrator-mcp/src/lifecycle.rs` | modify | `run_stdio_server_blocking_with_profile`. |
| `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs` | modify | Profile-aware allowlist default + intersection, `visible_tools_for_role` filtering, tests. |
| `crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs` | modify | HTTP status + JSON denial on `/v1/tools/call`; WS/HTTP shared denial in `call_tool_response_value`. |
| `crates/vox-orchestrator/src/orch_daemon/mod.rs` | modify | Ping response gains `"profile"`. |
| `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` | modify | Resolve `VOX_PROFILE` at boot (fail fast on `profile_unknown`), put it on `ServerState`. |
| `crates/vox-cli-core/src/daemon_ipc/orchestrator_daemon_ensure.rs` | modify | `for_profile`, `ping_profile_matches`, `profile_mismatch_error`, adopt refusal, `VOX_PROFILE` on spawn. |
| `crates/vox-gui/src/commands/daemon.rs` | modify | GUI requests `VOX_PROFILE`/`dev`, refuses mismatched adopt, passes `VOX_PROFILE` on spawn. |
| `crates/vox-cli/src/commands/profile_arg.rs` | create | `--profile` > `VOX_PROFILE` env > `~/.vox/config.toml` `VOX_PROFILE` > `dev`. |
| `crates/vox-cli/src/commands/serve.rs` | create | `vox serve --profile` (daemon + HTTP gateway, foreground, no stdio). |
| `crates/vox-cli/src/commands/mcp.rs` | modify | `run(profile)`. |
| `crates/vox-cli/src/commands/mcp_server/mod.rs` | modify | Re-export `run_stdio_server_blocking_with_profile`. |
| `crates/vox-cli/src/commands/mod.rs` | modify | `pub mod profile_arg; pub mod serve;`. |
| `crates/vox-cli/src/lib.rs` | modify | `Cli::Mcp { profile }`, `Cli::Serve { profile }`. |
| `crates/vox-cli/src/cli_dispatch/mod.rs` | modify | Dispatch arms + test update. |
| `contracts/operations/catalog.v1.yaml` | modify | New CLI op `serve`. |
| `contracts/cli/command-registry.yaml` | regenerate | Via `vox ci operations-sync --target cli --write`. |
| `crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt` | regenerate | Via `UPDATE_CLI_CATALOG_BASELINE=1`. |
| `docs/src/reference/cli-command-surface.generated.md` | regenerate | Via `vox ci command-sync`. |
| `docs/src/reference/cli.md` | modify | `### vox mcp` / `### vox serve` sections (required by `check_ref_cli`). |
| `contracts/config/env-vars.v1.yaml`, `docs/src/reference/env-vars.md` | modify | `VOX_PROFILE`. |
| `crates/vox-plugin-catalog/catalog.toml` | modify | `profiles = [...]` on bundles. |
| `crates/vox-plugin-catalog/src/schema.rs`, `src/lib.rs`, `src/docs.rs`, `build.rs` | modify | Parse, expose (`bundle_profiles`), render, validate bundle profiles. |
| `crates/vox-cli/src/commands/plugin_bundle/apply.rs` | modify | Write default profile into `~/.vox/config.toml`. |
| `docs/src/reference/distribution-bundles.generated.md` | regenerate | Via `vox ci generate-plugin-catalog-docs`. |
| `docs/src/reference/capability-profiles.md` | create | Profile reference (profiles, flags, env, errors). |
| `docs/src/architecture/where-things-live.md` | modify | Rows for profiles / `vox serve`; `vox-mcp-registry` row. |

---

### Task 1: `BootReport` and `ProfileDenial` in `vox-mcp-registry`

**Files:**
- Create: `crates/vox-mcp-registry/src/boot.rs`
- Modify: `crates/vox-mcp-registry/src/lib.rs` (module list near the top, after plan 01's `mod exposure; mod subsystem;`)
- Modify: `crates/vox-mcp-registry/Cargo.toml` (`[dependencies]`, `[dev-dependencies]`)

**Interfaces:**
- Consumes: `vox_mcp_registry::Subsystem` (plan 01; `Copy + Ord + Serialize`, `as_str()`).
- Produces (verbatim from 00-contracts):
  - `pub struct BootReport { pub booted: BTreeSet<Subsystem>, pub failed: BTreeMap<Subsystem, String> }` deriving `Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize` (Deserialize per 00-contracts Amendments "From plan 03")
  - `impl BootReport { pub fn is_up(&self, s: Subsystem) -> bool; pub fn missing_for(&self, requires: &[Subsystem]) -> Option<Subsystem>; }`
  - `pub enum ProfileDenial { ToolNotInProfile, SubsystemNotBooted(Subsystem), ExposureDenied, InsufficientRole }` with `pub const fn code(&self) -> &'static str` and `pub const fn http_status(&self) -> u16`.

Parallel-safe with: Task 13.

- [ ] **Step 1: Ensure crate deps.** In `crates/vox-mcp-registry/Cargo.toml`, `[dependencies]` must contain (add any line plan 01 did not add):

```toml
serde = { workspace = true, features = ["derive"] }
thiserror = { workspace = true }
```

and add:

```toml
[dev-dependencies]
serde_json = { workspace = true }
```

- [ ] **Step 2: Write the failing test file.** Create `crates/vox-mcp-registry/src/boot.rs` containing only the test module (the types do not exist yet):

```rust
//! Subsystem boot outcome and the typed denial shared by stdio MCP and HTTP.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Subsystem;

    #[test]
    fn default_report_has_nothing_up() {
        let r = BootReport::default();
        assert!(!r.is_up(Subsystem::Db));
        assert_eq!(r.missing_for(&[]), None);
        assert_eq!(r.missing_for(&[Subsystem::Db]), Some(Subsystem::Db));
    }

    #[test]
    fn missing_for_returns_first_not_up_in_requires_order() {
        let r = BootReport {
            booted: [Subsystem::Db].into_iter().collect(),
            failed: Default::default(),
        };
        assert!(r.is_up(Subsystem::Db));
        assert_eq!(
            r.missing_for(&[Subsystem::Db, Subsystem::SearchIndex, Subsystem::Memory]),
            Some(Subsystem::SearchIndex)
        );
        assert_eq!(r.missing_for(&[Subsystem::Db]), None);
    }

    #[test]
    fn failed_subsystem_is_not_up() {
        let r = BootReport {
            booted: Default::default(),
            failed: [(Subsystem::Db, "connect refused".to_string())].into_iter().collect(),
        };
        assert!(!r.is_up(Subsystem::Db));
    }

    #[test]
    fn report_serializes_with_snake_case_subsystem_ids() {
        let r = BootReport {
            booted: [Subsystem::LlmFacade, Subsystem::ModelRegistry].into_iter().collect(),
            failed: [(Subsystem::Db, "x".to_string())].into_iter().collect(),
        };
        let v = serde_json::to_value(&r).expect("serialize");
        assert_eq!(v["booted"], serde_json::json!(["model_registry", "llm_facade"]));
        assert_eq!(v["failed"]["db"], "x");
    }

    /// Amendment "From plan 03": stdio `vox mcp` deserializes the daemon's `orch.boot_report`.
    #[test]
    fn report_round_trips_through_json() {
        let r = BootReport {
            booted: [Subsystem::LlmFacade].into_iter().collect(),
            failed: [(Subsystem::Db, "no store".to_string())].into_iter().collect(),
        };
        let back: BootReport = serde_json::from_value(serde_json::to_value(&r).expect("ser")).expect("de");
        assert_eq!(back, r);
    }

    #[test]
    fn denial_codes_and_statuses_match_the_contract() {
        let cases = [
            (ProfileDenial::ToolNotInProfile, "tool_not_in_profile", 404),
            (ProfileDenial::SubsystemNotBooted(Subsystem::SearchIndex), "subsystem_not_booted", 503),
            (ProfileDenial::ExposureDenied, "exposure_denied", 403),
            (ProfileDenial::InsufficientRole, "insufficient_role", 403),
        ];
        for (d, code, status) in cases {
            assert_eq!(d.code(), code);
            assert_eq!(d.http_status(), status);
        }
    }
}
```

The `booted` JSON order assertion relies on `Subsystem`'s `Ord` (declaration order: `ModelRegistry` before `LlmFacade`).

Add to `crates/vox-mcp-registry/src/lib.rs` (next to plan 01's module declarations):

```rust
mod boot;
pub use boot::{BootReport, ProfileDenial};
```

- [ ] **Step 3: Run — expect compile failure.**

Run: `cargo nextest run -p vox-mcp-registry boot`
Expected: FAIL — `cannot find type BootReport in this scope` / `unresolved import boot::BootReport`.

- [ ] **Step 4: Implement.** Insert above the test module in `boot.rs`:

```rust
use std::collections::{BTreeMap, BTreeSet};

use crate::Subsystem;

/// Which subsystems the daemon actually brought up. Plan 03 fills this from the conditioned boot;
/// until then hosts use an "everything booted" report.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BootReport {
    pub booted: BTreeSet<Subsystem>,
    pub failed: BTreeMap<Subsystem, String>,
}

impl BootReport {
    pub fn is_up(&self, s: Subsystem) -> bool {
        self.booted.contains(&s)
    }

    /// First requirement (in `requires` order) that is not up.
    pub fn missing_for(&self, requires: &[Subsystem]) -> Option<Subsystem> {
        requires.iter().copied().find(|s| !self.is_up(*s))
    }
}

/// Typed denial, shared by stdio MCP and HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileDenial {
    ToolNotInProfile,
    SubsystemNotBooted(Subsystem),
    ExposureDenied,
    InsufficientRole,
}

impl ProfileDenial {
    pub const fn code(&self) -> &'static str {
        match self {
            ProfileDenial::ToolNotInProfile => "tool_not_in_profile",
            ProfileDenial::SubsystemNotBooted(_) => "subsystem_not_booted",
            ProfileDenial::ExposureDenied => "exposure_denied",
            ProfileDenial::InsufficientRole => "insufficient_role",
        }
    }

    pub const fn http_status(&self) -> u16 {
        match self {
            ProfileDenial::ToolNotInProfile => 404,
            ProfileDenial::SubsystemNotBooted(_) => 503,
            ProfileDenial::ExposureDenied | ProfileDenial::InsufficientRole => 403,
        }
    }
}
```

- [ ] **Step 5: Run — expect pass.**

Run: `cargo nextest run -p vox-mcp-registry boot`
Expected: PASS (5 tests).

- [ ] **Step 6: Checks and commit.**

```bash
cargo check -p vox-mcp-registry
cargo clippy -p vox-mcp-registry --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-mcp-registry/Cargo.toml crates/vox-mcp-registry/src/boot.rs crates/vox-mcp-registry/src/lib.rs
git commit -m "feat(mcp-registry): add BootReport and ProfileDenial"
```

---

### Task 2: Profile manifests, generated `PROFILES`, and `ActiveProfile`

**Files:**
- Create: `contracts/profiles/{models,knowledge,research,research-publish,mesh,attention,skills,orchestrator,lang,repo-read,browser,speech}.v1.yaml`
- Modify: `crates/vox-mcp-registry/build.rs` (append new items; one call added at the end of `main`)
- Create: `crates/vox-mcp-registry/src/profile.rs`
- Modify: `crates/vox-mcp-registry/src/lib.rs`

**Interfaces:**
- Consumes: plan-01 `build.rs` `ToolEntry { name, description, product_lane, http_read_role_eligible, exposure: String, side_effect_class: String, requires: Vec<String> }` and `PRODUCT_LANES`; runtime `Exposure` (`Ord`: `Public < Operator < Internal`), `Subsystem` (`ALL`, `as_str`), `subsystem_closure(&[Subsystem]) -> BTreeSet<Subsystem>`, `registry_entry`, `TOOL_REGISTRY`.
- Produces (verbatim from 00-contracts): `ProfileManifest`, `pub const DEV_PROFILE_ID: &str = "dev"`, `pub static PROFILES: &[ProfileManifest]`, `ActiveProfile { ids, tools, subsystems, axis_surfaces, max_exposure }` with `dev()`, `resolve(&[&str]) -> Result<Self, ProfileError>`, `from_env()`, `contains_tool(&str)`, `id_string()`, and `ProfileError::Unknown(String)` (`"profile_unknown: {0}"`).
- Generated-code semantics: tools sorted ascending; the synthesized `dev` has `requires: Subsystem::ALL`, every registry tool, `max_exposure: Exposure::Internal`, `axis_surfaces: &["*"]` (`"*"` = every Axis surface; see Self-Review / contract note).

Parallel-safe with: none; Task 13 runs right after this task (same push), because the new `"VOX_PROFILE"` read fails `config-registry-parity` until Task 13 registers it.

- [ ] **Step 1: Author the 12 manifests.** Create each file exactly as below. Tool names come from `contracts/mcp/tool-registry.canonical.yaml`; trailing `*` is a prefix glob.

`contracts/profiles/models.v1.yaml`:

```yaml
id: models
description: "Model registry, active-model selection, model suggestion, benchmarks, and chat through the selected model."
requires: [db, model_registry, llm_facade]   # D1: scoreboard, bench evidence, chat history
tools:
  include:
    - vox_list_models
    - vox_set_model
    - vox_get_active_model
    - vox_set_active_model
    - vox_suggest_model
    - vox_capability_model_manifest
    - vox_benchmark_list
    - vox_benchmark_record
    - vox_chat_*
  include_lanes: []
max_exposure: operator
axis_surfaces: [models, chat]
```

`contracts/profiles/knowledge.v1.yaml`:

```yaml
id: knowledge
description: "Agent memory, MCP sessions, knowledge bases, code/graph search, and knowledge queries."
requires: [memory, sessions, search_index]
tools:
  include:
    - vox_memory_*
    - vox_session_*
    - vox_kb_*
    - vox_search_*
    - vox_knowledge_query
  include_lanes: []
max_exposure: operator
axis_surfaces: [memory, knowledge, search]
```

`contracts/profiles/research.v1.yaml`:

```yaml
id: research
description: "Autonomous research runs and Scientia discovery, evaluation, and preparation — without publish, approve, or external-job controls."
requires: [research]
tools:
  include:
    - vox_research_*
    - vox_scientia_assist_suggestions
    - vox_scientia_worthiness_evaluate
    - vox_scientia_publication_decision_explain
    - vox_scientia_publication_discovery_explain
    - vox_scientia_publication_discovery_refresh_evidence
    - vox_scientia_publication_discovery_scan
    - vox_scientia_publication_external_pipeline_metrics
    - vox_scientia_publication_media_list
    - vox_scientia_publication_novelty_fetch
    - vox_scientia_publication_novelty_happy_path
    - vox_scientia_publication_preflight
    - vox_scientia_publication_prepare
    - vox_scientia_publication_route_simulate
    - vox_scientia_publication_scholarly_remote_status
    - vox_scientia_publication_scholarly_staging_export
    - vox_scientia_publication_status
    - vox_news_approval_status
    - vox_news_draft_research
    - vox_news_simulate_publish_gate
    - vox_db_research_session_upsert
    - vox_db_research_metric_linked
  include_lanes: []
max_exposure: operator
axis_surfaces: [research, scientia]
```

`contracts/profiles/research-publish.v1.yaml`:

```yaml
id: research-publish
description: "Scientia/news publish, approve, submission, and external-job controls. Operator-only; opt in explicitly (usually composed as research,research-publish)."
requires: [research]
tools:
  include:
    - vox_scientia_publication_approve
    - vox_scientia_publication_publish
    - vox_scientia_publication_submit_local
    - vox_scientia_publication_arxiv_handoff_record
    - vox_scientia_publication_retry_failed
    - vox_scientia_publication_external_jobs_dead_letter
    - vox_scientia_publication_external_jobs_due
    - vox_scientia_publication_external_jobs_replay
    - vox_scientia_publication_external_jobs_tick
    - vox_scientia_publication_scholarly_pipeline_run
    - vox_scientia_publication_scholarly_remote_status_sync_all
    - vox_scientia_publication_scholarly_remote_status_sync_batch
    - vox_scientia_publication_media_upsert
    - vox_scientia_publication_media_delete
    - vox_news_approve
    - vox_news_test_syndicate
  include_lanes: []
max_exposure: operator
axis_surfaces: [scientia]
```

`contracts/profiles/mesh.v1.yaml`:

```yaml
id: mesh
description: "Populi mesh nodes, dispatch and queue stats, local Populi status, and A2A messaging."
requires: [mesh]
tools:
  include:
    - vox_mesh_*
    - vox_populi_local_status
    - vox_a2a_*
    - vox_publish_message
  include_lanes: []
max_exposure: operator
axis_surfaces: [mesh]
```

`contracts/profiles/attention.v1.yaml`:

```yaml
id: attention
description: "Attention budget summary/history/reset, budget status, and trust overrides."
requires: [attention]
tools:
  include:
    - vox_attention_*
    - vox_budget_status
    - vox_trust_override
  include_lanes: []
max_exposure: operator
axis_surfaces: [attention]
```

`contracts/profiles/skills.v1.yaml`:

```yaml
id: skills
description: "Agent skill discovery, search, install/uninstall, inspection, and use."
requires: []
tools:
  include:
    - vox_skill_*
  include_lanes: []
max_exposure: operator
axis_surfaces: [skills]
```

`contracts/profiles/orchestrator.v1.yaml`:

```yaml
id: orchestrator
description: "Tasks, agents, queue, file locks, plans, handoffs, HITL approvals, and clarification questions."
requires: [agent_fleet, clarification, llm_facade]
tools:
  include:
    - vox_submit_task
    - vox_task_status
    - vox_complete_task
    - vox_fail_task
    - vox_cancel_task
    - vox_reorder_task
    - vox_doubt_task
    - vox_verify_task_claims
    - vox_orchestrator_status
    - vox_orchestrator_start
    - vox_spawn_agent
    - vox_retire_agent
    - vox_pause_agent
    - vox_resume_agent
    - vox_drain_agent
    - vox_rebalance
    - vox_set_agent_budget
    - vox_queue_status
    - vox_lock_status
    - vox_claim_file
    - vox_transfer_file
    - vox_my_files
    - vox_check_file_owner
    - vox_plan
    - vox_plan_status
    - vox_plan_resume
    - vox_plan_list_sessions
    - vox_replan
    - vox_handoff_context
    - vox_handoff_lineage
    - vox_agent_handoff
    - vox_agent_status
    - vox_agent_events
    - vox_agent_assess
    - vox_agent_continue
    - vox_heartbeat
    - vox_poll_events
    - vox_pending_approvals
    - vox_resolve_approval
    - vox_list_approval_allowlist
    - vox_ask_clarification
    - vox_answer_question
    - vox_pending_questions
  include_lanes: []
max_exposure: operator
axis_surfaces: [tasks, agents, approvals, plan]
```

`contracts/profiles/lang.v1.yaml` (exact names, not `vox_check_*`, because that glob also matches `vox_check_file_owner` and `vox_check_mood`):

```yaml
id: lang
description: "Vox language checks, validation, tests, builds/lints, grammar export, and compiler introspection."
requires: [compiler]
tools:
  include:
    - vox_check
    - vox_check_workspace
    - vox_validate_file
    - vox_validate_source
    - vox_validate_vuv
    - vox_run_tests
    - vox_test_all
    - vox_build_crate
    - vox_lint_crate
    - vox_coverage_report
    - vox_language_surface
    - vox_builtin_registry
    - vox_decorator_registry
    - vox_export_grammar_ebnf
    - "vox_compiler::ast_inspect"
    - vox_workspace_modules
    - vox_pipeline_status
  include_lanes: []
max_exposure: operator
axis_surfaces: [lang]
```

`contracts/profiles/repo-read.v1.yaml` (`max_exposure: public` — read-only by construction):

```yaml
id: repo-read
description: "Read-only repository views: git status/diff/log/blame, VCS status, repo catalog/index/queries, snapshots, change log."
requires: []
tools:
  include:
    - vox_git_status
    - vox_git_diff
    - vox_git_log
    - vox_git_blame
    - vox_vcs_status
    - vox_repo_status
    - vox_repo_catalog_list
    - vox_repo_index_status
    - vox_repo_query_file
    - vox_repo_query_history
    - vox_repo_query_text
    - vox_snapshot_list
    - vox_snapshot_diff
    - vox_change_log
  include_lanes: []
max_exposure: public
axis_surfaces: [repo]
```

`contracts/profiles/browser.v1.yaml`:

```yaml
id: browser
description: "Chromium browser automation (vox_browser_*). Never part of any bundle default; opt in explicitly."
requires: [browser]
tools:
  include:
    - vox_browser_*
  include_lanes: []
max_exposure: operator
axis_surfaces: [browser]
```

`contracts/profiles/speech.v1.yaml`:

```yaml
id: speech
description: "Oratio speech capture/transcription and speech-to-code."
requires: [speech]
tools:
  include:
    - vox_oratio_*
    - vox_speech_to_code
  include_lanes: []
max_exposure: operator
axis_surfaces: [speech]
```

- [ ] **Step 2: Write the failing tests.** Create `crates/vox-mcp-registry/src/profile.rs` containing only the test module:

```rust
//! Capability profiles: named tool + subsystem slices generated from `contracts/profiles/*.v1.yaml`.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TOOL_REGISTRY, registry_entry, subsystem_closure};
    use std::collections::BTreeSet;

    const FIRST_WAVE: &[&str] = &[
        "attention", "browser", "knowledge", "lang", "mesh", "models", "orchestrator",
        "repo-read", "research", "research-publish", "skills", "speech",
    ];

    #[test]
    fn profiles_are_first_wave_plus_dev() {
        let mut ids: Vec<&str> = PROFILES.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        let mut want: Vec<&str> = FIRST_WAVE.to_vec();
        want.push(DEV_PROFILE_ID);
        want.sort_unstable();
        assert_eq!(ids, want);
    }

    #[test]
    fn dev_is_every_tool_every_subsystem_internal_all_surfaces() {
        let dev = ActiveProfile::dev();
        assert_eq!(dev.ids, vec![DEV_PROFILE_ID]);
        assert_eq!(dev.tools.len(), TOOL_REGISTRY.len());
        for e in TOOL_REGISTRY {
            assert!(dev.contains_tool(e.name), "dev missing {}", e.name);
        }
        let all: BTreeSet<Subsystem> = Subsystem::ALL.iter().copied().collect();
        assert_eq!(dev.subsystems, all);
        assert_eq!(dev.max_exposure, Exposure::Internal);
        assert!(dev.axis_surfaces.contains("*"));
    }

    #[test]
    fn named_profiles_respect_max_exposure_and_subsystem_closure() {
        for p in PROFILES.iter().filter(|p| p.id != DEV_PROFILE_ID) {
            assert!(p.max_exposure < Exposure::Internal, "{} may not be internal", p.id);
            assert!(!p.tools.is_empty(), "{} resolves to no tools", p.id);
            let closure = subsystem_closure(p.requires);
            for name in p.tools {
                let e = registry_entry(name).unwrap_or_else(|| panic!("{}: unknown {name}", p.id));
                assert!(e.exposure <= p.max_exposure, "{}: {name} above max_exposure", p.id);
                for r in e.requires {
                    assert!(closure.contains(r), "{}: {name} requires {r:?} outside closure", p.id);
                }
            }
        }
    }

    #[test]
    fn manifest_tools_are_sorted_and_unique() {
        for p in PROFILES {
            let mut sorted = p.tools.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted, p.tools, "{} tools not sorted/unique", p.id);
        }
    }

    #[test]
    fn browser_profile_is_only_browser_tools() {
        let b = ActiveProfile::resolve(&["browser"]).expect("browser");
        assert!(b.tools.iter().all(|t| t.starts_with("vox_browser_")));
    }

    #[test]
    fn research_excludes_publish_and_approve_which_live_in_research_publish() {
        let r = ActiveProfile::resolve(&["research"]).expect("research");
        let p = ActiveProfile::resolve(&["research-publish"]).expect("research-publish");
        for t in ["vox_scientia_publication_publish", "vox_scientia_publication_approve", "vox_news_approve"] {
            assert!(!r.contains_tool(t), "research must not contain {t}");
        }
        assert!(r.tools.is_disjoint(&p.tools));
    }

    #[test]
    fn empty_resolve_is_dev() {
        assert_eq!(ActiveProfile::resolve(&[]).expect("empty"), ActiveProfile::dev());
        assert_eq!(ActiveProfile::resolve(&["", "  "]).expect("blank"), ActiveProfile::dev());
    }

    #[test]
    fn composition_is_union_and_canonical() {
        let m = ActiveProfile::resolve(&["models"]).expect("models");
        let k = ActiveProfile::resolve(&["knowledge"]).expect("knowledge");
        let mk = ActiveProfile::resolve(&[" models ", "knowledge"]).expect("models,knowledge");
        assert_eq!(mk, ActiveProfile::resolve(&["knowledge", "models", "models"]).expect("dup"));
        assert_eq!(mk.ids, vec!["knowledge", "models"]);
        assert_eq!(mk.id_string(), "knowledge,models");
        assert_eq!(mk.tools, m.tools.union(&k.tools).copied().collect::<BTreeSet<_>>());
        let roots: Vec<Subsystem> = m.subsystems.union(&k.subsystems).copied().collect();
        assert_eq!(mk.subsystems, subsystem_closure(&roots));
        assert!(mk.axis_surfaces.contains("models") && mk.axis_surfaces.contains("knowledge"));
    }

    #[test]
    fn dev_absorbs_other_ids() {
        assert_eq!(ActiveProfile::resolve(&["models", "dev"]).expect("dev"), ActiveProfile::dev());
    }

    #[test]
    fn unknown_id_is_profile_unknown() {
        let err = ActiveProfile::resolve(&["models", "nope"]).expect_err("unknown");
        assert_eq!(err, ProfileError::Unknown("nope".to_string()));
        assert_eq!(err.to_string(), "profile_unknown: nope");
    }

    #[test]
    fn max_exposure_is_max_over_composed_profiles() {
        assert_eq!(ActiveProfile::resolve(&["repo-read"]).expect("rr").max_exposure, Exposure::Public);
        assert_eq!(
            ActiveProfile::resolve(&["repo-read", "models"]).expect("rr,m").max_exposure,
            Exposure::Operator
        );
    }

    /// Decision D1 (00-contracts Amendments): `models` boots the DB; `attention` and `skills` stay DB-free.
    #[test]
    fn models_requires_db_attention_and_skills_are_db_free() {
        assert!(ActiveProfile::resolve(&["models"]).expect("models").subsystems.contains(&Subsystem::Db));
        for id in ["attention", "skills"] {
            let p = ActiveProfile::resolve(&[id]).expect(id);
            assert!(!p.subsystems.contains(&Subsystem::Db), "{id} must not require db");
        }
    }

    #[test]
    fn contains_tool_matches_resolved_set() {
        let m = ActiveProfile::resolve(&["models"]).expect("models");
        assert!(m.contains_tool("vox_list_models"));
        assert!(!m.contains_tool("vox_git_status"));
        assert!(!m.contains_tool("vox_does_not_exist"));
    }

    #[test]
    fn from_raw_parses_comma_list() {
        assert_eq!(ActiveProfile::from_raw(None).expect("none"), ActiveProfile::dev());
        assert_eq!(ActiveProfile::from_raw(Some("")).expect("empty"), ActiveProfile::dev());
        assert_eq!(
            ActiveProfile::from_raw(Some("models, knowledge")).expect("list").id_string(),
            "knowledge,models"
        );
        assert!(ActiveProfile::from_raw(Some("models,nope")).is_err());
    }

    #[test]
    fn from_env_reads_vox_profile() {
        let raw = std::env::var("VOX_PROFILE").ok();
        assert_eq!(ActiveProfile::from_env(), ActiveProfile::from_raw(raw.as_deref()));
    }
}
```

Add to `crates/vox-mcp-registry/src/lib.rs`:

```rust
mod profile;
pub use profile::{ActiveProfile, DEV_PROFILE_ID, PROFILES, ProfileError, ProfileManifest};
```

- [ ] **Step 3: Run — expect compile failure.**

Run: `cargo nextest run -p vox-mcp-registry profile`
Expected: FAIL — `cannot find value PROFILES` / `cannot find type ActiveProfile`.

- [ ] **Step 4: Extend `build.rs`.** Add to the `use` block at the top of `crates/vox-mcp-registry/build.rs`:

```rust
use std::collections::{BTreeSet, HashSet};
use std::path::Path;
```

Append these items to the end of `build.rs`:

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileFile {
    id: String,
    description: String,
    #[serde(default)]
    requires: Vec<String>,
    tools: ProfileTools,
    max_exposure: String,
    #[serde(default)]
    axis_surfaces: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileTools {
    #[serde(default)]
    include: Vec<String>,
    #[serde(default)]
    include_lanes: Vec<String>,
}

/// Mirrors `Subsystem::deps` (src/subsystem.rs). Drift is caught at runtime by
/// `profile::tests::named_profiles_respect_max_exposure_and_subsystem_closure`.
const SUBSYSTEM_DEPS: &[(&str, &[&str])] = &[
    ("db", &[]),
    ("sessions", &["db"]),
    ("agent_fleet", &["sessions"]),
    ("model_registry", &[]),
    ("llm_facade", &["model_registry"]),
    ("search_index", &["db"]),
    ("memory", &["db", "search_index"]),
    ("research", &["llm_facade", "search_index", "db"]),
    ("mesh", &["db"]),
    ("attention", &[]),
    ("clarification", &["db"]),
    ("flywheel", &["db"]),
    ("compiler", &[]),
    ("browser", &[]),
    ("speech", &[]),
];

fn exposure_rank(s: &str) -> u8 {
    match s {
        "public" => 0,
        "operator" => 1,
        "internal" => 2,
        other => panic!("unknown exposure `{other}`"),
    }
}

fn exposure_variant(s: &str) -> &'static str {
    match s {
        "public" => "Public",
        "operator" => "Operator",
        "internal" => "Internal",
        other => panic!("unknown exposure `{other}`"),
    }
}

fn subsystem_variant(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn subsystem_closure_names(roots: &[String]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack: Vec<String> = roots.to_vec();
    while let Some(s) = stack.pop() {
        let deps = SUBSYSTEM_DEPS
            .iter()
            .find(|(n, _)| *n == s)
            .unwrap_or_else(|| panic!("unknown subsystem `{s}`"))
            .1;
        if out.insert(s) {
            stack.extend(deps.iter().map(|d| d.to_string()));
        }
    }
    out
}

fn str_slice(items: impl IntoIterator<Item = String>) -> String {
    let parts: Vec<String> = items
        .into_iter()
        .map(|s| format!("\"{}\"", escape_rust_string(&s)))
        .collect();
    format!("&[{}]", parts.join(", "))
}

fn emit_profiles(workspace_root: &Path, tools: &[ToolEntry]) -> String {
    let dir = workspace_root.join("contracts/profiles");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.expect("profiles dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
        .collect();
    paths.sort();

    let mut out = String::new();
    out.push_str("// @generated by vox-mcp-registry/build.rs — do not edit.\n\n");
    out.push_str("pub static PROFILES: &[ProfileManifest] = &[\n");
    let mut seen = HashSet::new();
    for path in &paths {
        println!("cargo:rerun-if-changed={}", path.display());
        let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let p: ProfileFile =
            serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
        let stem = path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_suffix(".v1.yaml"))
            .unwrap_or_else(|| panic!("profile file {} must be named <id>.v1.yaml", path.display()));
        if p.id != stem {
            panic!("profile id `{}` does not match file name {}", p.id, path.display());
        }
        if p.id == "dev" {
            panic!("profile id `dev` is reserved (synthesized from the full registry)");
        }
        if !seen.insert(p.id.clone()) {
            panic!("duplicate profile id `{}`", p.id);
        }
        let max = exposure_rank(&p.max_exposure);
        if max == 2 {
            panic!("profile `{}`: named profiles may not set max_exposure: internal", p.id);
        }
        let closure = subsystem_closure_names(&p.requires);

        let mut resolved: BTreeSet<&str> = BTreeSet::new();
        for pat in &p.tools.include {
            if let Some(prefix) = pat.strip_suffix('*') {
                let matches: Vec<&ToolEntry> =
                    tools.iter().filter(|t| t.name.starts_with(prefix)).collect();
                if matches.is_empty() {
                    panic!("profile `{}`: include pattern `{pat}` matches no tool", p.id);
                }
                for t in matches {
                    if exposure_rank(&t.exposure) <= max {
                        resolved.insert(&t.name);
                    }
                }
            } else {
                let t = tools
                    .iter()
                    .find(|t| t.name == *pat)
                    .unwrap_or_else(|| panic!("profile `{}`: unknown tool `{pat}`", p.id));
                if exposure_rank(&t.exposure) > max {
                    panic!(
                        "profile `{}`: tool `{pat}` has exposure `{}` above max_exposure `{}`",
                        p.id, t.exposure, p.max_exposure
                    );
                }
                resolved.insert(&t.name);
            }
        }
        for lane in &p.tools.include_lanes {
            if !PRODUCT_LANES.contains(&lane.as_str()) {
                panic!("profile `{}`: unknown product lane `{lane}`", p.id);
            }
            for t in tools
                .iter()
                .filter(|t| t.product_lane == *lane && exposure_rank(&t.exposure) <= max)
            {
                resolved.insert(&t.name);
            }
        }
        if resolved.is_empty() {
            panic!("profile `{}` resolves to no tools", p.id);
        }
        for name in &resolved {
            let t = tools.iter().find(|t| t.name == *name).expect("resolved from registry");
            for r in &t.requires {
                if !closure.contains(r) {
                    panic!(
                        "profile `{}`: tool `{name}` requires subsystem `{r}`, which is not in the closure of requires {:?}",
                        p.id, p.requires
                    );
                }
            }
        }

        let requires: Vec<String> = p
            .requires
            .iter()
            .map(|s| format!("Subsystem::{}", subsystem_variant(s)))
            .collect();
        out.push_str(&format!(
            "    ProfileManifest {{ id: \"{}\", description: \"{}\", requires: &[{}], tools: {}, max_exposure: Exposure::{}, axis_surfaces: {} }},\n",
            escape_rust_string(&p.id),
            escape_rust_string(&p.description),
            requires.join(", "),
            str_slice(resolved.iter().map(|s| s.to_string())),
            exposure_variant(&p.max_exposure),
            str_slice(p.axis_surfaces.iter().cloned()),
        ));
    }

    let mut all: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
    all.sort();
    out.push_str(&format!(
        "    ProfileManifest {{ id: \"dev\", description: \"Every tool and every subsystem (default; today's behaviour).\", requires: Subsystem::ALL, tools: {}, max_exposure: Exposure::Internal, axis_surfaces: &[\"*\"] }},\n",
        str_slice(all),
    ));
    out.push_str("];\n");
    out
}
```

At the very end of `fn main()` (after the existing `fs::write(out_dir.join("tool_registry.rs"), …)`), add:

```rust
    let profiles_rs = emit_profiles(workspace_root, &root.tools);
    fs::write(out_dir.join("profiles.rs"), profiles_rs).expect("write profiles.rs");
```

- [ ] **Step 5: Implement `profile.rs` types.** Insert above the test module:

```rust
use std::collections::BTreeSet;

use crate::{Exposure, Subsystem, subsystem_closure};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileManifest {
    pub id: &'static str,
    pub description: &'static str,
    pub requires: &'static [Subsystem],
    /// Fully resolved at build time (globs/lanes expanded, clipped to `max_exposure`), sorted.
    pub tools: &'static [&'static str],
    pub max_exposure: Exposure,
    pub axis_surfaces: &'static [&'static str],
}

pub const DEV_PROFILE_ID: &str = "dev";

const PROFILE_ENV: &str = "VOX_PROFILE";

include!(concat!(env!("OUT_DIR"), "/profiles.rs"));

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("profile_unknown: {0}")]
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ActiveProfile {
    pub ids: Vec<&'static str>,
    pub tools: BTreeSet<&'static str>,
    pub subsystems: BTreeSet<Subsystem>,
    pub axis_surfaces: BTreeSet<&'static str>,
    pub max_exposure: Exposure,
}

fn manifest(id: &str) -> Option<&'static ProfileManifest> {
    PROFILES.iter().find(|p| p.id == id)
}

impl ActiveProfile {
    pub fn dev() -> Self {
        let dev = manifest(DEV_PROFILE_ID).expect("generated PROFILES always contains dev");
        Self::from_manifests(&[dev])
    }

    /// Union of the named profiles; empty (or all-blank) ⇒ `dev`; any `dev` id ⇒ `dev`.
    pub fn resolve(ids: &[&str]) -> Result<Self, ProfileError> {
        let mut picked: Vec<&'static ProfileManifest> = Vec::new();
        for raw in ids {
            let id = raw.trim();
            if id.is_empty() {
                continue;
            }
            picked.push(manifest(id).ok_or_else(|| ProfileError::Unknown(id.to_string()))?);
        }
        if picked.is_empty() || picked.iter().any(|m| m.id == DEV_PROFILE_ID) {
            return Ok(Self::dev());
        }
        Ok(Self::from_manifests(&picked))
    }

    /// Reads `VOX_PROFILE` (comma-separated ids); unset or empty ⇒ `dev`.
    pub fn from_env() -> Result<Self, ProfileError> {
        Self::from_raw(std::env::var(PROFILE_ENV).ok().as_deref())
    }

    fn from_raw(raw: Option<&str>) -> Result<Self, ProfileError> {
        let ids: Vec<&str> = raw.unwrap_or("").split(',').collect();
        Self::resolve(&ids)
    }

    pub fn contains_tool(&self, name: &str) -> bool {
        self.tools.contains(name)
    }

    /// Canonical id list: sorted, unique, joined with `,`.
    pub fn id_string(&self) -> String {
        self.ids.join(",")
    }

    fn from_manifests(manifests: &[&'static ProfileManifest]) -> Self {
        let ids: BTreeSet<&'static str> = manifests.iter().map(|m| m.id).collect();
        let roots: Vec<Subsystem> = manifests.iter().flat_map(|m| m.requires.iter().copied()).collect();
        Self {
            ids: ids.into_iter().collect(),
            tools: manifests.iter().flat_map(|m| m.tools.iter().copied()).collect(),
            subsystems: subsystem_closure(&roots),
            axis_surfaces: manifests.iter().flat_map(|m| m.axis_surfaces.iter().copied()).collect(),
            max_exposure: manifests
                .iter()
                .map(|m| m.max_exposure)
                .max()
                .unwrap_or(Exposure::Public),
        }
    }
}
```

- [ ] **Step 6: Run — expect pass, or a build-script panic with an actionable message.**

Run: `cargo nextest run -p vox-mcp-registry profile`
Expected: PASS (14 tests).

If the build script panics because plan 01's classification conflicts with a manifest, apply exactly one of these fixes and re-run (no other edits are allowed):

| Panic | Fix |
|---|---|
| `profile `models`: tool `T` requires subsystem `S` …` | Remove `T` from `models` (replace `vox_chat_*` with the remaining exact `vox_chat_*` names if `T` matched the glob). `models` must stay `db`-free (plan 03 boot test). |
| `profile `P` (≠ models): tool `T` requires subsystem `S` …` | Add `S` to `P`'s `requires`. |
| `tool `T` has exposure `X` above max_exposure …` | Remove `T` from that manifest's `include` (it does not belong in that profile). |
| `include pattern … matches no tool` / `unknown tool` | The canonical registry renamed the tool; use the name now in `contracts/mcp/tool-registry.canonical.yaml`. |

- [ ] **Step 7: Checks and commit.**

```bash
cargo check -p vox-mcp-registry
cargo clippy -p vox-mcp-registry --all-targets -- -D warnings
vox run scripts/fmt.vox
git add contracts/profiles crates/vox-mcp-registry/build.rs crates/vox-mcp-registry/src/profile.rs crates/vox-mcp-registry/src/lib.rs
git commit -m "feat(mcp-registry): generate capability profiles and ActiveProfile"
```

---

### Task 3: `authorize_tool` with the pinned check order

**Files:**
- Modify: `crates/vox-mcp-registry/src/profile.rs`
- Modify: `crates/vox-mcp-registry/src/lib.rs` (extend the `pub use profile::{…}` line)

**Interfaces:**
- Consumes: `BootReport`, `ProfileDenial` (Task 1), `ActiveProfile` (Task 2), `registry_entry`, `McpToolRegistryEntry`.
- Produces (verbatim): `pub enum CallerRole { Read, Write }`, `pub struct CallContext { pub role: CallerRole, pub loopback: bool }`, `CallContext::LOCAL_WRITE`, `pub fn authorize_tool(profile: &ActiveProfile, boot: &BootReport, ctx: CallContext, tool: &str) -> Result<&'static McpToolRegistryEntry, ProfileDenial>`.

- [ ] **Step 1: Write the failing tests.** Append a second test module to `profile.rs`:

```rust
#[cfg(test)]
mod authorize_tests {
    use super::*;
    use crate::{BootReport, McpToolRegistryEntry, ProfileDenial, TOOL_REGISTRY};
    use std::collections::BTreeSet;

    const READ_REMOTE: CallContext = CallContext { role: CallerRole::Read, loopback: false };
    const READ_LOCAL: CallContext = CallContext { role: CallerRole::Read, loopback: true };
    const WRITE_REMOTE: CallContext = CallContext { role: CallerRole::Write, loopback: false };

    fn profile_with(tools: &[&'static str]) -> ActiveProfile {
        ActiveProfile {
            ids: vec!["test"],
            tools: tools.iter().copied().collect(),
            subsystems: BTreeSet::new(),
            axis_surfaces: BTreeSet::new(),
            max_exposure: Exposure::Internal,
        }
    }

    fn full_boot() -> BootReport {
        BootReport { booted: Subsystem::ALL.iter().copied().collect(), failed: Default::default() }
    }

    fn find(pred: impl Fn(&McpToolRegistryEntry) -> bool, what: &str) -> &'static McpToolRegistryEntry {
        TOOL_REGISTRY
            .iter()
            .find(|e| pred(e))
            .unwrap_or_else(|| panic!("registry has no {what} tool (plan 01 classification)"))
    }

    #[test]
    fn step1_unknown_tool_is_not_in_profile() {
        let r = authorize_tool(&ActiveProfile::dev(), &full_boot(), CallContext::LOCAL_WRITE, "vox_does_not_exist");
        assert_eq!(r, Err(ProfileDenial::ToolNotInProfile));
    }

    #[test]
    fn step1_precedes_step2() {
        let internal = find(|e| e.exposure == Exposure::Internal, "internal");
        let r = authorize_tool(&profile_with(&[]), &BootReport::default(), READ_REMOTE, internal.name);
        assert_eq!(r, Err(ProfileDenial::ToolNotInProfile));
    }

    #[test]
    fn step2_internal_off_loopback_is_exposure_denied_before_role_checks() {
        let internal = find(|e| e.exposure == Exposure::Internal && !e.http_read_role_eligible, "internal non-read-eligible");
        let p = profile_with(&[internal.name]);
        assert_eq!(authorize_tool(&p, &BootReport::default(), WRITE_REMOTE, internal.name), Err(ProfileDenial::ExposureDenied));
        assert_eq!(authorize_tool(&p, &BootReport::default(), READ_REMOTE, internal.name), Err(ProfileDenial::ExposureDenied));
    }

    #[test]
    fn internal_on_loopback_is_allowed_for_write() {
        let internal = find(|e| e.exposure == Exposure::Internal, "internal");
        let p = profile_with(&[internal.name]);
        assert!(authorize_tool(&p, &full_boot(), CallContext::LOCAL_WRITE, internal.name).is_ok());
    }

    #[test]
    fn step3_operator_with_read_role_is_exposure_denied_before_step4() {
        let op = find(|e| e.exposure == Exposure::Operator && !e.http_read_role_eligible, "operator non-read-eligible");
        let p = profile_with(&[op.name]);
        assert_eq!(authorize_tool(&p, &BootReport::default(), READ_LOCAL, op.name), Err(ProfileDenial::ExposureDenied));
    }

    #[test]
    fn step4_public_not_read_eligible_is_insufficient_role_before_step5() {
        let e = find(
            |e| e.exposure == Exposure::Public && !e.http_read_role_eligible && !e.requires.is_empty(),
            "public, non-read-eligible, with requires",
        );
        let p = profile_with(&[e.name]);
        assert_eq!(authorize_tool(&p, &BootReport::default(), READ_LOCAL, e.name), Err(ProfileDenial::InsufficientRole));
    }

    #[test]
    fn step5_first_missing_subsystem_is_reported() {
        let e = find(|e| !e.requires.is_empty(), "with requires");
        let p = profile_with(&[e.name]);
        assert_eq!(
            authorize_tool(&p, &BootReport::default(), CallContext::LOCAL_WRITE, e.name),
            Err(ProfileDenial::SubsystemNotBooted(e.requires[0]))
        );
    }

    #[test]
    fn all_checks_pass_returns_the_registry_entry() {
        let e = find(|e| e.exposure == Exposure::Public && e.http_read_role_eligible, "public read-eligible");
        let p = profile_with(&[e.name]);
        let got = authorize_tool(&p, &full_boot(), READ_REMOTE, e.name).expect("authorized");
        assert_eq!(got.name, e.name);
    }

    #[test]
    fn local_write_constant_is_write_on_loopback() {
        assert_eq!(CallContext::LOCAL_WRITE, CallContext { role: CallerRole::Write, loopback: true });
    }
}
```

Extend the lib re-export:

```rust
pub use profile::{
    ActiveProfile, CallContext, CallerRole, DEV_PROFILE_ID, PROFILES, ProfileError, ProfileManifest,
    authorize_tool,
};
```

- [ ] **Step 2: Run — expect compile failure.**

Run: `cargo nextest run -p vox-mcp-registry authorize`
Expected: FAIL — `cannot find function authorize_tool` / `cannot find type CallContext`.

- [ ] **Step 3: Implement.** Add to `profile.rs` (below `impl ActiveProfile`), and extend the `use crate::{…}` line to `use crate::{BootReport, Exposure, McpToolRegistryEntry, ProfileDenial, Subsystem, registry_entry, subsystem_closure};`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallerRole {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallContext {
    pub role: CallerRole,
    pub loopback: bool,
}

impl CallContext {
    pub const LOCAL_WRITE: CallContext = CallContext { role: CallerRole::Write, loopback: true };
}

/// The single profile decision. Check order is part of the contract (00-contracts.md).
pub fn authorize_tool(
    profile: &ActiveProfile,
    boot: &BootReport,
    ctx: CallContext,
    tool: &str,
) -> Result<&'static McpToolRegistryEntry, ProfileDenial> {
    let entry = registry_entry(tool)
        .filter(|_| profile.contains_tool(tool))
        .ok_or(ProfileDenial::ToolNotInProfile)?;
    if !ctx.loopback && entry.exposure == Exposure::Internal {
        return Err(ProfileDenial::ExposureDenied);
    }
    if ctx.role == CallerRole::Read && entry.exposure == Exposure::Operator {
        return Err(ProfileDenial::ExposureDenied);
    }
    if ctx.role == CallerRole::Read && !entry.http_read_role_eligible {
        return Err(ProfileDenial::InsufficientRole);
    }
    if let Some(s) = boot.missing_for(entry.requires) {
        return Err(ProfileDenial::SubsystemNotBooted(s));
    }
    Ok(entry)
}
```

- [ ] **Step 4: Run — expect pass.**

Run: `cargo nextest run -p vox-mcp-registry authorize`
Expected: PASS (9 tests).

- [ ] **Step 5: Commit.**

```bash
cargo clippy -p vox-mcp-registry --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-mcp-registry/src/profile.rs crates/vox-mcp-registry/src/lib.rs
git commit -m "feat(mcp-registry): add authorize_tool with pinned check order"
```

- [ ] **Step 6: Mutation-verify step 1 and step 2 of `authorize_tool`.** For each mutation below: record the presence count, make the edit, run, confirm failure, restore with `git checkout`, confirm the count is back.

Mutation A (step 1 — profile membership):

```bash
grep -c '.filter(|_| profile.contains_tool(tool))' crates/vox-mcp-registry/src/profile.rs   # expect 1
```

Delete the line `        .filter(|_| profile.contains_tool(tool))` with the edit tool, then:

```bash
grep -c '.filter(|_| profile.contains_tool(tool))' crates/vox-mcp-registry/src/profile.rs   # expect 0
cargo nextest run -p vox-mcp-registry step1_precedes_step2   # expect FAIL (returns ExposureDenied)
git checkout -- crates/vox-mcp-registry/src/profile.rs
grep -c '.filter(|_| profile.contains_tool(tool))' crates/vox-mcp-registry/src/profile.rs   # expect 1
```

Mutation B (step 2 — internal off loopback):

```bash
grep -c 'if !ctx.loopback && entry.exposure == Exposure::Internal' crates/vox-mcp-registry/src/profile.rs   # expect 1
```

Replace `if !ctx.loopback && entry.exposure == Exposure::Internal {` with `if false {`, then:

```bash
grep -c 'if !ctx.loopback && entry.exposure == Exposure::Internal' crates/vox-mcp-registry/src/profile.rs   # expect 0
cargo nextest run -p vox-mcp-registry step2_internal_off_loopback   # expect FAIL
git checkout -- crates/vox-mcp-registry/src/profile.rs
grep -c 'if !ctx.loopback && entry.exposure == Exposure::Internal' crates/vox-mcp-registry/src/profile.rs   # expect 1
cargo nextest run -p vox-mcp-registry authorize   # expect PASS
```

---

### Task 4: `vox_orchestrator_mcp::profiles` and `ServerState` profile fields

**Files:**
- Create: `crates/vox-orchestrator-mcp/src/profiles.rs`
- Modify: `crates/vox-orchestrator-mcp/src/lib.rs` (add `pub mod profiles;` next to `pub mod pending_approvals;`)
- Modify: `crates/vox-orchestrator-mcp/src/server_state.rs` — struct (L57–135), constructors `new_full` (`let state = Self {` ~L296), `new_for_daemon` (~L366), `hermetic_stub` (~L688)

**Interfaces:**
- Consumes: Tasks 1–3.
- Produces:
  - `pub use vox_mcp_registry::{ActiveProfile, BootReport, CallContext, CallerRole, DEV_PROFILE_ID, Exposure, PROFILES, ProfileDenial, ProfileError, ProfileManifest, SideEffectClass, Subsystem, authorize_tool, registry_entry, subsystem_closure};` (contract: `vox_orchestrator_mcp::profiles::*`)
  - `pub fn is_dev(profile: &ActiveProfile) -> bool`
  - `pub fn full_boot_report() -> BootReport`
  - `pub fn guard_tool(state: &ServerState, ctx: CallContext, canonical: &str) -> Result<(), ProfileDenial>` — registry tools go through `authorize_tool`; names absent from the registry (workspace `@tool`s, skill macro tools, unknown names) pass under `dev` only (today's dispatch then handles them) and are `ToolNotInProfile` otherwise.
  - `pub fn denial_error_json(tool: &str, denial: &ProfileDenial, profile: &ActiveProfile) -> serde_json::Value` — the 00-contracts wire shape.
  - `pub fn is_profile_denial_json(v: &serde_json::Value) -> bool`
  - `ServerState { pub active_profile: Arc<ActiveProfile>, pub boot_report: Arc<parking_lot::RwLock<BootReport>> }` and `pub fn with_profile(self, profile: ActiveProfile) -> Self`.

- [ ] **Step 1: Write the failing tests.** Create `crates/vox-orchestrator-mcp/src/profiles.rs` with only:

```rust
//! Capability-profile surface for MCP/HTTP hosts: re-exports the `vox-mcp-registry` profile types
//! and adds the `ServerState`-aware guard every advertise and dispatch path calls.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_state::ServerState;

    #[test]
    fn is_dev_only_for_the_dev_profile() {
        assert!(is_dev(&ActiveProfile::dev()));
        assert!(!is_dev(&ActiveProfile::resolve(&["models"]).expect("models")));
    }

    #[test]
    fn full_boot_report_has_every_subsystem_up() {
        let r = full_boot_report();
        for s in Subsystem::ALL {
            assert!(r.is_up(*s));
        }
        assert!(r.failed.is_empty());
    }

    #[tokio::test]
    async fn default_state_is_dev_with_everything_booted() {
        let state = ServerState::new_test().await;
        assert!(is_dev(&state.active_profile));
        assert_eq!(*state.boot_report.read(), full_boot_report());
    }

    #[tokio::test]
    async fn guard_passes_non_registry_names_only_under_dev() {
        let dev = ServerState::new_test().await;
        assert_eq!(guard_tool(&dev, CallContext::LOCAL_WRITE, "read_file"), Ok(()));
        let models = dev.clone().with_profile(ActiveProfile::resolve(&["models"]).expect("models"));
        assert_eq!(
            guard_tool(&models, CallContext::LOCAL_WRITE, "read_file"),
            Err(ProfileDenial::ToolNotInProfile)
        );
    }

    #[tokio::test]
    async fn guard_filters_registry_tools_by_profile() {
        let state = ServerState::new_test()
            .await
            .with_profile(ActiveProfile::resolve(&["models"]).expect("models"));
        assert_eq!(guard_tool(&state, CallContext::LOCAL_WRITE, "vox_list_models"), Ok(()));
        assert_eq!(
            guard_tool(&state, CallContext::LOCAL_WRITE, "vox_git_status"),
            Err(ProfileDenial::ToolNotInProfile)
        );
    }

    #[tokio::test]
    async fn guard_reports_subsystem_not_booted_from_state_boot_report() {
        let profile = ActiveProfile::resolve(&["knowledge"]).expect("knowledge");
        let tool = *profile
            .tools
            .iter()
            .find(|t| !registry_entry(t).expect("registry").requires.is_empty())
            .expect("knowledge has a tool with requires");
        let first = registry_entry(tool).expect("registry").requires[0];
        let state = ServerState::new_test().await.with_profile(profile);
        *state.boot_report.write() = BootReport::default();
        assert_eq!(
            guard_tool(&state, CallContext::LOCAL_WRITE, tool),
            Err(ProfileDenial::SubsystemNotBooted(first))
        );
    }

    #[test]
    fn denial_json_matches_the_contract_shape() {
        let v = denial_error_json(
            "vox_kb_query",
            &ProfileDenial::SubsystemNotBooted(Subsystem::SearchIndex),
            &ActiveProfile::dev(),
        );
        assert_eq!(
            v,
            serde_json::json!({"error": {
                "code": "subsystem_not_booted",
                "message": "tool vox_kb_query requires subsystem search_index, which is not running",
                "subsystem": "search_index"
            }})
        );
        let v = denial_error_json(
            "vox_git_status",
            &ProfileDenial::ToolNotInProfile,
            &ActiveProfile::resolve(&["models"]).expect("models"),
        );
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
        assert_eq!(
            v["error"]["message"],
            "tool vox_git_status is not in the active profile (models)"
        );
        assert!(v["error"].get("subsystem").is_none());
    }

    #[test]
    fn is_profile_denial_json_recognizes_only_denial_codes() {
        let v = denial_error_json("x", &ProfileDenial::ExposureDenied, &ActiveProfile::dev());
        assert!(is_profile_denial_json(&v));
        assert!(!is_profile_denial_json(&serde_json::json!({"error": {"code": "other"}})));
        assert!(!is_profile_denial_json(&serde_json::json!({"success": false, "error": "boom"})));
    }
}
```

Add `pub mod profiles;` to `crates/vox-orchestrator-mcp/src/lib.rs`.

Add a test to the existing `#[cfg(test)] mod tests` in `server_state.rs`:

```rust
    #[tokio::test]
    async fn with_profile_replaces_active_profile_and_keeps_boot_report() {
        let state = ServerState::new_test().await;
        let models = crate::profiles::ActiveProfile::resolve(&["models"]).expect("models");
        let scoped = state.with_profile(models.clone());
        assert_eq!(*scoped.active_profile, models);
        assert_eq!(*scoped.boot_report.read(), crate::profiles::full_boot_report());
    }
```

- [ ] **Step 2: Run — expect compile failure.**

Run: `cargo nextest run -p vox-orchestrator-mcp profiles::tests with_profile`
Expected: FAIL — `cannot find function is_dev`, `no field active_profile on type ServerState`.

- [ ] **Step 3: Implement `profiles.rs`.** Insert above the test module:

```rust
pub use vox_mcp_registry::{
    ActiveProfile, BootReport, CallContext, CallerRole, DEV_PROFILE_ID, Exposure, PROFILES,
    ProfileDenial, ProfileError, ProfileManifest, SideEffectClass, Subsystem, authorize_tool,
    registry_entry, subsystem_closure,
};

use crate::server_state::ServerState;

const DENIAL_CODES: &[&str] = &[
    "tool_not_in_profile",
    "subsystem_not_booted",
    "exposure_denied",
    "insufficient_role",
];

pub fn is_dev(profile: &ActiveProfile) -> bool {
    profile.ids.as_slice() == [DEV_PROFILE_ID]
}

/// Boot report used until plan 03's conditioned boot fills the real one.
pub fn full_boot_report() -> BootReport {
    BootReport {
        booted: Subsystem::ALL.iter().copied().collect(),
        failed: Default::default(),
    }
}

/// Profile gate for one canonical tool name. Names outside `TOOL_REGISTRY` (workspace `@tool`s,
/// skill macro tools, unknown names) keep today's handling under `dev` and are denied otherwise.
pub fn guard_tool(state: &ServerState, ctx: CallContext, canonical: &str) -> Result<(), ProfileDenial> {
    if registry_entry(canonical).is_none() {
        return if is_dev(&state.active_profile) {
            Ok(())
        } else {
            Err(ProfileDenial::ToolNotInProfile)
        };
    }
    let boot = state.boot_report.read();
    authorize_tool(&state.active_profile, &boot, ctx, canonical).map(|_| ())
}

pub fn denial_error_json(tool: &str, denial: &ProfileDenial, profile: &ActiveProfile) -> serde_json::Value {
    let message = match denial {
        ProfileDenial::ToolNotInProfile => {
            format!("tool {tool} is not in the active profile ({})", profile.id_string())
        }
        ProfileDenial::SubsystemNotBooted(s) => {
            format!("tool {tool} requires subsystem {}, which is not running", s.as_str())
        }
        ProfileDenial::ExposureDenied => format!("tool {tool} is not exposed to this caller"),
        ProfileDenial::InsufficientRole => format!("tool {tool} requires a write token"),
    };
    let mut error = serde_json::json!({ "code": denial.code(), "message": message });
    if let ProfileDenial::SubsystemNotBooted(s) = denial {
        error["subsystem"] = serde_json::Value::String(s.as_str().to_string());
    }
    serde_json::json!({ "error": error })
}

pub fn is_profile_denial_json(v: &serde_json::Value) -> bool {
    v.get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_str())
        .is_some_and(|c| DENIAL_CODES.contains(&c))
}
```

- [ ] **Step 4: Add the `ServerState` fields.** In the `pub struct ServerState` (after `pub model_prompt_registry: Arc<ModelPromptRegistry>,`):

```rust

    /// Capability profile this host serves (`dev` unless `--profile` / `VOX_PROFILE`).
    pub active_profile: Arc<crate::profiles::ActiveProfile>,

    /// Subsystems actually up. "Everything booted" until plan 03's conditioned boot.
    pub boot_report: Arc<PrRwLock<crate::profiles::BootReport>>,
```

In each of the three struct literals (`new_full`, `new_for_daemon`, `hermetic_stub`), after `model_prompt_registry: Arc::new(ModelPromptRegistry::new()),` add:

```rust
            active_profile: Arc::new(crate::profiles::ActiveProfile::dev()),
            boot_report: Arc::new(PrRwLock::new(crate::profiles::full_boot_report())),
```

Add to the first `impl ServerState` block (after `pub fn feedback`):

```rust
    /// Scope this state to `profile` (advertising and dispatch both honor it).
    pub fn with_profile(mut self, profile: crate::profiles::ActiveProfile) -> Self {
        self.active_profile = Arc::new(profile);
        self
    }
```

- [ ] **Step 5: Run — expect pass.**

Run: `cargo nextest run -p vox-orchestrator-mcp profiles::tests with_profile`
Expected: PASS (9 tests).

- [ ] **Step 6: Commit.**

```bash
cargo check -p vox-orchestrator-mcp
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/profiles.rs crates/vox-orchestrator-mcp/src/lib.rs crates/vox-orchestrator-mcp/src/server_state.rs
git commit -m "feat(orchestrator-mcp): hold ActiveProfile and BootReport on ServerState"
```

---

### Task 5: Central dispatch guard

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/dispatch.rs` — `dispatch_tool_call` (L118–127, right after `let name_canonical = tool_aliases::canonical_tool_name(name);`); new test module at end of file
- Modify: `crates/vox-orchestrator-mcp/src/server_state.rs` — `tool_json_envelope_is_error` (~L734)
- Modify: `crates/vox-orchestrator-mcp/src/server.rs` — `tool_json_envelope_is_error` (L410–416) + its test module

**Interfaces:**
- Consumes: `profiles::{guard_tool, denial_error_json, is_profile_denial_json, CallContext}`.
- Produces: every caller of `handle_tool_call*` (daemon `orch.tool_call`, HTTP, WS, `McpToolDispatcher`) receives `Ok(<denial JSON>)` for a denied tool; `tool_json_envelope_is_error` is `true` for it, so stdio `call_tool` returns `isError: true`.

- [ ] **Step 1: Write the failing tests.** Append to `dispatch.rs`:

```rust
#[cfg(test)]
mod profile_guard_tests {
    use super::handle_tool_call;
    use crate::profiles::ActiveProfile;
    use crate::server_state::{ServerState, tool_json_envelope_is_error};
    use serde_json::json;

    #[tokio::test]
    async fn dispatch_denies_tool_outside_profile() {
        let state = ServerState::new_test()
            .await
            .with_profile(ActiveProfile::resolve(&["models"]).expect("models"));
        let out = handle_tool_call(&state, "vox_git_status", json!({}))
            .await
            .expect("denial is an Ok envelope");
        let v: serde_json::Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
        assert!(tool_json_envelope_is_error(&out));
    }

    #[tokio::test]
    async fn dev_dispatch_is_not_profile_gated() {
        let state = ServerState::new_test().await;
        let out = handle_tool_call(&state, "vox_git_status", json!({})).await;
        if let Ok(s) = out {
            let v: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
            assert!(!crate::profiles::is_profile_denial_json(&v), "dev must not deny: {s}");
        }
    }
}
```

Append to the `call_tool_tests` module in `server.rs`:

```rust
    #[test]
    fn envelope_error_for_profile_denial() {
        assert!(tool_json_envelope_is_error(
            r#"{"error":{"code":"tool_not_in_profile","message":"m"}}"#
        ));
        assert!(!tool_json_envelope_is_error(r#"{"error":{"code":"unrelated"}}"#));
    }
```

- [ ] **Step 2: Run — expect failure.**

Run: `cargo nextest run -p vox-orchestrator-mcp profile_guard_tests envelope_error_for_profile_denial`
Expected: FAIL — `dispatch_denies_tool_outside_profile` (no `error.code`, tool ran) and `envelope_error_for_profile_denial`.

- [ ] **Step 3: Implement the guard.** In `dispatch_tool_call`, directly after `let name_canonical = tool_aliases::canonical_tool_name(name);` insert:

```rust
    if let Err(denial) = crate::profiles::guard_tool(
        state,
        crate::profiles::CallContext::LOCAL_WRITE,
        name_canonical,
    ) {
        return Ok(
            crate::profiles::denial_error_json(name_canonical, &denial, &state.active_profile)
                .to_string(),
        );
    }
```

Replace the body of `tool_json_envelope_is_error` in **both** `server.rs` and `server_state.rs` with:

```rust
pub fn tool_json_envelope_is_error(json: &str) -> bool {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return false;
    };
    v.get("success").and_then(|s| s.as_bool()) == Some(false)
        || crate::profiles::is_profile_denial_json(&v)
}
```

- [ ] **Step 4: Run — expect pass, plus the existing dispatch probe.**

Run: `cargo nextest run -p vox-orchestrator-mcp profile_guard_tests call_tool_tests every_registry_tool_has_static_dispatch`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/dispatch.rs crates/vox-orchestrator-mcp/src/server.rs crates/vox-orchestrator-mcp/src/server_state.rs
git commit -m "feat(orchestrator-mcp): gate every tool dispatch on the active profile"
```

- [ ] **Step 6: Mutation-verify the dispatch guard.**

```bash
grep -c 'crate::profiles::guard_tool(' crates/vox-orchestrator-mcp/src/dispatch.rs   # expect 1
```

Delete the whole `if let Err(denial) = crate::profiles::guard_tool(` … `}` block inserted in Step 3, then:

```bash
grep -c 'crate::profiles::guard_tool(' crates/vox-orchestrator-mcp/src/dispatch.rs   # expect 0
cargo nextest run -p vox-orchestrator-mcp dispatch_denies_tool_outside_profile   # expect FAIL
git checkout -- crates/vox-orchestrator-mcp/src/dispatch.rs
grep -c 'crate::profiles::guard_tool(' crates/vox-orchestrator-mcp/src/dispatch.rs   # expect 1
cargo nextest run -p vox-orchestrator-mcp dispatch_denies_tool_outside_profile   # expect PASS
```

---

### Task 6: Stdio advertisement filtering + back-compat snapshot

**Files:**
- Create: `crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt`
- Modify: `crates/vox-orchestrator-mcp/src/registry.rs` (L51–120)
- Modify: `crates/vox-orchestrator-mcp/src/server.rs` — `list_tools` (L204–257), `call_tool` (L369–407), `VoxMcpServer::new` (L153–160)
- Modify: `crates/vox-orchestrator-mcp/src/lifecycle.rs` (L35–86)

**Interfaces:**
- Consumes: `profiles::guard_tool`; Task 9's `OrchestratorDaemonEnsure::for_profile` (Task 9 must land first, or do Task 9 Step 3 before Step 5 here).
- Produces:
  - `pub fn tool_registry_for(state: &ServerState) -> Vec<rmcp::model::Tool>` (feature-dispatchable ∧ profile-authorized).
  - `merged_tool_registry(state)` = `tool_registry_for(state)` + workspace tools that pass `guard_tool` (only under `dev`).
  - `pub fn profile_denial_result(state: &ServerState, name: &str) -> Option<CallToolResult>` in `server.rs`.
  - `pub async fn run_stdio_server_blocking_with_profile(profile: ActiveProfile) -> anyhow::Result<()>`; `run_stdio_server_blocking()` keeps its signature and resolves `ActiveProfile::from_env()`.

Parallel-safe with: Task 7, Task 8.

- [ ] **Step 1: Snapshot today's default-feature tool list** (before any change in this task):

```bash
awk '/^- name:/{print $3}' contracts/mcp/tool-registry.canonical.yaml \
  | grep -Ev '^(vox_news_|vox_scientia_|vox_browser_)' \
  | grep -Ev '^vox_visus_(audit|baseline)$' \
  | LC_ALL=C sort > crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt
wc -l crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt   # 271 on origin/main
```

The excluded prefixes mirror `dispatchable_under_features` with `news-publish`, `gui-visual-review`, `heavy-browser` off (the crate's default features).

- [ ] **Step 2: Prove the snapshot equals today's behaviour.** Append to `registry.rs`:

```rust
#[cfg(test)]
mod profile_backcompat_tests {
    use super::*;
    use crate::ServerState;
    use std::collections::BTreeSet;

    fn names(tools: Vec<rmcp::model::Tool>) -> Vec<String> {
        let mut v: Vec<String> = tools.into_iter().map(|t| t.name.to_string()).collect();
        v.sort();
        v
    }

    #[cfg(all(
        not(feature = "news-publish"),
        not(feature = "heavy-browser"),
        not(feature = "gui-visual-review")
    ))]
    #[test]
    fn snapshot_is_todays_default_feature_registry() {
        let want: Vec<String> = include_str!("../tests/fixtures/dev-advertised-tools.default-features.txt")
            .lines()
            .map(str::to_string)
            .collect();
        assert_eq!(names(tool_registry()), want);
    }
}
```

Run: `cargo nextest run -p vox-orchestrator-mcp snapshot_is_todays_default_feature_registry`
Expected: PASS (the fixture is today's list).

- [ ] **Step 3: Write the failing profile tests.** Add to `profile_backcompat_tests`:

```rust
    #[cfg(all(
        not(feature = "news-publish"),
        not(feature = "heavy-browser"),
        not(feature = "gui-visual-review")
    ))]
    #[tokio::test]
    async fn dev_profile_advertises_the_snapshot() {
        let state = ServerState::new_test().await;
        let want: Vec<String> = include_str!("../tests/fixtures/dev-advertised-tools.default-features.txt")
            .lines()
            .map(str::to_string)
            .collect();
        assert_eq!(names(tool_registry_for(&state)), want);
    }

    #[tokio::test]
    async fn dev_profile_advertises_exactly_the_feature_filtered_registry() {
        let state = ServerState::new_test().await;
        let got: BTreeSet<String> = names(tool_registry_for(&state)).into_iter().collect();
        let want: BTreeSet<String> = names(tool_registry()).into_iter().collect();
        assert_eq!(got, want);
    }

    #[tokio::test]
    async fn named_profile_advertises_only_its_tools_and_no_workspace_tools() {
        let state = ServerState::new_test()
            .await
            .with_profile(crate::profiles::ActiveProfile::resolve(&["models"]).expect("models"));
        let advertised = names(merged_tool_registry(&state));
        assert!(advertised.contains(&"vox_list_models".to_string()));
        assert!(!advertised.contains(&"vox_git_status".to_string()));
        assert!(!advertised.contains(&"read_file".to_string()));
        assert!(advertised.iter().all(|n| state.active_profile.contains_tool(n)));
    }

    #[tokio::test]
    async fn dev_profile_keeps_workspace_tools() {
        let state = ServerState::new_full(vox_orchestrator::OrchestratorConfig::default());
        assert!(names(merged_tool_registry(&state)).contains(&"read_file".to_string()));
    }
```

Add to `call_tool_tests` in `server.rs`:

```rust
    #[tokio::test]
    async fn profile_denial_result_is_error_for_out_of_profile_tool() {
        let state = crate::server_state::ServerState::new_test()
            .await
            .with_profile(crate::profiles::ActiveProfile::resolve(&["models"]).expect("models"));
        let r = super::profile_denial_result(&state, "vox_git_status").expect("denied");
        assert_eq!(r.is_error, Some(true));
        assert!(super::profile_denial_result(&state, "vox_list_models").is_none());
        let dev = crate::server_state::ServerState::new_test().await;
        assert!(super::profile_denial_result(&dev, "vox_git_status").is_none());
    }
```

- [ ] **Step 4: Run — expect compile failure.**

Run: `cargo nextest run -p vox-orchestrator-mcp profile_backcompat_tests profile_denial_result`
Expected: FAIL — `cannot find function tool_registry_for`, `cannot find function profile_denial_result`.

- [ ] **Step 5: Implement.** In `registry.rs`, add after `tool_registry()`:

```rust
/// [`tool_registry`] filtered to what the host's active profile and boot report authorize.
pub fn tool_registry_for(state: &crate::server_state::ServerState) -> Vec<rmcp::model::Tool> {
    tool_registry()
        .into_iter()
        .filter(|t| {
            crate::profiles::guard_tool(state, crate::profiles::CallContext::LOCAL_WRITE, &t.name)
                .is_ok()
        })
        .collect()
}
```

In `merged_tool_registry`, replace `let mut tool_list = tool_registry();` with `let mut tool_list = tool_registry_for(state);` and change the loop's skip condition to:

```rust
        if tool_list.iter().any(|t| t.name == entry.name)
            || crate::profiles::guard_tool(
                state,
                crate::profiles::CallContext::LOCAL_WRITE,
                &entry.name,
            )
            .is_err()
        {
            continue;
        }
```

In `server.rs` `list_tools`, inside the skill loop change `if !tool_list.iter().any(|t| t.name == *tool_name) {` to:

```rust
                if !tool_list.iter().any(|t| t.name == *tool_name)
                    && crate::profiles::guard_tool(
                        &self.state,
                        crate::profiles::CallContext::LOCAL_WRITE,
                        tool_name,
                    )
                    .is_ok()
                {
```

Add to `server.rs` (above `impl ServerHandler for VoxMcpServer`):

```rust
/// Local profile check for stdio `call_tool` (the daemon re-checks in `dispatch`).
pub fn profile_denial_result(state: &ServerState, name: &str) -> Option<CallToolResult> {
    let canonical = crate::canonical_tool_name(name);
    crate::profiles::guard_tool(state, crate::profiles::CallContext::LOCAL_WRITE, canonical)
        .err()
        .map(|denial| {
            let json = crate::profiles::denial_error_json(canonical, &denial, &state.active_profile);
            CallToolResult::error(vec![rmcp::model::Content::text(json.to_string())])
        })
}
```

In `call_tool`, after `let name_str = params.name.to_string();` insert:

```rust
        if let Some(denied) = profile_denial_result(&self.state, &name_str) {
            return Ok(denied);
        }
```

In `VoxMcpServer::new`, build the ensure helper for the state's profile:

```rust
    pub fn new(state: ServerState) -> Self {
        let profile_id = state.active_profile.id_string();
        Self {
            state,
            daemon: std::sync::Arc::new(
                vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::OrchestratorDaemonEnsure::for_profile(
                    profile_id,
                ),
            ),
        }
    }
```

In `lifecycle.rs`, replace `pub async fn run_stdio_server_blocking() -> anyhow::Result<()> {` … `let state = ServerState::new_full(config);` with:

```rust
pub async fn run_stdio_server_blocking() -> anyhow::Result<()> {
    let profile = crate::profiles::ActiveProfile::from_env()?;
    run_stdio_server_blocking_with_profile(profile).await
}

/// Stdio MCP (or HTTP-gateway-only when `VOX_MCP_HTTP_ENABLED=1`) scoped to `profile`.
pub async fn run_stdio_server_blocking_with_profile(
    profile: crate::profiles::ActiveProfile,
) -> anyhow::Result<()> {
    tracing::info!(profile = %profile.id_string(), "vox native mcp server starting...");

    let config = load_config();
    tracing::info!(?config, "orchestrator config loaded");

    let state = ServerState::new_full(config).with_profile(profile);
```

(keep the rest of the old body unchanged), and add a test at the end of `lifecycle.rs`:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn load_config_builds_without_panicking() {
        let _ = super::load_config();
    }
}
```

Re-export the new fn where `run_stdio_server_blocking` is re-exported in `crates/vox-orchestrator-mcp/src/lib.rs` (`grep -n "run_stdio_server_blocking" crates/vox-orchestrator-mcp/src/lib.rs`), adding `run_stdio_server_blocking_with_profile` to the same `pub use lifecycle::{…}` list.

- [ ] **Step 6: Run — expect pass.**

Run: `cargo nextest run -p vox-orchestrator-mcp profile_backcompat_tests profile_denial_result merged_registry_tests`
Expected: PASS.

- [ ] **Step 7: Commit.**

```bash
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/registry.rs crates/vox-orchestrator-mcp/src/server.rs crates/vox-orchestrator-mcp/src/lifecycle.rs crates/vox-orchestrator-mcp/src/lib.rs crates/vox-orchestrator-mcp/tests/fixtures/dev-advertised-tools.default-features.txt
git commit -m "feat(orchestrator-mcp): filter stdio tools/list and call_tool by profile"
```

- [ ] **Step 8: Mutation-verify the advertisement filter.**

```bash
grep -c 'crate::profiles::guard_tool(state, crate::profiles::CallContext::LOCAL_WRITE, &t.name)' crates/vox-orchestrator-mcp/src/registry.rs   # expect 1
```

Replace that `.filter(|t| { … })` in `tool_registry_for` with `.filter(|_| true)`, then:

```bash
grep -c 'crate::profiles::guard_tool(state, crate::profiles::CallContext::LOCAL_WRITE, &t.name)' crates/vox-orchestrator-mcp/src/registry.rs   # expect 0
cargo nextest run -p vox-orchestrator-mcp named_profile_advertises_only_its_tools   # expect FAIL
git checkout -- crates/vox-orchestrator-mcp/src/registry.rs
grep -c 'crate::profiles::guard_tool(state, crate::profiles::CallContext::LOCAL_WRITE, &t.name)' crates/vox-orchestrator-mcp/src/registry.rs   # expect 1
```

---

### Task 7: HTTP gateway + WebSocket filtering and allowlist composition

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs` — `spawn_http_gateway_if_enabled` (`parse_allowed_tools()?` ~L341), `parse_allowed_tools` (L468–472), `visible_tools_for_role` (L538–553), `parse_allowed_tools_from_value` (L555–584), `mod tests` (L746+)
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs` — `http_call_tool` (L43–63), `call_tool_response_value` (L73–112)

**Interfaces:**
- Consumes: `profiles::{guard_tool, denial_error_json, is_dev, CallContext}`, `crate::registry::dispatchable_under_features`.
- Produces:
  - `parse_allowed_tools(profile: &ActiveProfile) -> Result<HashSet<String>>`, `parse_allowed_tools_from_value(raw: Option<&str>, profile: &ActiveProfile) -> Result<HashSet<String>>`.
  - **Composition rule (documented in Task 15):** effective HTTP tool set = `VOX_MCP_HTTP_ALLOWED_TOOLS` ∩ profile-authorized tools. When `VOX_MCP_HTTP_ALLOWED_TOOLS` is unset: `dev` keeps `DEFAULT_ALLOWED_TOOLS` (unchanged), a named profile defaults to its own (feature-dispatchable) tools. Unknown names in the env var still fail startup. The read-role narrowing (`http_read_role_eligible`, `VOX_MCP_HTTP_READ_ROLE_ALLOWED_TOOLS`) is unchanged and applies on top.
  - `POST /v1/tools/call` for a denied tool → HTTP `ProfileDenial::http_status()` with the contract JSON body; WS `call_tool` → `success: false, is_error: true, data: <contract JSON>`; `GET /v1/tools`, WS `list_tools`, `/v1/info` → profile-filtered via `visible_tools_for_role`.

Parallel-safe with: Task 6, Task 8.

- [ ] **Step 1: Write the failing tests.** Append inside `mod tests` in `http_gateway/mod.rs`:

```rust
    fn models_profile() -> crate::profiles::ActiveProfile {
        crate::profiles::ActiveProfile::resolve(&["models"]).expect("models")
    }

    #[test]
    fn dev_allowlist_default_is_unchanged() {
        let got = parse_allowed_tools_from_value(None, &crate::profiles::ActiveProfile::dev()).expect("parse");
        let want: HashSet<String> = DEFAULT_ALLOWED_TOOLS
            .iter()
            .map(|n| canonical_tool_name(n).to_string())
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn named_profile_allowlist_defaults_to_profile_tools() {
        let p = models_profile();
        let got = parse_allowed_tools_from_value(None, &p).expect("parse");
        let want: HashSet<String> = p
            .tools
            .iter()
            .filter(|n| crate::registry::dispatchable_under_features(n))
            .map(|n| n.to_string())
            .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn explicit_allowlist_is_intersected_with_profile() {
        let got = parse_allowed_tools_from_value(Some("vox_list_models,vox_git_status"), &models_profile())
            .expect("parse");
        assert_eq!(got, ["vox_list_models".to_string()].into_iter().collect());
    }

    #[test]
    fn explicit_allowlist_still_rejects_unknown_names() {
        let err = parse_allowed_tools_from_value(Some("definitely_not_a_real_tool"), &models_profile())
            .expect_err("unknown");
        assert!(err.to_string().contains("unknown tool"));
    }

    #[tokio::test]
    async fn visible_tools_are_profile_filtered() {
        let server_state = ServerState::new_test().await.with_profile(models_profile());
        let allowed: HashSet<String> = ["vox_list_models", "vox_git_status"].iter().map(|s| s.to_string()).collect();
        let state = GatewayState::for_test_with_state(server_state, allowed).await;
        let visible = visible_tools_for_role(&state, AccessRole::Write);
        assert!(visible.contains("vox_list_models"));
        assert!(!visible.contains("vox_git_status"));
    }

    #[tokio::test]
    async fn dev_visible_tools_keep_non_registry_allowlisted_names() {
        let allowed: HashSet<String> = ["vox_run_shell", "vox_git_status"].iter().map(|s| s.to_string()).collect();
        let state = GatewayState::for_test_with_state(ServerState::new_test().await, allowed.clone()).await;
        assert_eq!(visible_tools_for_role(&state, AccessRole::Write), allowed);
    }

    #[tokio::test]
    async fn http_call_outside_profile_is_404_tool_not_in_profile() {
        use axum::body::{Body, to_bytes};
        use axum::http::Request;
        use tower::ServiceExt;
        let server_state = ServerState::new_test().await.with_profile(models_profile());
        let allowed: HashSet<String> = ["vox_git_status".to_string()].into_iter().collect();
        let app = build_app(GatewayState::for_test_with_state(server_state, allowed).await);
        let peer: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let mut req = Request::builder()
            .method("POST")
            .uri("/v1/tools/call")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "name": "vox_git_status", "args": {} }).to_string(),
            ))
            .unwrap();
        req.extensions_mut().insert(ConnectInfo(peer));
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        let v: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
    }

    #[tokio::test]
    async fn ws_call_outside_profile_returns_denial_json() {
        let server_state = ServerState::new_test().await.with_profile(models_profile());
        let state = GatewayState::for_test_with_state(server_state, HashSet::new()).await;
        let msg = WsMessageIn {
            id: Some("1".to_string()),
            msg_type: "call_tool".to_string(),
            name: Some("vox_git_status".to_string()),
            args: None,
        };
        let peer: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let out = ws_handle_message(&state, msg, peer, AccessRole::Write).await;
        assert!(!out.success && out.is_error);
        assert_eq!(out.data["error"]["code"], "tool_not_in_profile");
    }
```

- [ ] **Step 2: Run — expect compile failure.**

Run: `cargo nextest run -p vox-orchestrator-mcp http_gateway::tests`
Expected: FAIL — `this function takes 1 argument but 2 arguments were supplied` (`parse_allowed_tools_from_value`).

- [ ] **Step 3: Implement the allowlist composition.** Replace `parse_allowed_tools` and `parse_allowed_tools_from_value`:

```rust
pub(super) fn parse_allowed_tools(profile: &crate::profiles::ActiveProfile) -> Result<HashSet<String>> {
    parse_allowed_tools_from_value(
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMcpHttpAllowedTools).expose(),
        profile,
    )
}

/// `VOX_MCP_HTTP_ALLOWED_TOOLS` ∩ profile. Unset ⇒ `DEFAULT_ALLOWED_TOOLS` under `dev`, the
/// profile's own tools under a named profile.
pub(super) fn parse_allowed_tools_from_value(
    raw: Option<&str>,
    profile: &crate::profiles::ActiveProfile,
) -> Result<HashSet<String>> {
    let mut set = HashSet::new();
    if let Some(s) = raw {
        for v in s.split(',') {
            let trimmed = v.trim();
            if !trimmed.is_empty() {
                set.insert(canonical_tool_name(trimmed).to_string());
            }
        }
    }
    let explicit = !set.is_empty();
    let dev = crate::profiles::is_dev(profile);

    if !explicit {
        if dev {
            for n in DEFAULT_ALLOWED_TOOLS.iter() {
                set.insert(canonical_tool_name(n).to_string());
            }
        } else {
            for n in profile.tools.iter() {
                if crate::registry::dispatchable_under_features(n) {
                    set.insert((*n).to_string());
                }
            }
        }
    }

    let registry_names: HashSet<&str> = TOOL_REGISTRY.iter().map(|e| e.name).collect();
    for name in set.iter() {
        if !registry_names.contains(name.as_str()) {
            anyhow::bail!("unknown tool in VOX_MCP_HTTP_ALLOWED_TOOLS: {name}");
        }
    }

    if explicit && !dev {
        let before = set.len();
        set.retain(|n| profile.contains_tool(n));
        if set.len() < before {
            tracing::warn!(
                profile = %profile.id_string(),
                dropped = before - set.len(),
                "VOX_MCP_HTTP_ALLOWED_TOOLS entries outside the active profile are ignored"
            );
        }
    }
    Ok(set)
}
```

In `spawn_http_gateway_if_enabled`, replace `let allowed_tools = Arc::new(parse_allowed_tools()?);` with:

```rust
    let allowed_tools = Arc::new(parse_allowed_tools(&state.active_profile)?);
```

Replace `visible_tools_for_role`:

```rust
pub(super) fn visible_tools_for_role(state: &GatewayState, role: AccessRole) -> HashSet<String> {
    let mut visible = match role {
        AccessRole::Write => (*state.allowed_tools).clone(),
        AccessRole::Read => {
            let mut visible: HashSet<String> = state
                .allowed_tools
                .intersection(&state.read_role_eligible_tools)
                .cloned()
                .collect();
            if let Some(override_tools) = &state.read_role_tools_override {
                visible = visible.intersection(override_tools).cloned().collect();
            }
            visible
        }
    };
    visible.retain(|name| {
        crate::profiles::guard_tool(
            &state.server_state,
            crate::profiles::CallContext::LOCAL_WRITE,
            name,
        )
        .is_ok()
    });
    visible
}
```

- [ ] **Step 4: Implement HTTP/WS denial responses.** In `rpc_tools.rs` `http_call_tool`, replace the final line `call_tool_response(&state, req.name, req.args, Some(connect.0), role).await` with:

```rust
    let canonical = canonical_tool_name(req.name.as_str()).to_string();
    if let Err(denial) = crate::profiles::guard_tool(
        &state.server_state,
        crate::profiles::CallContext::LOCAL_WRITE,
        &canonical,
    ) {
        let status = StatusCode::from_u16(denial.http_status()).unwrap_or(StatusCode::FORBIDDEN);
        let body = crate::profiles::denial_error_json(&canonical, &denial, &state.server_state.active_profile);
        return (status, Json(body)).into_response();
    }
    call_tool_response(&state, req.name, req.args, Some(connect.0), role).await
```

In `call_tool_response_value`, right after `let canonical = canonical_tool_name(name.as_str()).to_string();` insert:

```rust
    if let Err(denial) = crate::profiles::guard_tool(
        &state.server_state,
        crate::profiles::CallContext::LOCAL_WRITE,
        &canonical,
    ) {
        return ToolCallResponse {
            success: false,
            is_error: true,
            result: crate::profiles::denial_error_json(
                &canonical,
                &denial,
                &state.server_state.active_profile,
            ),
        };
    }
```

- [ ] **Step 5: Run — expect pass (new + existing gateway tests).**

Run: `cargo nextest run -p vox-orchestrator-mcp http_gateway`
Expected: PASS, including the pre-existing `vox_run_shell` cross-consumer approval test (dev keeps non-registry names).

- [ ] **Step 6: Commit.**

```bash
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/http_gateway/mod.rs crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs
git commit -m "feat(http-gateway): compose allowlist with profile and return typed denials"
```

- [ ] **Step 7: Mutation-verify the HTTP call guard.**

```bash
grep -c 'StatusCode::from_u16(denial.http_status())' crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs   # expect 1
```

Delete the `if let Err(denial) = …` block added to `http_call_tool` in Step 4 (keep the `let canonical` line removed too), then:

```bash
grep -c 'StatusCode::from_u16(denial.http_status())' crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs   # expect 0
cargo nextest run -p vox-orchestrator-mcp http_call_outside_profile_is_404   # expect FAIL (status 200)
git checkout -- crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs
grep -c 'StatusCode::from_u16(denial.http_status())' crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs   # expect 1
```

---

### Task 8: Daemon resolves `VOX_PROFILE`; ping reports `"profile"`

**Files:**
- Modify: `crates/vox-orchestrator/src/orch_daemon/mod.rs` — `ping_caller_role` (L24–34), PING arm (L388–397), test `ping_response_includes_the_running_binary_version` (L1371–1393)
- Modify: `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` — `async_main` (L179–181, L321–327), its `#[cfg(test)]` module

**Interfaces:**
- Consumes: `vox_mcp_registry::ActiveProfile` (vox-orchestrator already depends on vox-mcp-registry); `vox_orchestrator_mcp::profiles::ActiveProfile` in the daemon binary.
- Produces: ping result `{ ..., "profile": "<id_string>" }` (invalid env ⇒ the `profile_unknown: …` text, which never matches a requested id); daemon exits non-zero with `profile_unknown: <id> (VOX_PROFILE)` on an unknown id; daemon `ServerState` carries the profile.

Parallel-safe with: Task 6, Task 7, Task 9.

- [ ] **Step 1: Write the failing tests.** In the PING test in `orch_daemon/mod.rs`, add before the closing `}`:

```rust
        assert_eq!(
            value.get("profile").and_then(|v| v.as_str()),
            Some(super::ping_profile().as_str()),
            "ping must report the daemon's capability profile for adopt checks"
        );
```

and a new test next to it:

```rust
    #[test]
    fn ping_profile_is_the_canonical_env_profile() {
        let want = vox_mcp_registry::ActiveProfile::from_env()
            .map(|p| p.id_string())
            .unwrap_or_else(|e| e.to_string());
        assert_eq!(super::ping_profile(), want);
    }
```

In the daemon binary's test module add:

```rust
    #[test]
    fn resolve_daemon_profile_matches_env_resolution() {
        let got = super::resolve_daemon_profile().map(|p| p.id_string()).map_err(|e| e.to_string());
        let want = vox_orchestrator_mcp::profiles::ActiveProfile::from_env()
            .map(|p| p.id_string())
            .map_err(|e| format!("{e} (VOX_PROFILE)"));
        assert_eq!(got, want);
    }
```

- [ ] **Step 2: Run — expect compile failure.**

Run: `cargo nextest run -p vox-orchestrator ping_ && cargo nextest run -p vox-orchestrator-d resolve_daemon_profile`
Expected: FAIL — `cannot find function ping_profile`, `cannot find function resolve_daemon_profile`.

- [ ] **Step 3: Implement.** In `orch_daemon/mod.rs` after `ping_caller_role`:

```rust
/// Canonical capability-profile id this daemon serves (from `VOX_PROFILE`).
fn ping_profile() -> String {
    vox_mcp_registry::ActiveProfile::from_env()
        .map(|p| p.id_string())
        .unwrap_or_else(|e| e.to_string())
}
```

and in the PING `json!` add `"profile": ping_profile(),` after `"caller_role": ping_caller_role(),`.

In `vox_orchestrator_d.rs` add above `async fn async_main`:

```rust
fn resolve_daemon_profile() -> anyhow::Result<vox_orchestrator_mcp::profiles::ActiveProfile> {
    vox_orchestrator_mcp::profiles::ActiveProfile::from_env()
        .map_err(|e| anyhow::anyhow!("{e} (VOX_PROFILE)"))
}
```

In `async_main`, right after `vox_foundation::tracing::try_init_from_default_env();`:

```rust
    let active_profile = resolve_daemon_profile()?;
    tracing::info!(profile = %active_profile.id_string(), "capability profile resolved");
```

and append `.with_profile(active_profile)` to the `ServerState::new_for_daemon(…)` expression:

```rust
    let mut state = vox_orchestrator_mcp::server_state::ServerState::new_for_daemon(
        orch.clone(),
        orch_config.clone(),
        build.repository.clone(),
        Arc::new(tokio::sync::Mutex::new(session_manager)),
        registry,
    )
    .with_profile(active_profile);
```

- [ ] **Step 4: Run — expect pass.**

Run: `cargo nextest run -p vox-orchestrator ping_ && cargo nextest run -p vox-orchestrator-d resolve_daemon_profile`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
cargo clippy -p vox-orchestrator --all-targets -- -D warnings
cargo clippy -p vox-orchestrator-d --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-orchestrator/src/orch_daemon/mod.rs crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs
git commit -m "feat(daemon): serve VOX_PROFILE and report it in orch.ping"
```

---

### Task 9: CLI-side daemon ensure: `profile_mismatch` and `VOX_PROFILE` on spawn

**Files:**
- Modify: `crates/vox-cli-core/src/daemon_ipc/orchestrator_daemon_ensure.rs` (struct L84–89, adopt branch L110–122, spawn L150–157, tests L236+)

**Interfaces:**
- Consumes: ping `"profile"` (Task 8). No new crate edge: `vox-cli-core` compares canonical id **strings**; callers pass `ActiveProfile::id_string()`.
- Produces:
  - `pub fn for_profile(profile_id: impl Into<String>) -> Self` (empty ⇒ `dev`)
  - `pub fn ping_profile_matches(ping: &serde_json::Value, requested: &str) -> bool` (missing `"profile"` ⇒ `"dev"`, i.e. older daemons)
  - `pub fn profile_mismatch_error(ping: &serde_json::Value, requested: &str) -> String` — begins with `profile_mismatch:`
  - `ensure()` refuses (returns `Err(profile_mismatch…)`) instead of adopting or spawning when an authenticated daemon serves another profile; spawn sets `VOX_PROFILE`.

Parallel-safe with: Task 8.

- [ ] **Step 1: Write the failing tests.** Append to the `tests` module:

```rust
    #[test]
    fn ping_profile_matches_treats_missing_as_dev() {
        let old = serde_json::json!({"ok": true});
        assert!(ping_profile_matches(&old, "dev"));
        assert!(ping_profile_matches(&old, ""));
        assert!(!ping_profile_matches(&old, "models"));
        let models = serde_json::json!({"ok": true, "profile": "knowledge,models"});
        assert!(ping_profile_matches(&models, "knowledge,models"));
        assert!(!ping_profile_matches(&models, "dev"));
    }

    #[test]
    fn profile_mismatch_error_names_both_profiles() {
        let msg = profile_mismatch_error(&serde_json::json!({"profile": "mesh"}), "models");
        assert!(msg.starts_with("profile_mismatch:"), "{msg}");
        assert!(msg.contains("`mesh`") && msg.contains("`models`"), "{msg}");
    }

    #[test]
    fn for_profile_defaults_empty_to_dev() {
        assert_eq!(OrchestratorDaemonEnsure::for_profile("").requested_profile(), "dev");
        assert_eq!(OrchestratorDaemonEnsure::default().requested_profile(), "dev");
        assert_eq!(OrchestratorDaemonEnsure::for_profile("models").requested_profile(), "models");
    }
```

- [ ] **Step 2: Run — expect compile failure.**

Run: `cargo nextest run -p vox-cli-core profile`
Expected: FAIL — `cannot find function ping_profile_matches`, `no function for_profile`.

- [ ] **Step 3: Implement.** Add a field to the struct:

```rust
#[derive(Default)]
pub struct OrchestratorDaemonEnsure {
    addr: OnceCell<String>,
    token: OnceCell<String>,
    child: Mutex<Option<std::process::Child>>,
    /// Canonical capability-profile id to adopt/spawn; empty means `dev`.
    profile: String,
}
```

Add at the top of `impl OrchestratorDaemonEnsure`:

```rust
    /// Ensure helper that only adopts (or spawns) a daemon serving `profile_id`
    /// (`ActiveProfile::id_string()`; empty ⇒ `dev`).
    pub fn for_profile(profile_id: impl Into<String>) -> Self {
        Self { profile: profile_id.into(), ..Default::default() }
    }

    fn requested_profile(&self) -> &str {
        if self.profile.trim().is_empty() { "dev" } else { self.profile.trim() }
    }
```

Replace the adopt block:

```rust
                if let Some(existing_token) = read_token_file()
                    && let Ok(resp) = OrchDaemonClient::with_token(addr.clone(), existing_token.clone())
                        .ping()
                        .await
                {
                    if !ping_profile_matches(&resp, self.requested_profile()) {
                        return Err(profile_mismatch_error(&resp, self.requested_profile()));
                    }
                    let _ = self.token.set(existing_token);
                    return Ok(addr);
                }
```

Add `.env("VOX_PROFILE", self.requested_profile())` after `.env("VOX_ORCHESTRATOR_DAEMON_TOKEN", &spawned_token)` in the spawn builder.

Add free functions above `ping_existing_daemon`:

```rust
fn ping_profile(ping: &serde_json::Value) -> &str {
    ping.get("profile").and_then(|v| v.as_str()).unwrap_or("dev")
}

/// True when the daemon's ping reports `requested` (canonical id string; empty ⇒ `dev`).
/// A daemon predating capability profiles reports none and counts as `dev`.
pub fn ping_profile_matches(ping: &serde_json::Value, requested: &str) -> bool {
    let requested = if requested.trim().is_empty() { "dev" } else { requested.trim() };
    ping_profile(ping) == requested
}

pub fn profile_mismatch_error(ping: &serde_json::Value, requested: &str) -> String {
    format!(
        "profile_mismatch: the running vox-orchestrator-d serves profile `{}`, but `{requested}` was requested; stop that daemon or request `--profile {}`",
        ping_profile(ping),
        ping_profile(ping)
    )
}
```

- [ ] **Step 4: Run — expect pass.**

Run: `cargo nextest run -p vox-cli-core profile token_is_none_before_ensure`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
cargo clippy -p vox-cli-core --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-cli-core/src/daemon_ipc/orchestrator_daemon_ensure.rs
git commit -m "feat(cli-core): refuse adopting a daemon with a different profile"
```

---

### Task 10: Axis (GUI) local daemon requests a profile and refuses mismatches

**Files:**
- Modify: `crates/vox-gui/src/commands/daemon.rs` — `reensure` (L199–345), add fn near `existing_daemon_is_adoptable_for_gui` (L488–495), tests (L517+)

**Interfaces:**
- Consumes: `vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::{ping_profile_matches, profile_mismatch_error}` (Task 9), `vox_orchestrator_mcp::profiles::ActiveProfile` (vox-gui depends on vox-orchestrator-mcp).
- Produces: `pub fn gui_requested_profile() -> Result<String, String>` — `VOX_PROFILE` (canonicalized) or `dev`; the GUI never reads the config-file default, so Axis-spawned daemons stay `dev` unless the user exports `VOX_PROFILE`.

- [ ] **Step 1: Write the failing test.** Append to `mod version_mismatch_tests`:

```rust
    #[test]
    fn gui_requested_profile_is_env_profile_or_dev() {
        let want = vox_orchestrator_mcp::profiles::ActiveProfile::from_env()
            .map(|p| p.id_string())
            .map_err(|e| e.to_string());
        assert_eq!(gui_requested_profile(), want);
    }
```

- [ ] **Step 2: Run — expect compile failure.**

Run: `cargo nextest run -p vox-gui gui_requested_profile`
Expected: FAIL — `cannot find function gui_requested_profile`. (Fresh worktree: if `tauri-build` fails on a missing sidecar or `ui/dist`, run `vox run scripts/gui-build.vox` and `pnpm --dir crates/vox-gui/ui install && pnpm --dir crates/vox-gui/ui build` once, per AGENTS.md §Perennial Bug Patterns.)

- [ ] **Step 3: Implement.** Add after `existing_daemon_is_adoptable_for_gui`:

```rust
/// Capability profile Axis asks its local daemon to serve: `VOX_PROFILE` when set, else `dev`.
pub fn gui_requested_profile() -> Result<String, String> {
    vox_orchestrator_mcp::profiles::ActiveProfile::from_env()
        .map(|p| p.id_string())
        .map_err(|e| e.to_string())
}
```

In `reensure`, right after the cache re-check (`if let Some((addr, _token)) = … { return Ok(addr); }`), add:

```rust
        let requested_profile = gui_requested_profile()?;
```

In the adopt branch, after the human-role refusal block and before `*self.resolved.write().await = Some(…)`:

```rust
            if !vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::ping_profile_matches(
                &resp,
                &requested_profile,
            ) {
                return Err(
                    vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::profile_mismatch_error(
                        &resp,
                        &requested_profile,
                    ),
                );
            }
```

In the spawn builder, after `.env("VOX_MCP_CALLER_ROLE", "human")` add `.env("VOX_PROFILE", &requested_profile)`.

- [ ] **Step 4: Run — expect pass.**

Run: `cargo nextest run -p vox-gui version_mismatch_tests`
Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
cargo clippy -p vox-gui --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-gui/src/commands/daemon.rs
git commit -m "feat(gui): pass VOX_PROFILE to the local daemon and refuse profile mismatches"
```

---

### Task 11: `vox mcp --profile <ids>`

**Files:**
- Create: `crates/vox-cli/src/commands/profile_arg.rs`
- Modify: `crates/vox-cli/src/commands/mod.rs` (L109–112)
- Modify: `crates/vox-cli/src/commands/mcp.rs`
- Modify: `crates/vox-cli/src/commands/mcp_server/mod.rs` (re-export list L7–11)
- Modify: `crates/vox-cli/src/lib.rs` (`Mcp,` at L463–464)
- Modify: `crates/vox-cli/src/cli_dispatch/mod.rs` (L401–403, test L605)

**Interfaces:**
- Consumes: `vox_mcp_registry::ActiveProfile` (non-optional dep of vox-cli), `vox_config::env_parse::resolve_config_opt_str` (env var, then `~/.vox/config.toml` key), `run_stdio_server_blocking_with_profile` (Task 6).
- Produces: `pub fn resolve_cli_profile(flag: Option<&str>) -> anyhow::Result<ActiveProfile>` (precedence: `--profile` > `VOX_PROFILE` env > config `VOX_PROFILE` > `dev`); `Cli::Mcp { profile: Option<String> }`; `commands::mcp::run(profile: Option<String>)`.

- [ ] **Step 1: Write the failing tests.** Create `crates/vox-cli/src/commands/profile_arg.rs` with:

```rust
//! Shared `--profile <ids>` resolution for `vox mcp` and `vox serve`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_wins_over_configured() {
        let p = resolve_profile_from(Some("models"), Some("mesh")).expect("resolve");
        assert_eq!(p.id_string(), "models");
    }

    #[test]
    fn configured_is_used_without_flag() {
        let p = resolve_profile_from(None, Some("knowledge, models")).expect("resolve");
        assert_eq!(p.id_string(), "knowledge,models");
    }

    #[test]
    fn blank_flag_falls_through_and_nothing_means_dev() {
        assert_eq!(resolve_profile_from(Some("  "), None).expect("dev").id_string(), "dev");
        assert_eq!(resolve_profile_from(None, None).expect("dev").id_string(), "dev");
    }

    #[test]
    fn unknown_id_is_profile_unknown() {
        let err = resolve_profile_from(Some("nope"), None).expect_err("unknown");
        assert_eq!(err.to_string(), "profile_unknown: nope");
    }

    #[test]
    fn resolve_cli_profile_with_flag_ignores_env_and_config() {
        assert_eq!(resolve_cli_profile(Some("mesh")).expect("mesh").id_string(), "mesh");
    }
}
```

In `crates/vox-cli/src/commands/mod.rs`, after `pub mod mcp_server;` add:

```rust
/// `--profile <ids>` resolution shared by `vox mcp` / `vox serve`.
pub mod profile_arg;
```

In `crates/vox-cli/src/commands/mcp.rs`, change the existing test call `let err = run()` to `let err = run(None)`, and add to it a new test module:

```rust
#[cfg(all(test, feature = "mcp-server"))]
mod profile_tests {
    #[tokio::test]
    async fn unknown_profile_fails_before_starting_the_server() {
        let err = super::run(Some("nope".to_string())).await.expect_err("unknown profile");
        assert_eq!(err.to_string(), "profile_unknown: nope");
    }
}
```

- [ ] **Step 2: Run — expect compile failure.**

Run: `cargo nextest run -p vox-cli profile_arg`
Expected: FAIL — `cannot find function resolve_profile_from`.

- [ ] **Step 3: Implement `profile_arg.rs`.** Insert above the tests:

```rust
use anyhow::Result;
use vox_mcp_registry::ActiveProfile;

const PROFILE_KEY: &str = "VOX_PROFILE";

/// `--profile` > `VOX_PROFILE` env > `VOX_PROFILE` in `~/.vox/config.toml` > `dev`.
pub fn resolve_cli_profile(flag: Option<&str>) -> Result<ActiveProfile> {
    let configured = vox_config::env_parse::resolve_config_opt_str(PROFILE_KEY);
    resolve_profile_from(flag, configured.as_deref())
}

fn resolve_profile_from(flag: Option<&str>, configured: Option<&str>) -> Result<ActiveProfile> {
    let raw = flag.filter(|s| !s.trim().is_empty()).or(configured).unwrap_or("");
    let ids: Vec<&str> = raw.split(',').collect();
    Ok(ActiveProfile::resolve(&ids)?)
}
```

- [ ] **Step 4: Wire `vox mcp`.** Replace `crates/vox-cli/src/commands/mcp.rs`'s `run` with:

```rust
/// Run the native in-process MCP server (stdio) if the `mcp-server` feature is enabled.
pub async fn run(profile: Option<String>) -> Result<()> {
    #[cfg(feature = "mcp-server")]
    {
        let profile = crate::commands::profile_arg::resolve_cli_profile(profile.as_deref())?;
        crate::commands::mcp_server::run_stdio_server_blocking_with_profile(profile).await?;
        Ok(())
    }

    #[cfg(not(feature = "mcp-server"))]
    {
        let _ = profile;
        anyhow::bail!(
            "Vox MCP server is not enabled in this build. Rebuild with: cargo build -p vox-cli --features mcp-server"
        )
    }
}
```

Add `run_stdio_server_blocking_with_profile` to the `pub use vox_orchestrator_mcp::{ … }` list in `commands/mcp_server/mod.rs`.

In `crates/vox-cli/src/lib.rs` replace `    Mcp,` with:

```rust
    Mcp {
        /// Capability profile ids (comma-separated), e.g. `models,knowledge`. Default: `VOX_PROFILE`, then `~/.vox/config.toml`, then `dev`.
        #[arg(long, value_name = "IDS")]
        profile: Option<String>,
    },
```

In `cli_dispatch/mod.rs` replace the arm with:

```rust
        Cli::Mcp { profile } => {
            crate::commands::mcp::run(profile).await?;
        }
```

and the test line with `assert_eq!(universal_reward_command_path(&Cli::Mcp { profile: None }), Some("command"));`.

- [ ] **Step 5: Run — expect pass.**

Run: `cargo nextest run -p vox-cli profile_arg universal_reward_maps && cargo nextest run -p vox-cli --features mcp-server unknown_profile_fails_before_starting_the_server`
Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
cargo check -p vox-cli && cargo check -p vox-cli --features mcp-server
cargo clippy -p vox-cli --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-cli/src/commands/profile_arg.rs crates/vox-cli/src/commands/mod.rs crates/vox-cli/src/commands/mcp.rs crates/vox-cli/src/commands/mcp_server/mod.rs crates/vox-cli/src/lib.rs crates/vox-cli/src/cli_dispatch/mod.rs
git commit -m "feat(cli): add vox mcp --profile"
```

---

### Task 12: `vox serve --profile <ids>` and command registration

**Files:**
- Create: `crates/vox-cli/src/commands/serve.rs`
- Modify: `crates/vox-cli/src/commands/mod.rs`, `crates/vox-cli/src/lib.rs` (after the `Mcp { … }` variant), `crates/vox-cli/src/cli_dispatch/mod.rs` (after the `Cli::Mcp` arm)
- Modify: `contracts/operations/catalog.v1.yaml` (insert a new op directly after the `- id: mcp` op, ~L16210–16237)
- Regenerate: `contracts/cli/command-registry.yaml`, `docs/src/reference/cli-command-surface.generated.md`, `crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt`
- Modify: `docs/src/reference/cli.md` (new sections after `### \`vox lsp\``, ~L665)

**Interfaces:**
- Consumes: `resolve_cli_profile` (Task 11), `ping_existing_daemon`, `ping_profile_matches`, `profile_mismatch_error` (Task 9), `process_supervision::resolve_managed_binary_path`, `vox_secrets::resolve_secret`.
- Produces: `vox serve --profile <ids>`: foreground `vox-orchestrator-d` with `VOX_PROFILE`, `VOX_MCP_HTTP_ENABLED=1`, `VOX_ORCHESTRATOR_DAEMON_SOCKET` (configured TCP address or `127.0.0.1:9745`), no stdio MCP. Refuses to start if a daemon is already running (`profile_mismatch:` when its profile differs) or when no gateway auth is configured. Bind/TLS flags are plan 04.

- [ ] **Step 1: Write the failing tests.** Create `crates/vox-cli/src/commands/serve.rs` with:

```rust
//! `vox serve` — run `vox-orchestrator-d` in the foreground with the HTTP gateway enabled and no
//! stdio MCP transport, scoped to a capability profile.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_env_sets_profile_gateway_and_default_socket() {
        let env = serve_child_env("knowledge,models", None);
        assert!(env.contains(&("VOX_PROFILE", "knowledge,models".to_string())));
        assert!(env.contains(&("VOX_MCP_HTTP_ENABLED", "1".to_string())));
        assert!(env.contains(&("VOX_ORCHESTRATOR_DAEMON_SOCKET", "127.0.0.1:9745".to_string())));
    }

    #[test]
    fn child_env_keeps_configured_socket() {
        let env = serve_child_env("dev", Some("127.0.0.1:9800".to_string()));
        assert!(env.contains(&("VOX_ORCHESTRATOR_DAEMON_SOCKET", "127.0.0.1:9800".to_string())));
    }

    #[test]
    fn refusal_for_same_profile_says_already_running() {
        let msg = existing_daemon_refusal(&serde_json::json!({"profile": "mesh"}), "mesh");
        assert!(msg.contains("already running"), "{msg}");
    }

    #[test]
    fn refusal_for_other_profile_is_profile_mismatch() {
        let msg = existing_daemon_refusal(&serde_json::json!({"profile": "mesh"}), "models");
        assert!(msg.starts_with("profile_mismatch:"), "{msg}");
    }

    #[test]
    fn truthy_matches_gateway_bool_parsing() {
        for v in ["1", "true", "YES", " on "] {
            assert!(truthy(v), "{v}");
        }
        for v in ["0", "false", "", "off"] {
            assert!(!truthy(v), "{v}");
        }
    }
}
```

Add to `commands/mod.rs` after `pub mod profile_arg;`:

```rust
/// Headless daemon + HTTP gateway scoped to a capability profile (`vox serve`).
pub mod serve;
```

- [ ] **Step 2: Run — expect compile failure.**

Run: `cargo nextest run -p vox-cli serve::tests`
Expected: FAIL — `cannot find function serve_child_env`.

- [ ] **Step 3: Implement `serve.rs`.** Insert above the tests:

```rust
use anyhow::{Context, Result};
use std::process::Stdio;
use vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::{
    ping_existing_daemon, ping_profile_matches, profile_mismatch_error,
};
use vox_cli_core::daemon_ipc::process_supervision::resolve_managed_binary_path;
use vox_secrets::SecretId;

const DEFAULT_DAEMON_ADDR: &str = "127.0.0.1:9745";

pub async fn run(profile_flag: Option<&str>) -> Result<()> {
    let profile = crate::commands::profile_arg::resolve_cli_profile(profile_flag)?;
    let profile_id = profile.id_string();

    if !gateway_auth_configured() {
        anyhow::bail!(
            "vox serve enables the HTTP gateway, which requires VOX_MCP_HTTP_BEARER_TOKEN or VOX_MCP_HTTP_READ_BEARER_TOKEN (or VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED=1 for local testing)"
        );
    }
    if let Some(client) = ping_existing_daemon().await {
        let ping = client.ping().await.unwrap_or_default();
        anyhow::bail!("{}", existing_daemon_refusal(&ping, &profile_id));
    }

    let socket = vox_secrets::resolve_secret(SecretId::VoxOrchestratorDaemonSocket)
        .expose()
        .filter(|s| s.contains(':'))
        .map(str::to_string);
    let daemon_bin = resolve_managed_binary_path("vox-orchestrator-d");
    let shown = daemon_bin.display().to_string();
    let mut cmd = std::process::Command::new(&daemon_bin);
    for (key, value) in serve_child_env(&profile_id, socket) {
        cmd.env(key, value);
    }
    cmd.stdin(Stdio::null());
    eprintln!("vox serve: profile `{profile_id}` — starting {shown}");
    let status = tokio::task::spawn_blocking(move || cmd.status())
        .await
        .context("joining vox-orchestrator-d")?
        .with_context(|| format!("failed to start {shown}"))?;
    if !status.success() {
        anyhow::bail!("vox-orchestrator-d exited with {status}");
    }
    Ok(())
}

fn serve_child_env(profile_id: &str, socket: Option<String>) -> Vec<(&'static str, String)> {
    vec![
        ("VOX_PROFILE", profile_id.to_string()),
        ("VOX_MCP_HTTP_ENABLED", "1".to_string()),
        (
            "VOX_ORCHESTRATOR_DAEMON_SOCKET",
            socket.unwrap_or_else(|| DEFAULT_DAEMON_ADDR.to_string()),
        ),
    ]
}

fn existing_daemon_refusal(ping: &serde_json::Value, requested: &str) -> String {
    if ping_profile_matches(ping, requested) {
        format!(
            "a vox-orchestrator-d serving profile `{requested}` is already running; vox serve will not start a second one"
        )
    } else {
        profile_mismatch_error(ping, requested)
    }
}

fn truthy(v: &str) -> bool {
    let t = v.trim();
    t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes") || t.eq_ignore_ascii_case("on")
}

fn gateway_auth_configured() -> bool {
    let present = |id: SecretId| {
        vox_secrets::resolve_secret(id).expose().is_some_and(|s| !s.trim().is_empty())
    };
    present(SecretId::VoxMcpHttpBearerToken)
        || present(SecretId::VoxMcpHttpReadBearerToken)
        || vox_secrets::resolve_secret(SecretId::VoxMcpHttpAllowUnauthenticated)
            .expose()
            .is_some_and(truthy)
}
```

In `crates/vox-cli/src/lib.rs`, after the `Mcp { … }` variant:

```rust
    /// Run the Vox daemon + HTTP gateway headless (no stdio), scoped to a capability profile
    Serve {
        /// Capability profile ids (comma-separated), e.g. `orchestrator,knowledge,mesh`. Default: `VOX_PROFILE`, then `~/.vox/config.toml`, then `dev`.
        #[arg(long, value_name = "IDS")]
        profile: Option<String>,
    },
```

In `cli_dispatch/mod.rs`, after the `Cli::Mcp` arm:

```rust
        Cli::Serve { profile } => {
            crate::commands::serve::run(profile.as_deref()).await?;
        }
```

- [ ] **Step 4: Run — expect pass.**

Run: `cargo nextest run -p vox-cli serve::tests`
Expected: PASS (5 tests).

- [ ] **Step 5: Register the command.** In `contracts/operations/catalog.v1.yaml`, immediately after the `- id: mcp` op block (which ends with `    reachability_required: null` before `- id: memory`), insert:

```yaml
- id: serve
  title: Serve
  description: CLI operation `vox serve`
  description_human: null
  product_lane: platform
  intent_tags: []
  side_effect_class: null
  scope_kind: null
  reversible: null
  requires_repo: null
  preferred_for_models: null
  human_takeover_friendly: null
  mens_planner_visible: null
  canonical_name: null
  latin_aliases: null
  mcp: null
  cli:
    path:
    - serve
    status: active
    latin_ns: null
    handler_rust: commands::serve::run
    feature_gate: null
    catalog_group: null
    ref_cli_required: true
    reachability_required: null
```

In `docs/src/reference/cli.md`, after the `### \`vox lsp\`` section, add:

```markdown
### `vox mcp`

Starts the stdio MCP server (feature `mcp-server`). Tool calls are forwarded to the shared `vox-orchestrator-d`. `--profile <ids>` scopes `tools/list` and dispatch to a capability profile (default: `VOX_PROFILE`, then `VOX_PROFILE` in `~/.vox/config.toml`, then `dev` = every tool). A running daemon with a different profile is not adopted (`profile_mismatch`). See [capability-profiles.md](capability-profiles.md).

### `vox serve`

Runs `vox-orchestrator-d` in the foreground with the HTTP gateway enabled and no stdio transport, scoped by `--profile <ids>` (same defaults as `vox mcp`). Requires gateway auth (`VOX_MCP_HTTP_BEARER_TOKEN` / `VOX_MCP_HTTP_READ_BEARER_TOKEN`, or `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED=1` for local testing). Refuses to start when a daemon is already running. See [capability-profiles.md](capability-profiles.md).
```

Regenerate and verify:

```bash
cargo run -p vox-cli -- ci operations-sync --target cli --write
cargo run -p vox-cli -- ci command-sync
UPDATE_CLI_CATALOG_BASELINE=1 cargo test -p vox-cli --test command_catalog_paths_baseline
grep -n '^serve$' crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt   # expect one line
grep -n -A2 -- '- serve$' contracts/cli/command-registry.yaml | head
cargo run -p vox-cli -- ci command-compliance
```

Expected: `command-compliance` passes (registry row, Cli variant, and `cli.md` reference all present).

- [ ] **Step 6: Commit.**

```bash
cargo clippy -p vox-cli --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-cli/src/commands/serve.rs crates/vox-cli/src/commands/mod.rs crates/vox-cli/src/lib.rs crates/vox-cli/src/cli_dispatch/mod.rs contracts/operations/catalog.v1.yaml contracts/cli/command-registry.yaml docs/src/reference/cli-command-surface.generated.md crates/vox-cli/tests/fixtures/command_catalog_paths_baseline.txt docs/src/reference/cli.md
git commit -m "feat(cli): add vox serve --profile"
```

---

### Task 13: Register `VOX_PROFILE`

**Files:**
- Modify: `contracts/config/env-vars.v1.yaml` (insert after the `VOX_MCP_CALLER_ROLE` entry, ~L1059–1065)
- Modify: `contracts/config/registry.v1.yaml` (append a knob to `knobs:`) — required because `vox ci config-hygiene` and `vox ci config-registry-parity` resolve registered names from `registry.v1.yaml` + Clavis + `CONFIG_KEYS`, **not** from `env-vars.v1.yaml`; without it the new `"VOX_PROFILE"` literal in `vox-mcp-registry` fails both gates.
- Modify: `docs/src/reference/env-vars.md` (insert after the `VOX_MCP_HTTP_TRUST_X_FORWARDED_FOR` row, ~L273)

**Interfaces:** Produces the env-var SSOT rows consumed by `vox ci config-hygiene`, `vox ci config-registry-parity` and `vox ci data-storage-guard`.

Parallel-safe with: Task 1, Tasks 4–12. Must land in the same push as Task 2 (which adds the `"VOX_PROFILE"` read).

- [ ] **Step 1: Run the gates first to see the gap** (after Task 2 has landed so `std::env::var("VOX_PROFILE")` exists in `vox-mcp-registry`):

```bash
cargo run -q -p vox-cli -- ci config-registry-parity
cargo run -q -p vox-cli -- ci config-hygiene
```

Expected: both FAIL naming `VOX_PROFILE` as unregistered (`config-hygiene` as `env-var-not-in-registry`).

- [ ] **Step 2: Add the contract rows.** `env-vars.v1.yaml`:

```yaml
  - name: "VOX_PROFILE"
    owner_crate: "vox-mcp-registry"
    kind: "string"
    required: false
    introduced_in: "0.6.0"
    default: "dev"
    description: "Comma-separated capability profile ids (contracts/profiles/<id>.v1.yaml) scoping the MCP/HTTP tool surface; unset or empty means dev (every tool). vox mcp / vox serve also read the VOX_PROFILE key in ~/.vox/config.toml; spawners (CLI, Axis) pass it to vox-orchestrator-d."
```

`registry.v1.yaml` (append to `knobs:`):

```yaml
  - name: capability_profile
    env_var: VOX_PROFILE
    description: Comma-separated capability profile ids scoping MCP/HTTP tools and daemon boot; unset or empty means dev.
    owner_crate: vox-mcp-registry
    status: active
    default_value: "dev"
    since: "2026-10-08"
```

- [ ] **Step 3: Add the docs row.**

```markdown
| `VOX_PROFILE` | Comma-separated capability profile ids (`models`, `knowledge`, …; see [capability-profiles.md](capability-profiles.md)). Scopes MCP `tools/list`, HTTP `/v1/tools`, and dispatch. Unset/empty ⇒ `dev` (every tool, today's behaviour). `vox mcp`/`vox serve` precedence: `--profile` > `VOX_PROFILE` > `VOX_PROFILE` in `~/.vox/config.toml` > `dev`. Passed by spawners to `vox-orchestrator-d`, whose `orch.ping` reports it; mismatched adopts fail with `profile_mismatch`. |
```

- [ ] **Step 4: Verify and commit.**

```bash
cargo run -q -p vox-cli -- ci config-registry-parity
cargo run -q -p vox-cli -- ci config-hygiene
cargo run -p vox-cli -- ci data-storage-guard
git add contracts/config/env-vars.v1.yaml contracts/config/registry.v1.yaml docs/src/reference/env-vars.md
git commit -m "docs(env): register VOX_PROFILE"
```

Expected: all three pass.

---

### Task 14: Bundle default profiles

**Files:**
- Modify: `crates/vox-plugin-catalog/catalog.toml` (bundles L244–311)
- Modify: `crates/vox-plugin-catalog/src/schema.rs` (`BundleEntry` L95–113)
- Modify: `crates/vox-plugin-catalog/src/lib.rs` (after `bundle_resolved`, L72–108; tests L110+)
- Modify: `crates/vox-plugin-catalog/build.rs` (`BundleEntry` L22–30; bundle cross-reference loop L155–172)
- Modify: `crates/vox-plugin-catalog/src/docs.rs` (`render_bundles_md` L84–100)
- Modify: `crates/vox-plugin-catalog/tests/docs_generation.rs`
- Modify: `crates/vox-cli/src/commands/plugin_bundle/apply.rs`
- Modify: `crates/vox-orchestrator-mcp/src/profiles.rs` (cross-crate validation test)
- Regenerate: `docs/src/reference/distribution-bundles.generated.md`

**Interfaces:**
- Produces: `BundleEntry.profiles: Vec<String>` (`#[serde(default)]`, **not** inherited through `extends`); `pub fn bundle_profiles(id: &str) -> Result<&'static [String], ResolveError>`; build-time validation: each id is `dev` or has `contracts/profiles/<id>.v1.yaml`, `dev` stands alone, `browser` never appears, no duplicates; `vox bundle apply <id>` writes `VOX_PROFILE=<canonical ids>` to `~/.vox/config.toml` after a fully successful install when the bundle declares profiles.
- No new crate edge: `vox-plugin-catalog` validates by file existence; `vox-orchestrator-mcp` (depends on both crates already) cross-checks ids against `PROFILES` in a test; `vox-cli` validates with `ActiveProfile::resolve` at apply time.

Parallel-safe with: Tasks 4–12 (no shared files).

- [ ] **Step 1: Write the failing tests.** Append to `mod bundle_membership_tests` in `vox-plugin-catalog/src/lib.rs`:

```rust
    #[test]
    fn bundle_profiles_match_the_spec_defaults() {
        use super::bundle_profiles;
        assert_eq!(bundle_profiles("vox-server").expect("server"), ["orchestrator", "knowledge", "mesh"]);
        assert_eq!(bundle_profiles("vox-mesh").expect("mesh"), ["mesh"]);
        assert_eq!(bundle_profiles("vox-dev").expect("dev"), ["dev"]);
        assert!(bundle_profiles("vox-base").expect("base").is_empty());
    }

    #[test]
    fn bundle_profiles_unknown_bundle_errors() {
        assert!(super::bundle_profiles("nope").is_err());
    }

    #[test]
    fn no_bundle_defaults_to_browser() {
        for b in super::all_bundles() {
            assert!(!b.profiles.iter().any(|p| p == "browser"), "{}", b.id);
        }
    }
```

Append to `crates/vox-plugin-catalog/tests/docs_generation.rs`:

```rust
#[test]
fn bundles_md_shows_default_profiles() {
    let md = render_bundles_md();
    assert!(md.contains("default profile"));
    assert!(md.contains("`orchestrator,knowledge,mesh`"));
}
```

Append to the tests in `crates/vox-orchestrator-mcp/src/profiles.rs`:

```rust
    #[test]
    fn every_bundle_profile_id_resolves() {
        for b in vox_plugin_catalog::all_bundles() {
            let ids: Vec<&str> = b.profiles.iter().map(String::as_str).collect();
            assert!(ActiveProfile::resolve(&ids).is_ok(), "bundle {} has unknown profile ids {ids:?}", b.id);
        }
    }
```

Add a test module at the end of `crates/vox-cli/src/commands/plugin_bundle/apply.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::default_profile_for_bundle;

    #[test]
    fn server_bundle_default_profile_is_canonical() {
        assert_eq!(
            default_profile_for_bundle("vox-server").expect("server").as_deref(),
            Some("knowledge,mesh,orchestrator")
        );
    }

    #[test]
    fn bundle_without_profiles_writes_nothing() {
        assert_eq!(default_profile_for_bundle("vox-base").expect("base"), None);
    }
}
```

- [ ] **Step 2: Run — expect compile failure.**

Run: `cargo nextest run -p vox-plugin-catalog bundle_profiles bundles_md_shows_default_profiles no_bundle_defaults_to_browser`
Expected: FAIL — `cannot find function bundle_profiles`, `no field profiles on type BundleEntry`.

- [ ] **Step 3: Schema + accessor.** In `schema.rs` `BundleEntry`, after `plugins`:

```rust

    /// Capability profiles written as the default `VOX_PROFILE` by `vox bundle apply`.
    /// Not inherited through `extends`; empty means "leave the configured profile alone".
    #[serde(default)]
    pub profiles: Vec<String>,
```

In `lib.rs`, after `bundle_resolved`:

```rust
/// Default capability profile ids declared directly on bundle `id` (not inherited).
pub fn bundle_profiles(id: &str) -> Result<&'static [String], ResolveError> {
    all_bundles()
        .iter()
        .find(|b| b.id == id)
        .map(|b| b.profiles.as_slice())
        .ok_or_else(|| ResolveError::UnknownBundle(id.to_string()))
}
```

- [ ] **Step 4: Catalog values.** In `catalog.toml`, add a `profiles = [...]` line after the `plugins = …` value of these bundles:

```toml
# vox-fullstack
profiles = ["dev"]
# vox-ml
profiles = ["dev"]
# vox-ml-metal
profiles = ["dev"]
# vox-mesh
profiles = ["mesh"]
# vox-server
profiles = ["orchestrator", "knowledge", "mesh"]
# vox-edge
profiles = ["lang", "skills"]
# vox-dev
profiles = ["dev"]
```

(`vox-base` and `vox-mobile` get no `profiles` line.) The `# bundle-id` comments above are only labels for this plan — write just the `profiles = …` line inside each `[[bundle]]` table.

- [ ] **Step 5: Build-time validation.** In `build.rs` `struct BundleEntry`, add:

```rust
    #[serde(default)]
    profiles: Vec<String>,
```

At the top of `main` (next to `println!("cargo:rerun-if-changed=catalog.toml");`) add:

```rust
    println!("cargo:rerun-if-changed=../../contracts/profiles");
```

Inside the existing `for b in &cat.bundles { … }` cross-reference loop, after the `extends` check, add:

```rust
        let mut seen_profiles = HashSet::new();
        for prof in &b.profiles {
            if !seen_profiles.insert(prof.clone()) {
                errors.push(format!("bundle '{}' lists profile '{}' twice", b.id, prof));
            }
            if prof == "browser" {
                errors.push(format!(
                    "bundle '{}' lists profile 'browser', which is never part of a bundle default",
                    b.id
                ));
            }
            if prof != "dev"
                && !std::path::Path::new("../../contracts/profiles")
                    .join(format!("{prof}.v1.yaml"))
                    .is_file()
            {
                errors.push(format!(
                    "bundle '{}' lists profile '{}', but contracts/profiles/{}.v1.yaml does not exist",
                    b.id, prof, prof
                ));
            }
        }
        if b.profiles.iter().any(|p| p == "dev") && b.profiles.len() > 1 {
            errors.push(format!("bundle '{}': profile 'dev' must stand alone", b.id));
        }
```

- [ ] **Step 6: Docs rendering.** In `docs.rs` replace the table header and row:

```rust
    out.push_str("| bundle | extends | direct plugins | total resolved plugins | default profile |\n");
    out.push_str("|--------|---------|----------------|------------------------|-----------------|\n");
    for b in all_bundles() {
        let resolved_count = bundle_resolved(&b.id)
            .map(|v| v.len().to_string())
            .unwrap_or_else(|e| format!("ERR: {e}"));
        let profile = if b.profiles.is_empty() {
            "—".to_string()
        } else {
            format!("`{}`", b.profiles.join(","))
        };
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            b.id,
            b.extends
                .as_deref()
                .map(|s| format!("`{s}`"))
                .unwrap_or_else(|| "—".to_string()),
            b.plugins.len(),
            resolved_count,
            profile,
        ));
    }
```

- [ ] **Step 7: `vox bundle apply` writes the default profile.** In `apply.rs`, add:

```rust
/// Canonical default profile for bundle `id`, or `None` when the bundle declares none.
pub fn default_profile_for_bundle(id: &str) -> Result<Option<String>> {
    let ids = vox_plugin_catalog::bundle_profiles(id)
        .with_context(|| format!("resolving bundle '{}'", id))?;
    if ids.is_empty() {
        return Ok(None);
    }
    let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
    Ok(Some(vox_mcp_registry::ActiveProfile::resolve(&ids)?.id_string()))
}
```

and replace the final `Ok(())` of `run` with:

```rust
    if let Some(profile) = default_profile_for_bundle(id)? {
        vox_config::toml_config::set_user_config_value("VOX_PROFILE", &profile)
            .map_err(anyhow::Error::msg)?;
        println!("Default capability profile set to `{profile}` (VOX_PROFILE in ~/.vox/config.toml).");
    }

    Ok(())
```

- [ ] **Step 8: Run — expect pass; regenerate docs.**

```bash
cargo nextest run -p vox-plugin-catalog
cargo nextest run -p vox-orchestrator-mcp every_bundle_profile_id_resolves
cargo nextest run -p vox-cli plugin_bundle::apply
cargo run -p vox-cli -- ci generate-plugin-catalog-docs
git diff --stat docs/src/reference/distribution-bundles.generated.md   # expect the new column
```

Expected: all PASS.

- [ ] **Step 9: Commit.**

```bash
cargo clippy -p vox-plugin-catalog --all-targets -- -D warnings
cargo clippy -p vox-cli --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-plugin-catalog crates/vox-cli/src/commands/plugin_bundle/apply.rs crates/vox-orchestrator-mcp/src/profiles.rs docs/src/reference/distribution-bundles.generated.md
git commit -m "feat(bundles): declare default capability profiles and apply them"
```

---

### Task 15: Reference docs and `where-things-live`

**Files:**
- Create: `docs/src/reference/capability-profiles.md`
- Modify: `docs/src/architecture/where-things-live.md` — `vox-mcp-registry` crate row (~L107); "Common tasks → exact path" table (after the "Add an MCP tool" row, ~L214)
- Modify: `crates/vox-mcp-registry/Cargo.toml` `description` (kept identical to the crate row)

**Interfaces:** Documentation only.

Parallel-safe with: Task 14 (after Tasks 1–12 so the documented behaviour exists).

- [ ] **Step 1: Write the reference page.** Create `docs/src/reference/capability-profiles.md`:

```markdown
---
title: "Capability profiles"
description: "Reference for Vox capability profiles: the first-wave profile ids, how they compose, the --profile flag and VOX_PROFILE, how HTTP allowlists combine with profiles, and the typed profile errors."
category: "Language Reference"
status: "current"
training_eligible: true
---

# Capability profiles

A capability profile is a named slice of Vox's MCP tool surface plus the subsystems it needs.
Running a server with a profile makes MCP `tools/list`, HTTP `GET /v1/tools`, WebSocket
`list_tools`, and **every** dispatch path (stdio → daemon, daemon `orch.tool_call`, HTTP, WS,
autonomous agents) expose only that slice. Design: [capability profiles spec](../../superpowers/specs/2026-10-08-capability-profiles-design.md).

## Profiles

Manifests live in `contracts/profiles/<id>.v1.yaml` and are compiled into `vox-mcp-registry`
(`PROFILES`) by its build script, which expands `tools.include` (exact names or trailing-`*`
prefixes) and `include_lanes`, clips to `max_exposure`, and fails the build on unknown tools,
unknown subsystems, or a tool whose `requires` is not covered by the profile's subsystem closure.

| Profile | Contents | Subsystems (`requires`) | Max exposure |
|---|---|---|---|
| `dev` | Every tool (synthesized; the default) | all | internal |
| `models` | Model list/select/suggest, model manifest, benchmarks, chat | `db`, `model_registry`, `llm_facade` | operator |
| `knowledge` | `vox_memory_*`, `vox_session_*`, `vox_kb_*`, `vox_search_*`, `vox_knowledge_query` | `memory`, `sessions`, `search_index` | operator |
| `research` | `vox_research_*`, Scientia discovery/evaluation/prepare, news drafts — no publish/approve | `research` | operator |
| `research-publish` | Scientia/news publish, approve, submission, external jobs | `research` | operator |
| `mesh` | `vox_mesh_*`, `vox_populi_local_status`, `vox_a2a_*`, `vox_publish_message` | `mesh` | operator |
| `attention` | `vox_attention_*`, `vox_budget_status`, `vox_trust_override` | `attention` | operator |
| `skills` | `vox_skill_*` | — | operator |
| `orchestrator` | Tasks, agents, queue, locks, plans, handoffs, approvals, clarifications | `agent_fleet`, `clarification`, `llm_facade` | operator |
| `lang` | `vox_check`, `vox_validate_*`, tests, crate build/lint, grammar, compiler introspection | `compiler` | operator |
| `repo-read` | Read-only git/VCS/repo-index/snapshot views | — | public |
| `browser` | `vox_browser_*` (never a bundle default) | `browser` | operator |
| `speech` | `vox_oratio_*`, `vox_speech_to_code` | `speech` | operator |

The authoritative tool list for each profile is its manifest. Feature-gated tools (for example
`vox_scientia_*` without `news-publish`, `vox_browser_*` without `heavy-browser`) are listed in a
profile but only advertised when the build can dispatch them.

## Composition

`--profile models,knowledge` is the union of the tool sets and of the subsystem closures; the
resulting `max_exposure` is the maximum. Ids are canonicalized (sorted, de-duplicated): the
canonical form of `models,knowledge` is `knowledge,models`. Any list containing `dev` is `dev`.

## Selecting a profile

| Surface | How |
|---|---|
| `vox mcp --profile <ids>` | stdio MCP; the shared daemon is adopted or spawned with the same profile. |
| `vox serve --profile <ids>` | `vox-orchestrator-d` in the foreground with the HTTP gateway on, no stdio. Requires gateway auth; refuses if a daemon is already running. |
| `VOX_PROFILE=<ids>` | Read by `vox-orchestrator-d`, `vox mcp`, `vox serve`, and Axis when spawning its local daemon. |
| `~/.vox/config.toml` `VOX_PROFILE = "<ids>"` | Default for `vox mcp` / `vox serve`; written by `vox bundle apply` for bundles that declare `profiles`. Axis ignores it (its local daemon stays `dev` unless `VOX_PROFILE` is exported). |

Precedence for `vox mcp` / `vox serve`: `--profile` > `VOX_PROFILE` > config file > `dev`.

Bundle defaults (`crates/vox-plugin-catalog/catalog.toml`): `vox-server` → `orchestrator,knowledge,mesh`;
`vox-mesh` → `mesh`; `vox-edge` → `lang,skills`; `vox-fullstack`, `vox-ml`, `vox-ml-metal`, `vox-dev` → `dev`;
`vox-base` and `vox-mobile` leave the configured profile alone.

## Daemon adoption

`orch.ping` reports `"profile": "<canonical ids>"` (a daemon predating profiles counts as `dev`).
A client asking for a different profile does not adopt that daemon and gets `profile_mismatch`;
stop the daemon or request its profile.

## HTTP gateway allowlists

The effective HTTP tool set is `VOX_MCP_HTTP_ALLOWED_TOOLS` ∩ the profile. When
`VOX_MCP_HTTP_ALLOWED_TOOLS` is unset, `dev` keeps the historical default allowlist and a named
profile defaults to its own tools. Unknown names in the env var still fail gateway startup; names
outside the profile are dropped with a warning. Read-token narrowing
(`http_read_role_eligible`, `VOX_MCP_HTTP_READ_ROLE_ALLOWED_TOOLS`) applies on top, unchanged.

Workspace `@tool`s and skill macro tools are not in the static registry, so they are served only
under `dev`.

## Errors

Denials use one JSON shape on stdio MCP (tool result with `isError: true`), WebSocket, and HTTP
(with the listed status):

    {"error":{"code":"subsystem_not_booted","message":"tool vox_kb_query requires subsystem search_index, which is not running","subsystem":"search_index"}}

| Code | HTTP | When |
|---|---|---|
| `tool_not_in_profile` | 404 | Tool exists but the active profile excludes it (or a non-registry tool under a named profile). |
| `subsystem_not_booted` | 503 | A subsystem the tool requires is not running (plan 03's conditioned boot). |
| `exposure_denied` | 403 | `internal` tool off loopback, or `operator` tool with a read token (plan 04). |
| `insufficient_role` | 403 | Read token for a tool that is not read-role eligible (plan 04). |
| `profile_unknown` | 400 | `--profile` / `VOX_PROFILE` names a manifest that does not exist; `vox-orchestrator-d` exits non-zero. |
| `profile_mismatch` | 409 | An adoptable daemon serves a different profile. |

The decision order is fixed and implemented once, in `vox_mcp_registry::authorize_tool`:
membership → internal off loopback → operator for read role → read-role eligibility → booted subsystems.
```

- [ ] **Step 2: Lint.**

Run: `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/capability-profiles.md`
Expected: PASS (fix any reported frontmatter/link issue and re-run).

- [ ] **Step 3: `where-things-live.md`.** Replace the `vox-mcp-registry` row description with:

```markdown
| [`vox-mcp-registry`](../../../crates/vox-mcp-registry/) | Compile-time MCP tool registry and capability profiles (`PROFILES`, `ActiveProfile`, `authorize_tool`) from contracts YAML (SSOT). |
```

and set the same text as `description` in `crates/vox-mcp-registry/Cargo.toml`:

```toml
description = "Compile-time MCP tool registry and capability profiles (PROFILES, ActiveProfile, authorize_tool) from contracts YAML (SSOT)."
```

Add after the "Add an MCP tool" row:

```markdown
| Add or change a capability profile (tool slice + subsystems) | Manifest `contracts/profiles/<id>.v1.yaml` (compiled into `PROFILES` by [`vox-mcp-registry/build.rs`](../../../crates/vox-mcp-registry/build.rs)); decision fn `authorize_tool` in [`profile.rs`](../../../crates/vox-mcp-registry/src/profile.rs); host guard [`vox-orchestrator-mcp/src/profiles.rs`](../../../crates/vox-orchestrator-mcp/src/profiles.rs); reference [capability-profiles.md](../reference/capability-profiles.md) |
| Run Vox headless with one capability slice | `vox serve --profile` in [`commands/serve.rs`](../../../crates/vox-cli/src/commands/serve.rs); `vox mcp --profile` in [`commands/mcp.rs`](../../../crates/vox-cli/src/commands/mcp.rs); bundle defaults `profiles = [...]` in [`catalog.toml`](../../../crates/vox-plugin-catalog/catalog.toml) |
```

- [ ] **Step 4: Verify links and commit.**

```bash
cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md
cargo run -q -p vox-cli -- ci check-links
git add docs/src/reference/capability-profiles.md docs/src/architecture/where-things-live.md crates/vox-mcp-registry/Cargo.toml
git commit -m "docs: capability profiles reference and where-things-live rows"
```

---

### Task 16: Final verification

**Files:** none (verification only).

- [ ] **Step 1: Per-crate test + lint sweep.**

```bash
for c in vox-mcp-registry vox-orchestrator-mcp vox-orchestrator vox-orchestrator-d vox-cli-core vox-plugin-catalog vox-cli vox-gui; do
  cargo clippy -p "$c" --all-targets -- -D warnings || exit 1
done
cargo nextest run -p vox-mcp-registry
cargo nextest run -p vox-orchestrator-mcp profiles registry http_gateway dispatch server
cargo nextest run -p vox-orchestrator ping_
cargo nextest run -p vox-cli-core
cargo nextest run -p vox-plugin-catalog
cargo nextest run -p vox-cli profile_arg serve:: plugin_bundle universal_reward
cargo nextest run -p vox-gui version_mismatch_tests
```

Expected: all PASS.

- [ ] **Step 2: Feature-unified back-compat check** (workspace feature unification turns on `news-publish`/`heavy-browser`, which skips the snapshot test; the feature-independent dev test must still pass):

Run: `cargo nextest run -p vox-orchestrator-mcp --features news-publish,heavy-browser,gui-visual-review dev_profile_advertises_exactly_the_feature_filtered_registry`
Expected: PASS.

- [ ] **Step 3: SSOT + pre-push gate.**

```bash
cargo run -p vox-cli -- ci operations-sync --target cli
cargo run -p vox-cli -- ci command-sync
cargo run -p vox-cli -- ci data-storage-guard
cargo run -q -p vox-arch-check
cargo run -p vox-cli -- ci crate-edges
VOX_FMT_CHECK=1 vox run scripts/fmt.vox -- --all
vox ci pre-push --complete
```

Expected: no drift, no new crate edges, formatting clean, gate green.

- [ ] **Step 4: Manual smoke** (loopback, throwaway token):

```bash
cargo build -p vox-cli --features mcp-server -p vox-orchestrator-d
VOX_MCP_HTTP_BEARER_TOKEN=smoke ./target/debug/vox serve --profile models &
sleep 5
curl -s -H 'Authorization: Bearer smoke' http://127.0.0.1:3921/v1/tools | grep -c vox_list_models   # expect ≥1
curl -s -o /dev/null -w '%{http_code}\n' -H 'Authorization: Bearer smoke' -H 'content-type: application/json' \
  -d '{"name":"vox_git_status","args":{}}' http://127.0.0.1:3921/v1/tools/call                       # expect 404
VOX_PROFILE=knowledge ./target/debug/vox mcp </dev/null; echo "exit=$?"                              # stdio exits on EOF; tool calls would hit profile_mismatch
kill %1
```

Record the observed outputs in the PR description (this is evidence, not a gate).

---

## Self-Review

| Spec / contract requirement | Task |
|---|---|
| §4 manifests `contracts/profiles/<id>.v1.yaml`, generated into `vox-mcp-registry` | 2 |
| §4 include exact + trailing-`*`, `include_lanes`, clip to `max_exposure` | 2 (build.rs) |
| §5 generator fails when a profile tool's `requires` ⊄ closure of profile `requires` | 2 (build.rs panic + runtime test) |
| §4 profiles compose (union of tools and closures) | 2 (`resolve`, `composition_is_union_and_canonical`) |
| §4 `dev` = every op + every subsystem; default for `vox mcp` and Axis | 2, 6, 10, 11 |
| §4 first-wave table (12 profiles), `research` minus publish/approve, `browser` never in a composite default | 2, 14 (build.rs `browser` ban) |
| 00-contracts types verbatim (`ProfileManifest`, `PROFILES`, `ActiveProfile`, `ProfileError`, `BootReport`, `ProfileDenial`, `CallerRole`, `CallContext`, `authorize_tool`) | 1, 2, 3 |
| `authorize_tool` 5-step order pinned by tests | 3 |
| Advertising = tools where `authorize_tool` is `Ok` (stdio `tools/list`, `GET /v1/tools`, WS `list_tools`) | 6, 7 |
| §5 runtime: list and dispatch filter on in-profile ∧ booted (BootReport default all-up until plan 03) | 4, 5, 6, 7 |
| Wire shape `{"error":{code,message,subsystem?}}`, `isError: true`; HTTP status from `http_status()` | 4, 5, 6, 7 |
| `VOX_MCP_HTTP_ALLOWED_TOOLS` keeps working; composition = intersection | 7, 15 |
| `vox mcp --profile`, `vox serve --profile` (no bind/TLS flags) + registry + command-sync | 11, 12 |
| `VOX_PROFILE` in env-vars contract + docs; spawners pass it | 8, 9, 10, 13 |
| Ping gains `"profile"`; mismatched adopt ⇒ `profile_mismatch` (CLI + GUI) | 8, 9, 10 |
| `profile_unknown` on bad ids | 2, 8, 11 |
| Bundles gain `profiles = [...]`; `vox bundle apply` writes default profile; docs regenerated | 14 |
| §9.5 back-compat: no profile ⇒ today's tool list (snapshot) | 6, 7 (`dev_allowlist_default_is_unchanged`), 16 |
| §9.2 mutation-verified guards | 3 (steps 1–2), 5, 6, 7 |
| Reference docs + `where-things-live` | 12 (`cli.md`), 13, 15 |
| No new crate edges | 9 (string compare in cli-core), 14 (file-existence check), 16 (`ci crate-edges`) |

Contract notes found while planning (all three are now recorded in `00-contracts.md` § Amendments "From plan 02"; plan 06 treats `"*"` as a wildcard in `remote_reason`):

1. `dev.axis_surfaces` is `["*"]` meaning "every surface" — plan 06's surface gating must treat `"*"` as a wildcard (00-contracts does not define dev's surfaces).
2. `ProfileDenial` is placed in `src/boot.rs` as in the 00-contracts code block; it is re-exported from `lib.rs`, so callers are unaffected.
3. Plan 02 adds non-contract helpers `vox_orchestrator_mcp::profiles::{is_dev, full_boot_report, guard_tool, denial_error_json, is_profile_denial_json}` and `vox_cli_core::…::{ping_profile_matches, profile_mismatch_error, OrchestratorDaemonEnsure::for_profile}`; plans 03/04 should reuse them rather than re-deriving.

## Risks

- **Plan 01 classification drives the manifests.** Which tools are `internal`/`operator` and what each tool `requires` is not known until plan 01 lands; Task 2 Step 6 gives a closed set of fixes, but `models` may lose `vox_chat_history` (if it requires `db`) and `repo-read` may lose tools classified above `public`. The `authorize_tests` locate tools by property and `expect` that plan 01 produced at least one tool of each needed kind.
- **Build-script subsystem table duplicates `Subsystem::deps`.** Drift is caught only by the runtime test `named_profiles_respect_max_exposure_and_subsystem_closure`, not at build time.
- **Non-registry tools under named profiles.** Workspace `@tool`s, skill macro tools, and `vox_run_shell`-style names are denied under any named profile. This is intentional (they have no exposure/requires metadata) but may surprise users of `vox serve --profile skills` who expect skill macro tools.
- **Chat tool selection is not profile-aware.** The chat agent loop may still offer the model tools outside the profile; dispatch denies them with `tool_not_in_profile`. A follow-up should filter chat tool selection by `state.active_profile`.
- **Axis and CLI can disagree.** Axis ignores the config-file default; after `vox bundle apply vox-server`, a CLI-spawned daemon serves `knowledge,mesh,orchestrator` and Axis will refuse to adopt it (`profile_mismatch`) until one side changes. This is the spec's intended behaviour but needs a clear UI message (plan 06).
- **One-shot stdio daemon spawns** (`vox_cli_core::daemon_ipc::dispatch::call_daemon`, used by `vox dei`/`generate`/…) inherit the parent's environment rather than receiving an explicit `VOX_PROFILE`; they serve CLI subcommands, not MCP, and are out of scope here.
- **`vox-gui` merge friction.** The main checkout holds uncommitted model-bench WIP under `crates/vox-gui/**`; Task 10 touches `crates/vox-gui/src/commands/daemon.rs`, which may conflict when that WIP lands.
- **`vox serve` gateway auth pre-check duplicates the gateway's own check** (in `spawn_http_gateway_if_enabled`). Plan 04 replaces both with the bind policy; until then a divergence would only make `vox serve` stricter, never looser.
- **`ServerState::new_test()` is heavy** (full repo-scoped orchestrator build); the new tests add several such constructions to `vox-orchestrator-mcp`'s suite runtime.
