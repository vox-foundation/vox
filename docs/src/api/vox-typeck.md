---
title: "Crate API: vox-typeck"
description: "Official documentation for Crate API: vox-typeck for the Vox language. Detailed technical reference, architecture guides, and implementat"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Crate API: vox-typeck

## Overview

**Constraint-based type inference and checking for the Vox language.**

## Overview

The vox-typeck crate implements a bidirectional type checking algorithm with Hindley-Milner style
type inference using union-find (UF) based constraint unification.

## Architecture

```
AST Module
    ↓
typecheck_module()
    ↓
╔═══════════════════════════════╗
║  TypeEnv                      ║  ← symbol table with scoped bindings
║  UnionFind                    ║  ← constraint solver (unification)
║  check_expr / check_stmt      ║  ← bidirectional type checking
╚═══════════════════════════════╝
    ↓
Vec<Diagnostic>  (errors + warnings)
```

### Key Components

| File | Purpose |
|------|---------|
| `check.rs` | Main type-checking logic: `check_expr`, `check_stmt`, `check_decl` |
| `env.rs` | `TypeEnv` — scoped symbol table with push/pop scope support |
| `ty.rs` | `Ty` enum — internal type representation used during checking |
| `unify.rs` | Union-Find constraint solver for type unification |
| `diagnostics.rs` | `Diagnostic` and `Severity` types for error reporting |

### Type Inference Algorithm

1. **Fresh type variables** — Unknown types are assigned fresh `Ty::TypeVar(id)` values
2. **Constraint generation** — Binary expressions, calls, and assignments generate equality constraints
3. **Unification** — The UF solver merges equivalent type variables, detecting conflicts
4. **Substitution** — After solving, type variables are replaced with their resolved types
5. **Error reporting** — Unresolvable conflicts produce `Diagnostic` errors

### Scoping

The `TypeEnv` maintains a scope stack:
- `push_scope()` — Enter a new lexical scope (function body, lambda, block)
- `pop_scope()` — Exit scope, discarding local bindings
- `define(name, ty)` — Bind a name to a type in the current scope
- `lookup(name)` — Resolve a name by searching outward through scopes

### Example Flow

```
let x = 42       →  TypeEnv.define("x", Ty::Int)
let y = x + 1    →  check x: Ty::Int, check 1: Ty::Int, unify(+): Int×Int→Int
fn f(a):          →  push_scope, define "a" as fresh TypeVar
    ret a + 1     →  unify TypeVar(a) with Int → a: Int
```

## Usage

```rust
use vox_typeck::{typecheck_module, diagnostics::Severity};

let diagnostics = typecheck_module(&ast_module, &source_content);
let errors: Vec<_> = diagnostics.iter()
    .filter(|d| d.severity == Severity::Error)
    .collect();
```

---

### `struct FixSuggestion`

A suggested fix for a diagnostic.


### `struct StubAutoFixer`

Default AutoFixer implementation: one fix per diagnostic, using suggestion/context when present.
Used by `vox check --force` to apply the first applicable fix.


### `struct BuiltinTypes`

Pre-registered type signatures for the Vox standard library.

This populates the root scope of a `TypeEnv` with:
- Built-in types (Option, Result as ADTs with proper constructors)
- Standard library functions (print, str, int, float, len)
- React/frontend bindings (use_state, use_effect)
- HTTP/network module bindings
- String, list, and record methods


### `fn typecheck_module`

Type-check a complete Vox module, returning diagnostics.

This performs a two-pass analysis:
1. **Registration pass**: Register all top-level declarations (types, functions,
actors, workflows) into the type environment so forward references work.
2. **Checking pass**: Type-check each function/handler body using the populated
environment, checking return types, mutability, and match exhaustiveness.


### `enum Severity`

Type checking diagnostic severity.


### `struct Diagnostic`

A structured diagnostic emitted by the type checker.


### `struct Binding`

A named binding in the environment.


### `enum BindingKind`

What kind of name this binding refers to.


### `struct AdtDef`

Registered ADT (Algebraic Data Type) with its variants.


### `struct VariantDef`

A single variant of an ADT.


### `struct TypeEnv`

Type environment for semantic analysis.

Tracks scoped variable bindings, registered types (ADTs), and
actor/workflow declarations. Supports lexical scoping with
push/pop for nested blocks.


### `struct ActorHandlerSig`

Signature of an actor handler.


### `struct WorkflowSig`

Signature of a workflow.


### `fn infer_expr`

Infer the type of an expression.


### `fn infer_stmt`

Infer the type produced by a statement.


## Module: `vox-typeck\src\lib.rs`

# vox-typeck

Bidirectional type checker with Hindley-Milner style inference for the
Vox language. Uses union-find based constraint unification to resolve
type variables and report diagnostics.


### `enum Ty`

Internal type representation for the type checker.


### `struct InferenceContext`

Inference context with union-find based type variable substitution.


