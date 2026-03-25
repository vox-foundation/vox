---
title: "match"
description: "Official documentation for match for the Vox language. Detailed technical reference, architecture guides, and implementation patterns for"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# match

## Syntax

```vox
# Skip-Test
match expr:
    | pattern -> body
```

## Description

Exhaustive pattern matching on values, typically ADTs.

## Code Generation
### Rust
```rust
match expr { pattern => body }
```
### TypeScript
```typescript
switch (expr.type) { case ... }
```

[← Back to API Index](../decorators/index.md)