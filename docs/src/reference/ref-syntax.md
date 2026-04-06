---
title: "Language Syntax Reference"
description: "A comprehensive, scannable syntax quick-reference page."
category: "reference"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---

# Reference: Language Syntax

This page provides the canonical structural layout for Vox v0.3 features. All code samples are grounded in the confirmed `examples/golden/` files.

## Primitive Types

| Type | Example | Description |
| :--- | :--- | :--- |
| `str` | `"hello world"` | Text string (UTF-8) |
| `int` | `42` | Signed 64-bit integer |
| `float` | `3.14159` | 64-bit floating point number |
| `bool` | `true`, `false` | Boolean value |
| `Unit` | `()` | Equivalent to `void` |

## Variable Binding

Variable assignments are immutable by default in Vox. Prefix with `mut` for mutability.

```vox
// Immutable binding
let name = "Alice"

// Mutable binding
let mut count = 0
count = count + 1
```

## Function Syntax

Functions map natively to specific networking, runtime, or internal constraints.

```vox
// Plain function execution
fn hello(name: str) to str {
    ret "Hello " + name + "!"
}

// Server endpoint (generates Axum handler & TypeScript client)
@server
fn add_task(title: str, owner: str) to Id[Task] {
    ret db.Task.insert({ title: title, done: false, priority: 1, owner: owner })
}

// Read-only logic guard
@query
fn current_temp() to int {
    ret 72
}

// Database-write logic guard
@mutation
fn switch_toggle() to Result[Unit] {
    ret Ok(())
}

// Inline raw HTTP handling (skips RPC mappings)
http get "/api/health" to str {
    ret "ok"
}

// Model Context Protocol API hook
@mcp.tool "Get the weather"
fn get_weather(location: str) to str {
    ret "Sunny in " + location
}
```

## Type Declarations

Lexical constraints and properties can be modeled strictly using Abstract Data Types (ADTs).

```vox
// Algebraic Data Type
type Status = 
    | Pending
    | Active(assigned_to: str)
    | Completed(at: int)

// Generic ADT
type Result[T] =
    | Ok(value: T)
    | Error(message: str)

// Persistent Data Structural Representation
@require(len(self.title) > 0)
@table type Task {
    title: str
    done: bool
    owner: str
}

// Database Index
@index Task.by_owner on (owner)
```

## Control Flow

### Branching
```vox
let greeting = if hour < 12 {
    "Morning"
} else {
    "Day"
}
```

### Pattern Matching (`match`)
```vox
match status {
    Pending             -> "Waiting"
    Active(person)      -> "Assigned to " + person
    Completed(_)        -> "Done"
}
```

### Loop Constructs
```vox
for item in items {
    print(item)
}

while count < 10 {
    count = count + 1
}
```

### Error Propagation (`?`)
The `?` suffix unpacks an `Ok` result, returning early if the result is an `Error(e)`.
```vox
fn build_report() to Result[str] {
    let raw_data = get_data()?
    ret Ok("Report { " + raw_data)
}
```

## Actors & State

Actors operate isolated asynchronous loops responding to discrete event handler payloads via `on`. Use `state_load` and `state_save` for durability within an actor.

```vox
actor TaskCounter {
    on Increment(amount: int) to int {
        let current = state_load("count")
        let next    = current + amount
        state_save("count", next)
        ret next
    }

    on Get() to int {
        ret state_load("count")
    }
}

// Spawning the actor instance
let counter = spawn TaskCounter()
```

## Workflows and Activities

Use `workflow` to group state machine processes that survive process restarts. Use `activity` to dictate atomic, retry-able execution sequences.

```vox
activity charge_payment(amount: int, token: str) to Result[str] {
    ret Ok("tx-" + token)
}

workflow process_order(customer: str, amount: int) to Result[str] {
    let payment = charge_payment(amount, "tok-abc")
        with { retries: 3, timeout: "30s", initial_backoff: "500ms" }

    match payment {
        Ok(tx)    -> ret Ok("Order for " + customer + " { " + tx)
        Error(e)  -> ret Error(e)
    }
}
```

## Island and UI Syntax

The `@island` directive dictates interactive DOM components. 

```tsx
import react.use_state

@island
fn TaskList(tasks: List[Task]) to Element {
    let (items, set_items) = use_state(tasks)

    <div class="task-list">
        {items.map(fn(task) {
            <label>
                <input type="checkbox" checked={task.done} /> 
                {task.title}
            </label>
        })}
    </div>
}

// Web Routing Layout Mapping
routes {
    "/"         to TaskList
    "/about"    to AboutPage
}
```

## Import Declarations

Vox imports use fully qualified paths. To interoperate natively with compiled Rust workspaces, prefix with `rust:`. 

```vox
// Frontend Framework specific imports
import react.use_state
import react.use_effect

// Native system FFI integrations 
import rust:serde_json as json
import rust:std::collections::HashMap
```
