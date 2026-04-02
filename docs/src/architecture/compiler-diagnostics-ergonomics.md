---
title: "Compiler diagnostics and Rust codegen ergonomics"
description: "Policy for miette vs custom diagnostics and when to adopt quote/prettyplease in Rust emitters."
category: "architecture"
status: "current"
sort_order: 0
last_updated: 2026-03-29
training_eligible: true
---

# Compiler diagnostics and Rust codegen ergonomics

## Diagnostics: `miette` vs custom errors

**Current state:**

- `miette` is a dependency of `vox-compiler` and is used for **Rust codegen** failures (`codegen_rust/pipeline.rs`, `emit/mod.rs`, projection validation).
- **Parse / typecheck / HIR** use bespoke error types (`ParseError`, `Diagnostic`, `HirValidationError`) mapped to LSP in `vox-lsp`.

**Decision (near term):**

- **No forced unification** until there is bandwidth to thread `Span` ↔ `miette::SourceSpan` (including UTF-16 LSP offsets) through the full pipeline.
- **Directional preference:** when adding **new** rich user-facing errors in codegen paths, use `miette`. For LSP-facing parse/type errors, keep the existing structured diagnostics until a deliberate migration plan exists.

**Rationale:** Unifying on `miette` everywhere is high-touch (CLI, MCP, tests, serde-stable diagnostics); partial adoption already delivers value on codegen.

## Rust emission: `quote` / `prettyplease`

**Current state:** Most Rust output is string emission under `crates/vox-compiler/src/codegen_rust/emit/`.

**Decision:**

- **Pilot first:** pick one hot file (e.g. a small `emit/*` module with heavy escaping) and try `quote!` for syntactic fragments; optionally run `prettyplease` on output in tests only to validate shape.
- **Not a goal:** rewriting the entire emitter to proc-macro style in one pass.

**Rationale:** `quote` reduces nested-quote bugs; full migration is a large formatting and snapshot-test churn.

## References

- `crates/vox-compiler/src/codegen_rust/pipeline.rs`
- `crates/vox-compiler/src/parser/error.rs`
- `crates/vox-compiler/src/typeck/diagnostics.rs`
- `crates/vox-lsp/src/lib.rs` (diagnostic mapping)
