---
title: "Contributing — parser and HIR"
description: "Onboarding for vox-compiler frontend and HIR lowering"
category: "how-to"
last_updated: 2026-03-25
---

# Contributing — parser and HIR

## Read first

- [Architecture — pipeline](../explanation/expl-architecture.md)
- [Parser ambiguity inventory](../reference/parser-ambiguity-inventory.md)
- [HIR legacy inventory](../reference/hir-legacy-inventory.md)
- [Diagnostic taxonomy](../reference/diagnostic-taxonomy.md)

## Key crates

| Path | Role |
|------|------|
| `crates/vox-compiler/src/lexer` | Tokenization |
| `crates/vox-compiler/src/parser` | Recursive descent → `ast::decl::Module` |
| `crates/vox-compiler/src/hir/lower` | AST → `HirModule` |
| `crates/vox-compiler/src/hir/validate.rs` | Structural invariants |
| `crates/vox-compiler/src/typeck` | HIR typechecking |

## Commands

```bash
cargo test -p vox-compiler
cargo test -p vox-compiler --test parser_recovery
```

## Definition of done

- Parser / HIR changes include tests (unit or `tests/*.rs`).
- New declaration kinds either get a dedicated `Hir*` vector or land in `legacy_ast_nodes` **only** with an inventory update and a graduation plan.
