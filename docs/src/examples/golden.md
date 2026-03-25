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
fn hello(name: str) to str {
    ret "Hello " + name + "!"
}
```

---

## CRUD API — Table, Query, Mutation, and Endpoint

A complete data layer in one file. `@table` generates the database schema, `@query` wires a read-only resolver, `@mutation` wires a write operation, and `@get` exposes an HTTP handler — all with the Rust Axum backend generated automatically.

```vox
@table type User {
    id: int
    name: str
    active: bool
}

@query
fn get_user(id: int) to str {
    ret db.User.find(id).name
}

@mutation
fn create_user(name: str) to Unit {
    db.User.insert(name)
}

@get("/api/users")
fn user_handler(req: str) to str {
    ret get_user(1)
}
```

---

## Counter Actor — Stateful Concurrent Actor

Actors are isolated units of concurrency. This actor holds an integer counter in its state and exposes an `Increment` message handler that returns the new count. Spawning the actor allocates a mailbox and an address.

```vox
actor CounterActor {
    state count: int = 0

    on Increment() to int {
        count = count + 1
        ret count
    }
}
```

---

## Checkout Workflow — Durable Execution with Error Handling

Workflows survive server restarts by journaling each activity result. The `charge_card` activity is idempotent and retryable. Pattern matching on `Result` makes both happy-path and error-path explicit.

```vox
activity charge_card(amount: int) to Result[str] {
    if amount > 1000 {
        ret Error("Amount too large")
    }
    ret Ok("tx_123")
}

workflow checkout(amount: int) to str {
    let result = charge_card(amount)
    match result {
        Ok(tx) -> "Success: " + tx,
        Error(msg) -> "Failed: " + msg
    }
}
```

---

## MCP Tools — AI-Callable Tool and Resource

The `@mcp.tool` decorator generates a Model Context Protocol tool schema from the function signature. AI agents (including Vox's built-in DEI orchestrator) can discover and call these functions without any glue code.

```vox
@mcp.tool "read_file: Reads a file from disk"
fn read_file(path: str) to str {
    ret "file contents"
}

@mcp.resource "file://{path}"
fn file_resource(path: str) to str {
    ret path
}
```

---

## Agent Pipeline — Multi-Agent Message Passing

Demonstrates an actor-based multi-agent system. `TaskMessage` is a structured message type. `WorkerAgent` receives `HandleTask` messages and tracks the number of processed tasks in its actor state.

```vox
message TaskMessage {
    id: int
    payload: str
}

agent WorkerAgent {
    state processed: int = 0

    on HandleTask(msg: TaskMessage) to str {
        processed = processed + 1
        ret "Task " + msg.id + " done"
    }
}
```

---

## Dashboard UI — Layout, Islands, and Routes

Full-stack UI composition. `@island` marks interactive components that get client-side hydration. `layout` wraps every route with shared chrome. `routes` maps URL paths to components.

```vox
type DashboardStatus = Loading | Ready(data: str)

@island
fn DataChart(data: list[int]) to Element {
    ret <div className="chart">Interactive Chart</div>
}

component fn DashboardView() to Element {
    ret <div className="dashboard">
        <h1>Dashboard</h1>
        <DataChart data=[1, 2, 3] />
    </div>
}

layout fn MainLayout(children: Element) to Element {
    ret <main>
        <nav>Menu</nav>
        {children}
    </main>
}

routes {
    "/" -> DashboardView
}
```

---

## Type System — ADTs, Generics, and Traits

Demonstrates algebraic data types with a type parameter, trait definition, and `impl` block. `AppResult[T]` is a generic union type (Vox's alternative to exceptions). The `Serializable` trait requires a `serialize` method.

```vox
type AppResult[T] = Success(value: T) | Failure(err: str)

trait Serializable {
    fn serialize(self) to str
}

impl Serializable for AppResult[int] {
    fn serialize(self) to str {
        match self {
            Success(val) -> "num:" + val,
            Failure(err) -> "err:" + err
        }
    }
}
```

---

## Test Suite — Fixtures, Mocks, and Assertions

`@fixture` sets up shared test data. `@mock` replaces external dependencies. `@test` declares a test function. The `|>` pipe operator and `len` built-in demonstrate Vox's functional style.

```vox
@fixture
fn setup_user() to list[str] {
    ret ["alice", "bob"]
}

@mock
fn mock_db_read() to str {
    ret "mock_data"
}

@test
fn test_user_count() to Unit {
    let users = setup_user()
    assert(users |> len > 0)
    let db_val = mock_db_read()
    assert(db_val == "mock_data")
}
```

---

## Config and Deploy — Environment Configuration

Typed configuration blocks and named environment definitions. `config` generates validated config structs. `environment` names deployment targets with typed key-value pairs.

```vox
config DatabaseConfig {
    url: str
    pool_size: int
}

environment prod_env {
    region: "us-west-2"
    replicas: 3
    debug: false
}
```
