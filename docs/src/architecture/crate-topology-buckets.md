---
title: "Crate topology buckets"
description: "Official documentation for Crate topology buckets for the Vox language. Detailed technical reference, architecture guides, and implementa"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# Crate topology buckets

Aggressive **like-with-like** map for every workspace member under `crates/*` (excludes `[workspace.exclude]`:
`vox-dei`, `vox-py`, `vox-wasm`). **`vox-codegen-html`** was removed from the exclude list—use **`vox-ssg`** for static HTML shells. Use this when deciding where new code lives or which crate to depend on.

| Bucket | Crates | Build / ownership notes |
|--------|--------|---------------------------|
| **Compiler core** | `vox-lexer`, `vox-parser`, `vox-ast`, `vox-hir`, `vox-typeck`, `vox-fmt` | Tight pipeline; keep independent for incremental builds. |
| **Codegen** | `vox-codegen-rust`, `vox-codegen-ts`, `vox-codegen-llvm`, `vox-codegen-wasm` | Backends isolated from ML and heavy I/O. |
| **Data plane** | `vox-db`, `vox-pm`, `vox-codex` (compat re-export over `vox-db`) | Canonical API: **`vox_db::VoxDb`**. `vox-codex` is legacy import path only. |
| **Repository / workspace** | `vox-repository`, `vox-config` | Repo root, `repository_id`, `Vox.toml` — do not reimplement layout detection in random CLI modules. |
| **Runtime / HTTP** | `vox-runtime`, `vox-codex-api` | Axum/Tokio surfaces; optional `database` feature on runtime. |
| **ML / training** | `vox-mens`, `vox-tensor`, `vox-corpus`, `vox-oratio` | Heavy deps; gate behind **`vox-cli`** features `gpu`, `oratio`, `mens-candle-cuda`. |
| **Agent / MCP / orchestration** | `vox-mcp`, `vox-orchestrator`, `vox-ars`, `vox-tools`, `vox-capability-registry` | Tooling and routing; optional in default CLI. |
| **Quality / policy** | `vox-toestub`, `vox-socrates-policy`, `vox-eval`, `vox-doc-inventory` | CI, lint policy, doc SSOT generation. |
| **Integration / harness** | `vox-integration-tests`, `vox-test-harness` | Not part of default `vox-cli` graph. |
| **Infra / misc** | `vox-cli`, `vox-lsp`, `vox-bootstrap`, `vox-container`, `vox-doc-pipeline`, `vox-forge`, `vox-gamify`, `vox-git`, `vox-ludus`, `vox-skills`, `vox-ssg`, `vox-storage`, `vox-webhook` | `vox-cli` fans out by feature; keep **default** lane lean. |

## Anti-patterns (aggressive)

- New `vox_codex::` imports in workspace crates — use **`vox_db::`**.
- Heavy ML deps on `vox-lsp` or default `vox-cli` without a feature gate.
- Duplicating `repository_id` / repo-root logic outside **`vox-repository`**.

## See also

- [crate-build-lanes-migration.md](crate-build-lanes-migration.md)
- [vox-cli-build-feature-inventory.md](vox-cli-build-feature-inventory.md)
- [external-repositories.md](../reference/external-repositories.md)
