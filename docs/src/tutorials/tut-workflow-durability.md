---
title: "Tutorial: Workflow Durability"
description: "Official documentation for Tutorial: Workflow Durability for the Vox language. Detailed technical reference, architecture guides, and imp"
category: "tutorial"
last_updated: 2026-03-28
training_eligible: true
---
# Tutorial: Workflow Durability

Learn how to build resilient, long-running processes using Vox workflows. This tutorial explains the durability story Vox supports today: interpreted workflow step replay, stable activity ids, and idempotent activities.

## 1. The Challenge of Long-Running Tasks

Traditional async functions lose their state if the server restarts or a network error occurs. Vox workflows are intended to solve that, but the durable path available today is the interpreted workflow runtime rather than every generated async function.

## 2. Defining a Workflow

Use workflow syntax to describe long-running orchestration. In the current repo, the durable replay story is tied to the interpreted runtime and stable activity ids.

```vox
# Skip-Test
@activity fn process_payment(amount: int) to Result[bool]:
    # This might fail or time out
    ret db.execute("UPDATE accounts...")

@workflow fn order_fulfillment(order_id: str):
    # In the interpreted runtime, a stable activity id can make this step resumable
    let success = process_payment(100)

    if success:
        print("Order " + order_id + " fulfilled!")
```

## 3. How It Works

1. **Step tracking**: The interpreted runtime records activity progress in Codex / `VoxDb` workflow tracking tables.
2. **Recovery**: If the workflow is restarted with the same run identity and activity ids, the runtime can skip steps that already finished.
3. **Idempotency**: Activities should still be safe to retry. Durable step replay is not the same thing as a universal exactly-once guarantee.

## 4. Workflows vs. Tasks

| Feature | Regular Task | Vox Workflow |
|---------|--------------|--------------|
| Survival | Dies on reboot | Interpreted workflow runtime can resume completed steps |
| Retry | Manual `try/catch`| `with { ... }` support is partial today |
| State | In-memory | Durable step tracking, not full arbitrary program snapshots |

## 5. Best Practices

- **Idempotency**: Activities should be idempotent since they might be retried after a crash.
- **Deterministic**: Workflow logic must be deterministic. Avoid using `rand()` or `Date.now()` directly inside the workflow body; use an activity instead.
- **Stable step ids**: Use explicit `activity_id` values for steps you expect to resume safely across restarts. `with { id: "..." }` is an alias for `activity_id`.

---

**Next Steps**:
- [Language Reference](../reference/ref-language.md) — Explore advanced workflow decorators.
- [First App](tut-first-app.md) — Integrate a workflow into your todo list.
