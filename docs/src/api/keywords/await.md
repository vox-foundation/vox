---
title: "await"
description: "Official documentation for await for the Vox language. Detailed technical reference, architecture guides, and implementation patterns for"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# await

## Syntax

```vox
# Skip-Test
await expr
```

## Description

Awaits the result of an asynchronous operation.

## Code Generation
### Rust
```rust
expr.await
```
### TypeScript
```typescript
await expr
```

[← Back to API Index](../decorators/index.md)