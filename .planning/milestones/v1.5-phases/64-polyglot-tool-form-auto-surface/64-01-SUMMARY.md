---
phase: 64-polyglot-tool-form-auto-surface
plan: 01
subsystem: codegen-polyglot-tool
tags: [tool, form, openapi, json-schema, nushell, mcp]
completed: 2026-10-15
---

# Plan 64-01 Summary: Polyglot `@tool` & `@form` Parameter Facets & Multi-Surface Codegen (`POLYTOOL-01..03`)

## Accomplishments
- Extended `FieldConstraint` (`crates/vox-ast/src/decl/form.rs`) and `HirFieldConstraint` (`crates/vox-compiler/src/hir/nodes/form.rs`) with `Step(f64)`, `Unit(String)`, `FileKind(String)`, `MutuallyExclusiveGroup(String)`, `Preset(String, Expr/HirExpr)`, and `CostCredits(u32)`.
- Wired `@form` constraint parsing (`crates/vox-compiler/src/parser/descent/decl/head_form.rs`) and HIR lowering (`crates/vox-compiler/src/hir/lower/decl.rs`).
- Implemented `PolyglotToolBundle` and `emit_tool_polyglot_artifacts` in `crates/vox-codegen/src/codegen_rust/emit/client.rs` (and re-exported from `crates/vox-codegen/src/codegen_rust/emit/mod.rs`) emitting JSON Schema Draft 2020-12 + `uiSchema`, CLI flags + Nushell `extern` completions, OpenAPI 3.1 `/api/tools/<name>` specs, and Axum/Tauri/MCP dispatch.

## Verification
- Verified via `vox-codegen` unit tests in `crates/vox-codegen/src/codegen_rust/emit/client.rs`.
