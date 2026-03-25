---
title: "Environment variables (SSOT)"
description: "Official documentation for Environment variables (SSOT) for the Vox language. Detailed technical reference, architecture guides, and impl"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Environment variables (SSOT)

Canonical names and precedence for tooling that spans CLI, MCP, orchestrator, and Codex. **Implementations** live in the crates cited below; update this page when adding or renaming variables.

## Codex / Turso (`vox-db`, `vox-pm`)

| Variable | Role |
|----------|------|
| `VOX_DB_URL` | Remote libSQL / Turso URL (with `VOX_DB_TOKEN`). |
| `VOX_DB_TOKEN` | Auth token for `VOX_DB_URL`. |
| `VOX_DB_PATH` | Local database file path (`local` / replication features). |
| `VOX_TURSO_URL` / `VOX_TURSO_TOKEN` | **Compatibility** aliases read after canonical `VOX_DB_*` fails in [`DbConfig::resolve_standalone`](../../../crates/vox-db/src/config.rs). |
| `TURSO_URL` / `TURSO_AUTH_TOKEN` | **Legacy** Turso env names; same compatibility tier as `VOX_TURSO_*`. |

**Precedence (remote):** `VOX_DB_URL`+`VOX_DB_TOKEN` → `VOX_TURSO_*` → `TURSO_*`. **Project VoxDb** (operational store + snippets/share) uses [`DbConfig::resolve_project_code_store_config`](../../../crates/vox-db/src/config.rs): empty env maps to the project-relative default store path, not the user-data default.

See [ADR 004: Codex / Arca / Turso](../adr/004-codex-arca-turso-ssot.md).

## Repository root (`vox-repository`, `vox ci`)

| Variable | Role |
|----------|------|
| `VOX_REPO_ROOT` | Absolute or normalized path to the logical repo root for **`vox ci`**, doc-inventory, and other tools that must not depend on cwd alone. |

## Orchestrator (`vox-orchestrator`)

| Variable | Role |
|----------|------|
| `VOX_ORCHESTRATOR_ENABLED` | Enable/disable orchestrator. |
| `VOX_ORCHESTRATOR_MAX_AGENTS` | Cap on concurrent agents. |
| `VOX_ORCHESTRATOR_LOCK_TIMEOUT_MS` | File lock TTL. |
| `VOX_ORCHESTRATOR_TOESTUB_GATE` | TOESTUB post-task gate. |
| `VOX_ORCHESTRATOR_MAX_DEBUG_ITERATIONS` | Re-route cap on validation failures. |
| `VOX_ORCHESTRATOR_SOCRATES_GATE_SHADOW` | Log Socrates decisions without blocking. |
| `VOX_ORCHESTRATOR_SOCRATES_GATE_ENFORCE` | Requeue on risky Socrates outcome. |
| `VOX_ORCHESTRATOR_SOCRATES_REPUTATION_ROUTING` | Blend Arca `agent_reliability` into routing. |
| `VOX_ORCHESTRATOR_SOCRATES_REPUTATION_WEIGHT` | Weight for reliability blend (default in config: `1.0`). |
| `VOX_ORCHESTRATOR_LOG_LEVEL` | Tracing/log level string. |
| `VOX_ORCHESTRATOR_FALLBACK_SINGLE` | Ambiguous routing → single agent. |
| `VOX_ORCHESTRATOR_MESH_CONTROL_URL` | Base URL of the mens HTTP control plane for **read-only** node snapshots in MCP/orchestrator (e.g. `http://mens-ctrl:9847`). See [mens SSOT](mens.md), [deployment compose SSOT](deployment-compose.md). |
| `VOX_ORCHESTRATOR_MESH_POLL_INTERVAL_SECS` | Poll interval for mens HTTP client (see [`OrchestratorConfig::merge_env_overrides`](../../../crates/vox-orchestrator/src/config.rs)). |
| `VOX_ORCHESTRATOR_MESH_HTTP_TIMEOUT_MS` | HTTP timeout for mens control-plane requests. |
| `VOX_ORCHESTRATOR_MESH_ROUTING_EXPERIMENTAL` | Experimental routing hooks (see [mens SSOT](mens.md)). |
| `VOX_ORCHESTRATOR_MIN_AGENTS` / `SCALING_*` / `COST_PREFERENCE` / `RESOURCE_*` | Scaling and economy knobs — see [`OrchestratorConfig::merge_env_overrides`](../../../crates/vox-orchestrator/src/config.rs). |
| `VOX_NEWS_PUBLISH_ARMED` | When `1`/`true`, satisfies the **armed** gate for live news syndication (in addition to two DB approvers). See [news syndication security](../architecture/news_syndication_security.md). |
| `VOX_NEWS_SITE_BASE_URL` | Public site base URL for RSS links (overrides `[orchestrator.news].site_base_url`). |
| `VOX_NEWS_RSS_FEED_PATH` | Repo-relative path to `feed.xml` (overrides `[orchestrator.news].rss_feed_path`). |
| `VOX_NEWS_SCAN_RECURSIVE` | `0`/`1`: whether `NewsService` walks `news_dir` recursively (default `1`). |
| `VOX_NEWS_TWITTER_TEXT_CHUNK_MAX` | Optional integer override for tweet chunk length (defaults to publisher contract value). |
| `VOX_NEWS_TWITTER_TRUNCATION_SUFFIX` | Optional suffix used when shortening non-thread tweets (default `...`). |

Socrates numeric thresholds default from [`vox-socrates-policy`](../../../crates/vox-socrates-policy/src/lib.rs); optional TOML overrides live under `[orchestrator]` as `socrates_policy` (see `OrchestratorConfig`).

## Mens / Candle

| Variable | Role |
|----------|------|
| `VOX_CANDLE_DEVICE` | Forces Candle device (e.g. `cpu`); see Mens training SSOT. |
| `VOX_INFERENCE_PROFILE` | `desktop_ollama` (default), `cloud_openai_compatible`, `mobile_litert`, `mobile_coreml`, `lan_gateway`; gates **vox-mcp** local Ollama + Ollama fallback to `desktop_ollama` / `lan_gateway` only; see [`vox_config::inference`](../../../crates/vox-config/src/inference.rs) and [mobile-edge-ai.md](mobile-edge-ai.md). |

## Mens (`vox-populi`, orchestrator probe)

Full table: [mens SSOT](mens.md). Common entries:

| Variable | Role |
|----------|------|
| `VOX_MESH_ENABLED` | Enables mens registry publish and related hooks. |
| `VOX_MESH_CONTROL_ADDR` | This process’s control plane URL (publish/join target). |
| `VOX_MESH_TOKEN` / `VOX_MESH_SCOPE_ID` | Auth and tenancy for the control plane. |
| `VOX_MESH_MAX_STALE_MS` | Client-side staleness filter for mens snapshots (MCP). |
| `VOX_MESH_CODEX_TELEMETRY` | Emit Codex `populi_control_event` rows when set. |
| `VOX_MESH_HTTP_JOIN` | `0`/`false` disables MCP HTTP join to the control plane; see [mens SSOT](mens.md). |
| `VOX_MESH_HTTP_HEARTBEAT_SECS` | MCP heartbeat interval after join (`0` = no background heartbeat). |
| `VOX_MESH_ADVERTISE_GPU` | Legacy: sets `gpu_cuda` on the host capability snapshot. |
| `VOX_MESH_ADVERTISE_VULKAN` | Sets `gpu_vulkan`. |
| `VOX_MESH_ADVERTISE_WEBGPU` | Sets `gpu_webgpu`. |
| `VOX_MESH_ADVERTISE_NPU` | Sets `npu`. |
| `VOX_MESH_DEVICE_CLASS` | Optional `TaskCapabilityHints.device_class` string. |

## GPU probe overrides (Mens training)

| Variable | Role |
|----------|------|
| `VOX_GPU_MODEL` | With `VOX_GPU_VRAM_MB`, overrides [`probe_gpu`](../../../crates/vox-mens/src/tensor/device.rs) (CI / headless / Android host injection). |
| `VOX_GPU_VRAM_MB` | Paired with `VOX_GPU_MODEL` for VRAM heuristics. |

## CI / diagnostics

| Variable | Role |
|----------|------|
| `VOX_BUILD_TIMINGS_BUDGET_WARN` | Soft budget warnings for **`vox ci build-timings`**. |
| `SKIP_CUDA_FEATURE_CHECK` | Skip optional `nvcc` gates (documented escape hatch in [runner contract](../ci/runner-contract.md)). |

## Web / Vite / TanStack codegen

| Variable | Role |
|----------|------|
| `VOX_WEB_TANSTACK_START` | When `1` / `true`, enables TanStack **Start** scaffold + TS codegen path (`VoxTanStackRouter` / `voxRouteTree` when `routes:` is present). Must stay aligned with **`Vox.toml`** `[web] tanstack_start` for **`vox build`**. See [`VoxConfig::merge_env_overrides`](../../../crates/vox-config/src/config.rs), [TanStack how-to](../how-to/tanstack-ssr-with-axum.md). |
| `VOX_EMIT_EXPRESS_SERVER` | Opt-in: emit legacy **`server.ts`** (Express-style) from `vox-codegen-ts`; default product is **Axum** + **`api.ts`**. See [vox-fullstack-artifacts.md](vox-fullstack-artifacts.md). |
| `VOX_ORCHESTRATE_VITE` | If `1`, **`vox run`** spawns **`pnpm run dev:ssr-upstream`** in `dist/.../app` (Vite on **3001**). See [`OrchestratedViteGuard`](../../../crates/vox-cli/src/frontend.rs). |
| `VOX_SSR_DEV_URL` | Origin (e.g. `http://127.0.0.1:3001`) for generated Axum to proxy non-`/api` **GET** document requests before `rust_embed`. Often injected when **`VOX_ORCHESTRATE_VITE=1`**. |
| `VOX_WEB_VITE_SMOKE` | Opt-in: set to **`1`** when running **`cargo test -p vox-integration-tests --test web_vite_smoke -- --ignored`** (full **`pnpm install`** + **`vite build`** on a golden `.vox` fixture). |
| `VOX_EXAMPLES_STRICT_PARSE` | When **`1`**, **`cargo test -p vox-parser --test parity_test`** fails if any `examples/**/*.vox` fails to parse (default CI only requires the **`MUST_PARSE`** golden set). See [`examples/PARSE_STATUS.md`](../../../examples/PARSE_STATUS.md). |

## Related

- [Deployment compose SSOT](deployment-compose.md) — Compose profiles and Coolify/GitLab notes.
- [CI runner contract](../ci/runner-contract.md) — self-hosted labels and CUDA workflow notes.
- [ADR 005 / Socrates](../adr/) — policy and orchestration gates (index in repo).
