---
phase: 64
phase_name: "polyglot-tool-form-auto-surface"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 64 Learnings: polyglot-tool-form-auto-surface

## Decisions

### Extended FieldConstraint (crates/vox-ast/src/decl/form.rs) and HirFieldC...
Extended `FieldConstraint` (`crates/vox-ast/src/decl/form.rs`) and `HirFieldConstraint` (`crates/vox-compiler/src/hir/nodes/form.rs`) with `Step(f64)`, `Unit(String)`, `FileKind(String)`, `MutuallyExclusiveGroup(String)`, `Preset(String, Expr/HirExpr)`, and `CostCredits(u32)`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 64-01-SUMMARY.md

---

### Wired @form constraint parsing (crates/vox-compiler/src/parser/descent/d...
Wired `@form` constraint parsing (`crates/vox-compiler/src/parser/descent/decl/head_form.rs`) and HIR lowering (`crates/vox-compiler/src/hir/lower/decl.rs`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 64-01-SUMMARY.md

---

### Implemented PolyglotToolBundle and emit_tool_polyglot_artifacts in crate...
Implemented `PolyglotToolBundle` and `emit_tool_polyglot_artifacts` in `crates/vox-codegen/src/codegen_rust/emit/client.rs` (and re-exported from `crates/vox-codegen/src/codegen_rust/emit/mod.rs`) emitting JSON Schema Draft 2020-12 + `uiSchema`, CLI flags + Nushell `extern` completions, OpenAPI 3.1 `/api/tools/<name>` specs, and Axum/Tauri/MCP dispatch.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 64-01-SUMMARY.md

---

### Upgraded emit_form in crates/vox-codegen-ts/src/form_emit.rs to render R...
Upgraded `emit_form` in `crates/vox-codegen-ts/src/form_emit.rs` to render `Range` (`min`/`max`), `Step`, `Unit` badge, `FileKind` (`type="file"` + `accept`), `MutuallyExclusiveGroup` validation, and `CostCredits` badge (`data-testid="vox-form-cost-credits"`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 64-02-SUMMARY.md

---

### Added emit_tool_form in crates/vox-codegen-ts/src/form_emit.rs generatin...
Added `emit_tool_form` in `crates/vox-codegen-ts/src/form_emit.rs` generating `<ToolForm_<name> />` React components with preset profile buttons, live CLI `argv` preview (`data-testid="tool-form-argv-preview"`), and streaming execution state.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 64-02-SUMMARY.md

## Lessons

## Patterns

## Surprises

