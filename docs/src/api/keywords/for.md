---
title: "for"
description: "Official documentation for for for the Vox language. Detailed technical reference, architecture guides, and implementation patterns for A"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# for

## Syntax

```vox
# Skip-Test
for item in iterable:
    body
```

## Description

Iteration over lists and strings.

## Code Generation
### Rust
```rust
for item in iterable { body }
```
### TypeScript
```typescript
for (const item of iterable) { body }
```

[← Back to API Index](../decorators/index.md)