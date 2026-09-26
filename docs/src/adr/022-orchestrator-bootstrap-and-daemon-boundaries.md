---
title: "ADR 022 — Orchestrator bootstrap factory and daemon boundaries"
description: "Single factory for repo-scoped Orchestrator construction; relationship to vox-mcp, vox-orchestrator-d, and optional future orchestrator daemon."
category: "Architecture Decisions (ADRs)"
training_eligible: true

schema_type: "TechArticle"
---

# ADR 022 — Orchestrator bootstrap factory and daemon boundaries

## Status

Accepted (2026-04-01)

## Context

Multiple surfaces (`vox-mcp`, `vox dei` / CLI, `vox live`, Ludus HUD) each constructed an `Orchestrator` by calling `repo_scoped_orchestrator_parts` plus `Orchestrator::with_groups`. That duplicated logic and risked subtle divergence (repository id, memory shard paths, affinity groups).

Separately, **`vox-orchestrator-d`** remains the RPC process for Mens-shaped AI flows (`ai.generate`, `ai.review`, `ai.plan.*`) with stable method ids in `vox-cli` `dei_daemon.rs`. It is **not** defined as the host for the full `Orchestrator` type today.

Mesh distribution uses **per-process** `Orchestrator` instances with **Turso-backed** coordination when mens is enabled; see [Mens coordination](../reference/populi-coordination.md) and [Unified orchestration](../reference/orchestration-unified.md).

## Decision

1. **Bootstrap SSOT:** Expose **`vox_orchestrator::build_repo_scoped_orchestrator`** and **`build_repo_scoped_orchestrator_for_repository`** returning **`RepoScopedOrchestratorBuild`** (`repository`, scoped `config`, `orchestrator`). All first-party embedders use this factory.
2. **`vox-orchestrator-d` boundary:** Keep **`vox-orchestrator-d`** focused on **DeI RPC / AI routing** and **Orchestrator** operations. MCP behaves as a thin client for many task/agent lifecycle slices.
3. **Trust-conditioned gates:** Optional **`trust_gate_relax_*`** config relaxes **Socrates enforce**, **completion grounding enforce**, and **strict scope** when Codex **`agent_reliability`** exceeds a configurable floor, reusing the same Laplace scores as reputation routing.
4. **Merged Authority:** The legacy orchestrator daemon binary name has been merged into **`vox-orchestrator-d`** to unify the AI plane and Coordination plane.
5. **Authority model (Phase B/IPC transition):** adopt a **split-plane transition model** until broad RPC parity exists: daemon-aligned RPC can own **task + agent lifecycle** slices under explicit MCP env flags, while MCP remains authoritative for VCS/context/event/session surfaces still backed by embedded stores. Promote to full thin MCP only after those stores gain explicit daemon contracts.

## Consequences

- New orchestrator embedders should call the bootstrap module only; avoid re-copying `repo_scoped_orchestrator_parts` + `with_groups` at new call sites.
- Parity tests can assert repeated builds yield identical `repository_id` and memory paths.
- A future daemon would reuse **`RepoScopedOrchestratorBuild`** internally; MCP would switch to IPC/HTTP without changing routing semantics.

## Phase B (optional) — single-process orchestrator owner

When product requirements justify fixing **cold-start** and **gravity** (one RAM image shared by many MCP attach/detach cycles), implement a long-lived process that:

1. **Done:** Binary **`vox-orchestrator-d`** (`crates/vox-orchestrator` `[[bin]]`) calls **`build_repo_scoped_orchestrator`**, optional **`Orchestrator::init_db`** via **`vox_db::connect_canonical_optional`**, listens on **`VOX_ORCHESTRATOR_DAEMON_SOCKET`**, and spawns the same long-lived sidecars as MCP when config/DB apply: **`mesh_federation_poll::spawn_populi_federation_poller`**, **`a2a::spawn_populi_remote_result_poller`** / **`a2a::spawn_populi_remote_worker_poller`**, **`orchestrator_event_log::spawn_orchestrator_event_log_sink`**, and (when Codex is attached) **`clarification_db_inbox_poll::spawn_clarification_db_inbox_poller`**. **`vox-mcp`** delegates those entry points to the same `vox-orchestrator` modules (it still owns **`ServerState`** and the full MCP tool surface).
2. **Done:** TCP or **stdio** newline **`DispatchRequest`** / **`DispatchPayload::Result`** plane; method ids in **`vox_protocol::orch_daemon_method`** (`orch.ping`, `orch.status`, `orch.task_status`, `orch.spawn_agent`, `orch.agent_ids`).
3. **Partial:** **`vox-mcp`** calls **`ServerState::probe_external_orchestrator_daemon_if_configured`** when **`VOX_ORCHESTRATOR_DAEMON_SOCKET`** points at a TCP peer (stdio skipped); **`orch.ping`** `repository_id` is compared to the embed’s repo (**WARN** / optional **ERROR** via **`VOX_MCP_ORCHESTRATOR_DAEMON_REPOSITORY_ID_STRICT`**). Optional **`VOX_MCP_ORCHESTRATOR_STATUS_TOOL_RPC`** (or umbrella **`VOX_MCP_ORCHESTRATOR_RPC_READS`**) makes **`vox_orchestrator_status`** attach the aligned daemon's **`orch.status`** JSON to its payload. The task-status, start, and write pilots were retired 2026-09-26: their implementations had been lost and nothing consumed them. The **in-process** **`Orchestrator`** remains default for VCS/context/event/session surfaces pending explicit contracts.

## Links

- [`crates/vox-orchestrator/src/bootstrap.rs`](../../../crates/vox-orchestrator/src/bootstrap.rs)
- [`crates/vox-orchestrator/src/orch_daemon/mod.rs`](../../../crates/vox-orchestrator/src/orch_daemon/mod.rs) — TCP RPC + `OrchDaemonClient`
- [`crates/vox-orchestrator/src/mesh_federation_poll.rs`](../../../crates/vox-orchestrator/src/mesh_federation_poll.rs) — shared Populi federation poll loop (MCP + daemon)
- [`crates/vox-orchestrator/src/mcp_tools/mod.rs`](../../../crates/vox-orchestrator/src/mcp_tools/mod.rs) — MCP tool wiring (legacy `orchestrator_snapshot` path removed; event/log sinks live alongside tool registration)
- [`crates/vox-orchestrator/src/clarification_db_inbox_poll.rs`](../../../crates/vox-orchestrator/src/clarification_db_inbox_poll.rs) — Codex clarification inbox drain
- [`crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs`](../../../crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs) — `vox-orchestrator-d` binary
- [`crates/vox-cli/src/dei_daemon.rs`](../../../crates/vox-cli/src/dei_daemon.rs)
- [Orphan surface inventory](../archive/research-2026-q1/orphan-surface-inventory.md) — `vox-orchestrator` staging crate vs `vox-orchestrator` SSOT

