---
title: "How-To: Test Your Logic"
description: "Write automated test cases using Vox."
category: "how-to"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---
# How-To: Test Your Logic

Learn how to write and run automated tests for your Vox application using the built-in test runner.

## 1. Writing Unit Tests

Use the `@test` decorator to mark functions as test cases. These functions can be run with the `vox test` command.

```vox
// Skip-Test
@test 
fn test_addition() -> Unit {
    assert(1 + 1 == 2)
}
```

## 2. Hand-Rolled Setup Helpers (Fixtures)

Rather than language-level magic, Vox encourages simple, plain functions for setup logic that can be reused across test cases.

```vox
// Skip-Test
fn setup_mock_db() -> Database {
    return spawn MockDatabase()
}

@test 
fn test_query() -> Unit {
    let db = setup_mock_db()
    let result = db.call(query("SELECT 1"))
    assert(result == [1])
}
```

> [!WARNING]
> Historical decorators `@fixture` and `@mock` are considered aspirational. Use standard helper functions for state-setup instead.

## 3. Integration Testing

Test your full-stack logic by running the compiler and checking the generated output or simulating HTTP requests.

```bash
# Run all tests in the project
vox test src/
```

## Summary
- Use `@test` to label individual test cases to be picked up by the compiler harness.
- Write standard functions that serve as setups, fixtures, and mocks explicitly.
- Run `vox test <path>` to execute blocks tagged with `@test`.

## Related
- [CLI Reference](../reference/cli.md) — `vox test` flags and configuration.
- [Durable Workflows](../tutorials/tut-workflow-durability.md) — Understanding testable workflows.
