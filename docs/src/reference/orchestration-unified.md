---
title: "Unified orchestration — SSOT"
description: "Official documentation for Unified orchestration — SSOT for the Vox language. Detailed technical reference, architecture guides, and impl"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Unified orchestration — SSOT

This document captures **compatibility rules** and **opt-in migration toggles** while MCP, CLI, and DeI share one orchestrator contract (`vox-orchestrator`).

## Contract surfaces

- **Types:** `vox_orchestrator::contract` — `TaskCapabilityHints`, `SessionContractEnvelope`, `OrchestrationMigrationFlags` (`orchestration_v2_enabled`, `legacy_orchestration_fallback`), MCP ↔ DeI plan tool alignment (`MCP_PLAN_TOOL_NAMES`, `DEI_PLAN_METHODS_NEW_REPLAN_STATUS`).
- **Runtime config:** `vox_orchestrator::OrchestratorConfig` — process-wide limits, Socrates gates, scaling knobs, and nested **`orchestration_migration`** (`OrchestrationMigrationFlags`). Loaded from `Vox.toml` `[orchestrator]` and **`VOX_ORCHESTRATOR_*`** env overrides via `OrchestratorConfig::merge_env_overrides` in `crates/vox-orchestrator/src/config.rs`.

### Agent queue capabilities (`TaskCapabilityHints`)

On **`Orchestrator::spawn_agent`**, each new [`AgentQueue`](../../../crates/vox-orchestrator/src/queue.rs) gets capabilities from **`merge_agent_capabilities`** (`crates/vox-orchestrator/src/capability_probe.rs`):

1. Start from **`default_agent_capabilities`** in config / TOML.
2. Overlay **host probe** via **`probe_host_capabilities`**: `cpu_cores` (from `available_parallelism`), `arch` (`std::env::consts::ARCH`), `hostname` (`HOSTNAME` / `COMPUTERNAME`, or `sysinfo` when built with **`system-metrics`**).
3. **Labels:** config labels preserved first; probe-supplied labels appended without duplicates.
4. **GPU / NPU flags:** operator config wins if already `true`; otherwise probe may set `gpu_cuda` when **`VOX_MESH_ADVERTISE_GPU=1|true`** (legacy workstation advertisement), or `gpu_vulkan` / `gpu_webgpu` / `npu` from the matching **`VOX_MESH_ADVERTISE_*`** vars (not driver probes). Optional **`VOX_MESH_DEVICE_CLASS`** fills `device_class`. See [mobile / edge AI SSOT](mobile-edge-ai.md).
5. **`min_vram_mb` / `min_cpu_cores`:** filled from probe only when unset in config.

Routing reads **`capability_requirements`** on tasks and applies GPU / VRAM / **`min_cpu_cores`** / **`prefer_gpu_compute`** soft penalties in `crates/vox-orchestrator/src/services/routing.rs` (mens / Mens-style training hints).

See also [mens SSOT](mens.md) for `VOX_MESH_*` and local registry.

## Environment and config

### `OrchestratorConfig` — `VOX_ORCHESTRATOR_*`

Boolean fields use Rust `bool` parsing (`true` / `false` only). Invalid values log a warning and leave the current setting unchanged.

| Variable | Maps to |
|----------|---------|
| `VOX_ORCHESTRATOR_ENABLED` | `enabled` |
| `VOX_ORCHESTRATOR_MAX_AGENTS` | `max_agents` |
| `VOX_ORCHESTRATOR_LOCK_TIMEOUT_MS` | `lock_timeout_ms` |
| `VOX_ORCHESTRATOR_TOESTUB_GATE` | `toestub_gate` |
| `VOX_ORCHESTRATOR_MAX_DEBUG_ITERATIONS` | `max_debug_iterations` |
| `VOX_ORCHESTRATOR_SOCRATES_GATE_SHADOW` | `socrates_gate_shadow` |
| `VOX_ORCHESTRATOR_SOCRATES_GATE_ENFORCE` | `socrates_gate_enforce` |
| `VOX_ORCHESTRATOR_SOCRATES_REPUTATION_ROUTING` | `socrates_reputation_routing` |
| `VOX_ORCHESTRATOR_SOCRATES_REPUTATION_WEIGHT` | `socrates_reputation_weight` |
| `VOX_ORCHESTRATOR_LOG_LEVEL` | `log_level` (raw string) |
| `VOX_ORCHESTRATOR_FALLBACK_SINGLE` | `fallback_to_single_agent` |
| `VOX_ORCHESTRATOR_MIN_AGENTS` | `min_agents` |
| `VOX_ORCHESTRATOR_SCALING_THRESHOLD` | `scaling_threshold` |
| `VOX_ORCHESTRATOR_IDLE_RETIREMENT_MS` | `idle_retirement_ms` |
| `VOX_ORCHESTRATOR_SCALING_ENABLED` | `scaling_enabled` |
| `VOX_ORCHESTRATOR_COST_PREFERENCE` | `cost_preference` (`performance` \| `economy`) |
| `VOX_ORCHESTRATOR_SCALING_LOOKBACK` | `scaling_lookback_ticks` |
| `VOX_ORCHESTRATOR_RESOURCE_WEIGHT` | `resource_weight` |
| `VOX_ORCHESTRATOR_RESOURCE_CPU_MULT` | `resource_cpu_multiplier` |
| `VOX_ORCHESTRATOR_RESOURCE_MEM_MULT` | `resource_mem_multiplier` |
| `VOX_ORCHESTRATOR_RESOURCE_EXPONENT` | `resource_exponent` |
| `VOX_ORCHESTRATOR_SCALING_PROFILE` | `scaling_profile` (`conservative` \| `balanced` \| `aggressive`) |
| `VOX_ORCHESTRATOR_MAX_SPAWN_PER_TICK` | `max_spawn_per_tick` |
| `VOX_ORCHESTRATOR_SCALING_COOLDOWN_MS` | `scaling_cooldown_ms` |
| `VOX_ORCHESTRATOR_URGENT_REBALANCE_THRESHOLD` | `urgent_rebalance_threshold` |
| `VOX_ORCHESTRATOR_MIGRATION_V2_ENABLED` | `orchestration_migration.orchestration_v2_enabled` |
| `VOX_ORCHESTRATOR_MIGRATION_LEGACY_FALLBACK` | `orchestration_migration.legacy_orchestration_fallback` |
| `VOX_ORCHESTRATOR_MESH_CONTROL_URL` | `populi_control_url` — HTTP base for **`GET /v1/mens/nodes`** (read-only); MCP `vox_orchestrator_status` includes **`mesh_snapshot`** JSON when set. Uses **`VOX_MESH_TOKEN`** on the client when present. Does not change task routing. |

### Other CLI / data plane

| Variable | Purpose |
|----------|---------|
| `VOX_BENCHMARK_TELEMETRY` | When `1` / `true`, CLI benchmark entry points append `benchmark_event` rows via `VoxDb::record_benchmark_event`. |
| `VOX_WORKFLOW_JOURNAL_CODEX` | When `1` / `true`, after `vox workflow run` / `vox mens workflow run` ( **`workflow-runtime`** ), append interpreted journal rows via `VoxDb::record_workflow_journal_entry` (session `workflow:<repository_id>`, metric `workflow_journal_entry`). Rows include **`ActivityStarted` / `ActivityCompleted`** and per-step payloads (e.g. **`MeshActivity`**) with **`activity_id`** when provided in `with { activity_id: … }`. |
| `VOX_MESH_MAX_STALE_MS` | Client-side filter for mens node lists in MCP snapshots (see [mens SSOT](mens.md)). |
| `VOX_MESH_CODEX_TELEMETRY` | When `1` / `true`, append `populi_control_event` rows via `VoxDb::record_populi_control_event` (session `mens:<repository_id>`): after **`vox run`** local registry publish when the CLI was built with **`populi`** (includes `vox-populi`), after **`vox-mcp`** startup publish when mens is enabled, and after MCP **`vox_orchestrator_status`** mens HTTP snapshot when Codex is connected. Implementation: [`vox_db::populi_registry_telemetry`](../../../crates/vox-db/src/populi_registry_telemetry.rs). **Never** stores `VOX_MESH_TOKEN`. |
| `VOX_MCP_LLM_COST_EVENTS` | Optional override for MCP LLM [`CostIncurred`](../../../crates/vox-orchestrator/src/events.rs) bus events vs Codex-only accounting; see [`vox-mcp.md`](../api/vox-mcp.md#llm-model-routing-modelstoml). |
| `VOX_REPOSITORY_ROOT` | Optional directory for `repository_id` discovery in benchmark telemetry (and other CLI paths that adopt the same pattern); align with MCP’s discovered repo root when subprocess CWD differs. |

**TOML:** under `[orchestrator]`, set `orchestration_migration = { orchestration_v2_enabled = true, … }` (field names match `OrchestrationMigrationFlags` in `crates/vox-orchestrator/src/contract.rs`). When v2 is enabled, MCP `vox_submit_task` success JSON may include **`orchestration_contract`: `"v2"`** as a client hint.

Optional **`[mens]`** in `Vox.toml` merges mens scope/URL/labels for CLI and MCP (see [mens SSOT](mens.md)); **env wins** per field when set.

Effective Socrates thresholds still merge from `vox-socrates-policy` with optional overrides in `OrchestratorConfig::socrates_policy` — no literal drift outside the policy crate + merge logic.

## Deprecation / compatibility matrix (current)

| Surface | Rule |
|---------|------|
| MCP tool names | Add aliases before removing names; `vox_plan`, `vox_replan`, `vox_plan_status` stay stable. |
| DeI RPC ids | `ai.plan.*` method strings unchanged (`vox_cli::dei_daemon::method`). |
| File sessions + Codex | Both remain valid; MCP `SessionManager` uses `with_db` when Codex is attached. |
| `vox db` | Remains implementation SSOT; `vox scientia` is a documented facade only. |

## Related docs

- [`external-repositories.md`](external-repositories.md) — `repository_id`, sessions, cache layout.
- [`socrates-protocol.md`](socrates-protocol.md) — Socrates telemetry and policy.
- [`mens-training.md`](mens-training.md) — training backends and env.
