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
```

## Description

Imports types and functions from another module.

## Code Generation
### Rust
```rust
use path::to::module;
```
### TypeScript
```typescript
import { ... } from 'path/to/module';
```

[← Back to API Index](../decorators/index.md)