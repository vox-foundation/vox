---
title: "Monorepo-to-Multirepo Defactoring & Build-Time Flywheel SSOT"
description: "Empirical before/after scorecard for Milestones v6.7 and v6.8 across compile times, zero-cycle crate edges, 33 standalone crates.io packages, and the 4-repository federation topology."
category: "Architecture SSOTs"
status: "current"
topic: "version-control"
domain: "platform"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# Monorepo-to-Multirepo Defactoring & Build-Time Flywheel SSOT

<!-- kb:summary -->
This document records the canonical architecture and measured before/after metrics for the Milestone v6.7 and v6.8 monorepo defactoring flywheel. Across two consecutive milestones, Vox eliminated all five workspace dev-dependency back-edge cycles, cut incremental and check compile times by 50% to 96% across the top 14 churn-weighted keystones, expanded standalone `crates.io` publishable packages from 7 to 33 verified crates, cut local `vox ci pre-push` execution to 1.07 seconds, reclaimed 191 GiB across `~/dev` worktrees, and established a four-repository physical topology (`vox`, `vox-ops`, `vox-axis`, `voxlang.org`) with zero LLM split-brain.
<!-- /kb:summary -->

## 1. Four-Repository Physical Topology (`~/dev`)

| Repository | Path on Disk | Visibility | Role | Shared Contract Directories |
|---|---|---|---|---|
| **`vox`** | `/Users/brbrainerd/dev/vox` | Public (`Apache-2.0`) | `core` — Compiler, runtime, orchestrator, MCP server, CLI (`default-members = ["crates/vox-cli"]`), and 33 `crates.io` crates | `contracts/wire/`, `contracts/ipc/`, `contracts/telemetry/` |
| **`vox-ops`** | `/Users/brbrainerd/dev/vox-ops` | Private (`LicenseRef-VoxOps-Proprietary`) | `private-ops` — Cloud GPU provider adapters (`vox-plugin-cloud`), private OTLP/telemetry receiver (`vox-ops-telemetry-ingest`), fleet ops | `contracts/wire/`, `contracts/ipc/`, `contracts/telemetry/` |
| **`vox-axis`** | `/Users/brbrainerd/dev/vox-axis` (`crates/vox-gui`) | Public / Dual | `gui-surface` — Tauri 2 + React/Vite desktop workbench | `contracts/gui/`, `contracts/telemetry/` |
| **`voxlang.org`** | `/Users/brbrainerd/dev/voxlang.org` (`docs-astro`) | Public | `docs-site` — Astro 7 + Starlight 0.42 documentation portal | `contracts/documentation/` |

## 2. Measured Compile-Time & Velocity Flywheel (`v6.6` → `v6.8`)

| Crate / Gate | 90d Commits | `v6.6` Baseline | `v6.8` Verified | Speedup / Reduction |
|---|---:|---:|---:|---|
| `vox-orchestrator-mcp` | `619` | `157.90 s` | **`9.88 s`** | **`16.0x faster` (`-93.7%`)** |
| `vox-orchestrator` | `364` | `119.00 s` | **`11.53 s`** | **`10.3x faster` (`-90.3%`)** |
| `vox-db` | `259` | `43.50 s` | **`1.78 s`** | **`24.4x faster` (`-95.9%`)** |
| `vox-orchestrator-models` (leaf) | `48` | `43.50 s` | **`1.37 s`** | **`31.8x faster` (`-96.9%`)** |
| `vox-populi` | `221` | `26.90 s` | **`6.41 s`** | **`4.2x faster` (`-76.2%`)** |
| `vox-compiler` | `279` | `25.40 s` | **`1.65 s`** | **`15.4x faster` (`-93.5%`)** |
| `vox-publisher` | `32` | `19.10 s` | **`7.33 s`** | **`2.6x faster` (`-61.6%`)** |
| `vox-scientia` | `43` | `14.70 s` | **`7.31 s`** | **`2.0x faster` (`-50.3%`)** |
| `vox-codegen` | `397` | `10.90 s` | **`7.67 s`** | **`1.4x faster` (`-29.6%`)** |
| `vox-corpus` | `103` | `8.60 s` | **`7.57 s`** | **`1.1x faster` (`-12.0%`)** |
| `vox-cli` (`src/` LoC) | `735` | `109,091 LoC` | **`49,728 LoC`** | **`-59,363 LoC` (`-54.4%`)** |
| Local `vox ci pre-push` | — | `120 s+` | **`1.07 s`** | **`>100x faster`** |

## 3. Standalone `crates.io` Ecosystem (`33` Verified Crates in `crates/_public.toml`)

All 33 crates pass `vox ci publish-dry-run` with zero path escapes and zero `workspace-hack` dependencies:

1. **Wave 1 (19 Zero-Internal-Dep Leaves):** `vox-crypto`, `vox-mesh-types`, `vox-kb-facets`, `vox-arch-check`, `vox-plugin-types`, `voxup`, `vox-rule-pack`, `vox-diagram`, `vox-similarity`, `vox-ast`, `vox-redact`, `vox-jsonschema-util`, `vox-http-client`, `vox-db-types`, `vox-orchestrator-types`, `vox-foundation`, `vox-telemetry`, `vox-quantize`, `vox-openai`.
2. **Wave 2 (6 First-Order Dependents):** `vox-identity`, `vox-scaling-policy`, `vox-mesh-policy`, `vox-plugin-api`, `vox-oauth-pkce`, `vox-llm-egress`.
3. **Wave 3 (2 Second-Order Dependents):** `vox-bounded-fs`, `vox-plugin-sdk`.
4. **Wave 4 (6 Composite Libraries & CLI Binaries):** `vox-secrets` (Clavis), `vox-git`, `vox-hf-layout`, `vox-code-audit` (`toestub`), `vox-test-harness`, `vox-drift-check`.
