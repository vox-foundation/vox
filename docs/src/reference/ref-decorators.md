---
title: "Reference: Decorator Registry"
description: "Official documentation for Reference: Decorator Registry for the Vox language. Detailed technical reference, architecture guides, and imp"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---
# Reference: Decorator Registry

Vox uses decorators to provide metadata to the compiler and runtime. This registry lists all available decorators and their technical effects.

## Backend & Logic

### `@server fn`
- **Goal**: Creates a backend API endpoint.
- **Effect**: Generates a Rust Axum handler and a TypeScript client.
- **Usage**: `@server fn my_fn(args) to Result[T]`

### `@query fn`
- **Goal**: Read-only database operation.
- **Effect**: Optimized for concurrent reads; cannot perform mutations.
- **Usage**: `@query fn get_data() to list[Item]`

### `@mutation fn`
- **Goal**: Write database operation.
- **Effect**: Wraps execution in a database transaction.
- **Usage**: `@mutation fn save_data() to bool`

### `@actor`
- **Goal**: Defines a stateful concurrency unit.
- **Effect**: Manages private state and a serial mailbox.
- **Usage**: `@actor type MyActor: ...`

### `@workflow`
- **Goal**: Durable, long-running process.
- **Effect**: Automatic journaling and checkpointing of state.
- **Usage**: `@workflow fn my_process()`

### `@activity`
- **Goal**: Retryable step within a workflow.
- **Effect**: Exactly-once execution guarantee with retry policy.
- **Usage**: `@activity fn reliable_step()`

## Data Modeling

### `@table`
- **Goal**: Defines a persistent database table.
- **Effect**: Generates Rust migrations and typed query interfaces.
- **Usage**: `@table type MyRecord: ...`

### `@require`
- **Goal**: Adds runtime validation guards.
- **Effect**: Injects validation checks before assignment/constructor.
- **Usage**: `@require(len(self.pwd) > 8) type User: ...`

## UI & Frontend

### `@component`
- **Goal**: Defines a reactive UI component.
- **Effect**: Compiles to a React component with scoped styles.
- **Usage**: `@component fn MyUI() to Element`

### `@v0`
- **Goal**: AI-generated React component via v0.dev (`V0_API_KEY`).
- **Effect**: `vox build` fetches or stubs `.tsx`; output is normalized to a **named** `export function Name` so **`routes:`** / **TanStack Router** can `import { Name } from "./Name.tsx"` (same rule for **`islands/`** v0 output).
- **Usage**: `@v0 "prompt" fn Dashboard() to Element` or `@v0 from "design.png" fn Dashboard() to Element`

### `@island`
- **Goal**: Declare a **React island** implemented under repo-root **`islands/`** (TSX), separate from the main Vite app.
- **Effect**: Parser emits `Decl::Island`; `vox-codegen-ts` writes `vox-islands-meta.ts` with declared names. `vox run` / bundle builds **`island-mount.js`**, which hydrates DOM nodes with **`data-vox-island="Name"`** (optional props via **`data-prop-*`** attributes).
- **Usage**:
  ```vox
  @island Counter:
    initial?: int
  ```

## Testing & Tooling

### `@test`
- **Goal**: Marks a function as a test case for `vox test`.
- **Effect**: Included in the project test suite.

### `@mock`
- **Goal**: Intercepts function calls for testing.
- **Effect**: Replaces implementation with a mock during test execution.

### `@fixture`
- **Goal**: Provides reusable setup logic for tests.
- **Effect**: Automatically injected into test functions.

### `@agent`
- **Goal**: Defines an AI agent role.
- **Effect**: Configures system prompts and tool access.

### `@mcp.tool`
- **Goal**: Exports a function as an MCP tool.
- **Effect**: Registered with the MCP server for discovery by AI agents.

## Python Interop

### `@py.import` {#pyimport}
- **Goal**: Import a Python library for native use in Vox code without writing any Python.
- **Effect**: Generates a `VoxPyRuntime` lazy singleton in the compiled Rust output; imports are resolved at runtime via `pyo3`.
- **Syntax**:
  ```vox
  @py.import torch               # alias defaults to "torch"
  @py.import torch.nn as nn      # explicit alias
  @py.import numpy as np
  ```
- **Usage**: After importing, call methods via the alias as if it were a Vox module. The Vox compiler routes calls to the Python runtime.
- **See also**: [PyTorch & Python Libraries how-to guide](../how-to/how-to-pytorch.md)
