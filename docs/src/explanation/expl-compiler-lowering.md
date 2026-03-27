---
title: "Explanation: Compiler Lowering Phases"
description: "Official documentation for Explanation: Compiler Lowering Phases for the Vox language. Detailed technical reference, architecture guides,"
category: "explanation"
last_updated: 2026-03-26
training_eligible: true
---
# Explanation: Compiler Lowering Phases

Understand how the Vox compiler transforms high-level source code into optimized Rust and TypeScript output.

Implementation note: current production code keeps these stages under `crates/vox-compiler/src/` with explicit modules for parser, HIR lowering, typecheck, and dual-target emitters.

## 1. Syntax to AST (Abstract Syntax Tree)

The `vox-parser` converts the raw `.vox` file into a tree of declarations. This phase ensures the code is syntactically valid but does not yet understand types or decorators.

## 2. AST to HIR (High-level Intermediate Representation)

The **Lowering** phase begins by transforming the AST into the HIR.
- **Symbol Resolution**: Linking variable names to their definitions.
- **Decorator Processing**: Expanding decorators like `@server` into their underlying architectural primitives (handlers, endpoints, clients).
- **Type Inference**: Deducing types for all expressions.

## 3. HIR to WebIR and LIR (Low-level intermediate layers)

[ADR 012](../adr/012-internal-web-ir-strategy.md) introduces **WebIR** (`crates/vox-compiler/src/web_ir/`) as the normative structured layer before React/TanStack printers. **`lower_hir_to_web_ir`** lowers reactive `view:` JSX (plus `routes:` contracts and behavior summaries) into **`WebIrModule`**; **`validate_web_ir`** checks DOM id references; **`emit_component_view_tsx`** is a JSX string preview used for parity tests.

Current production behavior (important for migration planning):

- `codegen_ts` still assembles production TS/TSX output on the primary path.
- `VOX_WEBIR_VALIDATE=1` runs WebIR lower/validate as a fail-fast gate.
- `VOX_WEBIR_EMIT_REACTIVE_VIEWS=1` enables reactive `view:` bridge output via WebIR preview emit only when parity checks pass.
- The two flags are related but not equivalent; validation can be enabled without switching reactive view emission.

**Operations catalog + gates:** [WebIR operations catalog](../architecture/internal-web-ir-implementation-blueprint.md#operations-catalog-op-0001op-0320) and [acceptance gates G1–G6](../architecture/internal-web-ir-implementation-blueprint.md#acceptance-gates-specific-filetest-thresholds) (includes supplemental **OP-S049–OP-S220** rustc/doc gates). **Roadmap link pass A (OP-S130, OP-S131, OP-S209–OP-S211):** keep lowering docs aligned when renaming validation stages.

Separately, **backend-oriented** lowering remains optimized for Rust emission (database, actors, HTTP). The older “Frontend LIR” label maps to this split: **WebIR** for structured web UI, **HIR emitters** for expedient TS until the printer fully migrates.

## 4. Code Generation (Emission)

The final phase where LIR is converted into source files:
- **`vox-codegen-rust`**: Produces `main.rs`, `models.rs`, and `api.rs`.
- **`vox-codegen-ts`**: Produces `App.tsx`, `client.ts`, and `types.ts`.

For frontend IR layering and migration phases, see [ADR 012 — Internal web IR strategy](../adr/012-internal-web-ir-strategy.md).
For detailed implementation sequencing, see [Internal Web IR implementation blueprint](../architecture/internal-web-ir-implementation-blueprint.md).
For ordered file-by-file migration operations, see [WebIR operations catalog](../architecture/internal-web-ir-implementation-blueprint.md#operations-catalog-op-0001op-0320).
For exact current-vs-target representation mapping, see [Internal Web IR side-by-side schema](../architecture/internal-web-ir-side-by-side-schema.md).
For quantified token+grammar+escape-hatch savings on the canonical app, see [WebIR K-complexity quantification](../architecture/internal-web-ir-side-by-side-schema.md#k-complexity-quantification).
For reproducible counting registries and equation trace, see [WebIR K-metric appendix](../architecture/internal-web-ir-side-by-side-schema.md#k-metric-appendix-reproducible).

## 5. Why Lowering Matters?

By having multiple intermediate representations, Vox can perform complex architectural optimizations—like automatically grouping database queries or optimizing actor communication—that would be impossible in a single-pass compiler.

---

**Related Reference**:
- [Architecture Index](expl-architecture.md) — High-level map of the compiler crates.
- [API Reference: vox-hir](../api/vox-hir.md) — Details on the HIR data structures.
