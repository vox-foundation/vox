---
phase: 64-polyglot-tool-form-auto-surface
status: passed
verified_at: "2026-10-15"
requirements_verified:
  - POLYTOOL-01
  - POLYTOOL-02
  - POLYTOOL-03
  - POLYTOOL-04
---

# Phase 64 Verification Report: Polyglot `@tool` & `@form` Auto-Surface (`POLYTOOL-01..04`)

## Requirement Matrix
| Requirement | Artifact / Symbol | Verification Command | Result |
|---|---|---|---|
| `POLYTOOL-01` | `FieldConstraint` / `HirFieldConstraint` (`Step`, `Unit`, `FileKind`, `MutuallyExclusiveGroup`, `Preset`, `CostCredits`) in `vox-ast` + `vox-compiler` | `cargo test -p vox-codegen --lib polyglot_tool_tests` | PASS |
| `POLYTOOL-02` | `PolyglotToolBundle` (`json_schema_and_ui`, `nushell_completions`, `openapi_paths`) in `crates/vox-codegen/src/codegen_rust/emit/client.rs` | `cargo test -p vox-codegen --lib polyglot_tool_tests` | PASS |
| `POLYTOOL-03` | `emit_tool_polyglot_artifacts` MCP + RPC bundle in `crates/vox-codegen/src/codegen_rust/emit/client.rs` | `cargo test -p vox-codegen --lib polyglot_tool_tests` | PASS |
| `POLYTOOL-04` | `emit_form` + `emit_tool_form` (`data-testid="tool-form-argv-preview"`, `data-testid="tool-form-preset-*"`) in `crates/vox-codegen-ts/src/form_emit.rs` | `cargo test -p vox-codegen --lib form_emit::tests` | PASS |
