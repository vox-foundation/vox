---
title: "@require"
description: "Official documentation for @require for the Vox language. Detailed technical reference, architecture guides, and implementation patterns "
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# @require

**Category:** function

## Syntax

```vox
# Skip-Test
@require(expr)
```

## Description

Add a precondition assertion. Panics at runtime if the expression is false.

## Code Generation
### Rust
```rust
assert!(expr, message)
```
### TypeScript
```typescript
if (!expr) throw new Error(message)
```

[← Back to Decorator Index](../decorators.md)