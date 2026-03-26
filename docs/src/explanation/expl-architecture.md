---
title: "Compiler Architecture"
description: "Official documentation for Compiler Architecture for the Vox language. Detailed technical reference, architecture guides, and implementat"
category: "explanation"
last_updated: 2026-03-26
training_eligible: true
---

# Compiler Architecture

The Vox compiler follows a modern, modular pipeline architecture. Each stage is implemented as an independent Rust crate within the `crates/` workspace.

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
│  vox-parser  │  Recursive descent → lossless GreenTree (Rowan)
└──────┬───────┘
       │ GreenTree / CST
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
┌──────────────────┬─────────────────────┐
│ vox-codegen-rust │  vox-codegen-ts     │
│  (quote! → .rs)  │  (string → .ts/tsx) │
└──────────────────┴─────────────────────┘
```

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

### 1. Lexer (`vox-lexer`)

**Purpose**: Converts source text into a flat stream of tokens.

**Implementation**: Uses the [`logos`](https://docs.rs/logos) crate for high-performance, zero-copy tokenization.

**Output**: `Vec<Token>` — each token carries its kind and span. See [vox-lexer API](../api/vox-lexer.md) for details.

---

### 2. Parser (`vox-parser`)

**Purpose**: Transforms a token stream into a lossless Concrete Syntax Tree (CST).

**Implementation**: A hand-written recursive descent parser producing a [Rowan](https://docs.rs/rowan)-based GreenTree. The parser is **resilient to errors**, meaning it continues parsing after encountering invalid syntax — this is critical for LSP support, where the user is actively typing.

**Key features**:
- Error recovery with synchronization points
- Trailing comma support in parameter lists
- Duplicate parameter name detection
- Indentation-aware formatting (`indent.rs`)

See [vox-parser API](../api/vox-parser.md) for implementation details.

**Output**: `GreenTree` — a lossless syntax tree preserving all whitespace and comments.

---

### 3. AST (`vox-ast`)

**Purpose**: Strongly-typed wrappers around the untyped CST nodes.

See [vox-ast API](../api/vox-ast.md) for the full node hierarchy.

---

### 6. Code Generation

#### Rust Codegen (`vox-codegen-rust`)

Emits Rust source using the [`quote!`](https://docs.rs/quote) macro. Each decorator maps to specific Rust constructs:

| Vox | Generated Rust |
|-----|---------------|
| `@server fn` | Axum handler + route registration |
| `@table type` | Struct + SQLite schema |
| `@test fn` | `#[test]` function |
| `@deprecated` | `#[deprecated]` attribute |
| `actor` | Tokio task + mpsc mailbox |
| `workflow` | State machine with durable step recording |

#### TypeScript Codegen (`vox-compiler` / `codegen_ts`)

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
Canonical current-vs-target representation mapping:
[Internal Web IR side-by-side schema](../architecture/internal-web-ir-side-by-side-schema.md).

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
6. **Codegen** — Emit code in both `crates/vox-compiler/src/codegen_rust/` and `crates/vox-compiler/src/codegen_ts/`
7. **Test** — Add an integration test in `vox-integration-tests/tests/`
8. **Docs** — Add frontmatter + code example in `docs/src/`
9. **Training** — Run `vox mens corpus extract` to include the new construct in ML data

---

## Next Steps

- [Language Guide](../reference/ref-language.md) — Full syntax and feature reference
- [Actors & Workflows](expl-actors-workflows.md) — Durable execution system
- [Ecosystem & Tooling](../how-to/how-to-cli-ecosystem.md) — CLI commands, package manager, LSP
