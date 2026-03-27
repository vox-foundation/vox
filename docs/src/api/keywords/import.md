---
title: "import"
description: "Official documentation for import for the Vox language. Detailed technical reference, architecture guides, and implementation patterns fo"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# import

## Syntax

```vox
# Skip-Test
import path.to.module
import rust:serde_json
import rust:serde_json(version: "1") as json
```

## Description

Imports symbols from Vox/builtin modules and declares Rust crate dependencies for script/app codegen.

- `import a.b.c` keeps existing symbol-path behavior.
- `import rust:<crate>` declares a Rust dependency lane.
- Optional Rust metadata keys: `version`, `path`, `git`, `rev`.
- Optional alias: `as <name>`.

## Rust crate imports (full SSOT)

See **[How-To: Rust crate imports in Vox scripts](../../how-to/how-to-rust-crate-imports.md)** for syntax, pipeline, limitations, and evolution notes.

## Code Generation
### Rust
```rust
use path::to::module;
// rust imports are emitted as Cargo.toml dependencies
```
### TypeScript
```typescript
import { ... } from 'path/to/module';
```

[← Back to API Index](../decorators/index.md)