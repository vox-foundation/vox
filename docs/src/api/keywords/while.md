---
title: "while"
description: "Official documentation for while for the Vox language. Detailed technical reference, architecture guides, and implementation patterns for"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# while

## Syntax

```vox
# Skip-Test
while cond:
    body
```

## Description

Loops while a condition is true.

## Code Generation
### Rust
```rust
while cond { body }
```
### TypeScript
```typescript
while (cond) { body }
```

[← Back to API Index](../decorators/index.md)