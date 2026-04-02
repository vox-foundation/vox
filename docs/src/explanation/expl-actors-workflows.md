---
title: "Actors & Workflows"
description: "Official documentation for Actors & Workflows for the Vox language. Detailed technical reference, architecture guides, and implementation"
category: "explanation"
last_updated: 2026-03-24
training_eligible: true
---
# Actors & Workflows

Vox provides two first-class concurrency primitives: **Actors** for lightweight message-passing and **Workflows** for orchestrating activities. Actor behavior is materially implemented today. Workflow durability is currently a mix of language intent, generated async code, and a separate interpreted runtime.

---

## Actors

Actors are isolated processes with their own state and a mailbox for receiving messages. They communicate exclusively via message passing — no shared memory.

### Defining an Actor

```vox
# Skip-Test
actor Counter:
    state count: int = 0

    on increment(amount: int) to int:
        count = count + amount
        count

    on get_count() to int:
        count

    on reset() to Unit:
        count = 0
```

Key concepts:
- **`state`** fields hold mutable internal data
- **`on`** handlers define message responses
- Each handler returns a typed result

### Spawning and Messaging

```vox
# Skip-Test
fn main():
    # spawn() creates a new actor instance, returns a handle (Pid)
    let counter = spawn(Counter)
    let greeter = spawn(Greeter)

    # .send() dispatches a message to the actor's mailbox
    let new_count = counter.send(increment(5))
    let greeting  = greeter.send(greet("Alice"))

    # Actors can receive multiple messages
    let _ = counter.send(increment(3))
    let total = counter.send(get_count())   # returns 8
```

### Messages

Define typed messages for inter-actor communication:

```vox
# Skip-Test
message Greeting:
    from_name: str
    text: str
```

### Durable Actors

Actors can persist state across restarts using `state_load` and `state_save`:

```vox
# Skip-Test
actor PersistentCounter:
    on increment() to int:
        let current = state_load("counter")
        let next = current + 1
        state_save("counter", next)
        ret next
```

This compiles to database-backed state management — the actor's count survives process restarts.

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
# Skip-Test
activity fetch_user_data(user_id: str) to Result[str]:
    # Would call an external API in production
    ret Ok("User data for " + user_id)

activity send_notification(email: str, body: str) to Result[bool]:
    # External email service call
    ret Ok(true)
```

Activities must always return a `Result` type, since they represent operations that can fail.

---

## Workflows

Workflows orchestrate activities with retry and journaling intent.

Current state:

- **Implemented semantics:** workflow syntax, `with { ... }` parsing/typechecking, generated async Rust functions, interpreted workflow planning/journaling, stored step-result replay, and retry/backoff for interpreted `mesh_*` activities.
- **Planned semantics:** full durable state-machine execution for the generated Rust path and richer replay models for branching/loops.
- **Escape hatch / current durable path:** the interpreted workflow runtime used by `vox mens workflow ...`.

```vox
# Skip-Test
workflow onboard_user(user_id: str, email: str) to Result[str]:
    # Step 1: Fetch user profile
    let profile = fetch_user_data(user_id) with { retries: 3, timeout: "30s" }

    # Step 2: Send welcome email
    let _ = send_notification(email, "Welcome! " + profile) with { retries: 5, timeout: "60s" }

    # Step 3: Return success
    ret Ok("Onboarding complete for " + user_id)
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

**Durable spine (today):** the supported replay/idempotency story is the interpreted `vox mens workflow …` runtime. Rust-emitted `async fn` workflows are orchestration helpers only until generated code adopts the same journaling contract. Generated-workflow parity remains intentionally out of scope until Vox has a formal replay model and ADR for it.

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
# Skip-Test
type OrderResult =
    | Ok(order_id: str)
    | Error(message: str)

activity validate_order(order_data: str) to Result[str]:
    let validated = "validated-" + order_data
    ret Ok(validated)

activity charge_payment(amount: int, card_token: str) to Result[str]:
    let tx = "tx-" + card_token
    ret Ok(tx)

activity send_confirmation(recipient: str, order_id: str) to Result[str]:
    let msg = "Order " + order_id + " confirmed for " + recipient
    ret Ok(msg)

workflow process_order(customer: str, order_data: str, amount: int) to Result[str]:
    # Validate with a short timeout and no retries
    let validated = validate_order(order_data) with { timeout: "5s" }

    # Charge payment with retries and backoff
    let payment = charge_payment(amount, "card-123") with { retries: 3, timeout: "30s", initial_backoff: "500ms" }

    # Send confirmation with basic retry
    let confirmation = send_confirmation(customer, "order-001") with { retries: 2, activity_id: "confirm-order-001" }

    ret confirmation
```

---

## Next Steps

- [Language Guide](../reference/ref-language.md) — Full syntax and type system reference
- [Compiler Architecture](expl-architecture.md) — How actors and workflows compile
- [Examples](../how-to/examples-corpus.md) — All example programs with annotations
