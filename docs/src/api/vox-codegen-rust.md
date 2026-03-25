---
title: "Crate API: vox-codegen-rust"
description: "Official documentation for Crate API: vox-codegen-rust for the Vox language. Detailed technical reference, architecture guides, and imple"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Crate API: vox-codegen-rust

## Overview

Rust code generator for the Vox compiler. Emits Axum server code, library types/functions, **Codex** (`vox-db`) connection + libSQL/Turso schema setup, optional TypeScript API client, and optional MCP stdio server.

## Source layout

| File | Purpose |
|------|---------|
| `src/lib.rs` | Re-exports: `generate`, `CodegenOutput`, `emit_fn`, `emit_expr`. |
| `src/emit.rs` | Single implementation module: full pipeline and helpers. |

## Public entry points

- **`generate(module, package_name) -> Result<CodegenOutput, miette::Error>`** — Builds `Cargo.toml`, `src/main.rs`, `src/lib.rs`, optional `src/mcp_server.rs`, and `api_client_ts`.
- **`emit_cargo_toml(name)`** — Dependency manifest for the synthesized crate (`vox-db`, Turso/libSQL, Axum, etc.).
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
