---
title: "@server"
description: "Official documentation for @server for the Vox language. Detailed technical reference, architecture guides, and implementation patterns f"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# @server

**Category:** infrastructure

## Syntax

```vox
# Skip-Test
@server
```

## Description

Server-only function. Generates both a Rust handler and a TypeScript API client.

## Code Generation
### Rust
```rust
axum POST handler
```
### TypeScript
```typescript
fetch() API client wrapper
```

[← Back to Decorator Index](../decorators.md)