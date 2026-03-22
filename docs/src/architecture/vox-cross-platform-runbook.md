---
title: "Cross-platform Vox — deployment lanes and Docker matrix (SSOT)"
category: architecture
last_updated: 2026-03-21
---

# Cross-platform Vox — runbook

This page ties together **how Vox is meant to run** on servers, generated apps, and mobile-adjacent clients. It complements [deployment compose SSOT](deployment-compose-ssot.md), [mobile / edge AI SSOT](mobile-edge-ai-ssot.md), and [mesh SSOT](mesh-ssot.md).

## Lane S — Server script / worker

- **Entry:** `vox run --mode script` on a path to a `.vox` file with a `fn main()`-style script surface.
- **Binary:** `vox-cli` must be built with feature **`script-execution`** (see [CLI scope policy](cli-scope-policy.md)).
- **Mesh (optional):** build with feature **`mesh`**. When **`VOX_MESH_ENABLED`** is set, `vox run` publishes to the local mesh registry and may HTTP-join the control plane (same env as MCP). Implementation: [`mesh_publish_best_effort_for_run`](../../../crates/vox-cli/src/commands/run.rs) calls [`publish_local_registry_best_effort`](../../../crates/vox-mesh/src/lib.rs) and [`mesh_http_join_best_effort`](../../../crates/vox-mesh/src/http_lifecycle.rs).
- **Compose:** [examples/mesh-compose.yml](../../../examples/mesh-compose.yml) uses `vox run --mode script` for the worker service with a shared volume and mesh control plane.

## Lane A — App / generated server

- **Entry:** `vox run` in **app** mode (default auto-detection or `RunMode::App`): compiler pipeline + generated server under `target/generated` (see [Vox full-stack web UI SSOT](vox-web-stack-ssot.md)).
- **Deploy:** `vox deploy` / `vox-container` and Compose emission — [deployment compose SSOT](deployment-compose-ssot.md).

## Lane M — Mobile native

- **No `vox` binary** on stock iOS/Android for full language stack or Ollama; see [mobile / edge AI SSOT](mobile-edge-ai-ssot.md).
- **Mesh:** native apps act as HTTP clients: register via **`POST /v1/mesh/join`** with a [`NodeRecord`](../../../crates/vox-mesh/src/lib.rs), using the same **`VOX_MESH_*`** / control URL conventions as servers.
- **Inference:** set **`VOX_INFERENCE_PROFILE`** (e.g. `mobile_litert`, `cloud_openai_compatible`) so MCP-compatible tooling does not assume desktop Ollama on loopback.

## WASM clarification

**WASI / Wasmtime** (`vox run --isolation wasm` on a workstation) is **not** the same as **in-browser WebGPU + WASM**. Browser tiers are optional and policy-gated; see [mobile / edge AI SSOT](mobile-edge-ai-ssot.md) (browser row).

## Docker image / feature matrix

Images are **operator-defined tags** unless your registry publishes blessed names. The table below is the **documentation convention** aligned with the repo [`Dockerfile`](../../../Dockerfile) and [examples/mesh-compose.yml](../../../examples/mesh-compose.yml).

| Documented tag (convention) | `VOX_CLI_FEATURES` (build-arg) | Primary `CMD` | Ports (typical) |
|-----------------------------|--------------------------------|---------------|-----------------|
| **`vox`** (default build) | *(empty)* | `vox mcp` | **3000** |
| **`vox:mesh-worker`** | `mesh,script-execution` | `vox mcp`, `vox mesh serve`, or `vox run --mode script` per service | **3000**, **9847** (control plane) |

- **Sidecar:** `VOX_MESH_MESH_SIDECAR=1` + [`docker/vox-entrypoint.sh`](../../../docker/vox-entrypoint.sh) can run **`vox mesh serve`** beside **`vox mcp`** in one container; see Dockerfile comments and [deployment compose SSOT](deployment-compose-ssot.md).
- **CI smoke tags:** default **`vox:ci-smoke`**; mesh/features matrix **`vox:ci-mesh`** and **`vox:ci-mesh-worker`** (same image, two names) — [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml).

## Env-over-features

Prefer **runtime environment** when behavior is already gated in-tree:

- **Mesh:** `VOX_MESH_ENABLED`, `VOX_ORCHESTRATOR_MESH_CONTROL_URL`, `VOX_MESH_HTTP_JOIN`, `VOX_MESH_TOKEN`, etc. — [mesh SSOT](mesh-ssot.md).
- **Inference / routing:** `VOX_INFERENCE_PROFILE` — [mobile / edge AI SSOT](mobile-edge-ai-ssot.md), [environment variables SSOT](../reference/env-vars-ssot.md).

Rebuild with different `VOX_CLI_FEATURES` only when you need **code paths** that are not linked in the default binary (e.g. **`mesh`**, **`script-execution`**).

## Related

- [Deployment compose SSOT](deployment-compose-ssot.md)
- [Mesh SSOT](mesh-ssot.md)
- [Mobile / edge AI SSOT](mobile-edge-ai-ssot.md)
- [Vox full-stack web UI SSOT](vox-web-stack-ssot.md)
