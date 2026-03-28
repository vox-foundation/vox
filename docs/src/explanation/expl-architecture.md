---
title: "Compiler Architecture"
description: "Official documentation for Compiler Architecture for the Vox language. Detailed technical reference, architecture guides, and implementat"
category: "explanation"
last_updated: 2026-03-26
training_eligible: true
---

# Compiler Architecture

The Vox compiler follows a modular pipeline architecture with conceptual stages. The current implementation is consolidated under `crates/vox-compiler/src/`, where each stage is represented by explicit modules.

Current implementation note: the practical pipeline is currently consolidated under `crates/vox-compiler/src/` for lexer, parser, AST, HIR, typecheck, and emitters. This document keeps conceptual stage boundaries while implementation modules may live in one crate.

---

## Pipeline Overview

```
Source Code (.vox)
    │
    ▼
┌──────────────┐
│  vox-lexer   │  Tokenization (logos)
└──────┬───────┘
       │ Vec<Token>
       ▼
┌──────────────┐
│  vox-parser  │  Recursive descent parser → AST Module
└──────┬───────┘
       │ Module (AST root)
       ▼
┌──────────────┐
│   vox-ast    │  Strongly-typed AST wrappers
└──────┬───────┘
       │ Module (Decl, Expr, Stmt, Pattern)
       ▼
┌──────────────┐
│   vox-hir    │  Desugaring + name resolution + dead code detection
└──────┬───────┘
       │ HirModule
       ▼
┌──────────────┐
│  vox-typeck  │  Bidirectional type checking + HM inference
└──────┬───────┘
       │ Typed HIR + Vec<Diagnostic>
       ▼
┌──────────────┐
│   web_ir     │  HIR→WebIR lower + validate
└──────┬───────┘
       │ WebIrModule
       ▼
┌──────────────┐
│ app_contract │  HIR→AppContract (HTTP/RPC/islands/server config)
└──────┬───────┘
       │ AppContractModule
       ▼
┌──────────────┐
│ runtime_proj │  HIR→RuntimeProjection (DB/task capability hints)
└──────┬───────┘
       │ RuntimeProjectionModule
       ▼
┌──────────────────┬─────────────────────┐
│ vox-codegen-rust │  vox-codegen-ts     │
│  (quote! → .rs)  │  (string → .ts/tsx) │
└──────────────────┴─────────────────────┘
```

Current path note:

- `codegen_ts` is still the production TS emitter path.
- `VOX_WEBIR_VALIDATE=1` runs WebIR lower/validate as a build gate.
- `app_contract::project_app_contract` is the SSOT for route/RPC/island/server-config codegen inputs.
- `runtime_projection::project_runtime_from_hir` is the SSOT for orchestration-facing DB capability projection.
- `VOX_WEBIR_EMIT_REACTIVE_VIEWS=1` enables reactive `view:` TSX bridge output only when parity checks pass.

---

## ML Training Pipeline

Vox has a native ML training loop powered by [Burn](https://burn.dev) (a pure-Rust deep learning framework):

```
docs/src/*.md + examples/*.vox
    │
    ▼
vox mens corpus extract   # produces validated.jsonl
    │
    ▼
vox mens corpus pairs     # produces train.jsonl (instruction-response pairs)
    │
    ▼
vox mens train            # native Burn / HF path (default CLI features)
    │
    ▼
mens/runs/v1/model_final.bin
```

The training loop is defined in `crates/vox-cli/src/training/native.rs`.

---

## Stage Details

### 1. Lexer (`vox-compiler::lexer`)

**Purpose**: Converts source text into a flat stream of tokens.

**Implementation**: Uses the [`logos`](https://docs.rs/logos) crate for high-performance, zero-copy tokenization.

**Output**: `Vec<Token>` — each token carries its kind and span.

---

### 2. Parser (`vox-compiler::parser`)

**Purpose**: Transforms a token stream into an AST module.

**Implementation**: A hand-written recursive descent parser producing `ast::decl::Module`. The parser is **resilient to errors**, meaning it continues parsing after encountering invalid syntax — this is critical for LSP support, where the user is actively typing.

**Key features**:
- Error recovery with synchronization points
- Trailing comma support in parameter lists
- Duplicate parameter name detection
- Indentation-aware formatting (`indent.rs`)

See `crates/vox-compiler/src/parser/descent/mod.rs` for the implementation entrypoint.

**Output**: `Module` (AST root) with source spans on declarations and expressions.

---

### 3. AST (`vox-compiler::ast`)

**Purpose**: Strongly-typed wrappers around the untyped CST nodes.

See `crates/vox-compiler/src/ast/` for the node hierarchy.

---

### 6. Code Generation

#### Rust Codegen (`vox-compiler::codegen_rust`)

Emits Rust source using the [`quote!`](https://docs.rs/quote) macro. Each decorator maps to specific Rust constructs:

| Vox | Generated Rust |
|-----|---------------|
| `@server fn` | Axum handler + route registration |
| `@table type` | Struct + SQLite schema |
| `@test fn` | `#[test]` function |
| `@deprecated` | `#[deprecated]` attribute |
| `actor` | Tokio task + mpsc mailbox |
| `workflow` | State machine with durable step recording |

#### TypeScript Codegen (`vox-compiler::codegen_ts`)

Emits TypeScript/TSX in modular files:

| Module | Output |
|--------|--------|
| `jsx.rs` | React JSX components |
| `component.rs` | Component declarations and hooks |
| `activity.rs` | Activity/workflow client wrappers |
| `emitter.rs` | TanStack Router trees, optional server fns, islands metadata |
| `adt.rs` | TypeScript discriminated union types |

Normative strategy for reducing frontend emitter complexity while preserving React interop:
[ADR 012 — Internal web IR strategy](../adr/012-internal-web-ir-strategy.md).
Detailed implementation sequencing and weighted task quotas:
[Internal Web IR implementation blueprint](../architecture/internal-web-ir-implementation-blueprint.md).
Ordered file-by-file execution map:
[WebIR operations catalog](../architecture/internal-web-ir-implementation-blueprint.md#operations-catalog-op-0001op-0320).
Canonical current-vs-target representation mapping:
[Internal Web IR side-by-side schema](../architecture/internal-web-ir-side-by-side-schema.md).
Quantified K-complexity delta for the canonical worked app:
[WebIR K-complexity quantification](../architecture/internal-web-ir-side-by-side-schema.md#k-complexity-quantification).
Reproducible per-token-class computation:
[WebIR K-metric appendix](../architecture/internal-web-ir-side-by-side-schema.md#k-metric-appendix-reproducible).

---

## Supporting Crates

| Crate | Purpose |
|-------|---------|
| `vox-cli` | `vox` command-line entry point — see [`ref-cli.md`](../reference/cli.md) for the implemented subcommand set |
| `vox-lsp` | Language Server Protocol implementation |
| `vox-runtime` | Tokio/Axum runtime: actors, scheduler, subscriptions, storage |
| `vox-pm` | Package manager: CAS store, dependency resolution, caching |
| `vox-db` | Database abstraction layer |
| `vox-gamify` | Gamification system |
| `vox-orchestrator` | Multi-agent orchestration |
| `vox-toestub` | AI anti-pattern detector |
| `vox-tensor` | Native ML tensors via Burn 0.19 (Wgpu/NdArray backends) |
| `vox-eval` | Automated evaluation of training data quality |
| `vox-doc-pipeline` | Rust-native doc extraction + SUMMARY.md generation |
| `vox-integration-tests` | End-to-end pipeline tests |

---

## Adding a Language Feature

The full checklist for adding a new language construct:

1. **Lexer** — Add tokens to `crates/vox-compiler/src/lexer/token.rs`
2. **Parser** — Add grammar rules in `crates/vox-compiler/src/parser/descent/`
3. **AST** — Add node types in `crates/vox-compiler/src/ast/`
4. **HIR** — Map AST → HIR in `crates/vox-compiler/src/hir/lower/`
5. **Type Check** — Add inference rules in `crates/vox-compiler/src/typeck/`
6. **WebIR** — Add/update lowering + validation semantics in `crates/vox-compiler/src/web_ir/` when the feature affects web-facing behavior
7. **Codegen** — Emit code in both `crates/vox-compiler/src/codegen_rust/` and `crates/vox-compiler/src/codegen_ts/`
8. **Test** — Add integration coverage in `vox-integration-tests/tests/` and WebIR/parity coverage where applicable
9. **Docs** — Add frontmatter + code example in `docs/src/`
10. **Training** — Run `vox mens corpus extract` to include the new construct in ML data

---

## Next Steps

- [Language Guide](../reference/ref-language.md) — Full syntax and feature reference
- [Actors & Workflows](expl-actors-workflows.md) — Durable execution system
- [Ecosystem & Tooling](../how-to/how-to-cli-ecosystem.md) — CLI commands, package manager, LSP
- [Web IR operations catalog](../architecture/internal-web-ir-implementation-blueprint.md#operations-catalog-op-0001op-0320) — numbered compiler/emitter tasks **OP-0001–OP-0320** + supplemental **OP-S049–OP-S220** batch map
- [Web IR acceptance gates G1–G6](../architecture/internal-web-ir-implementation-blueprint.md#acceptance-gates-specific-filetest-thresholds) — parser, K-metric, parity, and rollout thresholds
