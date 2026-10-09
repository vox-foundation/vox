---
title: "Tutorial: Actor Basics"
description: "Declare a Vox actor with message handlers, see what vox build generates for it, and learn which actor features are not wired up yet."
category: "Tutorials"
status: "current"
sort_order: 4
training_eligible: true

schema_type: "HowTo"
---

# Tutorial: Actor Basics

In Vox, an **actor** is a named unit that receives messages and handles each one with an `on` handler. This tutorial declares an actor, shows what `vox build` generates for it today, and lists the parts of the actor model that are not wired up yet — so you know what you can rely on.

## 1. Defining the Actor

An actor is defined with the `actor` keyword. Its body contains only `on` handlers: an event name, parameters, an optional return type, and a body.

```vox
actor Counter {
    on increment(amount: int) to int {
        return amount + 1
    }

    on reset() {
        return
    }
}
```

`vox check` accepts this file. Anything other than an `on` handler inside the block (for example a `state count: int` field) is a parse error: actor state fields have no syntax yet.

## 2. What `vox build` Generates

`vox build` lowers the actor into the generated Rust crate (`target/generated/src/lib.rs`):

| Vox construct | Generated Rust |
| :--- | :--- |
| `actor Counter { … }` | `struct CounterState` (empty today) and a `fn Counter()` that starts a mailbox loop with `vox_actor_runtime::spawn_process` |
| `on increment(amount: int) to int` | A plain function `Counter_increment(state: &mut CounterState, amount: i64) -> i64` |
| Mailbox loop | Reads each envelope with `ctx.receive()`; decodes a JSON payload `{"event": "<handler>", "args": [ … ]}`. A `Message` envelope is fire-and-forget, a `Request` envelope is answered with `ProcessContext::reply`, a `Signal` is ignored. |

The lowering lives in `crates/vox-codegen/src/codegen_rust/emit/durability_lower.rs` (`emit_actor_body`).

## 3. Current Limits

These are the gaps between the actor model and what ships today:

- **No message routing in `vox build` output yet.** The generator only fills the mailbox's `match` on the event name when the actor declares state fields, and the parser has no state-field syntax. So the generated loop receives messages but has no arms that call your handlers. (The codegen unit test `actor_dispatch_table_routes_to_handlers` exercises the routed form directly.)
- **No spawning or sending from Vox source.** There is no Vox expression that starts an actor or sends it a message; the generated `fn Counter()` is not called by anything else in the generated crate.
- **No persistence.** Actors do not save or reload state across restarts. For work that must survive a crash, use [durable workflows](tut-workflow-durability.md), which journal each completed step.

## 4. Summary Checklist

- [x] **Declare**: `actor Name { on event(params) [to T] { … } }`.
- [x] **Handlers**: each `on` handler compiles to a plain Rust function that takes the actor's state.
- [x] **Mailbox**: the actor shell compiles to a `vox_actor_runtime` process with a receive loop.
- [ ] **Routing, spawning, messaging, state, persistence**: not available yet (see Current Limits).

---

**Next Steps**:
- [Workflow Durability](tut-workflow-durability.md) — Orchestrate complex, multi-step long-running processes.
- [Actors & Workflows Explanation](../explanation/expl-actors-workflows.md) — Deep dive into the theory.
- [CLI Reference: vox run](../reference/cli.md#vox-run-file----args) — Run your actor-based applications.
