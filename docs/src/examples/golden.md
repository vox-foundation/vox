---
title: "Golden Examples: Working Vox Code"
description: "Complete, validated Vox code examples demonstrating server functions, actors, workflows, MCP tools, and UI components in the Vox programming language."
category: "example"
last_updated: 2026-03-24
training_eligible: true
---

# Golden Examples

Working code examples demonstrating Vox language features. Each `.vox` file is a complete, self-contained program validated by the CI pipeline. See [`examples/PARSE_STATUS.md`](../../../examples/PARSE_STATUS.md) for the latest parse matrix and [`examples/STYLE.md`](../../../examples/STYLE.md) for contribution guidelines.

---

## Hello World

The smallest valid Vox program: a typed function that returns a string. Demonstrates the `fn` keyword, explicit return type, string concatenation, and `ret`.

```vox
{{#include ../../../examples/golden/hello.vox}}
```

---

## CRUD API — Table, Query, Mutation, and Endpoint

A complete data layer in one file. `@table` generates the database schema, `@query` wires a read-only resolver, `@mutation` wires a write operation, and `@get` exposes an HTTP handler — all with the Rust Axum backend generated automatically.

```vox
{{#include ../../../examples/golden/crud_api.vox}}
```

---

## Counter Actor — Stateful Concurrent Actor

Actors are isolated units of concurrency. This actor holds an integer counter in its state and exposes an `Increment` message handler that returns the new count. Spawning the actor allocates a mailbox and an address.

```vox
{{#include ../../../examples/golden/counter_actor.vox}}
```

---

## Checkout Workflow — Durable Execution with Error Handling

Workflows survive server restarts by journaling each activity result. The `charge_card` activity is idempotent and retryable. Pattern matching on `Result` makes both happy-path and error-path explicit.

```vox
{{#include ../../../examples/golden/checkout_workflow.vox}}
```

---

## MCP Tools — AI-Callable Tool and Resource

The `@mcp.tool` decorator generates a Model Context Protocol tool schema from the function signature. AI agents (including Vox's built-in DEI orchestrator) can discover and call these functions without any glue code.

```vox
{{#include ../../../examples/golden/mcp_tools.vox}}
```

---

## Agent Pipeline — Multi-Agent Message Passing

Demonstrates an actor-based multi-agent system. `TaskMessage` is a structured message type. `WorkerAgent` receives `HandleTask` messages and tracks the number of processed tasks in its actor state.

```vox
{{#include ../../../examples/golden/agent_pipeline.vox}}
```

---

## Dashboard UI — Layout, Islands, and Routes

Full-stack UI composition. `@island` marks interactive components that get client-side hydration. `layout` wraps every route with shared chrome. `routes` maps URL paths to components.

```vox
{{#include ../../../examples/golden/dashboard_ui.vox}}
```

---

## Type System — ADTs, Generics, and Traits

Demonstrates algebraic data types with a type parameter, trait definition, and `impl` block. `AppResult[T]` is a generic union type (Vox's alternative to exceptions). The `Serializable` trait requires a `serialize` method.

```vox
{{#include ../../../examples/golden/type_system.vox}}
```

---

## Test Suite — Fixtures, Mocks, and Assertions

`@fixture` sets up shared test data. `@mock` replaces external dependencies. `@test` declares a test function. The `|>` pipe operator and `len` built-in demonstrate Vox's functional style.

```vox
{{#include ../../../examples/golden/test_suite.vox}}
```

---

## Config and Deploy — Environment Configuration

Typed configuration blocks and named environment definitions. `config` generates validated config structs. `environment` names deployment targets with typed key-value pairs.

```vox
{{#include ../../../examples/golden/config_deploy.vox}}
```
