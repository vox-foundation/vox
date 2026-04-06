---
title: "How-To: Scale Actors"
description: "Strategies for distributing and managing Vox actor lifecycles across a cluster."
category: "how-to"
status: "current"
last_updated: "2026-04-06"
training_eligible: true
---

# How-To: Scale Actors

As your application grows beyond a single executable, Vox Actors must scale horizontally across the Populi mesh or large orchestrated deployments.

## The Concept of Actor Affinity

By default, an initialized Actor runs in memory on the node where `spawn` was invoked. In a distributed environment, you rely on the **Codex** to synchronize and persist state securely.

```vox
actor SessionManager {
    on Login(user: str) to Result[str] {
        let current_sessions = state_load("active_users")
        // logic ...
        state_save("active_users", current_sessions)
        ret Ok("Success")
    }
}
```

Because `state_save` natively pushes updates to Codex, another node starting a `SessionManager` actor targeting the same specific state scope can seamlessly resume operations. 

## Load Balancing and Populi

When scaling the inference compute or orchestration logic via Populi Meshes, Vox abstracts message routing.

1. **Local Node Execution**: Functions run via Tokio threads in the core binary.
2. **Distributed GPU Execution**: LLM evaluation or heavy compute tasks explicitly placed on GPU labeled nodes. 

To dispatch an orchestration task externally, the framework determines placement inherently via the resource requests. 

> [!WARNING]
> Manual remote procedure calls (RPC) to force specific Actor placement remains in active development. As of v0.3, horizontal scaling predominantly operates seamlessly behind standard `routes { }` load-balancing and Turso replicated databases, rather than direct point-to-point remote actor message passing.

## Designing Stateless Workers

Where possible, convert stateful actor workloads into durable `workflow` execution graphs. Because a `workflow` relies entirely on journaled steps in the persistence layer, if the parent node dies midway, any healthy node observing the mesh queue will implicitly pick up the workflow and continue from the exact boundary checkpoint boundary of the last successful `activity`!
