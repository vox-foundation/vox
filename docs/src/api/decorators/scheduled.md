---
title: "@scheduled"
description: "Official documentation for @scheduled for the Vox language. Detailed technical reference, architecture guides, and implementation pattern"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# @scheduled

**Category:** infrastructure

## Syntax

```vox
# Skip-Test
@scheduled
```

## Description

Cron/interval scheduled function.

## Code Generation
### Rust
```rust
tokio::time::interval loop
```
### TypeScript
```typescript
N/A
```

[← Back to Decorator Index](../decorators.md)