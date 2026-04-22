---
title: "Actors & Workflows"
description: "Deep dive into Vox's primary concurrency primitives: Persistent Actors for stateful messaging and Durable Workflows for reliable orchestration."
category: "explanation"
last_updated: "2026-04-06"
status: "current"
training_eligible: true

schema_type: "TechArticle"
---
# Actors & Workflows

Vox provides two first-class concurrency primitives: **Actors** for lightweight message-passing and **Workflows** for orchestrating activities. Actor behavior is materially implemented today. Workflow durability is currently a mix of language intent, generated async code, and a separate interpreted runtime.

---

## Actors

Actors are isolated processes with their own state and a mailbox for receiving messages. They communicate exclusively via message passing — no shared memory.

### Defining an Actor

```vox
// vox:skip
actor Counter {
    let mut count: int = 0

    on increment(amount: int) -> int {
        count = count + amount;
        return count;
    }

    on get_count() -> int {
        return count;
    }

    on reset() {
        count = 0;
    }
}
```

Key concepts:
- **`state`** fields hold mutable internal data
- **`on`** handlers define message responses
- Each handler returns a typed result

### Spawning and Messaging

```vox
// vox:skip
fn main() {
    // spawn() creates a new actor instance, returns a handle (ActorRef)
    let counter = spawn Counter();
    let greeter = spawn Greeter();

    // .send() dispatches a message to the actor's mailbox
    counter.send increment(5);
    greeter.send greet("Alice");

    // Actors can receive multiple messages
    counter.send increment(3);
    let total = await counter.get_count(); 
}
```

### Messages

Define typed messages for inter-actor communication:

```vox
// vox:skip
type Greeting {
    from_name: str,
    text: str,
}
```

### Durable Actors

Actors can persist state across restarts using `state_load` and `state_save`:

```vox
// vox:skip
actor PersistentCounter {
    on increment() -> int {
        let current = state_load("counter");
        let next = current + 1;
        state_save("counter", next);
        return next;
    }
}
```

This compiles to database-backed state management — the actor's count survives process restarts.

> [!NOTE]
> `state_load(key: str) -> T` and `state_save(key: str, val: T) -> Unit` are **compiler-injected built-ins** available *only* inside `actor` blocks. They seamlessly marshal generic types directly to the persistence layer.

### How Actors Compile

| Vox Concept | Compiled Output (Rust) |
|-------------|----------------------|
| `actor Counter` | Tokio task + `mpsc::channel` mailbox |
| `spawn(Counter)` | `ProcessHandle` via `ProcessRegistry` |
| `counter.send(msg)` | Channel send + optional `oneshot` for reply |
| `state count: int = 0` | Struct field with default |
| `state_load` / `state_save` | Database read/write via `ProcessContext` |

---

## Activities

Activities are retryable units of work that may fail. They are the **only** place for side effects within workflows.

```vox
// vox:skip
activity fetch_user_data(user_id: str) -> Result[str] {
    // Would call an external API in production
    return Ok("User data for " + user_id);
}

activity send_notification(email: str, body: str) -> Result[bool] {
    // External email service call
    return Ok(true);
}
```

Activities must always return a `Result` type, since they represent operations that can fail.

---

## Quick Comparison

| Concept | Keyword | Survival | State |
|---|---|---|---|
| Actor | `actor` | Lives in memory; revive with same ID | `state_load`/`state_save` |
| Workflow | `workflow` | Interpreted runtime can replay completed steps | Journal in Codex |
| Activity | `activity` | Individual retryable step within a workflow | None (idempotent) |

---

## Workflows

Workflows orchestrate activities with retry and journaling intent.

Current state:

- **Implemented semantics:** workflow syntax, `with { ... }` parsing/typechecking, generated async Rust functions, interpreted workflow planning/journaling, stored step-result replay, and retry/backoff for interpreted `mesh_*` activities.
- **Planned semantics:** full durable state-machine execution for the generated Rust path and richer replay models for branching/loops.
- **Escape hatch / current durable path:** the interpreted workflow runtime used by `vox mens workflow ...`.

```vox
// vox:skip
workflow onboard_user(user_id: str, email: str) -> Result[str] {
    // Step 1: Fetch user profile
    let profile = fetch_user_data(user_id) with { retries: 3, timeout: "30s" };

    // Step 2: Send welcome email
    let _ = send_notification(email, "Welcome! " + profile) with { retries: 5, timeout: "60s" };

    // Step 3: Return success
    return Ok("Onboarding complete for " + user_id);
}
```

### The `with` Expression

The `with` expression carries workflow activity options. Some are honored today in the interpreted runtime, while others only matter on specific runtime paths:

| Option | Type | Description |
|--------|------|-------------|
| `retries` | `int` | Honored for interpreted `mesh_*` activity execution; local interpreted steps remain journal-only no-ops |
| `timeout` | `str` | Parsed today for interpreted runtime activity planning |
| `initial_backoff` | `str` | Honored for interpreted `mesh_*` retries |
| `activity_id` | `str` | Explicit durable/journal key |
| `id` | `str` | Alias for `activity_id` in `with { ... }`; honored in interpreted planning and generated Rust activity-option lowering |
| `mens` | `str` | Mesh control override for interpreted `mesh_*` activities |

### Durable Execution

The interpreted workflow runtime can skip previously completed activities when restarted with the same workflow, run id, and activity ids because it records journal/tracker data before replay and now stores step result payloads for linear replay. Generated Rust workflows do **not** yet compile into a durable state machine.

**Durable spine (today):** the supported replay/idempotency story is the interpreted `vox mens workflow …` runtime (see [ADR-019](../adr/019-durable-workflow-journal-contract-v1.md)). Rust-emitted `async fn` workflows are orchestration helpers only until generated code adopts the same journaling contract. Generated-workflow parity remains intentionally out of scope until Vox has a formal replay model and ADR for it (see [ADR-021](../adr/021-generated-workflow-durability-parity.md)).

### How Workflows Compile

| Vox Concept | Current generated / runtime behavior |
|-------------|------------------------------------|
| `workflow` | Generated as a plain `async fn` in Rust codegen |
| `activity` | Generated as a plain `async fn`; `with` lowering adds helper wiring in some paths |
| `with { retries: 3 }` | Interpreted runtime honors it for `mesh_*` activity execution; local interpreted steps stay journal-only |
| Step completion | Interpreted runtime journals versioned events and stores replayable step results; generated Rust path is not yet a durable state machine |

---

## Full Example: Order Processing

A complete workflow combining activities with different retry policies:

```vox
// vox:skip
type OrderResult {
    Ok { order_id: str }
    Error { message: str }
}

activity validate_order(order_data: str) -> Result[str] {
    let validated = "validated-" + order_data;
    return Ok(validated);
}

activity charge_payment(amount: int, card_token: str) -> Result[str] {
    let tx = "tx-" + card_token;
    return Ok(tx);
}

activity send_confirmation(recipient: str, order_id: str) -> Result[str] {
    let msg = "Order " + order_id + " confirmed for " + recipient;
    return Ok(msg);
}

workflow process_order(customer: str, order_data: str, amount: int) -> Result[str] {
    // Validate with a short timeout and no retries
    let validated = validate_order(order_data) with { timeout: "5s" };

    // Charge payment with retries and backoff
    let payment = charge_payment(amount, "card-123") 
        with { retries: 3, timeout: "30s", initial_backoff: "500ms" };

    // Send confirmation with basic retry
    let confirmation = send_confirmation(customer, "order-001") 
        with { retries: 2, activity_id: "confirm-order-001" };

    return confirmation;
}
```

---

## Next Steps

- [Language Reference](../reference/ref-syntax.md) — Full syntax and type system reference
- [Compiler Architecture](expl-architecture.md) — How actors and workflows compile

## Durability Taxonomy

Understanding the types of durability is crucial when reasoning about failure recovery in Vox:

1. **Persistent Actors** (state_load / state_save):
   State survives restarts because the logic explicitly reads from and writes to the Codex under specific keys. When the actor respawns, it resumes with the last saved state.
2. **Workflow Durability** (Interpreted Runtime):
   When running via `vox run` or `vox mens` workflow, the engine tracks execution steps natively in the database. If the process dies and restarts, completed activities are short-circuited.
3. **Compiled Rust Workflows** (Future Parity):
   Workflows that are compiled strictly down to standard Rust async equivalents do not automatically benefit from step-level replayable durability yet. This remains an active implementation target for parity with the interpreted path (see ADR-021).


