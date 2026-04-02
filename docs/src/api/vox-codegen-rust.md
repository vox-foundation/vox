---
title: "Compiler Module: vox-codegen-rust"
description: "Official documentation for Compiler Module: vox-codegen-rust for the Vox language. Detailed technical reference, architecture guides, and imple"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Compiler Module: vox-codegen-rust

> [!WARNING]
> This is not a standalone crate. It is a sub-module located at `crates/vox-compiler/src/codegen_rust/`.

## Overview

Rust code generator for the Vox compiler. Emits Axum server code, library types/functions, **Codex** (`vox-db`) connection + libSQL/Turso schema setup, optional TypeScript API client, and optional MCP stdio server.

## Source layout

| File | Purpose |
|------|---------|
| `src/lib.rs` | Re-exports: `generate`, `generate_script`, `CodegenOutput`, and `emit` submodule. |
| `src/emit/mod.rs` | Full-app emit: `Cargo.toml`, `main.rs`, `lib.rs`, tables, MCP, API client. |
| `src/pipeline.rs` | Script targets (`Native` / `Wasi`) and `generate_script_with_target`. |

## Public entry points

- **`generate(module, package_name) -> Result<CodegenOutput, miette::Error>`** — Builds `Cargo.toml`, `src/main.rs`, `src/lib.rs`, optional `src/mcp_server.rs`, and `api_client_ts`. Merges `[dependencies]` from `module.rust_imports` (from `import rust:…` in source).
- **`emit_cargo_toml(name, module)`** — Same dependency manifest as full-app `generate`; includes dynamic lines for each `HirRustImport`.
- **`generate_script_with_target(module, package_name, runtime_path, ScriptTarget::Native|Wasi)`** — Script cache crate; merges `module.rust_imports` into the generated `Cargo.toml` (with WASI guardrails for incompatible crates).
- **`emit_main` / `emit_lib`** — Lower-level; same output as used inside `generate`.
- **`emit_fn` / `emit_expr`** — Emit one function or expression (tests and tooling).
- **`emit_table_ddl` / `emit_index_ddl`** — SQL strings for tables and indexes.
- **`emit_api_client` / `emit_mcp_server`** — TS client and MCP server sources.

## Generated runtime behavior (`main.rs`)

- Listens on **`VOX_PORT`** (default `3000`).
- Opens **Codex** via **`vox_db::DbConfig::resolve_standalone()`** when `@table` is present (honors **`VOX_DB_URL` / `VOX_DB_TOKEN`**, **`VOX_DB_PATH`**, or legacy **`TURSO_*`**; see ADR 004 / `vox-db` config).
- Uses **`.expect(...)`** (not bare `.unwrap()`) on `TcpListener::bind` and `axum::serve`.

## Usage

```rust
use vox_codegen_rust::generate;

let output = generate(&hir_module, "my_app")?;
let main_rs = output.files.get("src/main.rs").expect("main");
```
