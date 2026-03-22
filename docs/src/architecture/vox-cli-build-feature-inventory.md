---
title: "vox-cli build and feature inventory"
category: architecture
last_updated: 2026-03-21
---

# vox-cli build and feature inventory

Single place to see **which Cargo features pull which dependency blocks** and how that affects compile time. Use with [CLI scope policy](cli-scope-policy.md), [trim-build-defer policy](trim-build-defer-policy.md), and `vox ci build-timings`.

## Default features (minimal compiler loop)

| Feature | Default | Compile impact (high level) |
|---------|---------|-------------------------------|
| *(none)* | when using `--no-default-features` | Compiler pipeline + `vox-db` + **`vox-corpus`** + **`vox-runtime`** (always linked for training JSONL / grammar paths); **no** `vox populi …` surface (`populi-base` off) and **no** Oratio / native train |
| `populi-base` | **yes** | Marker: enables `vox populi …` CLI (corpus commands, etc.) without `vox-populi` / Oratio — **`vox-corpus` / `vox-runtime` are not feature-gated** |
| `populi-oratio` | **no** (opt-in) | Implies `populi-base` + `vox-oratio` (Candle Whisper STT) — heavy; enables `vox populi oratio` |
| `gpu` | **no** (opt-in) | Adds `vox-populi` + `vox-tensor` with `train` / HF / Candle QLoRA stack — **largest** incremental cost |

## Optional features (alphabetical by concern)

| Feature | Extra deps / notes |
|---------|-------------------|
| `ars` | `vox-ars` |
| `coderabbit` | `vox-forge`, `vox-git`, `vox-toestub`, … |
| `codex` | `vox-eval`, `walkdir`, `dirs` — DB via **`vox-db`** (Codex types) |
| `dashboard` | No-op flag (reserved) |
| `execution-api` | `axum`, `tokio-stream`, implies `script-execution` + **`gpu`** |
| `extras-ludus` | `vox-ludus`, `vox-toestub` |
| `island` | `comfy-table`, `dirs`, `walkdir`, `which` |
| `live` | `vox-orchestrator` |
| `mesh` | `vox-mesh` + `transport` (axum / reqwest / tokio) — `vox mesh …` |
| `workflow-runtime` | `populi-dei` + `vox-workflow-runtime` (implies `mesh` via that crate) — interpreted workflow run |
| `populi-candle-cuda` | `gpu` + `vox-populi/candle-qlora-cuda` (nvcc / CUDA toolkit at build time) |
| `populi-candle-metal` | `gpu` + Metal Candle stack (macOS) |
| `populi-dei` | `vox-tensor/train` without full Populi (legacy `vox train` path) |
| `populi-oratio` | `populi-base` + `vox-oratio` — STT CLI (`vox populi oratio`, `vox ai oratio`) |
| `populi-qlora` | Alias for **`gpu`** (QLoRA is in the `train` feature chain) |
| `script-execution` | `wasmtime`, `wasmtime-wasi`, `landlock` / `win32job`, … |
| `stub-check` | `vox-toestub`, `vox-ludus`, … — DB via **`vox-db`** |

## Workspace binaries (`vox-cli`)

| Binary | `required-features` | Purpose |
|--------|---------------------|---------|
| `vox` | *(none)* | Main CLI |
| `vox-compilerd` | *(none)* | Watch / compile daemon |
| `vox-populi` | `populi-base` | Same CLI surface as `vox populi …` without typing the `populi` subcommand (argv injection) |

## Crate categories (where “like lives with like”)

| Bucket | Crates | Rationale |
|--------|--------|-----------|
| Compiler front | `vox-lexer`, `vox-parser`, `vox-ast`, `vox-hir`, `vox-typeck` | Tight pipeline; parallel incremental builds |
| Codegen | `vox-codegen-rust`, `vox-codegen-ts`, `vox-codegen-llvm`, `vox-codegen-wasm` | Backends isolated from CLI features |
| Data plane | `vox-db`, `vox-pm`, `vox-codex` (compat facade over `vox-db`) | Turso / Arca / Codex naming SSOT |
| ML / training | `vox-populi`, `vox-tensor`; `vox-corpus` linked always, native stack gated behind **`gpu`** | Keep Burn/Candle off default lane; corpus types shared |
| Agent / MCP | `vox-mcp`, `vox-orchestrator`, `vox-repository` | Optional tooling surfaces |

## Keyring / secrets

OS keyring helpers live on **`vox-db`** as `vox_db::secrets`. The `vox-codex` crate re-exports them for the historical `vox_codex::secrets` path.

## Measuring build time

- Local / CI: `vox ci build-timings` (human table or `--json`). Add **`--crates`** for extra isolated `cargo check -p …` lanes (`vox-cli --no-default-features`, `vox-db`, `vox-oratio`, `vox-populi --features train`) — see [crate-build-lanes migration](crate-build-lanes-migration.md).
- CUDA lane is skipped unless `nvcc` is on `PATH` (same policy as `vox ci cuda-features`).
