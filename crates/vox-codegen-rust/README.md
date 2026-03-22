# vox-codegen-rust

Rust code generator for the Vox compiler. Emits Axum server code, actor/workflow/activity helpers, Turso-backed `@table` types, MCP server stub, and test harnesses.

## Layout

| Path | Role |
|------|------|
| [`src/lib.rs`](src/lib.rs) | Crate root; re-exports `generate`, `CodegenOutput`, `emit_fn`, `emit_expr`. |
| [`src/emit.rs`](src/emit.rs) | **SSOT** for codegen: `generate`, `emit_main`, `emit_lib`, expression emission, table DDL/helpers, API client, MCP server. |

Historical split modules (`emit_main.rs`, `emit_lib.rs`, etc.) were removed; do not reintroduce parallel emit paths.

## Decorator / HIR mapping (high level)

| Vox / HIR | Generated Rust |
|-----------|----------------|
| `@server` / `HirServerFn` | Axum `post(...)` route + handler |
| `@table` / `HirTable` | `struct` + async Turso helpers + `CREATE TABLE` in `main` |
| `@index` / `HirIndex` | `CREATE INDEX` in `main` |
| `@test` / `module.tests` | `#[test]` or `#[tokio::test]` + emitted body |
| `actor` / `HirActor` | Message enum + handler scaffolding |
| `workflow` / `HirWorkflow` | `pub async fn` orchestrator body |
| JSX in Rust backend | `panic!(\"JSX cannot be rendered via the Rust backend yet\")` placeholder |

## Usage

```rust
use vox_codegen_rust::generate;

let output = generate(&hir_module, "my_app")?;
// output.files["src/main.rs"], output.files["src/lib.rs"], …
// output.api_client_ts — TS client when server fns exist
```
