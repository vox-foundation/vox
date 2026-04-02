---
title: "@mcp.resource"
description: "Expose a nullary Vox function as an MCP read-only resource."
category: "reference"
last_updated: 2026-03-29
training_eligible: true
---
# @mcp.resource

**Category:** infrastructure

**Architecture:** [`MCP exposure from the Vox language (SSOT)`](../../architecture/mcp-vox-language-exposure.md).

## Syntax

```vox
@mcp.resource("uri", "description")
fn name() to ReturnType {
    ...
}
```

Alternate surface syntax (two string literals):

```vox
@mcp.resource "uri" "description"
fn name() to ReturnType {
    ...
}
```

## Constraints

- The function must take **no parameters**. MCP `resources/read` only supplies a `uri`; routing is by exact URI match on the decorator’s first string.
- Each **URI** must be **unique** within the module.

## Code generation (Rust)

Emits `resources/list` and `resources/read` handling alongside `@mcp.tool` in generated `src/mcp_server.rs`, and lists the resource in **`app_contract.json`** (`mcp_resources`).

## TypeScript

N/A (server/MCP surface only).

[← Back to Decorator Index](../decorators.md)
