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

Variable assignments are immutable by default in Vox. Prefix with `mutable` for mutability.

```vox
{{#include ../../../examples/golden/ref_syntax.vox:variables}}
```

Functions mapping natively to networking, storage, or internal agentic constraints.

```vox
{{#include ../../../examples/golden/ref_syntax.vox:functions}}
```

```vox
{{#include ../../../examples/golden/ref_orchestrator.vox:mcp_tool}}
```

Lexical constraints and properties can be modeled strictly using Abstract Data Types (ADTs) and Table definitions.

```vox
{{#include ../../../examples/golden/ref_types.vox:adt}}
```

```vox
// vox:skip
@table type Task {
    title: str
    done: bool
    owner: str
}
```

### Branching
```vox
{{#include ../../../examples/golden/ref_syntax.vox:control_flow}}
```

### Pattern Matching (`match`)
```vox
{{#include ../../../examples/golden/ref_types.vox:matching}}
```

### Error Propagation (`?`)
The `?` suffix unpacks an `Ok` result, returning early if the result is an `Error(e)`.

```vox
// vox:skip
fn build_report() -> Result[str] {
    let raw_data = get_data()?
    return Ok("Report { " + raw_data)
}
```

Actors operate isolated asynchronous loops responding to discrete event handler payloads via `on`. 

```vox
{{#include ../../../examples/golden/ref_actors.vox:basic_actor}}
```

```vox
{{#include ../../../examples/golden/ref_actors.vox:spawn_and_send}}
```

## Agents

Agents define LLM-backed roles with systematic instructions and toolsets.

```vox
{{#include ../../../examples/golden/ref_agents.vox:basic_agent}}
```

Use `workflow` to group state machine processes that survive process restarts. Use `activity` to dictate atomic, retry-able execution sequences.

```vox
{{#include ../../../examples/golden/getting_started.vox:logic}}
```

## Island and UI Syntax

The `@island` directive dictates interactive DOM components. 

```tsx
// vox:skip
import react.use_state

@island
fn TaskList(tasks: list[Task]) -> Element {
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
    "/"         -> TaskList
    "/about"    -> AboutPage
}
```

Vox imports use fully qualified paths. Use `import rust:<crate>` for native interop.

```vox
// vox:skip
import react.use_state
import rust:serde_json as json
```
