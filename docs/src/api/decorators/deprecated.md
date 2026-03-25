---
title: "@deprecated"
description: "Official documentation for @deprecated for the Vox language. Detailed technical reference, architecture guides, and implementation patter"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# @deprecated

**Category:** function

## Syntax

```vox
# Skip-Test
@deprecated
```

## Description

Mark a function as deprecated. Emits a warning at every call site.

## Code Generation
### Rust
```rust
#[deprecated]
```
### TypeScript
```typescript
/** @deprecated */
```

[← Back to Decorator Index](../decorators.md)