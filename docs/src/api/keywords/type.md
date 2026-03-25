---
title: "type"
description: "Official documentation for type for the Vox language. Detailed technical reference, architecture guides, and implementation patterns for "
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# type

## Syntax

```vox
type Name = | Variant
```

## Description

Declares a new type, ADT, or record.

## Code Generation
### Rust
```rust
pub enum / pub struct
```
### TypeScript
```typescript
type / interface
```

[← Back to API Index](../decorators/index.md)