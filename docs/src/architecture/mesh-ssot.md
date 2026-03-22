---
title: "Mesh (CPU-first) — environment and registry SSOT"
category: architecture
last_updated: 2026-03-21
---

# Mesh SSOT (CPU-first)

Vox **mesh** is **opt-in at runtime**: default single-node behaviour is unchanged until operators set the variables below or use `vox mesh` (requires `vox-cli` feature **`mesh`**).

## Environment variables

| Variable | Meaning |
|----------|---------|
| `VOX_MESH_ENABLED` | `1` or `true` enables mesh hooks (registry publish, interpreted workflow mesh steps). |
| `VOX_MESH_NODE_ID` | Stable node id; generated if unset when publishing. |
| `VOX_MESH_LABELS` | Comma-separated labels merged into [`TaskCapabilityHints`](./orchestration-unified-ssot.md) `labels`. |
| `VOX_MESH_CONTROL_ADDR` | HTTP control plane URL, e.g. `http://127.0.0.1:9847` or `http://mesh-ctrl:9847` (scheme optional in clients; normalise to `http://` when missing). |
| `VOX_MESH_ADVERTISE_GPU` | `1` / `true` sets agent `gpu_cuda` in probes (**legacy** workstation advertisement; not a Vulkan/Android probe). See [mobile / edge AI SSOT](./mobile-edge-ai-ssot.md). |
| `VOX_MESH_ADVERTISE_VULKAN` | `1` / `true` sets `gpu_vulkan` on the host capability snapshot. |
| `VOX_MESH_ADVERTISE_WEBGPU` | `1` / `true` sets `gpu_webgpu`. |
| `VOX_MESH_ADVERTISE_NPU` | `1` / `true` sets `npu`. |
| `VOX_MESH_DEVICE_CLASS` | Optional label (`server`, `desktop`, `mobile`, `browser`, …) → `TaskCapabilityHints.device_class`. |
| `VOX_MESH_REGISTRY_PATH` | Override path for the local JSON registry (default `~/.vox/cache/mesh/local-registry.json`). |
| `VOX_MESH_TOKEN` | When set on **`vox mesh serve`**, all HTTP routes except **`GET /health`** require `Authorization: Bearer <token>`. Clients use **`MeshHttpClient::with_env_token`**. **Never log** this value. |
| `VOX_MESH_SCOPE_ID` | Opaque cluster / tenancy id. When set on **`vox mesh serve`**, **`POST /v1/mesh/join`** and **`POST /v1/mesh/heartbeat`** require the JSON [`NodeRecord`](../../crates/vox-mesh/src/lib.rs) `scope_id` field to match. Clients pick it up from the same env when building records via **`node_record_for_current_process`**. Use the **same** value for every process that should share a mesh; omit for backward-compatible local-only dev. |
| `VOX_MESH_CODEX_TELEMETRY` | When `1` / `true`, append Codex `mesh_control_event` rows (see [orchestration unified SSOT](./orchestration-unified-ssot.md)). |
| `VOX_MESH_MAX_STALE_MS` | Optional client-side staleness threshold (e.g. MCP mesh snapshots); compare with `last_seen_unix_ms` from the control plane (see [orchestration unified SSOT](./orchestration-unified-ssot.md)). |
| `VOX_MESH_HTTP_JOIN` | When `0` / `false`, skip MCP **`vox-mcp`** HTTP **`POST /v1/mesh/join`** even if a client-suitable control URL is set. Default: join when **`VOX_ORCHESTRATOR_MESH_CONTROL_URL`** or **`VOX_MESH_CONTROL_ADDR`** normalizes to a non-bind-all `http(s)://` base. |
| `VOX_MESH_HTTP_HEARTBEAT_SECS` | Interval for MCP background **`POST /v1/mesh/heartbeat`** after a successful join (`0` = join only, no loop). Default **30**. Uses **`VOX_ORCHESTRATOR_MESH_HTTP_TIMEOUT_MS`** (min 500ms, default **15000**) for request timeouts. |

## Local registry file

`MeshRegistryFile` JSON (`schema_version`, `nodes[]`) is stored at the path resolved by `vox_mesh::local_registry_path()` / `VOX_MESH_REGISTRY_PATH` — suitable for a **shared Docker volume** between a control-plane service and workers (dev/CI).

## HTTP control plane (Phase 3 baseline)

Implemented in **`vox-mesh`** feature **`transport`**:

- `GET /health` — process liveness (no bearer required; for load balancers / compose)
- `GET /v1/mesh/nodes` — list nodes
- `POST /v1/mesh/join` — upsert node
- `POST /v1/mesh/heartbeat` — refresh `last_seen` / listen addr
- `POST /v1/mesh/leave` — graceful leave (JSON body `{ "id": "<node_id>" }`; `204` removed, `404` unknown id)

**TLS/mTLS** is an operator concern in front of this API (see ADR 008).

For in-process tests or custom hosts, **`mesh_http_app_with_auth`** + **`MeshHttpAuth`** (`Open`, `Bearer(…)`, or `FromEnv`) avoid relying on ambient `VOX_MESH_TOKEN` in the test process.

### Operator notes (partition / stale nodes)

There is no in-tree gossip TTL yet: treat **`last_seen_unix_ms`** as a hint only. On partition, nodes may disappear from the control-plane view after **`leave`** or process restart; **heartbeats** refresh liveness. For automation, compare `last_seen_unix_ms` to a wall-clock threshold and re-`join` after long gaps. Set **`VOX_MESH_MAX_STALE_MS`** (or rely on MCP snapshot filtering) to drop visibly stale rows client-side.

**Heartbeats:** prefer a **≥ 15–30s** interval per node in steady state; sustained sub-second heartbeats can amplify load on shared control planes — add rate limits at the edge if operators observe abuse (no default middleware in-tree).

### Orchestrator federation (read-only) + experimental routing

When **`VOX_ORCHESTRATOR_MESH_CONTROL_URL`** (or TOML `[orchestrator].mesh_control_url` / `[mesh].control_url`) is set, **`vox-mcp`** polls **`GET /v1/mesh/nodes`** on an interval and exposes a cached snapshot on orchestrator status tools. This path is **visibility only** and does **not** execute tasks on remote nodes.

**Experimental:** `VOX_ORCHESTRATOR_MESH_ROUTING_EXPERIMENTAL=1` enables extra **in-process** scoring / tracing in `RoutingService` using cached remote labels (still **no remote execute**). Treat as **best-effort**; may be removed or replaced in a breaking release.

### Skills / agent labels

For **multi-node** pools, align **`VOX_MESH_LABELS`**, **`[mesh].labels`**, and task **`TaskCapabilityHints::labels`** with the same tokens your operators expect on workers (e.g. `pool=train`, `region=us-west`). Skills and MCP training tools should use the same strings as routing hints so federation snapshots and local queues stay comparable.

## Codegen (Rust servers)

`vox-codegen-rust` **does not** open mesh listeners or set federation URLs; mesh remains **worker / operator env** (`VOX_MESH_*`, `Vox.toml` `[mesh]`) when processes should register or call the control plane.

## CLI / MCP

- **`vox mesh status` / `vox mesh serve`** — `ref-cli.md`, feature **`mesh`**.
- **`vox_mesh_local_status`** (MCP) — returns env + registry JSON.
- **`vox-mcp` process** — when **`VOX_MESH_ENABLED`**, publishes to the local registry once at startup (`crates/vox-mcp/src/mesh_startup.rs`), mirroring **`vox run`**. With a **client-suitable** control URL (**`VOX_ORCHESTRATOR_MESH_CONTROL_URL`** first, else **`VOX_MESH_CONTROL_ADDR`**; bind-all hosts like `0.0.0.0` are skipped via [`normalize_http_control_base`](../../../crates/vox-mesh/src/lib.rs)), it also **`POST /v1/mesh/join`** and periodically **`POST /v1/mesh/heartbeat`** unless disabled (**`VOX_MESH_HTTP_JOIN`**, **`VOX_MESH_HTTP_HEARTBEAT_SECS`**). Optional Codex rows: **`mesh_http_join_ok` / `mesh_http_join_err`** when **`VOX_MESH_CODEX_TELEMETRY`**. Use the same env as workers so the node id matches **`vox run`** / compose peers.
- **Docker** — `Dockerfile` + `docker/vox-entrypoint.sh`: optional **`VOX_MESH_MESH_SIDECAR=1`** starts **`vox mesh serve`** in the background before **`vox mcp`**; set **`VOX_MESH_CONTROL_ADDR`** to the sidecar URL from other containers. Compose profiles and env SSOT: [deployment compose SSOT](./deployment-compose-ssot.md).

## Observability

- **Tracing target `vox.mesh`**: registry publish success logs `path` and `node_id` from **`vox run`** (`crates/vox-cli/src/commands/run.rs`); failures at `debug` only (best-effort).
- **HTTP**: `tower-http` **`TraceLayer`** and **`SetRequestIdLayer`** (`x-request-id`) wrap the control-plane router for request-scoped logs.
- **`vox run`**: mesh registry is published once at the start of the shared `run` entrypoint so **app** and **script** modes (and **`vox-compilerd`** `run`) behave consistently when **`VOX_MESH_ENABLED`** is set. When a client-suitable control URL is set (**`VOX_ORCHESTRATOR_MESH_CONTROL_URL`** / **`VOX_MESH_CONTROL_ADDR`**) and **`VOX_MESH_HTTP_JOIN`** is not disabled, it also performs the same **`POST /v1/mesh/join`** (+ optional heartbeat) path as **`vox-mcp`** via [`vox_mesh::http_lifecycle`](../../crates/vox-mesh/src/http_lifecycle.rs).

### Metrics

- **Today:** structured logs under tracing target **`vox.mesh`** (see above) plus optional Codex rows typed **`mesh_control_event`** when **`VOX_MESH_CODEX_TELEMETRY`** is enabled — append path in [`mesh_registry_telemetry.rs`](../../crates/vox-db/src/mesh_registry_telemetry.rs) / [`mesh_control_telemetry.rs`](../../crates/vox-db/src/mesh_control_telemetry.rs).
- **Future:** Prometheus-style counters or OpenTelemetry spans on control-plane routes (**`/v1/mesh/join`**, etc.) could sit behind the **`transport`** feature and dedicated env toggles if SRE needs SLO dashboards; not required for the baseline CPU-first mesh story.

## OpenAPI

Machine-readable contract: [`schemas/mesh-control-plane.openapi.yaml`](../../../schemas/mesh-control-plane.openapi.yaml) (paths under the served origin; no auth secret in spec).

## Related

- [Cross-platform Vox — lanes & Docker matrix (SSOT)](./vox-cross-platform-runbook.md) — Docker feature matrix vs mobile HTTP mesh clients.
- [Deployment compose SSOT](./deployment-compose-ssot.md) — Docker / Compose / Coolify / CI entry point.
- [Orchestration unified SSOT](./orchestration-unified-ssot.md) — capability probe merge, `VOX_MESH_ADVERTISE_*`.
- [Mobile / edge AI SSOT](./mobile-edge-ai-ssot.md) — inference profiles, mesh GPU/NPU advertisement, training handoff.
- [ADR 008: mesh transport](../adr/008-mesh-transport.md) — HTTP-first control plane, future TLS/quic.
- [ADR 009: hosted mesh BaaS (future)](../adr/009-mesh-hosted-baas.md) — trust model vs self-hosted clusters.
