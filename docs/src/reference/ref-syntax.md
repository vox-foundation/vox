---
title: "Language Syntax Reference"
description: "A comprehensive, scannable syntax quick-reference page."
category: "reference"
status: "current"
last_updated: "2026-04-06"
training_eligible: true

schema_type: "TechArticle"
keywords: ["Vox syntax reference", "Vox language keywords", "Vox grammar specification", "full-stack syntax guide"]
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

Variable assignments are immutable by default in Vox. Prefix with `mut` for mutability.

```vox
// ANCHOR: variables
fn demo_vars() {
    let x = 10
    let mut y = 20
    y = 30
}
// ANCHOR_END: variables
```

Functions mapping natively to networking, storage, or internal agentic constraints.

```vox
// ANCHOR: functions
fn add(a: int, b: int) -> int {
    return a + b;
}

component Button(label: str) {
    view: <button>{label}</button>
}
// ANCHOR_END: functions
```

```vox
// From examples/golden/ref_orchestrator.vox
@mcp.tool "search: Search the knowledge base"
fn search(query: str) -> List[str] {
    return ["result 1", "result 2"]
}
```

Lexical constraints and properties can be modeled strictly using Abstract Data Types (ADTs) and Table definitions.

```vox
type Shape =
    | Circle(radius: float)
    | Rect(w: float, h: float)
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
fn check(n: int) -> str {
    if n > 0 {
        return "positive"
    } else {
        return "other"
    }
}
```

### Pattern Matching (`match`)
```vox
fn area(s: Shape) -> float {
    match s {
        Circle(r) -> 3.14 * r * r
        Rect(w, h) -> w * h
    }
}
```

### Pipe Operator (`|>`)
The `|>` operator passes the expression on the left as the first argument to the function on the right. Works with any function.
```vox
// vox:skip
let value = " 123 " |> trim |> parse_int |> double
// Compiles to: double(parse_int(trim(" 123 ")))
```

### Loops
```vox
// vox:skip
loop {
    if should_exit() { break }
    continue
}
```

### Comments
Comments use `//`. Block comments and `#` comments are not supported.
```vox
// vox:skip
// This is a comment
let x = 1
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
actor Counter {
    count: int = 0
    on increment(n: int) {
        count += n
    }
    on get() -> int {
        return count
    }
}
```

```vox
let c = spawn Counter()
c ! increment(5)
let val = c.get()
```

## Agents

Agents define LLM-backed roles with systematic instructions and toolsets.

```vox
```vox
@llm(model="claude-3-opus")
fn summarize(text: str) -> str
```
```

Use `workflow` to group state machine processes that survive process restarts. Use `activity` to dictate atomic, retry-able execution sequences.

```vox
```vox
@query fn get_notes() -> List[Note] {
    return db.Note.all()
}

@mutation fn create_note(title: str, content: str) -> Result[Id[Note]] {
    let id = db.Note.insert({ title: title, content: content })?
    return Ok(id)
}
```
```

## Island and UI Syntax

The `@island` directive dictates interactive DOM components. 

```tsx
// vox:skip
@island TaskList { tasks: list[Task] }

// Web Routing Layout Mapping
routes {
    "/"         -> TaskList
    "/about"    -> AboutPage
}
```

### Return Keyword
`return` is the canonical way to return a value from a function.

> [!WARNING]
> The `ret` alias is **deprecated** and will be removed in a future version. Use `return` for all new code.

```vox
// vox:skip
fn double(x: int) -> int { return x * 2 }
fn square(x: int) -> int { return x * x }
```

Vox imports use fully qualified paths. Use `import rust:<crate>` for native interop.

```vox
// vox:skip
import react.use_state
import rust:serde_json as json
```
