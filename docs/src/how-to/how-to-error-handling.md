---
title: "How-To: Handle Errors Gracefully"
description: "Learn the best practices for error management in Vox to build robust applications."
category: "How-To Guides"
status: "current"
training_eligible: true

schema_type: "HowTo"
---
# How-To: Handle Errors Gracefully

Learn the best practices for error management in Vox to build robust, fault-tolerant applications.

## 1. The `Result` Type

Vox uses the functional `Result[T, E]` type for operations that can fail, rather than standard exceptions.

```vox
fn find_user(id: str) to Result[str] {
    if id is "" {
        return Error("Invalid ID")
    }
    return Ok(id)
}
```

## 2. Using the `?` Operator

The `?` operator provides ergonomic error propagation. If an expression evaluates to `Error`, the surrounding function returns that error immediately.

```vox
fn process_order(id: str) to Result[bool] {
    let user = find_user(id)?
    // `check_balance` might also return a Result
    // let balance = check_balance(user)?
    return Ok(true)
}
```

## 3. Error Handling

Vox allows you to handle `Result` types directly using exhaustive pattern matching. (Error display in UI is covered in the islands tutorial).

```vox
fn describe_user(id: str) to str {
    let result = find_user(id)

    return match result {
        Ok(user)   => "Found: " + user
        Error(msg) => "Failed: " + msg
    }
}
```

## 4. Converting Errors with `Result[T, E]`

You can transform results using explicit pattern matching or helper wrappers.

```vox
fn get_user_name(id: str) to Result[str] {
    return match find_user(id) {
        Ok(user) => Ok(user)
        Error(e) => Error("User fetch failed: " + e)
    }
}
```

## 5. Preconditions with `@require`

For invariant safety (assertions that must hold before constructing a value), use the `@require` decorator on the constructor function.

```vox
type Adult {
    name: str
    age: int
}

@require(age >= 18)
fn create_adult(name: str, age: int) to Adult {
    return { name: name, age: age }
}
```

If the condition fails during instantiation, a panic is triggered (or an error returned if used within a fallible constructor context).

---

## Best Practices

1. **Surface Results Early**: Always surface the `Result` type rather than attempting to `unwrap()` or panic inside production web routes.
2. **Contextualize Errors**: Use `.map_err()` to add context to low-level errors (e.g., "Database error" -> "Failed to save user").
3. **Use `?` for Flow**: The `?` operator is the preferred way to maintain a "happy path" while handling fallibility.

---

## Summary
- Use `Result` for operations that can gracefully fail.
- Use `?` to easily propagate `Error` up the call stack.
- Use pattern matching with `match` blocks to unwrap and inspect the branches safely.

## Related
- [Language Syntax](../reference/ref-syntax.md) — Syntax for `match` and `?`.
- [Durable Workflows](../tutorials/tut-workflow-durability.md) — Automatic error recovery in long-running tasks.
