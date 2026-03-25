---
title: "if"
description: "Official documentation for if for the Vox language. Detailed technical reference, architecture guides, and implementation patterns for AI"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# if

## Syntax

```vox
# Skip-Test
if cond:
    body
else:
    body
```

## Description

Conditional branching.

## Code Generation
### Rust
```rust
if cond { body } else { body }
```
### TypeScript
```typescript
if (cond) { body } else { body }
```

[← Back to API Index](../decorators/index.md)