---
title: "Capability Profiles — Plan 01: Exposure metadata and CI gate"
description: "Implementation plan that adds Exposure, SideEffectClass and Subsystem to vox-mcp-registry, classifies all 340 MCP operations in the operations catalog, removes the unused tier field, moves the workspace-tool opt-in to VOX_MCP_WORKSPACE_TOOLS (VOX_MCP_TIERS becomes a deprecated alias), and makes operations-sync fail on unclassified tools."
category: "architecture"
status: "proposed"
date: 2026-10-08
---

# Exposure Metadata Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every one of the 340 MCP operations in `contracts/operations/catalog.v1.yaml` carries a validated `exposure`, `side_effect_class`, and `requires`; the generated `TOOL_REGISTRY` exposes them as typed fields; the unused `tier` field is gone; the workspace-tool opt-in is `VOX_MCP_WORKSPACE_TOOLS=1` with `VOX_MCP_TIERS` as a deprecated alias; and `vox ci operations-sync --target mcp` (plus the `vox-mcp-registry` build script) fails when any tool is unclassified.

**Architecture:** The vocabulary (`Exposure`, `SideEffectClass`, `Subsystem`, `subsystem_closure`) and a single row validator (`classify_tool_row`) live in `vox-mcp-registry/src/`. The same three source files are compiled into `vox-mcp-registry/build.rs` via `#[path]` modules, so the build-time gate and the `vox-cli` operations-sync gate share one implementation and cannot drift. The catalog (hand-edited SSOT) gains `mcp.exposure` / `mcp.requires` and a required non-null `side_effect_class`; `operations-sync` projects them into the generated `tool-registry.canonical.yaml`, which `build.rs` turns into typed `McpToolRegistryEntry` rows. Classification of the 340 tools is split into ten parallel group tasks that each touch only their own op blocks.

**Tech Stack:** Rust 2024 (workspace toolchain), `serde` / `serde_yaml` (`serde_yaml_ng`), `serde_json`, JSON Schema (draft-07 catalog schema, 2020-12 tool-registry schema) via `vox_jsonschema_util`, `cargo nextest`, `rg`.

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

Plan-01-specific constraints:

- Work only in `/Users/brbrainerd/dev/vox/.worktrees/capability-profiles` (branch `feat/capability-profiles`). Never touch `/Users/brbrainerd/dev/vox` outside `.worktrees/`.
- Run every `cargo` command from the worktree root. If `vox` is not on `PATH`, replace `vox run …` / `vox ci …` with `./scripts/vox-dev.sh run …` / `./scripts/vox-dev.sh ci …`.
- Between Task 2 and Task 13 the branch is **intentionally red** for `vox ci operations-verify` / `vox ci ssot-drift` (the gate is live, the catalog is not yet classified). Do not push until Task 13 is merged; the pre-push hook will (correctly) refuse.
- Recorded deviation (also in `00-contracts.md`): `Subsystem` lives in `vox-mcp-registry`, not `vox-orchestrator-types`.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `crates/vox-mcp-registry/Cargo.toml` | Modify | Add `serde` (normal dep) and `serde_json` (dev-dep). External crates only — no workspace crate edge. |
| `crates/vox-mcp-registry/src/exposure.rs` | Create | `Exposure`, `SideEffectClass` + `as_str` / `parse`. |
| `crates/vox-mcp-registry/src/subsystem.rs` | Create | `Subsystem` (+ `ALL`, `as_str`, `parse`, `deps`) and `subsystem_closure`. |
| `crates/vox-mcp-registry/src/classification.rs` | Create | `ToolClassification` + `classify_tool_row` — the single validator used by `build.rs` and by `vox ci operations-sync`. |
| `crates/vox-mcp-registry/src/lib.rs` | Modify | Declare/re-export the three modules; new `McpToolRegistryEntry` shape (no `tier`); `registry_entry()`. |
| `crates/vox-mcp-registry/build.rs` | Modify | `#[path]`-include the three modules; parse/validate/emit `exposure`, `side_effect_class`, `requires`; panic on missing/unknown values. |
| `crates/vox-mcp-registry/src/semcov_wave46_tests.rs` | Modify | Drop the two `tier` tests and `VALID_TIERS`; add classification round-trip tests. |
| `crates/vox-mcp-registry/tests/tool_registry_smoke.rs` | Modify | Replace the `tier` assertion with a `registry_entry` assertion. |
| `crates/vox-mcp-registry/tests/exposure_classification.rs` | Create | Registry-wide invariants: spec internal set, exact internal set, populated (non-defaulted) classification, anchor rows. |
| `crates/vox-cli/src/commands/ci/operations_catalog_exposure.rs` | Create | `mcp_classification_errors` / `ensure_mcp_classified` (the operations-sync gate). Kept out of `operations_catalog.rs`, which is already far past the 500-line god-object limit. |
| `crates/vox-cli/src/commands/ci/mod.rs` | Modify | `mod operations_catalog_exposure;` |
| `crates/vox-cli/src/commands/ci/operations_catalog.rs` | Modify | `McpProjection` / `McpCanonicalTool` lose `tier`, gain `exposure` / `requires` (and canonical gains `side_effect_class`); projection both directions; call the gate from `sync("mcp")` and `verify`. |
| `contracts/operations/catalog.v1.schema.json` | Modify | `mcpProjection`: drop `tier`, require `exposure` + `requires`; `operationRow`: `side_effect_class` required non-null enum when `mcp` is an object; bump `x-vox-version` 1 → 2. |
| `contracts/operations/catalog.v1.yaml` | Modify | Task 2 removes 340 `tier: core` lines; Tasks 3–12 add `exposure` / `requires` and set `side_effect_class` for their own ops only. |
| `contracts/mcp/tool-registry.schema.json` | Modify | Drop `tier`; add required `exposure`, `side_effect_class`, `requires`. |
| `contracts/mcp/tool-registry.canonical.yaml` | Regenerate | `cargo run -q -p vox-cli -- ci operations-sync --target mcp --write`. |
| `contracts/capability/capability-registry.schema.json` | Modify | Add `remote_mutation` to the `side_effect_class` enum (the capability projection copies catalog values). |
| `contracts/capability/capability-registry.yaml`, `contracts/capability/model-manifest.generated.json`, `contracts/reports/operations-catalog-inventory.v1.json` | Regenerate | `operations-sync --target all --write`, `capability-sync --write`, `operations-verify`. |
| `crates/vox-orchestrator-mcp/src/server.rs` | Modify | `VOX_MCP_WORKSPACE_TOOLS` opt-in with `VOX_MCP_TIERS` as deprecated alias (marker, warn-once); `workspace_tools_opt_in`, `retain_advertised_tools`; skill meta key `vox_source`. |
| `crates/vox-orchestrator-mcp/src/registry.rs` | Modify | Task 15: stop emitting `vox_tier`; workspace meta key `vox_source`. Task 16: emit `vox_exposure`, `vox_side_effect_class`, `vox_requires`. |
| `crates/vox-orchestrator-mcp/src/lib.rs` | Modify | `pub mod profiles` re-export of the vocabulary (contract: `vox_orchestrator_mcp::profiles::*`). |
| `contracts/config/env-vars.v1.yaml` | Modify | `VOX_MCP_TIERS` description → deprecated alias; new `VOX_MCP_WORKSPACE_TOOLS` entry. |
| `contracts/config/registry.v1.yaml` | Modify | New `mcp_workspace_tools` knob (`VOX_MCP_WORKSPACE_TOOLS`) so `config-hygiene` / `config-registry-parity` accept the new read. |
| `docs/src/reference/env-vars.md` | Modify | Add `VOX_MCP_WORKSPACE_TOOLS` and deprecated `VOX_MCP_TIERS` rows in the MCP section. |
| `docs/src/architecture/mcp-vox-language-exposure.md` | Modify | Replace "MCP tier policy" with "MCP tool visibility". |
| `docs/src/architecture/where-things-live.md` | Modify | `vox-mcp-registry` row and "Add an MCP tool" row mention the exposure metadata. |

## Task Dependency Graph

```text
T1 ──► T2 ──► T3 … T12 (parallel) ──► T13 ──► T14 ──► T16 ──► T17
T15 (parallel with T1–T13) ─────────────────────┘ (T14 and T16 need T15)
```

---

## Classification Procedure (shared by Tasks 3–12)

Every group task applies these rules, in this order, to each tool in its table. The tables are the
**proposed** result of applying them to tool names, descriptions, existing catalog values, and a
first pass over handler code; the implementer confirms each row against the handler before writing it.

**Finding the handler.** `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs` gives the
dispatch arm; open the function it calls (usually in the `*_tools.rs` / `*_tools/` module named in the
group's "Start here" line) and read every code path, including error/fallback paths.

**`side_effect_class` — the strongest effect any code path can have.** Precedence (pick the first that applies):

1. `destructive` — irreversibly deletes or overwrites user data (restore over working tree, delete media, remove plugin, global emergency stop).
2. `remote_mutation` — mutates state in a remote service or another host (publish, submit, push, notify, subscribe, remote gateway call, page interaction that can submit forms).
3. `spawns_process` — starts an OS process or agent worker whose effects the caller does not fully control (cargo build/test, agy, audio capture, mesh dispatch, agent spawn/continue, task submission). A subprocess run **only to read** (e.g. `git status`, `git log`, `git diff`, `git blame` through `git_exec.rs` with a read-only argv) is **not** `spawns_process` — classify by its effect (`read_only`).
4. `writes_files` — any local persistent or in-process state mutation: files under the repo or `.vox/`, local DB rows, orchestrator queues/agent registry/locks, sessions, indexes, caches.
5. `network` — outbound network or LLM inference that does not mutate remote state and does not persist locally.
6. `read_only` — reads files, DB rows, process or orchestrator state; no writes.
7. `none` — pure computation over compiled-in/static data; no I/O at all.

An existing non-null catalog `side_effect_class` is kept unless the handler contradicts it; a contradiction is changed **with evidence** (the tables already mark the known ones).

**`exposure`.**

- **E1 `internal`** — spec §3 initial set: `vox_agy_*`, `vox_openclaw_*`, `*_gateway_call`, `vox_add_approval_allowlist_entry`, `vox_emergency_stop`, `vox_drain_agent`, `vox_secrets_doctor`, `vox_credentials_status`, `vox_gamify_*`; plus this plan's recorded extensions: the native agent-gateway family `vox_agent_list_remote`, `vox_agent_notify`, `vox_agent_subscribe`, `vox_agent_subscriptions`, `vox_agent_unsubscribe` (same surface as `vox_openclaw_*`), and `vox_browser_cookies_export` / `vox_browser_cookies_import` (credential transfer).
- **E2 `operator`** — the operation removes or resets data (`delete`, `remove`, `uninstall`, `reset`, `cleanup`, `restore`), or `side_effect_class` ∈ {`spawns_process`, `remote_mutation`, `destructive`}, or it mutates the workspace/repo, another agent or task, orchestrator configuration, approvals, or the host browser.
- **E3 `public`** — everything else: `none` / `read_only`, `network` reads, and `writes_files` confined to the tool's own subsystem state (active model, the caller's own session/memory/KB/index, chat history, preferences, benchmarks, cost/exec-time records, clarification requests).

**`requires`** — subsystem ids (from `Subsystem`) whose boot step the handler **hard-requires** (it errors or no-ops without it). Evidence: `state.db` / `vox_db::` → `db`; `state.session_manager` → `sessions`; agent spawn/queue/task APIs → `agent_fleet`; model registry / active-model APIs → `model_registry`; `vox_actor_runtime::llm::*` / inference bridge → `llm_facade`; search index / KB / graph search → `search_index`; `MemoryManager` / memory stores → `memory`; Scientia / Socrates / research pipeline → `research`; Populi / mesh / remote A2A → `mesh`; attention / budget manager → `attention`; clarification inbox → `clarification`; flywheel → `flywheel`; compiler / validator / grammar → `compiler`; browser plugin → `browser`; oratio / speech pipeline → `speech`. Orchestrator-core-only state (queues, locks, context store, approvals) needs no entry: the core always boots. List the smallest set — the closure is computed (`research` already implies `llm_facade`, `model_registry`, `search_index`, `db`). Keep the order shown in the tables.

**Mapping table (domain → default `requires`):**

| Domain | Default `requires` |
|---|---|
| `vox_kb_*`, `vox_search_*`, `vox_knowledge_query`, `vox_semantic_fs_discover` | `[search_index]` |
| `vox_memory_*`, `vox_behavior_*`, `vox_preference_*`, `vox_learn_pattern` | `[memory]` |
| `vox_session_*`, `vox_compaction_status`, `vox_map_agent_session` | `[sessions]` |
| `vox_db_*`, `vox_exec_time_*`, `vox_gamify_*`, `vox_chat_history` | `[db]` |
| `vox_mesh_*`, `vox_populi_local_status`, `vox_publish_message` | `[mesh]` |
| task / agent / queue tools, `vox_drain_agent`, `vox_emergency_stop` | `[agent_fleet]` |
| `vox_ask_clarification`, `vox_answer_question`, `vox_pending_questions`, `vox_questioning_*` | `[clarification]` |
| `vox_list_models`, `vox_get_active_model`, `vox_set_active_model`, `vox_set_model`, `vox_suggest_model`, `vox_capability_model_manifest` | `[model_registry]` |
| `vox_benchmark_*` | `[db, model_registry]` |
| `vox_chat_message` | `[llm_facade, db]` |
| `vox_research_*`, `vox_scientia_*`, `vox_news_*`, `vox_schola_submit` | `[research]` |
| `vox_browser_*` | `[browser]` |
| `vox_oratio_*` | `[speech]`; `vox_speech_to_code` → `[speech, llm_facade]` |
| `vox_validate_*`, `vox_check`, `vox_check_workspace`, `vox_compiler::ast_inspect`, `vox_run_tests`, `vox_test_all`, `vox_build_crate`, `vox_lint_crate`, `vox_coverage_report`, grammar/registry introspection | `[compiler]` |
| `vox_attention_*`, `vox_budget_status`, `vox_context_budget`, `vox_cost_history`, `vox_record_cost`, `vox_set_agent_budget` | `[attention]` |
| git / vcs / snapshot / repo-query / skills / plugins / approvals / config / openclaw / agy | `[]` |

**Gate invariants** (enforced by `classify_tool_row`, so the gate rejects a violating row):

- V4 `http_read_role_eligible: true` ⇒ `side_effect_class` ∈ {`none`, `read_only`} and `exposure` ≠ `operator`. Never flip `http_read_role_eligible` to satisfy this — it is governed by `contracts/mcp/http-read-role-governance.yaml`; stop and report instead.
- V5 `exposure: operator` ⇒ `side_effect_class` ∉ {`none`, `read_only`}.
- V6 `exposure: public` ⇒ `side_effect_class` ∉ {`spawns_process`, `remote_mutation`, `destructive`}.

**Anchor rows** pinned by Task 14's tests — do not change them; if the handler contradicts one, stop and report:
`vox_git_status` (public, read_only, []), `vox_list_models` (public, read_only, [model_registry]), `vox_kb_query` (public, read_only, [search_index]), `vox_snapshot_restore` (operator, destructive, [db]), `vox_browser_open` (operator, network, [browser]), `vox_agy_delegate` (internal, spawns_process, []).

**Edit shape.** Each op is located by its unique `mcp.name` line (`rg -n '^    name: <tool>$' contracts/operations/catalog.v1.yaml`). Edit only two places in that op's block: the op-level `side_effect_class:` line, and two new lines appended after `http_read_role_eligible:` in the `mcp:` block. Use serde_yaml's block style so a later `--target catalog` rewrite is a no-op. Example (`kb.query`, after Task 2 removed `tier`):

```yaml
- id: kb.query
  title: Query Knowledge Base
  description: Query knowledge bases with semantic search.
  description_human: null
  product_lane: ai
  intent_tags: []
  side_effect_class: read_only
  scope_kind: session
  reversible: false
  requires_repo: null
  preferred_for_models: null
  human_takeover_friendly: null
  mens_planner_visible: null
  canonical_name: null
  latin_aliases: null
  mcp:
    name: vox_kb_query
    http_read_role_eligible: true
    exposure: public
    requires:
    - search_index
  cli: null
```

An empty requirement list is written `    requires: []`.

**Parallel merge strategy.** Tasks 3–12 run in separate worktrees branched from the Task 2 commit. Each edits **only** the op blocks named in its own table, never regenerates `tool-registry.canonical.yaml`, and never touches another tracked file. Two edited ops are always separated by at least seven unchanged lines (`cli:` … `intent_tags:` of the next op), so git merges the ten branches without conflicts. Merge them into `feat/capability-profiles` one after another with `git merge --no-ff`; if a conflict ever appears, it means a task edited a foreign op — reject that task's branch and redo it.

**Evidence file and commit body (required).** Each group task writes `target/plan01-evidence/task<N>.txt` (`target/` is git-ignored; create the directory with `mkdir -p target/plan01-evidence`). Format:

- one line per tool whose final `side_effect_class` is not `none`/`read_only`, and one per changed preset, each of the form ``- vox_example_tool writes_files: crates/vox-orchestrator-mcp/src/example_tools.rs:123 — inserts a row via state.db``;
- then the line `Overrides: none`, or `Overrides:` followed by one line per row that differs from this plan's table, of the form ``- vox_example_tool: requires [db] -> [] (crates/vox-orchestrator-mcp/src/example_tools.rs:140 — db is optional)``.

The commit passes that file as its last `-m` paragraph, so the evidence lands in history.

---

### Task 1: Exposure, side-effect, and subsystem vocabulary in `vox-mcp-registry`

**Files:**
- Modify: `crates/vox-mcp-registry/Cargo.toml` (whole `[dependencies]` block, add `[dev-dependencies]`)
- Create: `crates/vox-mcp-registry/src/exposure.rs`
- Create: `crates/vox-mcp-registry/src/subsystem.rs`
- Create: `crates/vox-mcp-registry/src/classification.rs`
- Modify: `crates/vox-mcp-registry/src/lib.rs:1-4` (module declarations + re-exports only; the struct changes in Task 14)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub enum Exposure { Public, Operator, Internal }` with `pub const fn as_str(self) -> &'static str`, `pub fn parse(s: &str) -> Option<Self>`
  - `pub enum SideEffectClass { None, ReadOnly, WritesFiles, SpawnsProcess, Network, RemoteMutation, Destructive }` with `as_str` / `parse`
  - `pub enum Subsystem { Db, Sessions, AgentFleet, ModelRegistry, LlmFacade, SearchIndex, Memory, Research, Mesh, Attention, Clarification, Flywheel, Compiler, Browser, Speech }` with `pub const ALL: &'static [Subsystem]`, `as_str`, `parse`, `pub const fn deps(self) -> &'static [Subsystem]`
  - `pub fn subsystem_closure(roots: &[Subsystem]) -> std::collections::BTreeSet<Subsystem>`
  - `pub struct ToolClassification { pub exposure: Exposure, pub side_effect_class: SideEffectClass, pub requires: Vec<Subsystem> }`
  - `pub fn classify_tool_row(http_read_role_eligible: bool, exposure: Option<&str>, side_effect_class: Option<&str>, requires: Option<&[String]>) -> Result<ToolClassification, Vec<String>>`

Parallel-safe with: Task 15.

- [ ] **Step 1: Add dependencies**

Replace the `[dependencies]` block of `crates/vox-mcp-registry/Cargo.toml` and add a dev-dependency block (leave `[build-dependencies]` and `[lints]` as they are):

```toml
[dependencies]
serde = { workspace = true }
workspace-hack = { version = "0.6", path = "../workspace-hack" }

[dev-dependencies]
serde_json = { workspace = true }
```

- [ ] **Step 2: Write the failing tests (modules with tests only)**

Create `crates/vox-mcp-registry/src/exposure.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const EXPOSURES: [Exposure; 3] = [Exposure::Public, Exposure::Operator, Exposure::Internal];
    const SIDE_EFFECTS: [SideEffectClass; 7] = [
        SideEffectClass::None,
        SideEffectClass::ReadOnly,
        SideEffectClass::WritesFiles,
        SideEffectClass::SpawnsProcess,
        SideEffectClass::Network,
        SideEffectClass::RemoteMutation,
        SideEffectClass::Destructive,
    ];

    #[test]
    fn exposure_round_trips_through_str_and_serde() {
        for e in EXPOSURES {
            assert_eq!(Exposure::parse(e.as_str()), Some(e));
            assert_eq!(
                serde_json::to_string(&e).unwrap(),
                format!("\"{}\"", e.as_str())
            );
        }
    }

    #[test]
    fn exposure_orders_public_operator_internal() {
        assert!(Exposure::Public < Exposure::Operator);
        assert!(Exposure::Operator < Exposure::Internal);
    }

    #[test]
    fn exposure_parse_rejects_unknown_and_non_canonical() {
        for s in ["", "Public", "core", "admin", "dev"] {
            assert_eq!(Exposure::parse(s), None, "{s}");
        }
    }

    #[test]
    fn side_effect_round_trips_through_str_and_serde() {
        for c in SIDE_EFFECTS {
            assert_eq!(SideEffectClass::parse(c.as_str()), Some(c));
            assert_eq!(
                serde_json::to_string(&c).unwrap(),
                format!("\"{}\"", c.as_str())
            );
        }
    }

    #[test]
    fn side_effect_strings_are_the_catalog_vocabulary() {
        let got: Vec<&str> = SIDE_EFFECTS.iter().map(|c| c.as_str()).collect();
        assert_eq!(
            got,
            [
                "none",
                "read_only",
                "writes_files",
                "spawns_process",
                "network",
                "remote_mutation",
                "destructive"
            ]
        );
        for s in ["mixed", "unknown", "ReadOnly", "read-only"] {
            assert_eq!(SideEffectClass::parse(s), None, "{s}");
        }
    }
}
```

Create `crates/vox-mcp-registry/src/subsystem.rs` containing only its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn all_lists_every_variant_once_in_ord_order() {
        assert_eq!(Subsystem::ALL.len(), 15);
        let mut sorted = Subsystem::ALL.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.as_slice(), Subsystem::ALL);
    }

    #[test]
    fn as_str_round_trips_and_matches_serde() {
        for &s in Subsystem::ALL {
            assert_eq!(Subsystem::parse(s.as_str()), Some(s));
            assert_eq!(
                serde_json::to_string(&s).unwrap(),
                format!("\"{}\"", s.as_str())
            );
        }
    }

    #[test]
    fn parse_rejects_non_canonical_spellings() {
        for bad in ["", "AgentFleet", "agent-fleet", "orchestrator", "skills"] {
            assert_eq!(Subsystem::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn deps_match_contract_table() {
        use Subsystem::*;
        assert_eq!(Sessions.deps(), &[Db]);
        assert_eq!(AgentFleet.deps(), &[Sessions]);
        assert_eq!(LlmFacade.deps(), &[ModelRegistry]);
        assert_eq!(SearchIndex.deps(), &[Db]);
        assert_eq!(Memory.deps(), &[Db, SearchIndex]);
        assert_eq!(Research.deps(), &[LlmFacade, SearchIndex, Db]);
        for s in [Mesh, Clarification, Flywheel] {
            assert_eq!(s.deps(), &[Db], "{s:?}");
        }
        for s in [Db, ModelRegistry, Attention, Compiler, Browser, Speech] {
            assert!(s.deps().is_empty(), "{s:?}");
        }
    }

    #[test]
    fn closure_includes_roots_and_transitive_deps() {
        use Subsystem::*;
        assert_eq!(
            subsystem_closure(&[AgentFleet]),
            BTreeSet::from([AgentFleet, Sessions, Db])
        );
        assert_eq!(
            subsystem_closure(&[Research]),
            BTreeSet::from([Research, LlmFacade, ModelRegistry, SearchIndex, Db])
        );
        assert_eq!(
            subsystem_closure(&[ModelRegistry, Compiler]),
            BTreeSet::from([ModelRegistry, Compiler])
        );
        assert!(subsystem_closure(&[]).is_empty());
    }

    #[test]
    fn closure_of_all_is_all_and_closure_is_idempotent() {
        assert_eq!(subsystem_closure(Subsystem::ALL).len(), Subsystem::ALL.len());
        for &s in Subsystem::ALL {
            let once = subsystem_closure(&[s]);
            let roots: Vec<Subsystem> = once.iter().copied().collect();
            assert_eq!(subsystem_closure(&roots), once, "{s:?}");
        }
    }

    #[test]
    fn dependency_graph_is_acyclic() {
        for &s in Subsystem::ALL {
            assert!(
                !subsystem_closure(s.deps()).contains(&s),
                "{s:?} transitively depends on itself"
            );
        }
    }
}
```

Create `crates/vox-mcp-registry/src/classification.rs` containing only its test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const NO_REQS: &[String] = &[];

    fn reqs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn valid_row_parses_in_declared_order() {
        let r = reqs(&["search_index", "db"]);
        let c = classify_tool_row(true, Some("public"), Some("read_only"), Some(r.as_slice()))
            .expect("valid row");
        assert_eq!(
            c,
            ToolClassification {
                exposure: Exposure::Public,
                side_effect_class: SideEffectClass::ReadOnly,
                requires: vec![Subsystem::SearchIndex, Subsystem::Db],
            }
        );
    }

    #[test]
    fn empty_requires_is_valid() {
        let c = classify_tool_row(false, Some("operator"), Some("writes_files"), Some(NO_REQS))
            .expect("valid row");
        assert!(c.requires.is_empty());
    }

    #[test]
    fn every_missing_field_is_reported() {
        let errs = classify_tool_row(false, None, None, None).unwrap_err();
        assert_eq!(errs.len(), 3, "{errs:?}");
        assert!(errs[0].contains("missing `exposure`"), "{errs:?}");
        assert!(errs[1].contains("missing `side_effect_class`"), "{errs:?}");
        assert!(errs[2].contains("missing `requires`"), "{errs:?}");
    }

    #[test]
    fn unknown_values_and_duplicates_are_reported() {
        let r = reqs(&["db", "orchestrator", "db"]);
        let errs =
            classify_tool_row(false, Some("core"), Some("mixed"), Some(r.as_slice())).unwrap_err();
        assert_eq!(
            errs,
            vec![
                "unknown exposure `core`".to_string(),
                "unknown side_effect_class `mixed`".to_string(),
                "unknown subsystem `orchestrator` in requires".to_string(),
                "duplicate subsystem `db` in requires".to_string(),
            ]
        );
    }

    #[test]
    fn read_role_tools_must_be_side_effect_free_and_not_operator() {
        assert!(classify_tool_row(true, Some("public"), Some("writes_files"), Some(NO_REQS)).is_err());
        assert!(classify_tool_row(true, Some("internal"), Some("read_only"), Some(NO_REQS)).is_ok());
        assert!(classify_tool_row(true, Some("public"), Some("none"), Some(NO_REQS)).is_ok());
    }

    #[test]
    fn operator_requires_a_mutating_side_effect() {
        let errs =
            classify_tool_row(false, Some("operator"), Some("read_only"), Some(NO_REQS)).unwrap_err();
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].starts_with("operator exposure requires"), "{errs:?}");
    }

    #[test]
    fn public_cannot_spawn_mutate_remote_or_destroy() {
        for c in ["spawns_process", "remote_mutation", "destructive"] {
            assert!(
                classify_tool_row(false, Some("public"), Some(c), Some(NO_REQS)).is_err(),
                "{c}"
            );
        }
        for c in ["none", "read_only", "writes_files", "network"] {
            assert!(
                classify_tool_row(false, Some("public"), Some(c), Some(NO_REQS)).is_ok(),
                "{c}"
            );
        }
    }
}
```

Add the module declarations to `crates/vox-mcp-registry/src/lib.rs`, directly after the three `//!` doc lines (lines 1–3) and before the `/// One MCP tool row` comment:

```rust
mod classification;
mod exposure;
mod subsystem;

pub use classification::{ToolClassification, classify_tool_row};
pub use exposure::{Exposure, SideEffectClass};
pub use subsystem::{Subsystem, subsystem_closure};
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo nextest run -p vox-mcp-registry exposure subsystem classification`
Expected: compile failure — `error[E0432]: unresolved imports classification::ToolClassification …` / `cannot find type Exposure in this scope`.

- [ ] **Step 4: Implement `exposure.rs`**

Prepend to `crates/vox-mcp-registry/src/exposure.rs` (above the test module):

```rust
//! Exposure class and side-effect class of an MCP tool (capability profiles).

use serde::{Deserialize, Serialize};

/// Who may be served a tool. Ordered `Public < Operator < Internal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Exposure {
    /// Read-only, or writes only the owning subsystem's own state.
    Public,
    /// Mutates the workspace, repo, remote services, or other agents.
    Operator,
    /// Never exposed through a named profile; loopback `dev` only.
    Internal,
}

impl Exposure {
    /// Catalog / wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Exposure::Public => "public",
            Exposure::Operator => "operator",
            Exposure::Internal => "internal",
        }
    }

    /// Inverse of [`Self::as_str`]; `None` for any other spelling.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "public" => Some(Exposure::Public),
            "operator" => Some(Exposure::Operator),
            "internal" => Some(Exposure::Internal),
            _ => None,
        }
    }
}

/// Strongest effect any code path of a tool handler can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    /// Pure computation over compiled-in data.
    None,
    /// Reads files, rows, or process state.
    ReadOnly,
    /// Local persistent or in-process state mutation (files, DB rows, queues, sessions).
    WritesFiles,
    /// Starts an OS process or agent worker.
    SpawnsProcess,
    /// Outbound network or inference without remote mutation.
    Network,
    /// Mutates state in a remote service or another host.
    RemoteMutation,
    /// Irreversibly deletes or overwrites user data.
    Destructive,
}

impl SideEffectClass {
    /// Catalog / wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            SideEffectClass::None => "none",
            SideEffectClass::ReadOnly => "read_only",
            SideEffectClass::WritesFiles => "writes_files",
            SideEffectClass::SpawnsProcess => "spawns_process",
            SideEffectClass::Network => "network",
            SideEffectClass::RemoteMutation => "remote_mutation",
            SideEffectClass::Destructive => "destructive",
        }
    }

    /// Inverse of [`Self::as_str`]; `None` for any other spelling.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "none" => Some(SideEffectClass::None),
            "read_only" => Some(SideEffectClass::ReadOnly),
            "writes_files" => Some(SideEffectClass::WritesFiles),
            "spawns_process" => Some(SideEffectClass::SpawnsProcess),
            "network" => Some(SideEffectClass::Network),
            "remote_mutation" => Some(SideEffectClass::RemoteMutation),
            "destructive" => Some(SideEffectClass::Destructive),
            _ => None,
        }
    }
}
```

- [ ] **Step 5: Implement `subsystem.rs`**

Prepend to `crates/vox-mcp-registry/src/subsystem.rs`:

```rust
//! Bootable daemon subsystems and their dependency graph (capability profiles).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// A daemon boot step a tool can require.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Subsystem {
    Db,
    Sessions,
    AgentFleet,
    ModelRegistry,
    LlmFacade,
    SearchIndex,
    Memory,
    Research,
    Mesh,
    Attention,
    Clarification,
    Flywheel,
    Compiler,
    Browser,
    Speech,
}

impl Subsystem {
    /// Every variant, in `Ord` order.
    pub const ALL: &'static [Subsystem] = &[
        Subsystem::Db,
        Subsystem::Sessions,
        Subsystem::AgentFleet,
        Subsystem::ModelRegistry,
        Subsystem::LlmFacade,
        Subsystem::SearchIndex,
        Subsystem::Memory,
        Subsystem::Research,
        Subsystem::Mesh,
        Subsystem::Attention,
        Subsystem::Clarification,
        Subsystem::Flywheel,
        Subsystem::Compiler,
        Subsystem::Browser,
        Subsystem::Speech,
    ];

    /// Catalog / wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Subsystem::Db => "db",
            Subsystem::Sessions => "sessions",
            Subsystem::AgentFleet => "agent_fleet",
            Subsystem::ModelRegistry => "model_registry",
            Subsystem::LlmFacade => "llm_facade",
            Subsystem::SearchIndex => "search_index",
            Subsystem::Memory => "memory",
            Subsystem::Research => "research",
            Subsystem::Mesh => "mesh",
            Subsystem::Attention => "attention",
            Subsystem::Clarification => "clarification",
            Subsystem::Flywheel => "flywheel",
            Subsystem::Compiler => "compiler",
            Subsystem::Browser => "browser",
            Subsystem::Speech => "speech",
        }
    }

    /// Inverse of [`Self::as_str`]; `None` for any other spelling.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|x| x.as_str() == s)
    }

    /// Direct dependencies (must boot first).
    pub const fn deps(self) -> &'static [Subsystem] {
        use Subsystem::*;
        match self {
            Sessions => &[Db],
            AgentFleet => &[Sessions],
            LlmFacade => &[ModelRegistry],
            SearchIndex => &[Db],
            Memory => &[Db, SearchIndex],
            Research => &[LlmFacade, SearchIndex, Db],
            Mesh | Clarification | Flywheel => &[Db],
            Db | ModelRegistry | Attention | Compiler | Browser | Speech => &[],
        }
    }
}

/// Transitive closure including the roots, deterministic order.
pub fn subsystem_closure(roots: &[Subsystem]) -> BTreeSet<Subsystem> {
    let mut out = BTreeSet::new();
    let mut stack: Vec<Subsystem> = roots.to_vec();
    while let Some(s) = stack.pop() {
        if out.insert(s) {
            stack.extend_from_slice(s.deps());
        }
    }
    out
}
```

- [ ] **Step 6: Implement `classification.rs`**

Prepend to `crates/vox-mcp-registry/src/classification.rs`:

```rust
//! Validation of one MCP tool's exposure metadata. Compiled into both the library and
//! `build.rs` (via `#[path]`), and called by `vox ci operations-sync`, so every gate applies
//! the same rules.

use crate::exposure::{Exposure, SideEffectClass};
use crate::subsystem::Subsystem;

/// Parsed, validated exposure metadata of one tool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolClassification {
    pub exposure: Exposure,
    pub side_effect_class: SideEffectClass,
    pub requires: Vec<Subsystem>,
}

/// Parse and validate one registry/catalog row. Returns every problem, not just the first.
pub fn classify_tool_row(
    http_read_role_eligible: bool,
    exposure: Option<&str>,
    side_effect_class: Option<&str>,
    requires: Option<&[String]>,
) -> Result<ToolClassification, Vec<String>> {
    let mut errors = Vec::new();

    let exposure = match exposure {
        None => {
            errors.push("missing `exposure` (public|operator|internal)".to_string());
            None
        }
        Some(s) => {
            let parsed = Exposure::parse(s);
            if parsed.is_none() {
                errors.push(format!("unknown exposure `{s}`"));
            }
            parsed
        }
    };

    let side_effect = match side_effect_class {
        None => {
            errors.push(
                "missing `side_effect_class` (none|read_only|writes_files|spawns_process|network|remote_mutation|destructive)"
                    .to_string(),
            );
            None
        }
        Some(s) => {
            let parsed = SideEffectClass::parse(s);
            if parsed.is_none() {
                errors.push(format!("unknown side_effect_class `{s}`"));
            }
            parsed
        }
    };

    let mut subsystems = Vec::new();
    match requires {
        None => errors.push("missing `requires` (use [] when none)".to_string()),
        Some(list) => {
            for s in list {
                match Subsystem::parse(s) {
                    None => errors.push(format!("unknown subsystem `{s}` in requires")),
                    Some(x) if subsystems.contains(&x) => {
                        errors.push(format!("duplicate subsystem `{s}` in requires"));
                    }
                    Some(x) => subsystems.push(x),
                }
            }
        }
    }

    if let (Some(e), Some(c)) = (exposure, side_effect) {
        let side_effect_free = matches!(c, SideEffectClass::None | SideEffectClass::ReadOnly);
        if http_read_role_eligible && (!side_effect_free || e == Exposure::Operator) {
            errors.push(format!(
                "http_read_role_eligible tool must be side-effect free and not operator (got exposure `{}`, side_effect_class `{}`)",
                e.as_str(),
                c.as_str()
            ));
        }
        if e == Exposure::Operator && side_effect_free {
            errors.push(format!(
                "operator exposure requires a mutating side_effect_class (got `{}`)",
                c.as_str()
            ));
        }
        if e == Exposure::Public
            && matches!(
                c,
                SideEffectClass::SpawnsProcess
                    | SideEffectClass::RemoteMutation
                    | SideEffectClass::Destructive
            )
        {
            errors.push(format!(
                "public exposure cannot have side_effect_class `{}`",
                c.as_str()
            ));
        }
    }

    match (exposure, side_effect) {
        (Some(exposure), Some(side_effect_class)) if errors.is_empty() => Ok(ToolClassification {
            exposure,
            side_effect_class,
            requires: subsystems,
        }),
        _ => Err(errors),
    }
}
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo nextest run -p vox-mcp-registry exposure subsystem classification`
Expected: all 19 new tests PASS (5 exposure, 7 subsystem, 7 classification); existing tests still pass.

- [ ] **Step 8: Lint, format, commit**

```bash
cargo check -p vox-mcp-registry
cargo clippy -p vox-mcp-registry --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-mcp-registry/Cargo.toml crates/vox-mcp-registry/src/exposure.rs crates/vox-mcp-registry/src/subsystem.rs crates/vox-mcp-registry/src/classification.rs crates/vox-mcp-registry/src/lib.rs Cargo.lock
git commit -m "feat(mcp-registry): add Exposure, SideEffectClass, Subsystem vocabulary" -m "Capability profiles plan 01 Task 1. Adds the exposure/side-effect/subsystem enums, the subsystem dependency closure, and classify_tool_row, the single validator later shared by build.rs and vox ci operations-sync."
```

If `cargo hakari verify` (workspace-hack) reports drift after the new `serde` dependency, run `cargo hakari generate && cargo hakari manage-deps`, include `crates/workspace-hack/` in the same commit, and mention it in the commit body.

---

### Task 2: Catalog plumbing and the operations-sync gate

**Files:**
- Create: `crates/vox-cli/src/commands/ci/operations_catalog_exposure.rs`
- Modify: `crates/vox-cli/src/commands/ci/mod.rs:11` (add a sibling `mod` line)
- Modify: `crates/vox-cli/src/commands/ci/operations_catalog.rs` — imports (L10–19), `McpProjection` + `default_mcp_tier` (L145–156), `McpCanonicalTool` (L182–191), `build_catalog_from_live_registries` MCP assignment (L322–326), `verify` (after the `verify_catalog_nomenclature(&catalog)?;` call, ≈L566), `project_mcp_registry` (L717–734), `sync` `"mcp"` arm (L941–950), test module (append)
- Modify: `contracts/operations/catalog.v1.schema.json` (L2, L63–76, L99–132)
- Modify: `contracts/operations/catalog.v1.yaml` (remove all 340 `    tier: core` lines)

**Interfaces:**
- Consumes: `vox_mcp_registry::classify_tool_row` (Task 1).
- Produces:
  - `pub struct McpProjection { pub name: String, pub http_read_role_eligible: bool, pub exposure: Option<String>, pub requires: Option<Vec<String>> }`
  - `pub(crate) fn mcp_classification_errors(catalog: &OperationsCatalog) -> Vec<String>` — lines of the form ``op `<id>` (mcp `<tool>`): <problem>``
  - `pub(crate) fn ensure_mcp_classified(catalog: &OperationsCatalog) -> anyhow::Result<()>`
  - Behaviour: `vox ci operations-sync --target mcp|all` and `vox ci operations-verify` fail while any MCP op is unclassified.

Parallel-safe with: Task 15. Depends on: Task 1.

- [ ] **Step 1: Write the failing gate tests**

Create `crates/vox-cli/src/commands/ci/operations_catalog_exposure.rs` with only the test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ci::operations_catalog::{CliProjection, McpProjection, OperationRow};

    fn mcp(
        name: &str,
        read: bool,
        exposure: Option<&str>,
        requires: Option<&[&str]>,
    ) -> Option<McpProjection> {
        Some(McpProjection {
            name: name.to_string(),
            http_read_role_eligible: read,
            exposure: exposure.map(str::to_string),
            requires: requires.map(|r| r.iter().map(|s| s.to_string()).collect()),
        })
    }

    fn row(id: &str, side_effect_class: Option<&str>, mcp: Option<McpProjection>) -> OperationRow {
        OperationRow {
            id: id.to_string(),
            title: String::new(),
            description: String::new(),
            description_human: None,
            product_lane: "ai".to_string(),
            intent_tags: Vec::new(),
            side_effect_class: side_effect_class.map(str::to_string),
            scope_kind: None,
            reversible: None,
            requires_repo: None,
            preferred_for_models: None,
            human_takeover_friendly: None,
            mens_planner_visible: None,
            canonical_name: None,
            latin_aliases: None,
            cli: mcp.is_none().then(|| CliProjection {
                path: vec![id.to_string()],
                status: "active".to_string(),
                latin_ns: None,
                handler_rust: None,
                feature_gate: None,
                catalog_group: None,
                ref_cli_required: None,
                reachability_required: None,
            }),
            mcp,
        }
    }

    fn catalog(operations: Vec<OperationRow>) -> OperationsCatalog {
        OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            operations,
            exemptions: Default::default(),
        }
    }

    #[test]
    fn classification_gate_lists_every_missing_field() {
        let cat = catalog(vec![row(
            "x.unclassified",
            None,
            mcp("vox_x_unclassified", false, None, None),
        )]);
        let errs = mcp_classification_errors(&cat);
        assert_eq!(errs.len(), 3, "{errs:?}");
        assert!(
            errs.iter()
                .all(|e| e.starts_with("op `x.unclassified` (mcp `vox_x_unclassified`): ")),
            "{errs:?}"
        );
        assert!(errs.iter().any(|e| e.contains("missing `exposure`")));
        assert!(errs.iter().any(|e| e.contains("missing `side_effect_class`")));
        assert!(errs.iter().any(|e| e.contains("missing `requires`")));
    }

    #[test]
    fn classification_gate_rejects_unknown_values() {
        let cat = catalog(vec![row(
            "x.bad",
            Some("mixed"),
            mcp("vox_x_bad", false, Some("core"), Some(&["orchestrator"])),
        )]);
        assert_eq!(mcp_classification_errors(&cat).len(), 3);
    }

    #[test]
    fn classification_gate_enforces_read_role_invariant() {
        let cat = catalog(vec![row(
            "x.leaky",
            Some("writes_files"),
            mcp("vox_x_leaky", true, Some("public"), Some(&[])),
        )]);
        let errs = mcp_classification_errors(&cat);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].contains("http_read_role_eligible"), "{errs:?}");
    }

    #[test]
    fn classification_gate_ignores_cli_only_rows_and_accepts_classified_rows() {
        let cat = catalog(vec![
            row("cli.only", None, None),
            row(
                "x.ok",
                Some("read_only"),
                mcp("vox_x_ok", true, Some("public"), Some(&["db"])),
            ),
            row(
                "x.op",
                Some("writes_files"),
                mcp("vox_x_op", false, Some("operator"), Some(&[])),
            ),
        ]);
        assert!(mcp_classification_errors(&cat).is_empty());
        assert!(ensure_mcp_classified(&cat).is_ok());
    }

    #[test]
    fn ensure_reports_problem_count_and_catalog_path() {
        let cat = catalog(vec![row(
            "x.half",
            None,
            mcp("vox_x_half", false, Some("public"), None),
        )]);
        let err = ensure_mcp_classified(&cat).unwrap_err().to_string();
        assert!(
            err.starts_with(
                "2 MCP exposure-metadata problem(s) in contracts/operations/catalog.v1.yaml"
            ),
            "{err}"
        );
        assert!(err.contains("op `x.half` (mcp `vox_x_half`): missing `side_effect_class`"));
        assert!(err.contains("op `x.half` (mcp `vox_x_half`): missing `requires`"));
    }
}
```

Add after `mod operations_catalog;` (line 11) of `crates/vox-cli/src/commands/ci/mod.rs`:

```rust
mod operations_catalog_exposure;
```

Append this test to the existing `#[cfg(test)] mod tests` in `crates/vox-cli/src/commands/ci/operations_catalog.rs` (it reuses the module's `create_row` helper):

```rust
    #[test]
    fn mcp_projection_emits_exposure_metadata_and_no_tier() {
        let mut row = create_row("x.ok", None, None, "active");
        row.cli = None;
        row.side_effect_class = Some("read_only".to_string());
        row.mcp = Some(McpProjection {
            name: "vox_x_ok".to_string(),
            http_read_role_eligible: false,
            exposure: Some("public".to_string()),
            requires: Some(vec!["db".to_string(), "search_index".to_string()]),
        });
        let cat = OperationsCatalog {
            schema_version: 1,
            capability: Default::default(),
            exemptions: Default::default(),
            operations: vec![row],
        };
        let text = mcp_registry_yaml_text(&cat).expect("serialize");
        assert!(text.contains("  exposure: public\n"), "{text}");
        assert!(text.contains("  side_effect_class: read_only\n"), "{text}");
        assert!(text.contains("  requires:\n  - db\n  - search_index\n"), "{text}");
        assert!(!text.contains("tier"), "{text}");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo nextest run -p vox-cli classification_gate ensure_reports mcp_projection_emits`
Expected: compile failure — `cannot find function mcp_classification_errors in this scope`, `struct McpProjection has no field named exposure`.

- [ ] **Step 3: Implement the gate**

Prepend to `crates/vox-cli/src/commands/ci/operations_catalog_exposure.rs` (above the test module):

```rust
//! CI gate: every MCP operation in the operations catalog carries capability-profile exposure
//! metadata (`mcp.exposure`, `mcp.requires`, non-null `side_effect_class`).

use anyhow::{Result, anyhow};

use super::operations_catalog::{OPERATIONS_CATALOG_REL, OperationsCatalog};

/// One line per problem, prefixed with the op id and MCP tool name, so one run lists every gap.
pub(crate) fn mcp_classification_errors(catalog: &OperationsCatalog) -> Vec<String> {
    let mut out = Vec::new();
    for row in &catalog.operations {
        let Some(m) = &row.mcp else {
            continue;
        };
        if let Err(errs) = vox_mcp_registry::classify_tool_row(
            m.http_read_role_eligible,
            m.exposure.as_deref(),
            row.side_effect_class.as_deref(),
            m.requires.as_deref(),
        ) {
            for e in errs {
                out.push(format!("op `{}` (mcp `{}`): {e}", row.id, m.name));
            }
        }
    }
    out
}

/// Fails with every classification problem when any MCP op is unclassified or invalid.
pub(crate) fn ensure_mcp_classified(catalog: &OperationsCatalog) -> Result<()> {
    let errors = mcp_classification_errors(catalog);
    if errors.is_empty() {
        return Ok(());
    }
    Err(anyhow!(
        "{} MCP exposure-metadata problem(s) in {OPERATIONS_CATALOG_REL} (each `mcp:` op needs `mcp.exposure`, `mcp.requires`, and a non-null `side_effect_class`; vocabulary: vox_mcp_registry::{{Exposure, SideEffectClass, Subsystem}}):\n{}",
        errors.len(),
        errors.join("\n")
    ))
}
```

- [ ] **Step 4: Rewire `operations_catalog.rs`**

Add to the imports (after the `use crate::command_registry_model::…` line):

```rust
use super::operations_catalog_exposure::ensure_mcp_classified;
```

Replace `McpProjection` and `default_mcp_tier` (L145–156) with:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpProjection {
    pub name: String,
    #[serde(default)]
    pub http_read_role_eligible: bool,
    /// `public | operator | internal`; required for every MCP op (capability profiles).
    #[serde(default)]
    pub exposure: Option<String>,
    /// Subsystem ids the tool needs booted; `[]` when none. Required for every MCP op.
    #[serde(default)]
    pub requires: Option<Vec<String>>,
}
```

Replace `McpCanonicalTool` (L182–191) with:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
struct McpCanonicalTool {
    name: String,
    description: String,
    product_lane: String,
    #[serde(default)]
    http_read_role_eligible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    exposure: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    side_effect_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    requires: Option<Vec<String>>,
}
```

In `build_catalog_from_live_registries`, replace the `row.mcp = Some(McpProjection { … tier: t.tier, });` statement (L322–326) with:

```rust
        if row.side_effect_class.is_none() {
            row.side_effect_class = t.side_effect_class.clone();
        }
        row.mcp = Some(McpProjection {
            name: t.name,
            http_read_role_eligible: t.http_read_role_eligible,
            exposure: t.exposure,
            requires: t.requires,
        });
```

In `verify`, immediately after `verify_catalog_nomenclature(&catalog)?;`, add:

```rust
    ensure_mcp_classified(&catalog)?;
```

Replace the body of `project_mcp_registry`'s `filter_map` closure (L723–729) with:

```rust
            let m = row.mcp.as_ref()?;
            Some(McpCanonicalTool {
                name: m.name.clone(),
                description: row.description.clone(),
                product_lane: row.product_lane.clone(),
                http_read_role_eligible: m.http_read_role_eligible,
                exposure: m.exposure.clone(),
                side_effect_class: row.side_effect_class.clone(),
                requires: m.requires.clone(),
            })
```

Replace the `"mcp"` arm of `sync` (L941–950) with:

```rust
        "mcp" => {
            let catalog = read_catalog(repo_root)?;
            ensure_mcp_classified(&catalog)?;
            let text = mcp_registry_yaml_text(&catalog)?;
            let path = repo_root.join(MCP_TOOL_REGISTRY_REL);
            write_or_verify_text(
                &path,
                &text,
                write,
                "run `vox ci operations-sync --target mcp --write`",
            )
        }
```

- [ ] **Step 5: Remove `tier` from the catalog**

Run: `rg -c '^    tier: core$' contracts/operations/catalog.v1.yaml` — Expected: `340`.
Delete every line that is exactly `    tier: core` (4-space indent; it only occurs inside `mcp:` blocks) with your editor's replace-all on the full line including its newline.
Run again — Expected: no output (rg exit 1). Run `rg -n '^\s+tier:' contracts/operations/catalog.v1.yaml` — Expected: no output.

- [ ] **Step 6: Update the catalog JSON Schema**

In `contracts/operations/catalog.v1.schema.json` change line 2 to `"x-vox-version": 2,` and replace the `mcpProjection` definition (L63–76) with:

```json
    "mcpProjection": {
      "type": "object",
      "required": ["name", "exposure", "requires"],
      "properties": {
        "name": { "type": "string" },
        "http_read_role_eligible": { "type": "boolean", "default": false },
        "exposure": {
          "type": "string",
          "enum": ["public", "operator", "internal"],
          "description": "Capability-profile exposure class (vox_mcp_registry::Exposure)."
        },
        "requires": {
          "type": "array",
          "uniqueItems": true,
          "items": {
            "type": "string",
            "enum": ["db", "sessions", "agent_fleet", "model_registry", "llm_facade", "search_index", "memory", "research", "mesh", "attention", "clarification", "flywheel", "compiler", "browser", "speech"]
          },
          "description": "Subsystems the tool needs booted (vox_mcp_registry::Subsystem); [] when none."
        }
      },
      "additionalProperties": false
    },
```

In `operationRow`, insert these two keys between `"required": [...]` and `"properties"`:

```json
      "if": { "required": ["mcp"], "properties": { "mcp": { "type": "object" } } },
      "then": {
        "required": ["side_effect_class"],
        "properties": {
          "side_effect_class": {
            "type": "string",
            "enum": ["none", "read_only", "writes_files", "spawns_process", "network", "remote_mutation", "destructive"]
          }
        }
      },
```

- [ ] **Step 7: Run the tests and the gate**

Run: `cargo nextest run -p vox-cli classification_gate ensure_reports mcp_projection_emits`
Expected: 6 PASS.

Run: `cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | head -1`
Expected: the error begins `988 MCP exposure-metadata problem(s) in contracts/operations/catalog.v1.yaml` (340 missing `exposure` + 340 missing `requires` + 308 null `side_effect_class`). This red state is intended until Task 13.

Run: `cargo run -q -p vox-cli -- ci operations-sync --target cli && cargo run -q -p vox-cli -- ci operations-sync --target capability`
Expected: both print `OK …` (those projections are unaffected).

- [ ] **Step 8: Lint, format, commit**

```bash
cargo check -p vox-cli
cargo clippy -p vox-cli --all-targets -- -D warnings
vox run scripts/fmt.vox
git add crates/vox-cli/src/commands/ci/operations_catalog_exposure.rs crates/vox-cli/src/commands/ci/mod.rs crates/vox-cli/src/commands/ci/operations_catalog.rs contracts/operations/catalog.v1.schema.json contracts/operations/catalog.v1.yaml
git commit -m "feat(ci): gate operations-sync on MCP exposure metadata; drop tier" -m "Capability profiles plan 01 Task 2. McpProjection gains exposure/requires, the canonical projection carries exposure/side_effect_class/requires, and operations-sync --target mcp plus operations-verify fail with every unclassified op listed. The unused mcp.tier is removed from catalog and schema. The catalog is intentionally unclassified until Tasks 3-12 land."
```

---

### Task 3: Classify research / scientia / news ops (41)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task3.txt`

**Interfaces:**
- Consumes: the gate from Task 2 (`operations-sync --target mcp` error lines ``op `<id>` (mcp `<tool>`): …``).
- Produces: 41 classified ops.

Parallel-safe with: Tasks 4, 5, 6, 7, 8, 9, 10, 11, 12, 15 (each edits only its own ops). Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/scientia_tools/`, `news_tools.rs`, `memory_tools/` (research), `dispatch.rs`. `vox_news_*` and `vox_scientia_*` are only dispatchable with the `news-publish` feature (`registry.rs::dispatchable_under_features`) — classify them from the feature-gated handlers anyway.

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_news_approval_status` | public | read_only | [research] |
| `vox_news_approve` | operator | writes_files | [research] |
| `vox_news_draft_research` | public | writes_files | [research] |
| `vox_news_simulate_publish_gate` | public | read_only | [research] |
| `vox_news_test_syndicate` | operator | network | [research] |
| `vox_research_get` | public | read_only | [research] |
| `vox_research_run` | public | writes_files | [research] |
| `vox_research_search` | public | network | [research] |
| `vox_research_start` | public | writes_files | [research] |
| `vox_research_status` | public | read_only | [research] |
| `vox_schola_submit` | operator | remote_mutation | [research] |
| `vox_scientia_assist_suggestions` | public | network | [research] |
| `vox_scientia_publication_approve` | operator | writes_files | [research] |
| `vox_scientia_publication_arxiv_handoff_record` | operator | writes_files | [research] |
| `vox_scientia_publication_decision_explain` | public | read_only | [research] |
| `vox_scientia_publication_discovery_explain` | public | read_only | [research] |
| `vox_scientia_publication_discovery_refresh_evidence` | public | writes_files | [research] |
| `vox_scientia_publication_discovery_scan` | public | writes_files | [research] |
| `vox_scientia_publication_external_jobs_dead_letter` | operator | writes_files | [research] |
| `vox_scientia_publication_external_jobs_due` | public | read_only | [research] |
| `vox_scientia_publication_external_jobs_replay` | operator | remote_mutation | [research] |
| `vox_scientia_publication_external_jobs_tick` | operator | remote_mutation | [research] |
| `vox_scientia_publication_external_pipeline_metrics` | public | read_only | [research] |
| `vox_scientia_publication_media_delete` | operator | destructive | [research] |
| `vox_scientia_publication_media_list` | public | read_only | [research] |
| `vox_scientia_publication_media_upsert` | operator | writes_files | [research] |
| `vox_scientia_publication_novelty_fetch` | public | network | [research] |
| `vox_scientia_publication_novelty_happy_path` | public | writes_files | [research] |
| `vox_scientia_publication_preflight` | public | read_only | [research] |
| `vox_scientia_publication_prepare` | operator | writes_files | [research] |
| `vox_scientia_publication_publish` | operator | remote_mutation | [research] |
| `vox_scientia_publication_retry_failed` | operator | remote_mutation | [research] |
| `vox_scientia_publication_route_simulate` | public | read_only | [research] |
| `vox_scientia_publication_scholarly_pipeline_run` | operator | remote_mutation | [research] |
| `vox_scientia_publication_scholarly_remote_status` | public | network | [research] |
| `vox_scientia_publication_scholarly_remote_status_sync_all` | public | writes_files | [research] |
| `vox_scientia_publication_scholarly_remote_status_sync_batch` | public | writes_files | [research] |
| `vox_scientia_publication_scholarly_staging_export` | operator | writes_files | [research] |
| `vox_scientia_publication_status` | public | read_only | [research] |
| `vox_scientia_publication_submit_local` | operator | writes_files | [research] |
| `vox_scientia_worthiness_evaluate` | public | read_only | [research] |

Group note: approve / publish / submit / external-job tools are the spec's `research-publish` slice — they must stay `operator`. If `vox_news_test_syndicate` actually posts to a syndication endpoint, it is `remote_mutation` (still `operator`).

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_news_approval_status|vox_news_approve|vox_news_draft_research|vox_news_simulate_publish_gate|vox_news_test_syndicate|vox_research_get|vox_research_run|vox_research_search|vox_research_start|vox_research_status|vox_schola_submit|vox_scientia_assist_suggestions|vox_scientia_publication_approve|vox_scientia_publication_arxiv_handoff_record|vox_scientia_publication_decision_explain|vox_scientia_publication_discovery_explain|vox_scientia_publication_discovery_refresh_evidence|vox_scientia_publication_discovery_scan|vox_scientia_publication_external_jobs_dead_letter|vox_scientia_publication_external_jobs_due|vox_scientia_publication_external_jobs_replay|vox_scientia_publication_external_jobs_tick|vox_scientia_publication_external_pipeline_metrics|vox_scientia_publication_media_delete|vox_scientia_publication_media_list|vox_scientia_publication_media_upsert|vox_scientia_publication_novelty_fetch|vox_scientia_publication_novelty_happy_path|vox_scientia_publication_preflight|vox_scientia_publication_prepare|vox_scientia_publication_publish|vox_scientia_publication_retry_failed|vox_scientia_publication_route_simulate|vox_scientia_publication_scholarly_pipeline_run|vox_scientia_publication_scholarly_remote_status|vox_scientia_publication_scholarly_remote_status_sync_all|vox_scientia_publication_scholarly_remote_status_sync_batch|vox_scientia_publication_scholarly_staging_export|vox_scientia_publication_status|vox_scientia_publication_submit_local|vox_scientia_worthiness_evaluate)`'
```
Expected: `123`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Keep a row unchanged unless the handler contradicts it. Write `target/plan01-evidence/task3.txt` in the evidence-file format.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task3.txt` — Expected: at least `28` (the table's non-read rows).

- [ ] **Step 3: Apply the edits**

For each of the 41 ops: set `side_effect_class:` and append `exposure:` / `requires:` under `http_read_role_eligible:` exactly as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output (rg exit 1). Any line printed is either an unedited op or a V4–V6 invariant violation — fix and repeat.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: '` — Expected: `41`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify research/scientia/news MCP ops" -m "Capability profiles plan 01 Task 3 (41 ops)." -m "$(cat target/plan01-evidence/task3.txt)"
```

---

### Task 4: Classify orchestrator / agent / task / A2A / clarification ops (53)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task4.txt`

**Interfaces:**
- Consumes: the Task 2 gate.
- Produces: 53 classified ops.

Parallel-safe with: Tasks 3, 5, 6, 7, 8, 9, 10, 11, 12, 15. Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/task_tools/`, `agent_tools.rs`, `a2a_tools.rs`, `questioning_tools.rs`, `pending_approvals.rs`, `approval_allowlist.rs`, `lifecycle.rs`, `receipt_tools.rs`, `persistence_tools.rs`, `dei_tools/`.

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_a2a_ack` | public | writes_files | [] |
| `vox_a2a_broadcast` | operator | writes_files | [] |
| `vox_a2a_history` | public | read_only | [db] |
| `vox_a2a_inbox` | public | read_only | [] |
| `vox_a2a_send` | operator | writes_files | [] |
| `vox_a2a_tasks` | public | read_only | [agent_fleet] |
| `vox_agent_assess` | public | read_only | [agent_fleet] |
| `vox_agent_continue` | operator | spawns_process | [agent_fleet] |
| `vox_agent_events` | public | read_only | [agent_fleet] |
| `vox_agent_handoff` | operator | writes_files | [agent_fleet] |
| `vox_agent_status` | public | read_only | [agent_fleet] |
| `vox_answer_question` | operator | writes_files | [clarification] |
| `vox_ask_agent` | operator | writes_files | [agent_fleet, llm_facade] |
| `vox_ask_clarification` | public | writes_files | [clarification] |
| `vox_broadcast` | operator | writes_files | [] |
| `vox_cancel_task` | operator | writes_files | [agent_fleet] |
| `vox_complete_task` | operator | writes_files | [agent_fleet] |
| `vox_config_get` | public | read_only | [] |
| `vox_config_set` | operator | writes_files | [] |
| `vox_doubt_task` | operator | writes_files | [agent_fleet] |
| `vox_fail_task` | operator | writes_files | [agent_fleet] |
| `vox_handoff_context` | public | read_only | [db] |
| `vox_handoff_lineage` | public | read_only | [db] |
| `vox_heartbeat` | public | writes_files | [agent_fleet] |
| `vox_list_approval_allowlist` | public | read_only | [] |
| `vox_lock_status` | public | read_only | [] |
| `vox_orchestrator_persistence_outbox_lifecycle` | public | read_only | [db] |
| `vox_orchestrator_persistence_outbox_queue` | public | read_only | [db] |
| `vox_orchestrator_start` | operator | spawns_process | [] |
| `vox_orchestrator_status` | public | read_only | [] |
| `vox_pause_agent` | operator | writes_files | [agent_fleet] |
| `vox_pending_approvals` | public | read_only | [] |
| `vox_pending_questions` | public | read_only | [clarification] |
| `vox_pipeline_status` | public | read_only | [] |
| `vox_plan` | public | writes_files | [llm_facade, db] |
| `vox_plan_list_sessions` | public | read_only | [db] |
| `vox_plan_resume` | operator | spawns_process | [agent_fleet, db] |
| `vox_plan_status` | public | read_only | [db] |
| `vox_poll_events` | public | read_only | [] |
| `vox_questioning_pending` | public | read_only | [clarification] |
| `vox_questioning_submit_answer` | operator | writes_files | [clarification] |
| `vox_questioning_sync_ssot` | operator | writes_files | [clarification] |
| `vox_queue_status` | public | read_only | [agent_fleet] |
| `vox_rebalance` | operator | writes_files | [agent_fleet] |
| `vox_reorder_task` | operator | writes_files | [agent_fleet] |
| `vox_replan` | public | writes_files | [llm_facade, db] |
| `vox_resolve_approval` | operator | writes_files | [] |
| `vox_resume_agent` | operator | writes_files | [agent_fleet] |
| `vox_retire_agent` | operator | writes_files | [agent_fleet] |
| `vox_spawn_agent` | operator | spawns_process | [agent_fleet] |
| `vox_submit_task` | operator | spawns_process | [agent_fleet] |
| `vox_task_status` | public | read_only | [agent_fleet] |
| `vox_verify_task_claims` | public | read_only | [agent_fleet] |

Group note: the six `http_read_role_eligible` tools here (`vox_handoff_lineage`, both outbox tools, `vox_orchestrator_status`, `vox_plan_status`, `vox_task_status`) must stay side-effect free (V4). `vox_resolve_approval` keeps its existing `writes_files`. `vox_emergency_stop` and `vox_drain_agent` belong to Task 12.

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_a2a_ack|vox_a2a_broadcast|vox_a2a_history|vox_a2a_inbox|vox_a2a_send|vox_a2a_tasks|vox_agent_assess|vox_agent_continue|vox_agent_events|vox_agent_handoff|vox_agent_status|vox_answer_question|vox_ask_agent|vox_ask_clarification|vox_broadcast|vox_cancel_task|vox_complete_task|vox_config_get|vox_config_set|vox_doubt_task|vox_fail_task|vox_handoff_context|vox_handoff_lineage|vox_heartbeat|vox_list_approval_allowlist|vox_lock_status|vox_orchestrator_persistence_outbox_lifecycle|vox_orchestrator_persistence_outbox_queue|vox_orchestrator_start|vox_orchestrator_status|vox_pause_agent|vox_pending_approvals|vox_pending_questions|vox_pipeline_status|vox_plan|vox_plan_list_sessions|vox_plan_resume|vox_plan_status|vox_poll_events|vox_questioning_pending|vox_questioning_submit_answer|vox_questioning_sync_ssot|vox_queue_status|vox_rebalance|vox_reorder_task|vox_replan|vox_resolve_approval|vox_resume_agent|vox_retire_agent|vox_spawn_agent|vox_submit_task|vox_task_status|vox_verify_task_claims)`'
```
Expected: `158`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Write `target/plan01-evidence/task4.txt` in the evidence-file format.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task4.txt` — Expected: at least `29`.

- [ ] **Step 3: Apply the edits**

For each of the 53 ops: set `side_effect_class:` and append `exposure:` / `requires:` under `http_read_role_eligible:` as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: '` — Expected: `53`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify orchestrator/agent/task MCP ops" -m "Capability profiles plan 01 Task 4 (53 ops)." -m "$(cat target/plan01-evidence/task4.txt)"
```

---

### Task 5: Classify browser ops (32)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task5.txt`

**Interfaces:**
- Consumes: the Task 2 gate.
- Produces: 32 classified ops.

Parallel-safe with: Tasks 3, 4, 6, 7, 8, 9, 10, 11, 12, 15. Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/browser_tools.rs` (dispatchable only with the `heavy-browser` feature), `tool_images.rs`.

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_browser_act` | operator | remote_mutation | [browser] |
| `vox_browser_back` | operator | network | [browser] |
| `vox_browser_click` | operator | remote_mutation | [browser] |
| `vox_browser_click_ref` | operator | remote_mutation | [browser] |
| `vox_browser_click_xy` | operator | remote_mutation | [browser] |
| `vox_browser_close` | operator | writes_files | [browser] |
| `vox_browser_cookies_export` | internal | read_only | [browser] |
| `vox_browser_cookies_import` | internal | writes_files | [browser] |
| `vox_browser_extract` | public | read_only | [browser] |
| `vox_browser_extract_json` | public | read_only | [browser] |
| `vox_browser_fill` | operator | remote_mutation | [browser] |
| `vox_browser_fill_ref` | operator | remote_mutation | [browser] |
| `vox_browser_forward` | operator | network | [browser] |
| `vox_browser_goto` | operator | network | [browser] |
| `vox_browser_html` | public | read_only | [browser] |
| `vox_browser_list_pages` | public | read_only | [browser] |
| `vox_browser_open` | operator | network | [browser] |
| `vox_browser_open_ex` | operator | network | [browser] |
| `vox_browser_page_info` | public | read_only | [browser] |
| `vox_browser_press` | operator | remote_mutation | [browser] |
| `vox_browser_reload` | operator | network | [browser] |
| `vox_browser_screencast_frame` | public | read_only | [browser] |
| `vox_browser_screenshot` | public | read_only | [browser] |
| `vox_browser_screenshot_viewport` | public | read_only | [browser] |
| `vox_browser_scroll` | operator | writes_files | [browser] |
| `vox_browser_set_control_lock` | operator | writes_files | [browser] |
| `vox_browser_set_viewport` | operator | writes_files | [browser] |
| `vox_browser_snapshot` | public | read_only | [browser] |
| `vox_browser_stop` | operator | writes_files | [browser] |
| `vox_browser_text` | public | read_only | [browser] |
| `vox_browser_type` | operator | remote_mutation | [browser] |
| `vox_browser_wait_for` | public | read_only | [browser] |

Group note: navigation drives the host's (possibly logged-in) browser, so it is `operator` even though it is a network read; anything that can submit a form is `remote_mutation`. `vox_browser_open` is an anchor row. If `vox_browser_extract_json` calls the LLM facade, change it to `network` and add `llm_facade` to `requires` (still `public`).

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_browser_act|vox_browser_back|vox_browser_click|vox_browser_click_ref|vox_browser_click_xy|vox_browser_close|vox_browser_cookies_export|vox_browser_cookies_import|vox_browser_extract|vox_browser_extract_json|vox_browser_fill|vox_browser_fill_ref|vox_browser_forward|vox_browser_goto|vox_browser_html|vox_browser_list_pages|vox_browser_open|vox_browser_open_ex|vox_browser_page_info|vox_browser_press|vox_browser_reload|vox_browser_screencast_frame|vox_browser_screenshot|vox_browser_screenshot_viewport|vox_browser_scroll|vox_browser_set_control_lock|vox_browser_set_viewport|vox_browser_snapshot|vox_browser_stop|vox_browser_text|vox_browser_type|vox_browser_wait_for)`'
```
Expected: `96`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Write `target/plan01-evidence/task5.txt` in the evidence-file format.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task5.txt` — Expected: at least `20`.

- [ ] **Step 3: Apply the edits**

For each of the 32 ops: set `side_effect_class:` and append `exposure:` / `requires:` under `http_read_role_eligible:` as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: '` — Expected: `32`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify browser MCP ops" -m "Capability profiles plan 01 Task 5 (32 ops)." -m "$(cat target/plan01-evidence/task5.txt)"
```

---

### Task 6: Classify memory / session / KB / search / context ops (52)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task6.txt`

**Interfaces:**
- Consumes: the Task 2 gate.
- Produces: 52 classified ops.

Parallel-safe with: Tasks 3, 4, 5, 7, 8, 9, 10, 11, 12, 15. Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/memory_tools/`, `kb_tools/`, `graph_tools.rs`, `rag_tools.rs`, `feedback_tools.rs`, `tool_search.rs`, `mcp_context.rs`.

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_behavior_record` | public | writes_files | [memory] |
| `vox_behavior_summary` | public | read_only | [memory] |
| `vox_check_mood` | public | read_only | [] |
| `vox_compaction_status` | public | read_only | [sessions] |
| `vox_feedback_list` | public | read_only | [] |
| `vox_get_context` | public | read_only | [] |
| `vox_kb_add_entry` | public | writes_files | [search_index] |
| `vox_kb_add_rule` | public | writes_files | [search_index] |
| `vox_kb_clip` | public | writes_files | [search_index] |
| `vox_kb_create` | public | writes_files | [search_index] |
| `vox_kb_delete` | operator | writes_files | [search_index] |
| `vox_kb_delete_entry` | operator | writes_files | [search_index] |
| `vox_kb_get_feed` | public | read_only | [search_index] |
| `vox_kb_list` | public | read_only | [search_index] |
| `vox_kb_list_entries` | public | read_only | [search_index] |
| `vox_kb_list_rules` | public | read_only | [search_index] |
| `vox_kb_query` | public | read_only | [search_index] |
| `vox_kb_review_entry` | public | writes_files | [search_index] |
| `vox_knowledge_query` | public | read_only | [search_index] |
| `vox_learn_pattern` | public | writes_files | [memory] |
| `vox_list_context` | public | read_only | [] |
| `vox_memory_list_keys` | public | read_only | [memory] |
| `vox_memory_log` | public | writes_files | [memory] |
| `vox_memory_recall` | public | read_only | [memory] |
| `vox_memory_recall_db` | public | read_only | [memory] |
| `vox_memory_save_db` | public | writes_files | [memory] |
| `vox_memory_search` | public | read_only | [memory] |
| `vox_memory_store` | public | writes_files | [memory] |
| `vox_preference_get` | public | read_only | [memory] |
| `vox_preference_list` | public | read_only | [memory] |
| `vox_preference_set` | public | writes_files | [memory] |
| `vox_resolve_feedback` | public | writes_files | [] |
| `vox_search_callees` | public | read_only | [search_index] |
| `vox_search_callers` | public | read_only | [search_index] |
| `vox_search_compare` | public | read_only | [search_index] |
| `vox_search_history` | public | writes_files | [search_index] |
| `vox_search_neighbors` | public | read_only | [search_index] |
| `vox_search_path` | public | read_only | [search_index] |
| `vox_search_rebuild` | public | writes_files | [search_index] |
| `vox_search_set_ttl` | public | writes_files | [search_index] |
| `vox_search_status` | public | read_only | [search_index] |
| `vox_search_structural` | public | read_only | [search_index] |
| `vox_semantic_fs_discover` | public | read_only | [search_index] |
| `vox_session_cleanup` | operator | writes_files | [sessions] |
| `vox_session_compact` | public | writes_files | [sessions] |
| `vox_session_create` | public | writes_files | [sessions] |
| `vox_session_info` | public | read_only | [sessions] |
| `vox_session_list` | public | read_only | [sessions] |
| `vox_session_reset` | operator | writes_files | [sessions] |
| `vox_set_context` | public | writes_files | [] |
| `vox_tool_search` | public | none | [] |
| `vox_visual_rag_query` | public | network | [llm_facade] |

Group note: seven `vox_search_*` ops (`callees`, `callers`, `compare`, `neighbors`, `path`, `status`, `structural`) currently carry `side_effect_class: none` but read an on-disk index/graph — they become `read_only` (as in the table) and each gets an evidence line. Existing `writes_files` on the `vox_kb_*` and `vox_search_history|rebuild|set_ttl` rows is kept. `vox_kb_query` is an anchor row. The `vox_search_*` graph tools may read the Graphify corpus rather than the DB index; if a handler needs no `search_index` boot step, set `requires: []` and record it under `Overrides:`.

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_behavior_record|vox_behavior_summary|vox_check_mood|vox_compaction_status|vox_feedback_list|vox_get_context|vox_kb_add_entry|vox_kb_add_rule|vox_kb_clip|vox_kb_create|vox_kb_delete|vox_kb_delete_entry|vox_kb_get_feed|vox_kb_list|vox_kb_list_entries|vox_kb_list_rules|vox_kb_query|vox_kb_review_entry|vox_knowledge_query|vox_learn_pattern|vox_list_context|vox_memory_list_keys|vox_memory_log|vox_memory_recall|vox_memory_recall_db|vox_memory_save_db|vox_memory_search|vox_memory_store|vox_preference_get|vox_preference_list|vox_preference_set|vox_resolve_feedback|vox_search_callees|vox_search_callers|vox_search_compare|vox_search_history|vox_search_neighbors|vox_search_path|vox_search_rebuild|vox_search_set_ttl|vox_search_status|vox_search_structural|vox_semantic_fs_discover|vox_session_cleanup|vox_session_compact|vox_session_create|vox_session_info|vox_session_list|vox_session_reset|vox_set_context|vox_tool_search|vox_visual_rag_query)`'
```
Expected: `139`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Write `target/plan01-evidence/task6.txt` in the evidence-file format, including the seven `none` → `read_only` changes.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task6.txt` — Expected: at least `30` (23 non-read rows + 7 preset changes).

- [ ] **Step 3: Apply the edits**

For each of the 52 ops: set `side_effect_class:` and append `exposure:` / `requires:` under `http_read_role_eligible:` as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: '` — Expected: `52`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify memory/session/kb/search MCP ops" -m "Capability profiles plan 01 Task 6 (52 ops)." -m "$(cat target/plan01-evidence/task6.txt)"
```

---

### Task 7: Classify git / vcs / repo / workspace / file ops (38)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task7.txt`

**Interfaces:**
- Consumes: the Task 2 gate.
- Produces: 38 classified ops.

Parallel-safe with: Tasks 3, 4, 5, 6, 8, 9, 10, 11, 12, 15. Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/git_tools.rs`, `git_exec.rs`, `vcs_tools/`, `repo_catalog_tools.rs`, `repo_index.rs`, `project_init_tools.rs`, `workspace_mcp/`, `scope_guard.rs` (`WRITE_TOOLS`), `lock_guard.rs`, `history_tools.rs`.

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_apply_structured_edit` | operator | writes_files | [] |
| `vox_change_create` | operator | writes_files | [] |
| `vox_change_log` | public | read_only | [] |
| `vox_check_file_owner` | public | read_only | [] |
| `vox_claim_file` | operator | writes_files | [] |
| `vox_conflict_diff` | public | read_only | [] |
| `vox_conflicts` | public | read_only | [] |
| `vox_file_graph` | public | read_only | [] |
| `vox_git_blame` | public | read_only | [] |
| `vox_git_diff` | public | read_only | [] |
| `vox_git_log` | public | read_only | [] |
| `vox_git_status` | public | read_only | [] |
| `vox_inline_edit` | operator | writes_files | [] |
| `vox_map_agent_session` | public | writes_files | [sessions] |
| `vox_my_files` | public | read_only | [] |
| `vox_oplog` | public | read_only | [] |
| `vox_project_init` | operator | writes_files | [] |
| `vox_redo` | operator | writes_files | [] |
| `vox_repo_catalog_list` | public | read_only | [db] |
| `vox_repo_catalog_refresh` | public | writes_files | [db] |
| `vox_repo_index_refresh` | public | writes_files | [] |
| `vox_repo_index_status` | public | read_only | [] |
| `vox_repo_query_file` | public | read_only | [] |
| `vox_repo_query_history` | public | read_only | [] |
| `vox_repo_query_text` | public | read_only | [] |
| `vox_repo_status` | public | read_only | [] |
| `vox_resolve_conflict` | operator | writes_files | [] |
| `vox_snapshot_diff` | public | read_only | [] |
| `vox_snapshot_list` | public | read_only | [] |
| `vox_snapshot_restore` | operator | destructive | [db] |
| `vox_transfer_file` | operator | writes_files | [] |
| `vox_undo` | operator | writes_files | [] |
| `vox_vcs_status` | public | read_only | [] |
| `vox_workspace_create` | operator | writes_files | [] |
| `vox_workspace_mcp_refresh` | operator | writes_files | [compiler] |
| `vox_workspace_merge` | operator | writes_files | [] |
| `vox_workspace_modules` | public | read_only | [] |
| `vox_workspace_status` | public | read_only | [] |

Group note: git read tools shell out to `git` with a read-only argv; per the procedure they are `read_only`, not `spawns_process`. `vox_git_status` and `vox_snapshot_restore` are anchor rows. Existing values on `vox_apply_structured_edit`, `vox_project_init`, `vox_repo_catalog_list`, `vox_repo_status`, `vox_snapshot_restore`, `vox_workspace_create` are kept.

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_apply_structured_edit|vox_change_create|vox_change_log|vox_check_file_owner|vox_claim_file|vox_conflict_diff|vox_conflicts|vox_file_graph|vox_git_blame|vox_git_diff|vox_git_log|vox_git_status|vox_inline_edit|vox_map_agent_session|vox_my_files|vox_oplog|vox_project_init|vox_redo|vox_repo_catalog_list|vox_repo_catalog_refresh|vox_repo_index_refresh|vox_repo_index_status|vox_repo_query_file|vox_repo_query_history|vox_repo_query_text|vox_repo_status|vox_resolve_conflict|vox_snapshot_diff|vox_snapshot_list|vox_snapshot_restore|vox_transfer_file|vox_undo|vox_vcs_status|vox_workspace_create|vox_workspace_mcp_refresh|vox_workspace_merge|vox_workspace_modules|vox_workspace_status)`'
```
Expected: `108`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Write `target/plan01-evidence/task7.txt` in the evidence-file format; for each git read tool also add a line citing its read-only argv.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task7.txt` — Expected: at least `16`.

- [ ] **Step 3: Apply the edits**

For each of the 38 ops: set `side_effect_class:` and append `exposure:` / `requires:` under `http_read_role_eligible:` as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: '` — Expected: `38`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify git/vcs/repo/workspace MCP ops" -m "Capability profiles plan 01 Task 7 (38 ops)." -m "$(cat target/plan01-evidence/task7.txt)"
```

---

### Task 8: Classify compiler / validation / test / codegen / GUI-registry ops (25)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task8.txt`

**Interfaces:**
- Consumes: the Task 2 gate.
- Produces: 25 classified ops.

Parallel-safe with: Tasks 3, 4, 5, 6, 7, 9, 10, 11, 12, 15. Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/compiler_tools.rs`, `code_validator.rs`, `grammar_tools.rs`, `introspection_tools.rs`, `gui_registry_tools.rs`, `visus_tools.rs`, `toestub_tools.rs`, `qa_tools.rs`, `scope_guard.rs`.

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_build_crate` | operator | spawns_process | [compiler] |
| `vox_builtin_registry` | public | none | [compiler] |
| `vox_check` | public | read_only | [compiler] |
| `vox_check_workspace` | public | read_only | [compiler] |
| `vox_code_audit_findings_upsert` | public | writes_files | [db] |
| `vox_compiler::ast_inspect` | public | read_only | [compiler] |
| `vox_coverage_report` | operator | spawns_process | [compiler] |
| `vox_decorator_registry` | public | none | [compiler] |
| `vox_export_grammar_ebnf` | public | none | [compiler] |
| `vox_generate_code` | operator | writes_files | [llm_facade, compiler] |
| `vox_ghost_text` | public | network | [llm_facade] |
| `vox_gui_components` | public | read_only | [] |
| `vox_gui_rules` | public | read_only | [] |
| `vox_gui_tokens` | public | read_only | [] |
| `vox_journey_canonical_steps` | public | none | [] |
| `vox_language_surface` | public | none | [compiler] |
| `vox_lint_crate` | operator | spawns_process | [compiler] |
| `vox_run_tests` | operator | spawns_process | [compiler] |
| `vox_test_all` | operator | spawns_process | [compiler] |
| `vox_test_decision` | public | read_only | [] |
| `vox_validate_file` | public | read_only | [compiler] |
| `vox_validate_source` | public | read_only | [compiler] |
| `vox_validate_vuv` | public | read_only | [compiler] |
| `vox_visus_audit` | public | read_only | [] |
| `vox_visus_baseline` | operator | writes_files | [] |

Group note: `vox_generate_code` currently says `none`; it is listed in `crates/vox-orchestrator-mcp/src/scope_guard.rs:15` (`WRITE_TOOLS`) and in `crates/vox-foundation/src/primitives/agentos_mutation.rs` as an external side effect — the evidence file's line for it is ``- vox_generate_code writes_files: crates/vox-orchestrator-mcp/src/scope_guard.rs:15 — listed in WRITE_TOOLS (preset none corrected)`` plus the handler's own write site. If `vox_coverage_report` only reads an existing report file, it is `public` / `read_only` (record under `Overrides:`). If `vox_visus_audit` launches a browser or subprocess to capture screenshots, it is `operator` / `spawns_process`.

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_build_crate|vox_builtin_registry|vox_check|vox_check_workspace|vox_code_audit_findings_upsert|vox_compiler::ast_inspect|vox_coverage_report|vox_decorator_registry|vox_export_grammar_ebnf|vox_generate_code|vox_ghost_text|vox_gui_components|vox_gui_rules|vox_gui_tokens|vox_journey_canonical_steps|vox_language_surface|vox_lint_crate|vox_run_tests|vox_test_all|vox_test_decision|vox_validate_file|vox_validate_source|vox_validate_vuv|vox_visus_audit|vox_visus_baseline)`'
```
Expected: `74`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Write `target/plan01-evidence/task8.txt` in the evidence-file format.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task8.txt` — Expected: at least `9`.

- [ ] **Step 3: Apply the edits**

For each of the 25 ops: set `side_effect_class:` and append `exposure:` / `requires:` under `http_read_role_eligible:` as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: '` — Expected: `25`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify compiler/validate/test MCP ops" -m "Capability profiles plan 01 Task 8 (25 ops)." -m "$(cat target/plan01-evidence/task8.txt)"
```

---

### Task 9: Classify DB ops (15)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task9.txt`

**Interfaces:**
- Consumes: the Task 2 gate.
- Produces: 15 classified ops.

Parallel-safe with: Tasks 3, 4, 5, 6, 7, 8, 10, 11, 12, 15. Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/db_tools.rs`, `trust_tools.rs`, `codex_tools.rs`.

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_db_conversation_edge_insert` | operator | writes_files | [db] |
| `vox_db_conversation_version_append` | operator | writes_files | [db] |
| `vox_db_data_flow` | public | read_only | [db] |
| `vox_db_explain_query` | public | read_only | [db] |
| `vox_db_relationships` | public | read_only | [db] |
| `vox_db_research_metric_linked` | public | read_only | [db] |
| `vox_db_research_session_upsert` | operator | writes_files | [db] |
| `vox_db_sample_data` | public | read_only | [db] |
| `vox_db_schema` | public | read_only | [db] |
| `vox_db_suggest_query` | public | network | [db, llm_facade] |
| `vox_db_topic_evolution_append` | operator | writes_files | [db] |
| `vox_db_trust_drift` | public | read_only | [db] |
| `vox_db_trust_propagate` | operator | writes_files | [db] |
| `vox_db_trust_rollups` | public | read_only | [db] |
| `vox_db_trust_summary` | public | read_only | [db] |

Group note: generic DB writes are `operator` — the DB is shared state, not one profile's own. If `vox_db_suggest_query` is template-based (no LLM call), it is `read_only` with `requires: [db]`.

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_db_conversation_edge_insert|vox_db_conversation_version_append|vox_db_data_flow|vox_db_explain_query|vox_db_relationships|vox_db_research_metric_linked|vox_db_research_session_upsert|vox_db_sample_data|vox_db_schema|vox_db_suggest_query|vox_db_topic_evolution_append|vox_db_trust_drift|vox_db_trust_propagate|vox_db_trust_rollups|vox_db_trust_summary)`'
```
Expected: `45`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Write `target/plan01-evidence/task9.txt` in the evidence-file format.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task9.txt` — Expected: at least `6`.

- [ ] **Step 3: Apply the edits**

For each of the 15 ops: set `side_effect_class:` and append `exposure:` / `requires:` under `http_read_role_eligible:` as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: '` — Expected: `15`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify db MCP ops" -m "Capability profiles plan 01 Task 9 (15 ops)." -m "$(cat target/plan01-evidence/task9.txt)"
```

---

### Task 10: Classify skill / plugin ops (17)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task10.txt`

**Interfaces:**
- Consumes: the Task 2 gate.
- Produces: 17 classified ops.

Parallel-safe with: Tasks 3, 4, 5, 6, 7, 8, 9, 11, 12, 15. Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/skills_tools.rs`, `plugin_tools.rs`, `skill_promotion.rs`, `skill_search_index.rs`, `plugin_skills_bridge.rs`.

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_plugin_catalog` | public | read_only | [] |
| `vox_plugin_info` | public | read_only | [] |
| `vox_plugin_install` | operator | writes_files | [] |
| `vox_plugin_list` | public | read_only | [] |
| `vox_plugin_remove` | operator | destructive | [] |
| `vox_propose_skill` | public | writes_files | [] |
| `vox_skill_add` | operator | writes_files | [] |
| `vox_skill_discover` | public | read_only | [] |
| `vox_skill_info` | public | read_only | [] |
| `vox_skill_install` | operator | writes_files | [] |
| `vox_skill_list` | public | read_only | [] |
| `vox_skill_parse` | public | read_only | [] |
| `vox_skill_remove` | operator | writes_files | [] |
| `vox_skill_run` | operator | spawns_process | [] |
| `vox_skill_search` | public | read_only | [] |
| `vox_skill_uninstall` | operator | writes_files | [] |
| `vox_skill_use` | public | read_only | [] |

Group note: skills and plugins are not a boot-gated subsystem, so `requires` is `[]`. Existing `writes_files` (`vox_plugin_install`) and `destructive` (`vox_plugin_remove`) are kept. If `vox_skill_run` only renders a skill body without executing anything, it is `public` / `read_only`; a download inside `vox_skill_install` does not change its class (the local write ranks higher).

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_plugin_catalog|vox_plugin_info|vox_plugin_install|vox_plugin_list|vox_plugin_remove|vox_propose_skill|vox_skill_add|vox_skill_discover|vox_skill_info|vox_skill_install|vox_skill_list|vox_skill_parse|vox_skill_remove|vox_skill_run|vox_skill_search|vox_skill_uninstall|vox_skill_use)`'
```
Expected: `49`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Write `target/plan01-evidence/task10.txt` in the evidence-file format.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task10.txt` — Expected: at least `8`.

- [ ] **Step 3: Apply the edits**

For each of the 17 ops: set `side_effect_class:` and append `exposure:` / `requires:` under `http_read_role_eligible:` as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: '` — Expected: `17`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify skill/plugin MCP ops" -m "Capability profiles plan 01 Task 10 (17 ops)." -m "$(cat target/plan01-evidence/task10.txt)"
```

---

### Task 11: Classify models / chat / attention / budget / mesh / speech ops (30)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task11.txt`

**Interfaces:**
- Consumes: the Task 2 gate.
- Produces: 30 classified ops.

Parallel-safe with: Tasks 3, 4, 5, 6, 7, 8, 9, 10, 12, 15. Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/models_tools.rs`, `benchmark_tools.rs`, `chat_tools/`, `attention_policy.rs`, `exec_time_tools.rs`, `populi_tools.rs`, `oratio_tools.rs`, `speech_pipeline_tools.rs`, `trust_tools.rs`, `affinity_tools.rs`.

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_attention_history` | public | read_only | [attention] |
| `vox_attention_reset` | operator | writes_files | [attention] |
| `vox_attention_summary` | public | read_only | [attention] |
| `vox_benchmark_list` | public | read_only | [db, model_registry] |
| `vox_benchmark_record` | public | writes_files | [db, model_registry] |
| `vox_budget_status` | public | read_only | [attention] |
| `vox_capability_model_manifest` | public | read_only | [model_registry] |
| `vox_chat_history` | public | read_only | [db] |
| `vox_chat_message` | public | writes_files | [llm_facade, db] |
| `vox_context_budget` | public | read_only | [attention] |
| `vox_cost_history` | public | read_only | [attention] |
| `vox_exec_time_query` | public | read_only | [db] |
| `vox_exec_time_record` | public | writes_files | [db] |
| `vox_get_active_model` | public | read_only | [model_registry] |
| `vox_list_models` | public | read_only | [model_registry] |
| `vox_mesh_dispatch` | operator | spawns_process | [mesh] |
| `vox_mesh_nodes` | public | read_only | [mesh] |
| `vox_mesh_queue_stats` | public | read_only | [mesh] |
| `vox_oratio_listen` | operator | spawns_process | [speech] |
| `vox_oratio_status` | public | read_only | [speech] |
| `vox_oratio_transcribe` | operator | spawns_process | [speech] |
| `vox_populi_local_status` | public | read_only | [mesh] |
| `vox_publish_message` | operator | remote_mutation | [mesh] |
| `vox_record_cost` | public | writes_files | [attention] |
| `vox_set_active_model` | public | writes_files | [model_registry] |
| `vox_set_agent_budget` | operator | writes_files | [attention] |
| `vox_set_model` | operator | writes_files | [model_registry] |
| `vox_speech_to_code` | public | network | [speech, llm_facade] |
| `vox_suggest_model` | public | read_only | [model_registry] |
| `vox_trust_override` | operator | writes_files | [] |

Group note: `vox_set_active_model` changes only the model selector's own state (spec §3 example of `public`); `vox_set_model` overrides a model per agent, which mutates other agents, so it is `operator`. Existing `spawns_process` / `read_only` on the mesh and oratio rows are kept. `vox_list_models` is an anchor row. If `vox_speech_to_code` writes generated code into the workspace, it is `operator` / `writes_files`.

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_attention_history|vox_attention_reset|vox_attention_summary|vox_benchmark_list|vox_benchmark_record|vox_budget_status|vox_capability_model_manifest|vox_chat_history|vox_chat_message|vox_context_budget|vox_cost_history|vox_exec_time_query|vox_exec_time_record|vox_get_active_model|vox_list_models|vox_mesh_dispatch|vox_mesh_nodes|vox_mesh_queue_stats|vox_oratio_listen|vox_oratio_status|vox_oratio_transcribe|vox_populi_local_status|vox_publish_message|vox_record_cost|vox_set_active_model|vox_set_agent_budget|vox_set_model|vox_speech_to_code|vox_suggest_model|vox_trust_override)`'
```
Expected: `86`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Write `target/plan01-evidence/task11.txt` in the evidence-file format.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task11.txt` — Expected: at least `14`.

- [ ] **Step 3: Apply the edits**

For each of the 30 ops: set `side_effect_class:` and append `exposure:` / `requires:` under `http_read_role_eligible:` as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: '` — Expected: `30`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify models/chat/attention/mesh/speech MCP ops" -m "Capability profiles plan 01 Task 11 (30 ops)." -m "$(cat target/plan01-evidence/task11.txt)"
```

---

### Task 12: Classify the internal set — agy / openclaw / agent gateway / gamify / secrets / emergency (37)

**Files:**
- Modify: `contracts/operations/catalog.v1.yaml` — only the op blocks whose `mcp.name` is in the table below.
- Create (untracked): `target/plan01-evidence/task12.txt`

**Interfaces:**
- Consumes: the Task 2 gate.
- Produces: 37 classified ops, all `exposure: internal`.

Parallel-safe with: Tasks 3, 4, 5, 6, 7, 8, 9, 10, 11, 15. Depends on: Task 2.

Start here: `crates/vox-orchestrator-mcp/src/agy_tools.rs`, `agy_exec.rs`, `agy_pipeline.rs`, `agy_doctor.rs`, `agy_ledger.rs`, `openclaw_tools.rs`, `gamify_tools.rs`, `secrets_tools.rs`, `approval_allowlist.rs`, `task_tools/` (emergency stop, drain).

| Tool | exposure | side_effect_class | requires |
|---|---|---|---|
| `vox_add_approval_allowlist_entry` | internal | writes_files | [] |
| `vox_agent_gateway_call` | internal | remote_mutation | [] |
| `vox_agent_list_remote` | internal | network | [] |
| `vox_agent_notify` | internal | remote_mutation | [] |
| `vox_agent_subscribe` | internal | remote_mutation | [] |
| `vox_agent_subscriptions` | internal | network | [] |
| `vox_agent_unsubscribe` | internal | remote_mutation | [] |
| `vox_agy_delegate` | internal | spawns_process | [] |
| `vox_agy_delegate_batch` | internal | spawns_process | [] |
| `vox_agy_doctor` | internal | read_only | [] |
| `vox_agy_ledger_digest` | internal | read_only | [] |
| `vox_agy_pipeline` | internal | spawns_process | [] |
| `vox_agy_review` | internal | spawns_process | [] |
| `vox_credentials_status` | internal | read_only | [] |
| `vox_drain_agent` | internal | writes_files | [agent_fleet] |
| `vox_emergency_stop` | internal | destructive | [agent_fleet] |
| `vox_gamify_battle_start` | internal | writes_files | [db] |
| `vox_gamify_battle_submit` | internal | writes_files | [db] |
| `vox_gamify_collegium_join` | internal | writes_files | [db] |
| `vox_gamify_notification_ack` | internal | writes_files | [db] |
| `vox_gamify_notifications_ack_all` | internal | writes_files | [db] |
| `vox_gamify_notifications_list` | internal | read_only | [db] |
| `vox_gamify_progress_snapshot` | internal | read_only | [db] |
| `vox_gamify_quest_list` | internal | read_only | [db] |
| `vox_gamify_shop_buy` | internal | writes_files | [db] |
| `vox_gamify_shop_catalog` | internal | read_only | [db] |
| `vox_openclaw_discover` | internal | network | [] |
| `vox_openclaw_gateway_call` | internal | remote_mutation | [] |
| `vox_openclaw_health` | internal | network | [] |
| `vox_openclaw_import_skill` | internal | writes_files | [] |
| `vox_openclaw_list_remote` | internal | network | [] |
| `vox_openclaw_notify` | internal | remote_mutation | [] |
| `vox_openclaw_search_remote` | internal | network | [] |
| `vox_openclaw_subscribe` | internal | remote_mutation | [] |
| `vox_openclaw_subscriptions` | internal | network | [] |
| `vox_openclaw_unsubscribe` | internal | remote_mutation | [] |
| `vox_secrets_doctor` | internal | read_only | [] |

Group note: `exposure` is fixed (`internal`) for every row — Task 14's tests pin the exact internal set; only `side_effect_class` and `requires` are subject to handler evidence. `vox_agy_doctor`, `vox_agy_ledger_digest`, `vox_credentials_status` are `http_read_role_eligible` and must stay `read_only` (V4). `vox_agy_delegate` is an anchor row. Existing `writes_files` on `vox_add_approval_allowlist_entry` is kept.

- [ ] **Step 1: Confirm the gate reports this group**

Run:
```bash
cargo run -q -p vox-cli -- ci operations-sync --target mcp 2>&1 | rg -c 'mcp `(vox_add_approval_allowlist_entry|vox_agent_gateway_call|vox_agent_list_remote|vox_agent_notify|vox_agent_subscribe|vox_agent_subscriptions|vox_agent_unsubscribe|vox_agy_delegate|vox_agy_delegate_batch|vox_agy_doctor|vox_agy_ledger_digest|vox_agy_pipeline|vox_agy_review|vox_credentials_status|vox_drain_agent|vox_emergency_stop|vox_gamify_battle_start|vox_gamify_battle_submit|vox_gamify_collegium_join|vox_gamify_notification_ack|vox_gamify_notifications_ack_all|vox_gamify_notifications_list|vox_gamify_progress_snapshot|vox_gamify_quest_list|vox_gamify_shop_buy|vox_gamify_shop_catalog|vox_openclaw_discover|vox_openclaw_gateway_call|vox_openclaw_health|vox_openclaw_import_skill|vox_openclaw_list_remote|vox_openclaw_notify|vox_openclaw_search_remote|vox_openclaw_subscribe|vox_openclaw_subscriptions|vox_openclaw_unsubscribe|vox_secrets_doctor)`'
```
Expected: `110`.

- [ ] **Step 2: Verify every proposal against its handler and write the evidence file**

For each tool: `rg -n '"<tool>" =>' crates/vox-orchestrator-mcp/src/dispatch.rs`, read the called function, apply the Classification Procedure. Write `target/plan01-evidence/task12.txt` in the evidence-file format.
Check: `rg -c ':[0-9]+ — ' target/plan01-evidence/task12.txt` — Expected: at least `29`.

- [ ] **Step 3: Apply the edits**

For each of the 37 ops: set `side_effect_class:` and append `exposure: internal` / `requires:` under `http_read_role_eligible:` as in the Edit-shape example.

- [ ] **Step 4: Re-run the gate**

Run the Step 1 command again. Expected: no output.

- [ ] **Step 5: Confirm only this group's ops changed**

Run: `git diff --stat` — Expected: only `contracts/operations/catalog.v1.yaml`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg -c '^\+    exposure: internal$'` — Expected: `37`.
Run: `git diff -U0 contracts/operations/catalog.v1.yaml | rg '^[-+]    name: '` — Expected: no output.

- [ ] **Step 6: Commit**

```bash
git add contracts/operations/catalog.v1.yaml
git commit -m "feat(catalog): classify internal-only MCP ops (agy/openclaw/gamify/...)" -m "Capability profiles plan 01 Task 12 (37 ops). Spec section 3 internal set plus the native agent-gateway family." -m "$(cat target/plan01-evidence/task12.txt)"
```

---

### Task 13: Regenerate derived registries with the new fields

**Files:**
- Modify: `contracts/mcp/tool-registry.schema.json` (whole file)
- Modify: `contracts/capability/capability-registry.schema.json` (`side_effect_class` enum, ≈L65–80)
- Regenerate: `contracts/mcp/tool-registry.canonical.yaml`, `contracts/capability/capability-registry.yaml`, `contracts/cli/command-registry.yaml` (expected unchanged), `contracts/capability/model-manifest.generated.json`, `contracts/reports/operations-catalog-inventory.v1.json`

**Interfaces:**
- Consumes: Tasks 3–12 merged (gate green).
- Produces: a canonical registry where every row has `exposure`, `side_effect_class`, `requires` and no `tier`.

Parallel-safe with: Task 15. Depends on: Tasks 3–12.

- [ ] **Step 1: Confirm the gate is green and the derived files are stale**

Run: `cargo run -q -p vox-cli -- ci operations-sync --target mcp`
Expected: fails with `…/contracts/mcp/tool-registry.canonical.yaml is stale; run \`vox ci operations-sync --target mcp --write\`` and no `MCP exposure-metadata problem(s)` line (if that line appears, a group task is unmerged or wrong).

- [ ] **Step 2: Update the tool-registry schema**

Replace `contracts/mcp/tool-registry.schema.json` with:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://voxlang.org/schemas/mcp/tool-registry.schema.json",
  "title": "MCP canonical tool registry",
  "type": "object",
  "additionalProperties": false,
  "required": ["version", "tools"],
  "properties": {
    "version": { "type": "integer", "const": 1 },
    "tools": {
      "type": "array",
      "minItems": 1,
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["name", "description", "product_lane", "exposure", "side_effect_class", "requires"],
        "properties": {
          "name": { "type": "string", "minLength": 1 },
          "description": { "type": "string", "minLength": 1 },
          "product_lane": {
            "type": "string",
            "enum": ["app", "workflow", "ai", "interop", "data", "platform"],
            "description": "Bell-curve product lane (parity with vox-cli command-registry product_lane)."
          },
          "http_read_role_eligible": {
            "type": "boolean",
            "description": "Optional flag for MCP HTTP read-role eligibility. When true, this tool can be included in read-token visibility/invocation after gateway allowlist/env intersections."
          },
          "exposure": {
            "type": "string",
            "enum": ["public", "operator", "internal"],
            "description": "Capability-profile exposure class (vox_mcp_registry::Exposure)."
          },
          "side_effect_class": {
            "type": "string",
            "enum": ["none", "read_only", "writes_files", "spawns_process", "network", "remote_mutation", "destructive"],
            "description": "Strongest effect any handler path can have (vox_mcp_registry::SideEffectClass)."
          },
          "requires": {
            "type": "array",
            "uniqueItems": true,
            "items": {
              "type": "string",
              "enum": ["db", "sessions", "agent_fleet", "model_registry", "llm_facade", "search_index", "memory", "research", "mesh", "attention", "clarification", "flywheel", "compiler", "browser", "speech"]
            },
            "description": "Subsystems that must be booted for the tool (vox_mcp_registry::Subsystem); [] when none."
          }
        }
      }
    }
  }
}
```

- [ ] **Step 3: Admit `remote_mutation` in the capability registry schema**

In `contracts/capability/capability-registry.schema.json`, in the `side_effect_class` string `enum`, insert `"remote_mutation",` between `"network",` and `"destructive",`.

- [ ] **Step 4: Regenerate**

```bash
cargo run -q -p vox-cli -- ci operations-sync --target all --write
cargo run -q -p vox-cli -- ci capability-sync --write
```
Expected: `wrote …/tool-registry.canonical.yaml`, `wrote …/command-registry.yaml`, `wrote …/capability-registry.yaml`, then the capability manifest write message.

- [ ] **Step 5: Verify**

```bash
cargo run -q -p vox-cli -- ci operations-verify
cargo run -q -p vox-cli -- ci operations-sync --target all
cargo run -q -p vox-cli -- ci capability-sync
cargo run -q -p vox-cli -- ci command-compliance
cargo run -q -p vox-cli -- ci gui-catalog-parity
rg -c '^  exposure: ' contracts/mcp/tool-registry.canonical.yaml
rg -n '^\s+tier:' contracts/mcp/tool-registry.canonical.yaml
cargo check -p vox-mcp-registry
cargo nextest run -p vox-cli command_compliance operations_catalog
```
Expected: `operations-verify OK (1298 catalog rows, inventory report updated)`; every sync/verify prints `OK …`; `command-compliance` and `gui-catalog-parity` exit 0; `340`; no output for `tier`; `vox-mcp-registry` still builds (its old `build.rs` defaults the now-missing `tier`); tests pass.

- [ ] **Step 6: Commit**

```bash
git add contracts/mcp/tool-registry.schema.json contracts/mcp/tool-registry.canonical.yaml contracts/capability/capability-registry.schema.json contracts/capability/capability-registry.yaml contracts/capability/model-manifest.generated.json contracts/cli/command-registry.yaml contracts/reports/operations-catalog-inventory.v1.json
git commit -m "chore(contracts): regenerate MCP registry with exposure metadata" -m "Capability profiles plan 01 Task 13. tool-registry.canonical.yaml rows now carry exposure, side_effect_class, requires (tier removed); schemas updated; capability projections regenerated. operations-verify is green again."
```

---

### Task 14: Typed registry rows and a strict `build.rs`

**Files:**
- Modify: `crates/vox-mcp-registry/build.rs` (whole file)
- Modify: `crates/vox-mcp-registry/src/lib.rs` (top block through the `include!`, test module appended)
- Modify: `crates/vox-mcp-registry/src/semcov_wave46_tests.rs` (L4–9 imports/consts, L127–147 tier tests)
- Modify: `crates/vox-mcp-registry/tests/tool_registry_smoke.rs` (L3–5 imports, L30)
- Create: `crates/vox-mcp-registry/tests/exposure_classification.rs`

**Interfaces:**
- Consumes: Task 13 canonical registry; Task 15 (no remaining `.tier` reader in `vox-orchestrator-mcp`).
- Produces (contract, verbatim):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpToolRegistryEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub product_lane: &'static str,
    pub http_read_role_eligible: bool,
    pub exposure: Exposure,
    pub side_effect_class: SideEffectClass,
    pub requires: &'static [Subsystem],
}
pub fn registry_entry(name: &str) -> Option<&'static McpToolRegistryEntry>;
```

Parallel-safe with: none (it changes a type every MCP crate consumes). Depends on: Tasks 13, 15.

- [ ] **Step 1: Write the failing tests**

Create `crates/vox-mcp-registry/tests/exposure_classification.rs`:

```rust
//! Exposure-classification invariants over the generated `TOOL_REGISTRY`.

use std::collections::{BTreeSet, HashSet};

use vox_mcp_registry::{
    Exposure, SideEffectClass, Subsystem, TOOL_REGISTRY, classify_tool_row, registry_entry,
};

/// Spec §3 initial `internal` set.
const SPEC_INTERNAL_PREFIXES: &[&str] = &["vox_agy_", "vox_openclaw_", "vox_gamify_"];
const SPEC_INTERNAL_SUFFIXES: &[&str] = &["_gateway_call"];
const SPEC_INTERNAL_EXACT: &[&str] = &[
    "vox_add_approval_allowlist_entry",
    "vox_emergency_stop",
    "vox_drain_agent",
    "vox_secrets_doctor",
    "vox_credentials_status",
];
/// Internal beyond the spec list: the native agent-gateway family and browser cookie transfer.
const RECORDED_INTERNAL_EXTENSIONS: &[&str] = &[
    "vox_agent_list_remote",
    "vox_agent_notify",
    "vox_agent_subscribe",
    "vox_agent_subscriptions",
    "vox_agent_unsubscribe",
    "vox_browser_cookies_export",
    "vox_browser_cookies_import",
];

fn in_spec_internal_set(name: &str) -> bool {
    SPEC_INTERNAL_PREFIXES.iter().any(|p| name.starts_with(p))
        || SPEC_INTERNAL_SUFFIXES.iter().any(|s| name.ends_with(s))
        || SPEC_INTERNAL_EXACT.contains(&name)
}

#[test]
fn spec_internal_set_is_internal() {
    let matched: Vec<_> = TOOL_REGISTRY
        .iter()
        .filter(|e| in_spec_internal_set(e.name))
        .collect();
    assert!(
        matched.len() >= 32,
        "spec §3 patterns matched only {} tools",
        matched.len()
    );
    for e in matched {
        assert_eq!(e.exposure, Exposure::Internal, "{} must be internal", e.name);
    }
}

#[test]
fn internal_tools_are_exactly_spec_set_plus_recorded_extensions() {
    let actual: BTreeSet<&str> = TOOL_REGISTRY
        .iter()
        .filter(|e| e.exposure == Exposure::Internal)
        .map(|e| e.name)
        .collect();
    let expected: BTreeSet<&str> = TOOL_REGISTRY
        .iter()
        .map(|e| e.name)
        .filter(|n| in_spec_internal_set(n) || RECORDED_INTERNAL_EXTENSIONS.contains(n))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn classification_is_populated_not_defaulted() {
    let exposures: BTreeSet<Exposure> = TOOL_REGISTRY.iter().map(|e| e.exposure).collect();
    assert_eq!(exposures.len(), 3, "all three exposure classes must be in use");
    let side_effects: HashSet<SideEffectClass> =
        TOOL_REGISTRY.iter().map(|e| e.side_effect_class).collect();
    assert!(
        side_effects.len() >= 5,
        "only {} side-effect classes in use",
        side_effects.len()
    );
    let with_requires = TOOL_REGISTRY.iter().filter(|e| !e.requires.is_empty()).count();
    assert!(
        with_requires * 3 >= TOOL_REGISTRY.len(),
        "only {with_requires} of {} tools declare requires",
        TOOL_REGISTRY.len()
    );
    for e in TOOL_REGISTRY {
        let req: Vec<String> = e.requires.iter().map(|s| s.as_str().to_string()).collect();
        classify_tool_row(
            e.http_read_role_eligible,
            Some(e.exposure.as_str()),
            Some(e.side_effect_class.as_str()),
            Some(req.as_slice()),
        )
        .unwrap_or_else(|errs| panic!("{}: {errs:?}", e.name));
    }
}

#[test]
fn anchor_tools_carry_expected_classification() {
    let cases: &[(&str, Exposure, SideEffectClass, &[Subsystem])] = &[
        ("vox_git_status", Exposure::Public, SideEffectClass::ReadOnly, &[]),
        (
            "vox_list_models",
            Exposure::Public,
            SideEffectClass::ReadOnly,
            &[Subsystem::ModelRegistry],
        ),
        (
            "vox_kb_query",
            Exposure::Public,
            SideEffectClass::ReadOnly,
            &[Subsystem::SearchIndex],
        ),
        ("vox_snapshot_restore", Exposure::Operator, SideEffectClass::Destructive, &[Subsystem::Db]),
        (
            "vox_browser_open",
            Exposure::Operator,
            SideEffectClass::Network,
            &[Subsystem::Browser],
        ),
        ("vox_agy_delegate", Exposure::Internal, SideEffectClass::SpawnsProcess, &[]),
    ];
    for &(name, exposure, side_effect_class, requires) in cases {
        let e = registry_entry(name).unwrap_or_else(|| panic!("{name} missing"));
        assert_eq!(e.exposure, exposure, "{name}");
        assert_eq!(e.side_effect_class, side_effect_class, "{name}");
        assert_eq!(e.requires, requires, "{name}");
    }
}
```

Append to `crates/vox-mcp-registry/src/lib.rs`:

```rust
#[cfg(test)]
mod registry_entry_tests {
    use super::*;

    #[test]
    fn registry_entry_finds_every_row_by_name() {
        for e in TOOL_REGISTRY {
            assert_eq!(registry_entry(e.name), Some(e));
        }
    }

    #[test]
    fn registry_entry_rejects_unknown_and_alias_names() {
        assert_eq!(registry_entry("vox_no_such_tool"), None);
        assert_eq!(
            registry_entry("vox_model_list"),
            None,
            "wire aliases resolve in vox-orchestrator-mcp, not here"
        );
    }
}
```

In `crates/vox-mcp-registry/src/semcov_wave46_tests.rs`: change the `use crate::{…}` list (L4–6) to

```rust
    use crate::{
        A2A_MESSAGE_TYPES, Exposure, McpToolRegistryEntry, ORCHESTRATOR_TOOLS, SKILL_TOOLS,
        SideEffectClass, Subsystem, TOOL_REGISTRY,
    };
```

delete the `const VALID_TIERS` line (L9), and replace the two tier tests (L127–147, `all_entries_have_non_empty_tier` and `all_tier_values_are_recognised`) with:

```rust
    #[test]
    // Catches: build.rs emitting a classification whose wire string no longer parses
    fn all_classification_values_round_trip() {
        for e in TOOL_REGISTRY {
            assert_eq!(Exposure::parse(e.exposure.as_str()), Some(e.exposure), "{}", e.name);
            assert_eq!(
                SideEffectClass::parse(e.side_effect_class.as_str()),
                Some(e.side_effect_class),
                "{}",
                e.name
            );
            for s in e.requires {
                assert_eq!(Subsystem::parse(s.as_str()), Some(*s), "{}", e.name);
            }
        }
    }

    #[test]
    // Catches: duplicate subsystem ids emitted into one tool's requires slice
    fn requires_has_no_duplicates() {
        for e in TOOL_REGISTRY {
            let uniq: std::collections::BTreeSet<_> = e.requires.iter().collect();
            assert_eq!(uniq.len(), e.requires.len(), "{}", e.name);
        }
    }
```

In `crates/vox-mcp-registry/tests/tool_registry_smoke.rs` change the import (L3–5) to

```rust
use vox_mcp_registry::{
    A2A_MESSAGE_TYPES, McpToolRegistryEntry, ORCHESTRATOR_TOOLS, SKILL_TOOLS, TOOL_REGISTRY,
    registry_entry,
};
```

and replace `assert!(!entry.tier.is_empty());` (L30) with `assert_eq!(registry_entry("vox_submit_task"), Some(entry));`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo nextest run -p vox-mcp-registry`
Expected: compile failure — `no field exposure on type &McpToolRegistryEntry`, `unresolved import vox_mcp_registry::registry_entry`.

- [ ] **Step 3: Replace `build.rs`**

Replace `crates/vox-mcp-registry/build.rs` with:

```rust
//! Emit `TOOL_REGISTRY` from `contracts/mcp/tool-registry.canonical.yaml`.

use std::env;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[allow(dead_code)]
#[path = "src/exposure.rs"]
mod exposure;
#[allow(dead_code)]
#[path = "src/subsystem.rs"]
mod subsystem;
#[allow(dead_code)]
#[path = "src/classification.rs"]
mod classification;

#[derive(Debug, Deserialize)]
struct Root {
    #[allow(dead_code)]
    version: u32,
    tools: Vec<ToolEntry>,
}

#[derive(Debug, Deserialize)]
struct ToolEntry {
    name: String,
    description: String,
    product_lane: String,
    #[serde(default)]
    http_read_role_eligible: bool,
    exposure: Option<String>,
    side_effect_class: Option<String>,
    requires: Option<Vec<String>>,
}

const PRODUCT_LANES: &[&str] = &["app", "workflow", "ai", "interop", "data", "platform"];

fn escape_rust_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("vox-mcp-registry must live at crates/vox-mcp-registry");
    let yaml_path = workspace_root.join("contracts/mcp/tool-registry.canonical.yaml");
    println!("cargo:rerun-if-changed={}", yaml_path.display());
    println!("cargo:rerun-if-changed=src/exposure.rs");
    println!("cargo:rerun-if-changed=src/subsystem.rs");
    println!("cargo:rerun-if-changed=src/classification.rs");

    let raw = fs::read_to_string(&yaml_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", yaml_path.display()));
    let root: Root =
        serde_yaml::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", yaml_path.display()));

    let mut names = std::collections::HashSet::<&str>::new();
    for t in &root.tools {
        if !names.insert(t.name.as_str()) {
            panic!("duplicate tool name in MCP registry: {}", t.name);
        }
        if !PRODUCT_LANES.contains(&t.product_lane.as_str()) {
            panic!(
                "unknown product_lane `{}` for MCP tool `{}`",
                t.product_lane, t.name
            );
        }
    }

    let mut lines = String::new();
    lines.push_str("// @generated by vox-mcp-registry/build.rs — do not edit.\n\n");
    lines.push_str("pub const TOOL_REGISTRY: &[McpToolRegistryEntry] = &[\n");
    for t in &root.tools {
        let class = classification::classify_tool_row(
            t.http_read_role_eligible,
            t.exposure.as_deref(),
            t.side_effect_class.as_deref(),
            t.requires.as_deref(),
        )
        .unwrap_or_else(|errs| {
            panic!(
                "MCP tool `{}` in {}: {} (fix contracts/operations/catalog.v1.yaml, then `vox ci operations-sync --target mcp --write`)",
                t.name,
                yaml_path.display(),
                errs.join("; ")
            )
        });
        let en = escape_rust_string(&t.name);
        let ed = escape_rust_string(&t.description);
        let el = escape_rust_string(&t.product_lane);
        let read_eligible = t.http_read_role_eligible;
        let requires = class
            .requires
            .iter()
            .map(|s| format!("Subsystem::{s:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push_str(&format!(
            "    McpToolRegistryEntry {{ name: \"{en}\", description: \"{ed}\", product_lane: \"{el}\", http_read_role_eligible: {read_eligible}, exposure: Exposure::{:?}, side_effect_class: SideEffectClass::{:?}, requires: &[{requires}] }},\n",
            class.exposure, class.side_effect_class
        ));
    }
    lines.push_str("];\n");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("tool_registry.rs"), lines).expect("write tool_registry.rs");
}
```

- [ ] **Step 4: Replace the registry struct in `lib.rs`**

Replace the top of `crates/vox-mcp-registry/src/lib.rs` — from line 1 through the `include!(concat!(env!("OUT_DIR"), "/tool_registry.rs"));` line, which now also contains the Task 1 module block — with:

```rust
//! MCP tool registry built from `contracts/mcp/tool-registry.canonical.yaml`.
//!
//! Consume [`TOOL_REGISTRY`] instead of duplicating name/description tables.

mod classification;
mod exposure;
mod subsystem;

pub use classification::{ToolClassification, classify_tool_row};
pub use exposure::{Exposure, SideEffectClass};
pub use subsystem::{Subsystem, subsystem_closure};

/// One MCP tool row: identity, bell-curve [`product_lane`](McpToolRegistryEntry::product_lane),
/// and capability-profile exposure metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpToolRegistryEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub product_lane: &'static str,
    pub http_read_role_eligible: bool,
    pub exposure: Exposure,
    pub side_effect_class: SideEffectClass,
    pub requires: &'static [Subsystem],
}

include!(concat!(env!("OUT_DIR"), "/tool_registry.rs"));

/// Row for an exact canonical tool name; wire aliases are not resolved here.
pub fn registry_entry(name: &str) -> Option<&'static McpToolRegistryEntry> {
    TOOL_REGISTRY.iter().find(|e| e.name == name)
}
```

Check: `rg -c '^mod (classification|exposure|subsystem);' crates/vox-mcp-registry/src/lib.rs` → `3`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo nextest run -p vox-mcp-registry`
Expected: all tests PASS, including the 4 in `exposure_classification.rs`, the 2 `registry_entry_tests`, and the 2 new semcov tests.

- [ ] **Step 6: Check every consumer still compiles**

```bash
cargo check -p vox-mcp-registry -p vox-orchestrator-mcp -p vox-orchestrator -p vox-cli -p vox-corpus -p vox-skill-discovery
cargo clippy -p vox-mcp-registry --all-targets -- -D warnings
rg -n '\btier\b' crates/vox-mcp-registry
```
Expected: clean builds; no clippy warnings; no `tier` hits.

- [ ] **Step 7: Format and commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-mcp-registry/build.rs crates/vox-mcp-registry/src/lib.rs crates/vox-mcp-registry/src/semcov_wave46_tests.rs crates/vox-mcp-registry/tests/tool_registry_smoke.rs crates/vox-mcp-registry/tests/exposure_classification.rs
git commit -m "feat(mcp-registry): typed exposure fields on TOOL_REGISTRY; drop tier" -m "Capability profiles plan 01 Task 14. build.rs now compiles the shared classification module and panics on a missing or unknown exposure/side_effect_class/requires; McpToolRegistryEntry carries the typed values and registry_entry() looks rows up by name. Tests pin the spec internal set and anchor classifications."
```

---

### Task 15: Move the workspace-tool opt-in to `VOX_MCP_WORKSPACE_TOOLS`; `VOX_MCP_TIERS` becomes a deprecated alias

Per `00-contracts.md` Amendments ("From plan 01"): today's default (workspace-federated tools hidden from
`tools/list`) is preserved; the opt-in moves from `VOX_MCP_TIERS` to `VOX_MCP_WORKSPACE_TOOLS=1`;
`VOX_MCP_TIERS` is still honoured as a **deprecated alias** with a one-time warning. Catalog tools never
carried a non-`core` tier, so the tier filter collapses to "show or hide workspace-federated tools".

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/server.rs` — `list_tools` (L210–224 tier filter, L234 skill meta key), new helpers before `#[cfg(test)] mod call_tool_tests` (L418), new test module at end
- Modify: `crates/vox-orchestrator-mcp/src/registry.rs` — L70 (delete), L92 (meta key), L112–119 (test)
- Modify: `contracts/config/env-vars.v1.yaml` — the `VOX_MCP_TIERS` entry (≈L1180–1185) and a new `VOX_MCP_WORKSPACE_TOOLS` entry directly after it
- Modify: `contracts/config/registry.v1.yaml` — new `mcp_workspace_tools` knob (append at the end of `knobs:`); required because `config-hygiene` / `config-registry-parity` resolve registered names from `registry.v1.yaml` + Clavis + `CONFIG_KEYS`, **not** from `env-vars.v1.yaml`
- Modify: `docs/src/reference/env-vars.md` — MCP section (after the `VOX_MCP_TEST_INFER_STUB_BODY` row, ≈L262)
- Modify: `docs/src/architecture/mcp-vox-language-exposure.md` — "## MCP tier policy" section (≈L44–54)

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `pub(crate) enum WorkspaceToolsOptIn { Off, On, OnViaDeprecatedTiers }` and
    `pub(crate) fn workspace_tools_opt_in(workspace_tools: Option<&str>, legacy_tiers: Option<&str>) -> WorkspaceToolsOptIn` in `server.rs` (pure; env values passed in so tests never mutate process env)
  - `pub(crate) fn retain_advertised_tools(tools: &mut Vec<rmcp::model::Tool>, include_workspace: bool)` in `server.rs`
  - Tool meta key `vox_source` (`"workspace"` | `"skill"`) replaces `vox_tier`; catalog tools no longer carry `vox_tier`.
  - Behaviour: unset ⇒ `tools/list` equals today's default (all compiled catalog tools + skill macro tools; workspace-federated tools dispatchable but not advertised). `VOX_MCP_WORKSPACE_TOOLS=1|true` ⇒ workspace-federated tools are also advertised. If `VOX_MCP_WORKSPACE_TOOLS` is set to any value, it wins over `VOX_MCP_TIERS`. Otherwise `VOX_MCP_TIERS` containing `workspace` or `all` (the values that revealed them before) turns the opt-in on; `dev`, `advanced` and other values keep today's behaviour (hidden). Whenever `VOX_MCP_TIERS` is set, one `tracing::warn!` per process names `VOX_MCP_WORKSPACE_TOOLS` as the replacement.
  - Plan 02 restricts workspace/skill tools to the `dev` profile through `guard_tool` (non-registry names are `ToolNotInProfile` under any named profile); this task does not know about profiles.

Parallel-safe with: Tasks 1–13. Must land before Task 14.

- [ ] **Step 1: Write the failing tests**

Append to `crates/vox-orchestrator-mcp/src/server.rs`:

```rust
#[cfg(test)]
mod list_filter_tests {
    use std::borrow::Cow;
    use std::sync::Arc;

    use rmcp::model::{Meta, Tool};

    use super::{retain_advertised_tools, workspace_tools_opt_in, WorkspaceToolsOptIn};

    fn tool(name: &'static str, source: Option<&str>) -> Tool {
        let mut meta = serde_json::Map::new();
        if let Some(s) = source {
            meta.insert("vox_source".into(), serde_json::Value::String(s.into()));
        }
        Tool::new_with_raw(
            Cow::Borrowed(name),
            Some(Cow::Borrowed("test tool")),
            Arc::new(serde_json::Map::new()),
        )
        .with_meta(Meta(meta))
    }

    fn sample() -> Vec<Tool> {
        vec![
            tool("vox_git_status", None),
            tool("read_file", Some("workspace")),
            tool("skill_macro", Some("skill")),
        ]
    }

    fn names(tools: &[Tool]) -> Vec<&str> {
        tools.iter().map(|t| t.name.as_ref()).collect()
    }

    #[test]
    fn default_hides_workspace_federated_and_keeps_catalog_and_skill_tools() {
        let mut tools = sample();
        retain_advertised_tools(&mut tools, false);
        assert_eq!(names(&tools), vec!["vox_git_status", "skill_macro"]);
    }

    #[test]
    fn opt_in_keeps_workspace_federated_tools() {
        let mut tools = sample();
        retain_advertised_tools(&mut tools, true);
        assert_eq!(names(&tools), vec!["vox_git_status", "read_file", "skill_macro"]);
    }

    #[test]
    fn opt_in_resolution() {
        use WorkspaceToolsOptIn::*;
        assert_eq!(workspace_tools_opt_in(None, None), Off);
        assert_eq!(workspace_tools_opt_in(Some("1"), None), On);
        assert_eq!(workspace_tools_opt_in(Some("true"), None), On);
        assert_eq!(workspace_tools_opt_in(Some("0"), None), Off);
        // The new variable wins over the deprecated alias when set.
        assert_eq!(workspace_tools_opt_in(Some("0"), Some("all")), Off);
        assert_eq!(workspace_tools_opt_in(None, Some("core")), Off);
        // Not widened: these never revealed workspace tools and still do not.
        assert_eq!(workspace_tools_opt_in(None, Some("dev")), Off);
        assert_eq!(workspace_tools_opt_in(None, Some("advanced")), Off);
        for legacy in ["workspace", "all", "core,workspace", " all "] {
            assert_eq!(workspace_tools_opt_in(None, Some(legacy)), OnViaDeprecatedTiers, "{legacy}");
        }
    }
}
```

In `crates/vox-orchestrator-mcp/src/registry.rs` replace the test `merged_workspace_tools_carry_workspace_tier_meta` (L112–119) with:

```rust
    #[tokio::test]
    async fn merged_workspace_tools_carry_workspace_source_meta() {
        let state = ServerState::new_full(OrchestratorConfig::default());
        let tools = merged_tool_registry(&state);
        let read_file = tools.iter().find(|t| t.name == "read_file").unwrap();
        let meta = &read_file.meta.as_ref().unwrap().0;
        assert_eq!(meta["vox_source"], "workspace");
        assert!(meta.get("vox_tier").is_none());
    }

    #[test]
    fn catalog_tools_no_longer_carry_tier_meta() {
        for t in super::tool_registry() {
            let meta = &t.meta.as_ref().expect("catalog tools carry meta").0;
            assert!(meta.get("vox_tier").is_none(), "{}", t.name);
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo nextest run -p vox-orchestrator-mcp list_filter merged_workspace_tools catalog_tools_no_longer`
Expected: compile failure `cannot find function retain_advertised_tools in module super` (and `workspace_tools_opt_in`).

- [ ] **Step 3: Implement in `server.rs`**

Replace the tier block in `list_tools` (from the `// E.56 MCP tool tiering` comment through the closing `}` of `if !allowed_tiers.contains(&"all") { … }`, L210–224) with:

```rust
        let opt_in = workspace_tools_opt_in(
            std::env::var("VOX_MCP_WORKSPACE_TOOLS").ok().as_deref(),
            std::env::var("VOX_MCP_TIERS").ok().as_deref(),
        );
        warn_if_deprecated_tiers_env_set();
        retain_advertised_tools(&mut tool_list, opt_in != WorkspaceToolsOptIn::Off);
```

Change the skill meta key (L234) from `"vox_tier".to_string()` to `"vox_source".to_string()`.

Insert before `#[cfg(test)] mod call_tool_tests` (L418):

```rust
/// How the workspace-federated `@tool` opt-in was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceToolsOptIn {
    Off,
    On,
    OnViaDeprecatedTiers,
}

/// `VOX_MCP_WORKSPACE_TOOLS` (when set) decides; otherwise the deprecated `VOX_MCP_TIERS`
/// alias opts in only for `workspace` or `all` (the values that revealed them before).
pub(crate) fn workspace_tools_opt_in(
    workspace_tools: Option<&str>,
    legacy_tiers: Option<&str>,
) -> WorkspaceToolsOptIn {
    if let Some(v) = workspace_tools {
        return match v.trim() {
            "1" | "true" => WorkspaceToolsOptIn::On,
            _ => WorkspaceToolsOptIn::Off,
        };
    }
    let legacy_on = legacy_tiers.is_some_and(|v| {
        v.split(',')
            .map(str::trim)
            .any(|t| matches!(t, "workspace" | "all"))
    });
    if legacy_on {
        WorkspaceToolsOptIn::OnViaDeprecatedTiers
    } else {
        WorkspaceToolsOptIn::Off
    }
}

/// Workspace-federated `@tool` functions are always dispatchable by name; they are advertised in
/// `tools/list` only when `include_workspace` is set.
pub(crate) fn retain_advertised_tools(tools: &mut Vec<rmcp::model::Tool>, include_workspace: bool) {
    if include_workspace {
        return;
    }
    tools.retain(|t| {
        t.meta
            .as_ref()
            .and_then(|m| m.0.get("vox_source"))
            .and_then(serde_json::Value::as_str)
            != Some("workspace")
    });
}

// vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="capability-profiles" canonical="VOX_MCP_WORKSPACE_TOOLS"
fn warn_if_deprecated_tiers_env_set() {
    static WARNED: std::sync::Once = std::sync::Once::new();
    if std::env::var_os("VOX_MCP_TIERS").is_some() {
        WARNED.call_once(|| {
            tracing::warn!(
                "VOX_MCP_TIERS is deprecated; use VOX_MCP_WORKSPACE_TOOLS=1 to advertise workspace-federated tools"
            );
        });
    }
}
```

- [ ] **Step 4: Implement in `registry.rs`**

Delete line 70 (`meta_map.insert("vox_tier".to_string(), Value::String(e.tier.to_string()));`).
In `merged_tool_registry`, change `"vox_tier".to_string()` (L92) to `"vox_source".to_string()`.

- [ ] **Step 5: Update env-var SSOT, config registry, and docs**

In `contracts/config/env-vars.v1.yaml`, set the `VOX_MCP_TIERS` entry's `description` value to:

```yaml
    description: "DEPRECATED (2026-10-08): alias for VOX_MCP_WORKSPACE_TOOLS. Values workspace or all advertise workspace-federated MCP tools (other values, including dev and advanced, do not); a warning is logged whenever it is set. Ignored when VOX_MCP_WORKSPACE_TOOLS is set."
```

and insert directly after that entry:

```yaml
  - name: "VOX_MCP_WORKSPACE_TOOLS"
    owner_crate: "vox-orchestrator-mcp"
    kind: "bool"
    required: false
    introduced_in: "0.6.0"
    default: "0"
    description: "1 = advertise workspace-federated @tool functions in MCP tools/list (they are always dispatchable by name). Served only under the dev capability profile."
```


In `contracts/config/registry.v1.yaml`, append to `knobs:`:

```yaml
  - name: mcp_workspace_tools
    env_var: VOX_MCP_WORKSPACE_TOOLS
    description: Advertise workspace-federated @tool functions in MCP tools/list (dev profile only). Replaces the deprecated VOX_MCP_TIERS opt-in.
    owner_crate: vox-orchestrator-mcp
    status: active
    default_value: "0"
    since: "2026-10-08"
```

In `docs/src/reference/env-vars.md`, add after the `VOX_MCP_TEST_INFER_STUB_BODY` row:

```markdown
| `VOX_MCP_WORKSPACE_TOOLS` | `1` = also advertise workspace-federated `@tool` functions in MCP `tools/list` (they are always dispatchable by name). Only served under the `dev` capability profile. Default off. |
| `VOX_MCP_TIERS` | **DEPRECATED (2026-10-08):** alias for `VOX_MCP_WORKSPACE_TOOLS`; only `workspace` or `all` opt in. Logs a warning when set; ignored when `VOX_MCP_WORKSPACE_TOOLS` is set. |
```

In `docs/src/architecture/mcp-vox-language-exposure.md`, replace the whole `## MCP tier policy` section (heading, sentence, table, and the `VOX_MCP_TIERS=all` line) with:

```markdown
## MCP tool visibility

`tools/list` advertises every catalog tool compiled into the server plus instructional macro tools
from installed skills (`vox_source: skill`). Federated `@tool` functions (`vox_source: workspace`)
are dispatchable by name; set `VOX_MCP_WORKSPACE_TOOLS=1` to also advertise them.

`VOX_MCP_TIERS` is a deprecated alias for that opt-in (`workspace` or `all` only);
setting it logs a warning.
```

- [ ] **Step 6: Run the tests and checks**

```bash
cargo nextest run -p vox-orchestrator-mcp list_filter merged_workspace_tools catalog_tools_no_longer call_tool_tests prompt_tests
cargo check -p vox-orchestrator-mcp
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
cargo run -q -p vox-cli -- ci config-hygiene
cargo run -q -p vox-cli -- ci config-registry-parity
cargo run -q -p vox-cli -- ci retirement-audit
cargo run -p vox-doc-pipeline -- --lint-only --paths reference/env-vars.md architecture/mcp-vox-language-exposure.md
```
Expected: tests PASS; clean build and clippy; `config-hygiene` reports no new violation (the `server.rs|VOX_MCP_TIERS` key stays grandfathered because the read stays in `server.rs`; `VOX_MCP_WORKSPACE_TOOLS` is registered in `registry.v1.yaml`); `config-registry-parity` OK; `retirement-audit` lists the new `capability-profiles` marker without failing; doc lint OK.

- [ ] **Step 7: Format and commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/server.rs crates/vox-orchestrator-mcp/src/registry.rs contracts/config/env-vars.v1.yaml contracts/config/registry.v1.yaml docs/src/reference/env-vars.md docs/src/architecture/mcp-vox-language-exposure.md
git commit -m "refactor(mcp): VOX_MCP_WORKSPACE_TOOLS opt-in; VOX_MCP_TIERS deprecated alias" -m "Capability profiles plan 01 Task 15. The tier filter collapses to the only thing it ever did: show or hide workspace-federated tools. Default unchanged (hidden); VOX_MCP_WORKSPACE_TOOLS=1 opts in; VOX_MCP_TIERS keeps working as a deprecated alias with a one-time warning. vox_tier meta becomes vox_source."
```

---

### Task 16: Exposure metadata on advertised tools and docs pointers

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/registry.rs` — `tool_registry()` meta block (where L70 used to be), test module
- Modify: `crates/vox-orchestrator-mcp/src/lib.rs:199` (add `pub mod profiles` after `pub use vox_mcp_registry::TOOL_REGISTRY;`)
- Modify: `docs/src/architecture/mcp-vox-language-exposure.md` ("MCP tool visibility" section)
- Modify: `docs/src/architecture/where-things-live.md` (L107 `vox-mcp-registry` row, L214 "Add an MCP tool" row)

**Interfaces:**
- Consumes: Task 14 (`McpToolRegistryEntry` typed fields), Task 15.
- Produces:
  - Tool meta keys on every catalog tool: `vox_exposure` (string), `vox_side_effect_class` (string), `vox_requires` (array of subsystem strings).
  - `vox_orchestrator_mcp::profiles::{Exposure, McpToolRegistryEntry, SideEffectClass, Subsystem, registry_entry, subsystem_closure}` (plan 02 extends this module).

Parallel-safe with: none. Depends on: Tasks 14, 15.

- [ ] **Step 1: Write the failing test**

Append to the test module in `crates/vox-orchestrator-mcp/src/registry.rs`:

```rust
    #[test]
    fn catalog_tools_carry_exposure_metadata() {
        let tools = super::tool_registry();
        let git = tools
            .iter()
            .find(|t| t.name == "vox_git_status")
            .expect("vox_git_status advertised");
        let meta = &git.meta.as_ref().expect("meta").0;
        assert_eq!(meta["vox_exposure"], "public");
        assert_eq!(meta["vox_side_effect_class"], "read_only");
        assert_eq!(meta["vox_requires"], serde_json::json!([]));
        let models = tools
            .iter()
            .find(|t| t.name == "vox_list_models")
            .expect("vox_list_models advertised");
        assert_eq!(
            models.meta.as_ref().expect("meta").0["vox_requires"],
            serde_json::json!(["model_registry"])
        );
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo nextest run -p vox-orchestrator-mcp catalog_tools_carry_exposure_metadata`
Expected: FAIL — `assertion left == right failed` (`meta["vox_exposure"]` is `Null`, expected `"public"`).

- [ ] **Step 3: Implement**

In `tool_registry()`, after the `vox_http_read_role_eligible` insert, add:

```rust
            meta_map.insert(
                "vox_exposure".to_string(),
                Value::String(e.exposure.as_str().to_string()),
            );
            meta_map.insert(
                "vox_side_effect_class".to_string(),
                Value::String(e.side_effect_class.as_str().to_string()),
            );
            meta_map.insert(
                "vox_requires".to_string(),
                Value::Array(
                    e.requires
                        .iter()
                        .map(|s| Value::String(s.as_str().to_string()))
                        .collect(),
                ),
            );
```

In `crates/vox-orchestrator-mcp/src/lib.rs`, after `pub use vox_mcp_registry::TOOL_REGISTRY;` add:

```rust
/// Capability-profile vocabulary shared by stdio MCP, the HTTP gateway, and the daemon.
pub mod profiles {
    pub use vox_mcp_registry::{
        Exposure, McpToolRegistryEntry, SideEffectClass, Subsystem, registry_entry,
        subsystem_closure,
    };
}
```

In `docs/src/architecture/mcp-vox-language-exposure.md`, append to the "MCP tool visibility" section's first paragraph:

```markdown
Each catalog tool's `_meta` carries `vox_exposure`, `vox_side_effect_class`, and `vox_requires`,
projected from `mcp.exposure`, `side_effect_class`, and `mcp.requires` in
`contracts/operations/catalog.v1.yaml`.
```

In `docs/src/architecture/where-things-live.md`:
- L107: change the `vox-mcp-registry` description to `Compile-time MCP tool registry from contracts YAML (SSOT), plus the exposure / side-effect / subsystem vocabulary (Exposure, SideEffectClass, Subsystem).`
- L214 "Add an MCP tool": append `; add its row to [`catalog.v1.yaml`](../../../contracts/operations/catalog.v1.yaml) with `mcp.exposure`, `mcp.requires`, and `side_effect_class` (gate: `vox ci operations-sync --target mcp`)`.

- [ ] **Step 4: Run tests and checks**

```bash
cargo nextest run -p vox-orchestrator-mcp catalog_tools_carry_exposure_metadata catalog_tools_no_longer merged_workspace_tools list_filter
cargo check -p vox-orchestrator-mcp
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/mcp-vox-language-exposure.md architecture/where-things-live.md
```
Expected: PASS / clean / lint OK.

- [ ] **Step 5: Format and commit**

```bash
vox run scripts/fmt.vox
git add crates/vox-orchestrator-mcp/src/registry.rs crates/vox-orchestrator-mcp/src/lib.rs docs/src/architecture/mcp-vox-language-exposure.md docs/src/architecture/where-things-live.md
git commit -m "feat(mcp): advertise exposure metadata; add profiles re-export" -m "Capability profiles plan 01 Task 16. Catalog tools carry vox_exposure, vox_side_effect_class and vox_requires in _meta; vox_orchestrator_mcp::profiles re-exports the vocabulary for plan 02."
```

---

### Task 17: Mutation-verify the gates and sweep for stragglers

**Files:**
- No intended changes. Temporary mutations of `contracts/operations/catalog.v1.yaml`, `contracts/mcp/tool-registry.canonical.yaml`, and `crates/vox-mcp-registry/src/classification.rs` are reverted within the task.

**Interfaces:**
- Consumes: Tasks 1–16.
- Produces: evidence that each gate fails closed, and a zero-straggler reference sweep.

Parallel-safe with: none (it mutates shared files temporarily). Depends on: Task 16.

- [ ] **Step 1: Presence check before mutating**

```bash
git status --short
rg -c '^    exposure: ' contracts/operations/catalog.v1.yaml
rg -n -A3 '^    name: vox_git_status$' contracts/operations/catalog.v1.yaml
```
Expected: clean tree; `340`; the block shows `http_read_role_eligible: true`, `exposure: public`, `requires: []`.

- [ ] **Step 2: Mutation A — operations-sync gate**

Delete the single line `    exposure: public` that follows `    name: vox_git_status` / `    http_read_role_eligible: true`. Then:

```bash
rg -c '^    exposure: ' contracts/operations/catalog.v1.yaml
cargo run -q -p vox-cli -- ci operations-sync --target mcp --write; echo "exit=$?"
git diff --stat contracts/mcp/tool-registry.canonical.yaml
cargo run -q -p vox-cli -- ci operations-verify; echo "exit=$?"
```
Expected: `339`; operations-sync prints ``1 MCP exposure-metadata problem(s) in contracts/operations/catalog.v1.yaml`` and ``op `git.status` (mcp `vox_git_status`): missing `exposure` (public|operator|internal)`` with `exit=1`; the canonical registry is **not** rewritten (empty diff stat); operations-verify also exits non-zero.

Restore: `git checkout -- contracts/operations/catalog.v1.yaml`, then `rg -c '^    exposure: ' contracts/operations/catalog.v1.yaml` → `340`.

- [ ] **Step 3: Mutation B — `build.rs` gate**

In `contracts/mcp/tool-registry.canonical.yaml` (mutation only — this file is never hand-edited otherwise) delete the `  exposure: public` line of the `vox_git_status` row. Then:

```bash
cargo check -p vox-mcp-registry 2>&1 | rg 'MCP tool `vox_git_status`'
```
Expected: the build script panics with ``MCP tool `vox_git_status` in …/tool-registry.canonical.yaml: missing `exposure` …``.

Restore: `git checkout -- contracts/mcp/tool-registry.canonical.yaml`; `cargo check -p vox-mcp-registry` → clean.

- [ ] **Step 4: Mutation C — the shared validator**

```bash
rg -c 'http_read_role_eligible tool must be side-effect free' crates/vox-mcp-registry/src/classification.rs
```
Expected: `1`. Change the condition `if http_read_role_eligible && (!side_effect_free || e == Exposure::Operator) {` to `if false {`. Then:

```bash
cargo nextest run -p vox-mcp-registry read_role_tools_must_be_side_effect_free
```
Expected: FAIL (`assertion failed: classify_tool_row(true, Some("public"), Some("writes_files"), …).is_err()`).

Restore: `git checkout -- crates/vox-mcp-registry/src/classification.rs`; rerun the `rg -c` → `1`; rerun the test → PASS.

- [ ] **Step 5: Reference sweep**

Run each command; the expected result follows it.

```bash
rg -n '^\s+tier:' contracts/operations/catalog.v1.yaml contracts/mcp/tool-registry.canonical.yaml
```
Expected: no output.

```bash
rg -n '"tier"' contracts/mcp/tool-registry.schema.json contracts/operations/catalog.v1.schema.json
```
Expected: no output.

```bash
rg -n '\btier\b' crates/vox-mcp-registry
```
Expected: no output.

```bash
rg -n 'default_mcp_tier|default_tier|VALID_TIERS' crates docs/src contracts scripts --glob '!docs/src/archive/**'
```
Expected: no output.

```bash
rg -n 'vox_tier' crates docs/src contracts scripts --glob '!docs/src/archive/**'
```
Expected exactly two hits, both absence assertions in `crates/vox-orchestrator-mcp/src/registry.rs` (`meta.get("vox_tier").is_none()` in `merged_workspace_tools_carry_workspace_source_meta` and `catalog_tools_no_longer_carry_tier_meta`).

```bash
rg -n '\.tier\b' crates/vox-orchestrator-mcp/src/registry.rs crates/vox-cli/src/commands/ci/operations_catalog.rs
```
Expected: no output.

```bash
rg -n 'std::env::var\("VOX_MCP_TIERS"\)' crates
```
Expected: exactly one hit, the deprecated-alias read in `crates/vox-orchestrator-mcp/src/server.rs` `list_tools` (next to the `VOX_MCP_WORKSPACE_TOOLS` read).

```bash
rg -l 'VOX_MCP_TIERS' crates contracts docs/src scripts --glob '!docs/src/archive/**' | sort
```
Expected exactly these 7 files (registry/baseline entries, the deprecated-alias read, the env and visibility docs, and one historical findings doc):

```text
contracts/config/config-hygiene-baseline.txt
contracts/config/config-registry-baseline.txt
contracts/config/env-vars.v1.yaml
crates/vox-orchestrator-mcp/src/server.rs
docs/src/architecture/cross-project-environment-variable-handoff-findings-2026-09-01.md
docs/src/architecture/mcp-vox-language-exposure.md
docs/src/reference/env-vars.md
```

```bash
rg -n 'McpToolRegistryEntry \{' crates
```
Expected exactly two hits: the struct definition in `crates/vox-mcp-registry/src/lib.rs` and the format string in `crates/vox-mcp-registry/build.rs`.

```bash
rg -c '^  exposure: ' contracts/mcp/tool-registry.canonical.yaml
rg -n 'vox-deprecated-since=.*reason="capability-profiles"' crates
```
Expected: `340`; exactly one hit in `crates/vox-orchestrator-mcp/src/server.rs`.

Any unexpected hit is a missed consumer: fix it in a follow-up commit in this task (message `fix(mcp): remove leftover tier reference in crates/…` naming the file), then rerun the whole sweep.

- [ ] **Step 6: Final checks**

```bash
cargo nextest run -p vox-mcp-registry
cargo nextest run -p vox-cli operations_catalog classification_gate ensure_reports command_compliance
cargo nextest run -p vox-orchestrator-mcp registry list_filter call_tool_tests
cargo clippy -p vox-mcp-registry -p vox-orchestrator-mcp -p vox-cli --all-targets -- -D warnings
cargo run -q -p vox-cli -- ci operations-verify
cargo run -q -p vox-cli -- ci ssot-drift
vox ci line-endings
VOX_FMT_CHECK=1 vox run scripts/fmt.vox
git status --short
```
Expected: all PASS / OK; `git status` clean. No commit is needed unless Step 5 found a straggler.

---

## Self-Review

| Requirement (spec / contract) | Task(s) |
|---|---|
| §3 every op gains a required `exposure` | 2 (schema + gate), 3–12 (values), 13 (projection) |
| §3 `side_effect_class` becomes required (non-null for MCP ops) | 2 (schema `if/then` + gate), 3–12 |
| §5 each catalog op declares `requires: [..]` | 2, 3–12 (mapping table in the Classification Procedure) |
| §3 `operations-sync` fails if any op lacks `exposure` / `side_effect_class` / `requires` | 2 (gate), 14 (`build.rs` second gate), 17 (mutations A/B) |
| §3 initial `internal` set | 12 (values), 14 (`spec_internal_set_is_internal`, exact-set test) |
| §3 `tier` removed from the registry schema | 2 (catalog + schema), 13 (`tool-registry.schema.json`), 14 (struct, tests), 17 (sweep) |
| §3 `VOX_MCP_TIERS` retired with deprecation marker (Amendment: kept as deprecated alias of `VOX_MCP_WORKSPACE_TOOLS`) | 15 (opt-in resolver, marker, warn-once, env SSOT + `registry.v1.yaml`, docs), 17 (sweep) |
| §9.1 contract tests in the registry generator | 1 (`classify_tool_row` tests), 2 (gate tests), 14 (registry-wide tests) |
| §9.2 mutation-verified exposure guard | 17 (mutations A, B, C with presence checks) |
| 00-contracts `Exposure`, `SideEffectClass`, `Subsystem`, `subsystem_closure` verbatim | 1 |
| 00-contracts `McpToolRegistryEntry` (no `tier`) + `registry_entry` | 14 |
| 00-contracts `vox_orchestrator_mcp::profiles::*` re-export | 16 |
| 00-contracts catalog / canonical / schema shapes | 2, 13 |
| 00-contracts env rows for `VOX_MCP_TIERS` (deprecated alias) and `VOX_MCP_WORKSPACE_TOOLS` | 15 |
| Classification of all 340 ops, split by domain, with handler evidence | 3–12 (41+53+32+52+38+25+15+17+30+37 = 340) |
| Reference sweep with exact expected results | 17 Step 5 |
| Global constraint: `dev` reproduces today's behaviour | 15 (`tools/list` with both vars unset equals the old `VOX_MCP_TIERS`-unset default; `workspace`/`all` still opt in, `dev`/`advanced` do not) |
| Global constraint: no new crate edges | 1 (only external `serde` / `serde_json`) |

Out of scope here (later plans): profile manifests and the "profile tools covered by subsystem closure" generator check (02), runtime filtering/dispatch denial (02–04), boot gating (03).

## Risks

1. **Proposals are not evidence.** The per-group tables come from names, descriptions, existing values, and a first pass over handlers; some rows will change on full handler review (especially `requires` for graph search, and the LLM-or-not question for `vox_db_suggest_query`, `vox_browser_extract_json`, `vox_speech_to_code`). The procedure forces evidence and an `Overrides:` record, and the gate rejects invariant violations, but reviewers should read the evidence in each commit body rather than the tables.
2. **Intentional red window.** Between Task 2 and Task 13, `operations-verify` and `ssot-drift` fail. Pushing in that window would trip the pre-push hook and CI. Mitigation: integrate Tasks 3–12 quickly; push only after Task 13.
3. **Deprecated alias is not widened.** Only the `VOX_MCP_TIERS` values that reveal workspace-federated tools today (`workspace`, `all`) opt in through the alias; `dev`, `advanced`, `core` and anything else keep today's behaviour (workspace tools stay hidden). Pinned by `opt_in_resolution`.
4. **`side_effect_class` naming.** `writes_files` also covers in-process orchestrator/session state, and the precedence ranks local writes above `network`, so tools that both call an LLM and persist read as `writes_files`. The enum is fixed by the contract; the convention is documented in the Classification Procedure and should be copied into the contract doc.
5. **Schema versioning.** The catalog schema change is breaking for a reader that relied on `tier` or on optional `side_effect_class`. `x-vox-version` is bumped, but the file is not renamed (`catalog.v1.*`) because it has one in-repo producer and consumer; renaming paths across tooling is not worth it. Flag for review against the data-storage policy.
6. **Large-file edits.** `catalog.v1.yaml` is 33.5k lines; line numbers shift with every group merge. All instructions locate ops by their unique `name:` line, never by line number.
7. **`workspace-hack` drift.** Adding `serde` to `vox-mcp-registry` may require `cargo hakari generate` if hakari verification is enforced; Task 1 Step 8 covers it.
8. **`gui-catalog-parity` safety classes.** `to_safety_class` maps every non-read class (including `network`) to `mutating`; populated values will change GUI safety labels for MCP-only actions. Not a correctness break, but visible in the action manifest.
9. **Recorded contract additions.** `classification.rs` (`ToolClassification`, `classify_tool_row`), the `vox_source` / `vox_exposure` / `vox_side_effect_class` / `vox_requires` meta keys, the internal-set extensions (agent-gateway family, browser cookie export/import), `remote_mutation` in the capability-registry schema, and the `VOX_MCP_WORKSPACE_TOOLS` opt-in are recorded in `00-contracts.md` § Amendments ("From plan 01").
