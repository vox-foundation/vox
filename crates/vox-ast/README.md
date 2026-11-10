# vox-ast

Pure-data Abstract Syntax Tree (AST) definitions for the [Vox](https://github.com/vox-foundation/vox) programming language.

## Overview

`vox-ast` provides the zero-runtime-dependency, serde-serializable AST data structures representing Vox programs:

- **Declarations**: functions, types, state machines, webhooks, reactive forms, and configurations.
- **Expressions & Statements**: typed expressions, control flow, block statements, and variable bindings.
- **Patterns**: pattern matching constructs and destructuring.
- **Spans & Types**: source spans, type syntax, and scalar mappings.

This is a Layer 0 leaf crate extracted from `vox-compiler` (where it is re-exported as `vox_compiler::ast`).

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
vox-ast = "0.6"
```

## License

Apache-2.0
