---
title: "Deployment: Docker, Compose, Coolify, CI (SSOT)"
description: "Official documentation for Deployment: Docker, Compose, Coolify, CI (SSOT) for the Vox language. Detailed technical reference, architectu"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Deployment: Docker, Compose, Coolify, CI (SSOT)

**Single navigation hub** for container images, Compose files, hosted deploy (Coolify), CI checks, and how they relate to **mesh** and **mobile/edge** (which are *not* the same shape as a Linux OCI image).

## Compose profiles (which file when)

| Profile | Purpose | Compose / template | Default image / build | Ports (typical) |
|---------|---------|--------------------|------------------------|-----------------|
| **MCP single-node** | Run `vox mcp` with API keys + optional Codex (Turso) | Repo root [`docker-compose.yml`](../../../docker-compose.yml) | Root [`Dockerfile`](../../../Dockerfile) (`CMD vox mcp`) | **3000** |
| **MCP + mesh (multi-service)** | Control plane + MCP + worker; shared registry volume | [`examples/mesh-compose.yml`](../../../docker-compose.yml) | Same `Dockerfile` with build-arg `VOX_CLI_FEATURES=mesh,script-execution` | **9847** (mesh), **3000** (MCP) |
| **Codex API (BaaS template)** | Self-hosted Codex-style HTTP API on Turso (placeholder service name) | [`infra/coolify/docker-compose.yml`](../../../infra/coolify/docker-compose.yml) | **`VOX_CODEX_IMAGE`** (you build/push); not the default `vox` MCP image unless you retag/repurpose | **8080** (template) |
| **Generated app stack** | `vox deploy` / `vox-container` sample (Node + nginx + optional mesh env) | Emitted by [`generate_compose_file`](../../../crates/vox-container/src/generate.rs) | Project `Dockerfile` from `@environment` / package flow | **3000** + **80/443** |

**Do not** assume root `docker-compose.yml` and `infra/coolify/docker-compose.yml` are interchangeable: they target **different workloads** (MCP vs Codex API template). See [Codex BaaS](../architecture/codex-baas.md) and [infra/coolify/README.md](../adr/README.md).

## OCI image (repo `Dockerfile`)

- **Binary:** `vox` (release), optional features via `VOX_CLI_FEATURES` (e.g. `mesh,script-execution`).
- **Data:** volume **`/root/.vox`**; align with `VOX_DB_*` / local SQLite layout per [ADR 004](../adr/004-codex-arca-turso-ssot.md).
- **Mesh sidecar (single container):** `VOX_MESH_MESH_SIDECAR=1` + entrypoint [`docker/vox-entrypoint.sh`](../../../docker/vox-entrypoint.sh); exposes **9847** when used.
- **Health:** `vox doctor` (see `Dockerfile` `HEALTHCHECK`).

## Environment SSOT (Compose-friendly)

- **Codex / Turso:** `VOX_DB_URL`, `VOX_DB_TOKEN`, `VOX_DB_PATH` — [env-vars SSOT](env-vars.md), [ADR 004](../adr/004-codex-arca-turso-ssot.md).
- **Mesh:** full `VOX_MESH_*` table — [mesh SSOT](mesh.md). Optional **`VOX_ORCHESTRATOR_MESH_CONTROL_URL`** for MCP to read mesh nodes (see [`examples/mesh-compose.yml`](../../../docker-compose.yml)). With a client-suitable URL, **`vox-mcp`** also **HTTP join/heartbeat** to the control plane (see mesh SSOT **`VOX_MESH_HTTP_*`**).
- **Optional mesh env block (one text SSOT):** [`docker/vox-compose-mesh-environment.block.yaml`](../../../docker/vox-compose-mesh-environment.block.yaml) — embedded into generated Compose in `vox-container`; keep [`examples/mesh-compose.yml`](../../../docker-compose.yml) semantically aligned (comments in that file point here).
- **Inference / mobile:** `VOX_INFERENCE_PROFILE` and LAN/cloud patterns — [mobile / edge AI SSOT](mobile-edge-ai.md) (phones do **not** run this `Dockerfile`).

## Runtimes: Docker vs Podman

- **CLI / deploy:** [`vox-container`](../../../crates/vox-container/src/lib.rs) implements **`ContainerRuntime`** for Docker and Podman; Compose execution prefers **`podman-compose`** then **`docker compose`** ([`deploy_target.rs`](../../../crates/vox-container/src/deploy_target.rs)).
- **CI:** GitHub self-hosted jobs use **Docker** (see [workflow enumeration](../ci/workflow-enumeration.md)). Validate Podman locally for rootless/volume/DNS differences before claiming parity.

## Coolify

- Coolify deploys **Docker Compose** bundles; use `${VAR}` / `${VAR:-default}` so secrets and toggles stay in the UI — [Coolify environment variables](https://coolify.io/docs/knowledge-base/environment-variables), [Compose on Coolify](https://coolify.io/docs/knowledge-base/docker/compose).
- Vox template: [`infra/coolify/`](../../../infra/coolify/) — read the README for image vs `Dockerfile` MCP split and build-time vs runtime vars.

## CI (GitHub & GitLab)

- **GitHub:** `docker compose … config` on the mesh example + `docker build` default and mesh feature matrix — [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml).
- **GitLab:** see [workflow enumeration](../ci/workflow-enumeration.md) for parity jobs (compose config + optional image smoke).

## Related docs

- [Cross-platform Vox — lanes & Docker matrix (SSOT)](../architecture/vox-cross-platform-runbook.md) — script worker vs app vs mobile; feature matrix.
- [How to deploy](../how-to/how-to-deploy.md) — `vox deploy`, `Vox.toml`, registry login.
- [Zig-inspired deployment](../explanation/zig-inspired-deployment.md) — unified `vox deploy` targets and crates.
- [Mesh SSOT](mesh.md), [orchestration unified SSOT](orchestration-unified.md).
- [Mobile / edge AI SSOT](mobile-edge-ai.md).

## Do’s and don’ts (short)

- **Do** keep variable names identical to [env-vars SSOT](env-vars.md) / mesh / ADR 004.
- **Do** use persistent volumes for `/root/.vox` (or documented `VOX_DB_PATH`) in production Compose.
- **Don’t** embed secrets in committed defaults; use substitution + CI/secret stores.
- **Don’t** document “run the MCP `Dockerfile` on mobile”; use mobile-edge SSOT profiles and mesh HTTP from the app.
