---
title: "How-To: Handle Errors Gracefully"
description: "Learn the best practices for error management in Vox to build robust applications."
category: "how-to"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---
# How-To: Handle Errors Gracefully

Learn the best practices for error management in Vox to build robust, fault-tolerant applications.

## 1. The `Result` Type

Vox uses the functional `Result[T, E]` type for operations that can fail, rather than standard exceptions.

```vox
# Skip-Test: ui-only
fn find_user(id: str) to Result[str] {
    if id == "" {
        ret Error("Invalid ID")
    }
    ret Ok(id)
}
```

## 2. Using the `?` Operator

The `?` operator provides ergonomic error propagation. If an expression evaluates to `Error`, the surrounding function returns that error immediately.

```vox
# Skip-Test: ui-only
fn process_order(id: str) to Result[bool] {
    let user = find_user(id)?
    // `check_balance` might also return a Result
    // let balance = check_balance(user)?
    ret Ok(true)
}
```

## 3. Error Handling

Vox allows you to handle `Result` types directly using exhaustive pattern matching. (Error display in UI is covered in the islands tutorial).

```vox
# Skip-Test: ui-only
let result = find_user("123")

match result {
    Ok(user)   -> print("Found { " + user)
    Error(msg) -> print("Failed: " + msg)
}
```

## 4. Panic vs. Error

- **Errors (`Result`)**: Use for expected failures (e.g., user not found, validation error).
- **Panics**: Use for unrecoverable logic errors or violated invariants (e.g., array out of bounds). Panics trigger actor restarts in stateful systems.

---

## Summary
- Use `Result` for operations that can gracefully fail.
- Use `?` to easily propagate `Error` up the call stack.
- Use pattern matching with `match` blocks to unwrap and inspect the branches safely.

## Related
- [Language Syntax](../reference/ref-syntax.md) — Syntax for `match` and `?`.
- [Durable Workflows](../tutorials/tut-workflow-durability.md) — Automatic error recovery in long-running tasks.
