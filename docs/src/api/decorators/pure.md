---
title: "@pure"
description: "Official documentation for @pure for the Vox language. Detailed technical reference, architecture guides, and implementation patterns for"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# @pure

**Category:** function

## Syntax

```vox
# Skip-Test
@pure
```

## Description

Enforce function purity — no side effects allowed in the function body.

## Code Generation
### Rust
```rust
/* @pure */ comment
```
### TypeScript
```typescript
/** @__PURE__ */
```

[← Back to Decorator Index](../decorators.md)