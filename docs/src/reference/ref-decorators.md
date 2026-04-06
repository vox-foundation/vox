---
title: "Reference: Decorator Registry"
description: "All available decorators and their technical effects."
category: "reference"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---
# Reference: Decorator Registry

Vox uses decorators to provide metadata to the compiler and runtime. This registry lists all available decorators and their technical effects. Note that `actor`, `workflow`, and `activity` are core keywords, not decorators.

## Backend & Logic

### `@server`
- **Goal**: Creates a backend API endpoint.
- **Effect**: Generates a Rust Axum handler and a TypeScript client.
- **Usage**: `@server fn my_fn(args: ...)`

### `@query`
- **Goal**: Read-only database operation.
- **Effect**: Optimized for concurrent reads; cannot perform mutations.
- **Usage**: `@query fn get_data() -> List[Item] { ... }`

### `@mutation`
- **Goal**: Write database operation.
- **Effect**: Wraps execution in a database transaction.
- **Usage**: `@mutation fn save_data() -> bool { ... }`

### `@scheduled`
> [!NOTE]
> Planned — not yet parseable.
- **Goal**: Run a background task periodically.
- **Effect**: Compiles to a Tokio timer loop or cron job scheduling block.
- **Usage**:
```vox
// vox:skip
@scheduled("0 * * * *")
fn hourly_task() { 
    // Logic here
}
```

### `@pure`
> [!NOTE]
> Planned — not yet parseable.
- **Goal**: Designates a function as side-effect free.
- **Effect**: Allows the compiler to aggressively optimize and caching the output.
- **Usage**: `@pure fn compute_hash(data: str) -> str { ... }`

### `@deprecated`
> [!NOTE]
> Planned — not yet parseable.
- **Goal**: Marks a function or type as pending removal.
- **Effect**: Emits compiler warnings when used.
- **Usage**: `@deprecated("Use new_function instead")`

## Data Modeling

### `@table`
- **Goal**: Defines a persistent database table.
- **Effect**: Generates Rust migrations and typed query interfaces.
- **Usage**:
```vox
// vox:skip
@table type MyRecord {
    id: str
}
```

### `@index`
- **Goal**: Creates a database index.
- **Effect**: Generates SQL for fast lookup on specified properties.
- **Usage**: `@index MyRecord.by_id on (id)`

### `@require`
- **Goal**: Adds runtime validation guards.
- **Effect**: Injects validation checks before assignment/constructor.
- **Usage**:
```vox
// vox:skip
@require(len(self.pwd) > 8)
type User {
    pwd: str
}
```

## UI & Frontend

### `@island`
- **Goal**: Declare a **React island** implemented under repo-root **`islands/`** (TSX), separate from the main Vite app.
- **Effect**: Parser emits `HirIsland`. Writes `vox-islands-meta.ts`. Mounts onto the client.
- **Usage**:
  ```vox
  // vox:skip
  @island Counter { initial: Option[int] }
  ```

### `@loading`
- **Goal**: Suspense / transition UI for TanStack Router while a lazy route or data boundary resolves.
- **Effect**: Emits `{Name}.tsx`. When `routes { }` produces the router shim, this becomes the `pendingComponent`.
- **Usage**:
```vox
// vox:skip
@loading
fn Spinner() -> Element { 
    <div class="spinner">"…"</div>
}
```

### `@v0`
- **Goal**: Retrieve an AI-generated React component natively via Vercel's unofficial CLI.
- **Effect**: Downloads `.tsx` implementation and wraps it as an island.
- **Usage**: `@v0 "chat-id" fn Dashboard() -> Element { }`

## Testing & Tooling

### `@test`
- **Goal**: Marks a function as a test case for `vox test`.
- **Effect**: Included in the project test suite.
- **Usage**: `@test fn check_auth() { ... }`

### `@mock`
> [!NOTE] 
> **Planned.** Not yet supported by the parser. Use standard functions for test setup or `spawn` dependencies.

### `@fixture`
> [!NOTE] 
> **Planned.** Not yet supported by the parser. Use helper functions called within `@test` blocks instead.

### `agent` (Keyword)
Agents are defined using the `agent` keyword (not a decorator).
```vox
// vox:skip
agent Assistant { 
    instructions: "Help the user"
    tools: [search_kb]
}
```
### `@mcp.tool`
- **Goal**: Exports a function as an MCP tool.
- **Effect**: Registered with the MCP server for discovery by AI agents.

```vox
{{#include ../../../examples/golden/ref_orchestrator.vox:mcp_tool}}
```

### `@mcp.resource`
- **Goal**: Exposes dynamic readable content to MCP.
- **Effect**: Registers a resource URI endpoint via `getResources`.

```vox
{{#include ../../../examples/golden/ref_orchestrator.vox:mcp_resource}}
```
