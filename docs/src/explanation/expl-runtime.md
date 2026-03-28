---
title: "Explanation: The Vox Runtime"
description: "Official documentation for Explanation: The Vox Runtime for the Vox language. Detailed technical reference, architecture guides, and impl"
category: "explanation"
last_updated: 2026-03-24
training_eligible: true
---
# Explanation: The Vox Runtime

Understand the inner workings of the Vox runtime—the engine that powers AI-native, stateful applications.

## Implementation map (current)

The runtime-facing story in today’s codebase is split across:

- `crates/vox-runtime/src/lib.rs`: actor/process/runtime primitives and exported runtime modules.
- `crates/vox-runtime/src/builtins.rs`: standard builtin implementations used by generated Rust code.
- `crates/vox-compiler/src/codegen_rust/emit/http.rs`: generated Axum app host for routes/server/query/mutation handlers.
- `crates/vox-compiler/src/app_contract.rs`: app-surface contract projection used to keep route/RPC/server config mapping centralized.

## 1. Actor-Based Concurrency

At its core, Vox is an actor-based system. Unlike traditional shared-memory concurrency (threads + locks), Vox processes communicate via message passing.

- **Isolation**: Each actor has its own private stack and heap.
- **Mailbox**: Messages are queued and processed sequentially, eliminating race conditions by design.
- **Mailbox Backpressure**: Mailboxes have bounded capacities to prevent memory exhaustion during spikes.

## 2. The Cooperative Scheduler

Vox uses a custom cooperative scheduler built on top of the Tokio runtime.

- **Fibers/Processes**: Vox "processes" are lightweight tasks managed by the scheduler.
- **Reduction Counting**: To ensure fairness, each process has a "reduction budget." When it performs operations (like I/O or computations), the budget decreases. Once empty, the process yields control to other pending tasks.
- **Work Stealing**: The scheduler automatically moves processes between CPU cores to optimize throughput.

## 3. Technical Unification

Vox achieves "Technical Unification" by abstracting the boundary between frontend and backend.

- **RPC-as-Function**: Calling a `@server fn` from the UI looks like a local function call but is actually a type-safe RPC.
- **State Synchronization**: Actors can bridge state between the server and the UI using persistent subscriptions.

## 4. Error Recovery & Supervision

Superior reliability is achieved through hierarchies of supervisors.

- **Let It Crash**: If an actor fails, its supervisor detects the death and determines whether to restart it based on a strategy (e.g., `OneForOne`).
- **State Restoration**: Actors can be configured to restore their state from the `vox.db` upon restart.

---

**Related Reference**:
- [Actor Basics Tutorial](../tutorials/tut-actor-basics.md) — Learn how to use actors in practice.
- [Scheduler API](../api/vox-runtime.md) — Technical details of the `Scheduler` struct.
