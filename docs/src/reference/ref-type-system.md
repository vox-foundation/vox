---
title: "Reference: Type System"
description: "Deep dive into the Vox type system: ADTs, generics, zero-null discipline, and bidirectional inference."
category: "reference"
last_updated: 2026-04-05
training_eligible: true
---

# Reference: Type System

Vox features a strongly-typed, expressive type system designed for technical unification between Rust (backend) and TypeScript (frontend). It is designed to be **AI-readable**, meaning the type signatures provide enough context for an LLM to generate correct code without hallucinating field names.

## 1. Core Philosophy: Zero-Null Discipline

In Vox, `null` and `undefined` do not exist. Absence must be modeled explicitly using `Option[T]`, and fallible operations must use `Result[T, E]`.

| Feature | Vox Implementation | Benefit |
|---------|-------------------|---------|
| **Absence** | `Option[T]` | Forced handling of empty states; no "null pointer" crashes. |
| **Failure**| `Result[T, E]` | Errors are part of the type signature; cannot be ignored. |
| **Branching** | Pattern Matching | Compiler ensures all cases (variants) are handled. |

---

## 2. Primitive Types

| Type | Description | Rust Equivalent | TS Equivalent |
|------|-------------|-----------------|---------------|
| `str` | UTF-8 String | `String` | `string` |
| `int` | 64-bit Integer | `i64` | `number` / `BigInt` |
| `float`| 64-bit Float | `f64` | `number` |
| `bool` | Boolean | `bool` | `boolean` |
| `Unit` | Empty placeholder | `()` | `void` |

---

## 3. Algebraic Data Types (ADTs)

### Structs (Product Types)
A named collection of fields.

```vox
// vox:skip
@table type Task {
    id:       Id[Task]
    title:    str
    done:     bool
    priority: int
}
```

### Enums (Sum Types / Tagged Unions)
Types that can be one of several variants, potentially carrying extra data.

```vox
{{#include ../../../examples/golden/ref_types.vox:adt}}
```

---

Vox uses the `match` keyword for exhaustive destructuring of ADTs. The compiler will reject a match expression that does not cover every possible variant.

```vox
{{#include ../../../examples/golden/ref_types.vox:matching}}
```

---

### `Option[T]`
Used for values that might be missing.

```vox
// vox:skip
fn find_user(id: int) -> Option[User] {
    return db.User.find(id)
}
```

### `Result[T, E]`
Used for operations that can fail.

```vox
// vox:skip
@server fn update_task(id: Id[Task], title: str) -> Result[Unit, str] {
    if title.len() == 0 {
        return Err("Title cannot be empty")
    }
    db.patch(id, { title: title })
    return Ok(())
}
```

---

Similar to Rust, the `?` operator can be used to early-return on `None` or `Err`.

```vox
// vox:skip
fn get_user_email(id: int) -> Option[str] {
    let user = find_user(id)? // If None, returns None early
    return Some(user.email)
}
```

---

## 7. Bidirectional Type Inference

You rarely need Type annotations for local variables. Vox infers them from the right-hand side or from how the variable is used.

```vox
// vox:skip
let x = 10                  // inferred as int
let names = ["Alice", "Bob"] // inferred as list[str]
let result = add_task("Hi")  // inferred from add_task signature
```

Explicit types are **required** on:
1. Function parameters
2. Function return types
3. `@table` and `type` definitions

---

## 8. Collection Types

### `list[T]`
An ordered sequence of elements.
- **Usage**: `list[int]`
- **Literals**: `[1, 2, 3]`

### `map[K, V]`
A collection of key-value pairs.
- **Usage**: `map[str, int]`
- **Literals**: `{ "key": 10 }`

---

## 9. Next Steps

- **[Language Guide](./ref-syntax.md)** — General syntax overview.
- **[Decorator Registry](./ref-decorators.md)** — How types interact with `@table` and `@server`.
- **[Functions](./ref-syntax.md)** — Detailed function signature reference.
