---
title: "Layer 3 — Compiler, VoxDb, Orchestrator, Search & Runtimes (Generated)"
description: "Core execution engines: unified Vox compiler/codegen, VoxDb/Turso, multi-agent orchestrator, hybrid search, and Populi/Mens."
category: "Architecture SSOTs"
status: "current"
sort_order: 13
code_anchors:
  - "docs/src/architecture/layers.toml"
---

# Layer 3 — Compiler, VoxDb, Orchestrator, Search & Runtimes

Core execution engines: unified Vox compiler/codegen, VoxDb/Turso, multi-agent orchestrator, hybrid search, and Populi/Mens.

## Subsystem Topology

```vox-diagram
graph {
    direction: down
    node l3_heavy_runtimes: "Layer 3 — Compiler, VoxDb, Orchestrator, Search & Runtimes"
    node vox_actor_runtime: "vox-actor-runtime (library)"
    l3_heavy_runtimes -> vox_actor_runtime
    node vox_cli_core: "vox-cli-core (library)"
    l3_heavy_runtimes -> vox_cli_core
    node vox_code_audit: "vox-code-audit (library)"
    l3_heavy_runtimes -> vox_code_audit
    node vox_codegen: "vox-codegen (library)"
    l3_heavy_runtimes -> vox_codegen
    node vox_codegen_ts: "vox-codegen-ts (library)"
    l3_heavy_runtimes -> vox_codegen_ts
    node vox_compiler: "vox-compiler (library)"
    l3_heavy_runtimes -> vox_compiler
    node vox_container: "vox-container (library)"
    l3_heavy_runtimes -> vox_container
    node vox_corpus: "vox-corpus (library)"
    l3_heavy_runtimes -> vox_corpus
    node vox_db: "vox-db (library)"
    l3_heavy_runtimes -> vox_db
    node vox_deploy_codegen: "vox-deploy-codegen (library)"
    l3_heavy_runtimes -> vox_deploy_codegen
    node vox_doc_pipeline: "vox-doc-pipeline (library)"
    l3_heavy_runtimes -> vox_doc_pipeline
    node vox_drift_check: "vox-drift-check (binary)"
    l3_heavy_runtimes -> vox_drift_check
    node vox_effort_audit: "vox-effort-audit (library)"
    l3_heavy_runtimes -> vox_effort_audit
    node vox_effort_route: "vox-effort-route (library)"
    l3_heavy_runtimes -> vox_effort_route
    node vox_forge: "vox-forge (library)"
    l3_heavy_runtimes -> vox_forge
    node vox_gamify: "vox-gamify (library)"
    l3_heavy_runtimes -> vox_gamify
    node vox_git: "vox-git (library)"
    l3_heavy_runtimes -> vox_git
    node vox_lsp: "vox-lsp (library)"
    l3_heavy_runtimes -> vox_lsp
    node vox_ml_cli: "vox-ml-cli (binary)"
    l3_heavy_runtimes -> vox_ml_cli
    node vox_oauth_pkce: "vox-oauth-pkce (library)"
    l3_heavy_runtimes -> vox_oauth_pkce
}
```

## Crates in Layer

| Crate | Kind | Fan-In Cap | LoC Baseline |
|---|---|---|---|
| [`vox-actor-runtime`](../../api/vox-actor-runtime.generated.md) | `library` | — | 15000 |
| [`vox-cli-core`](../../api/vox-cli-core.generated.md) | `library` | — | — |
| [`vox-code-audit`](../../api/vox-code-audit.generated.md) | `library` | — | 25000 |
| [`vox-codegen`](../../api/vox-codegen.generated.md) | `library` | — | 25000 |
| [`vox-codegen-ts`](../../api/vox-codegen-ts.generated.md) | `library` | — | 18000 |
| [`vox-compiler`](../../api/vox-compiler.generated.md) | `library` | 21 | 45000 |
| [`vox-container`](../../api/vox-container.generated.md) | `library` | — | — |
| [`vox-corpus`](../../api/vox-corpus.generated.md) | `library` | — | 15000 |
| [`vox-db`](../../api/vox-db.generated.md) | `library` | 27 | 40000 |
| [`vox-deploy-codegen`](../../api/vox-deploy-codegen.generated.md) | `library` | — | — |
| [`vox-doc-pipeline`](../../api/vox-doc-pipeline.generated.md) | `library` | — | — |
| [`vox-drift-check`](../../api/vox-drift-check.generated.md) | `binary` | — | 8000 |
| [`vox-effort-audit`](../../api/vox-effort-audit.generated.md) | `library` | — | 4000 |
| [`vox-effort-route`](../../api/vox-effort-route.generated.md) | `library` | — | 4000 |
| [`vox-forge`](../../api/vox-forge.generated.md) | `library` | — | — |
| [`vox-gamify`](../../api/vox-gamify.generated.md) | `library` | — | 20000 |
| [`vox-git`](../../api/vox-git.generated.md) | `library` | — | — |
| [`vox-lsp`](../../api/vox-lsp.generated.md) | `library` | — | — |
| [`vox-ml-cli`](../../api/vox-ml-cli.generated.md) | `binary` | — | 20000 |
| [`vox-oauth-pkce`](../../api/vox-oauth-pkce.generated.md) | `library` | — | 2000 |
| [`vox-openclaw-runtime`](../../api/vox-openclaw-runtime.generated.md) | `library` | — | — |
| [`vox-orchestrator`](../../api/vox-orchestrator.generated.md) | `library` | 30 | 70000 |
| [`vox-orchestrator-driver`](../../api/vox-orchestrator-driver.generated.md) | `library` | — | — |
| [`vox-orchestrator-mcp`](../../api/vox-orchestrator-mcp.generated.md) | `library` | — | 40000 |
| [`vox-orchestrator-models`](../../api/vox-orchestrator-models.generated.md) | `library` | — | — |
| [`vox-orchestrator-queue`](../../api/vox-orchestrator-queue.generated.md) | `library` | — | — |
| [`vox-orchestrator-test-helpers`](../../api/vox-orchestrator-test-helpers.generated.md) | `test-only` | 25 | — |
| [`vox-package`](../../api/vox-package.generated.md) | `library` | — | — |
| [`vox-plugin-catalog`](../../api/vox-plugin-catalog.generated.md) | `library` | — | — |
| [`vox-plugin-host`](../../api/vox-plugin-host.generated.md) | `library` | — | — |
| [`vox-plugin-test-harness`](../../api/vox-plugin-test-harness.generated.md) | `test-only` | — | — |
| [`vox-populi`](../../api/vox-populi.generated.md) | `library` | — | 20000 |
| [`vox-publisher`](../../api/vox-publisher.generated.md) | `library` | — | 22000 |
| [`vox-research-shim`](../../api/vox-research-shim.generated.md) | `library` | — | 8000 |
| [`vox-rn-codegen`](../../api/vox-rn-codegen.generated.md) | `library` | — | 6000 |
| [`vox-runtime-rn`](../../api/vox-runtime-rn.generated.md) | `library` | — | — |
| [`vox-scientia`](../../api/vox-scientia.generated.md) | `library` | — | 15000 |
| [`vox-scientia-jsonschema-codegen`](../../api/vox-scientia-jsonschema-codegen.generated.md) | `binary` | — | — |
| [`vox-search`](../../api/vox-search.generated.md) | `library` | — | — |
| [`vox-skill-discovery`](../../api/vox-skill-discovery.generated.md) | `binary` | — | 8000 |
| [`vox-skills`](../../api/vox-skills.generated.md) | `library` | — | — |
| [`vox-speech`](../../api/vox-speech.generated.md) | `library` | — | — |
| [`vox-sql`](../../api/vox-sql.generated.md) | `library` | 20 | 8000 |
| [`vox-tauri-codegen`](../../api/vox-tauri-codegen.generated.md) | `library` | — | 6000 |
| [`vox-tauri-stt`](../../api/vox-tauri-stt.generated.md) | `library` | — | — |
| [`vox-telemetry-otlp`](../../api/vox-telemetry-otlp.generated.md) | `library` | — | 3000 |
| [`vox-tensor`](../../api/vox-tensor.generated.md) | `library` | — | — |
| [`vox-terminal-core`](../../api/vox-terminal-core.generated.md) | `library` | — | 25000 |
| [`vox-test-harness`](../../api/vox-test-harness.generated.md) | `library` | — | — |
| [`vox-vcs`](../../api/vox-vcs.generated.md) | `library` | — | 20000 |
| [`vox-wasm-engine`](../../api/vox-wasm-engine.generated.md) | `library` | — | — |
| [`vox-wire-format-validator`](../../api/vox-wire-format-validator.generated.md) | `binary` | — | 4000 |
| [`vox-workflow-runtime`](../../api/vox-workflow-runtime.generated.md) | `library` | — | — |
