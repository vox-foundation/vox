---
title: "Research: Planning Mode Capability Map"
description: "Detailed mapping of tool capabilities and agent constraints in planning mode."
category: "architecture"
status: "research"
training_eligible: false

schema_type: "TechArticle"
---

# Planning Capability Implementation Map

The current implementation status across Vox's major planning capabilities in the V2 Agentic Architecture.

## Execution Matrix

| Capability Category | Status | Primary Component | Notes |
| :--- | :--- | :--- | :--- |
| **Agentic Task Decomposition** | Fully Delivered | `vox-mcp` (chat_tools) | The LLM effectively segments goals into verifiable tasks complete with complexity heuristics and sequential DAG wiring. |
| **Execution Policy Routing** | Delivered | `vox-orchestrator` | Tasks are classified by discrete categories; `ExecutionPolicy` controls the active operational bounds and skills authorized per step. |
| **RequiresApproval Gates** | Delivered | `vox-orchestrator` | Task queues dynamically defer manual execution via the `TaskStatus::BlockedOnApproval` orchestrator state loop. |
| **Determinism Enforcement** | Delivered | `plan_adequacy.rs` | Quality gates reject proposals aggressively if exact test enforcement logic is absent from generated task properties. |
| **Socratic Ambiguity Checks** | Delivered | `task_submit.rs` | Nonsensical, disjointed, or abusive planning instructions are strictly vetoed prior to queuing via contextual risk evaluation. |
| **Centralized Complexity Judging**| Delivered | `vox-socrates-policy` | The legacy 1-10 string estimates are completely retired for the global `SocratesComplexityJudge` heuristics integration. |
| **Context Assembly Disipline** | Delivered | `vox-mcp` | Planning context limits and memory queries natively prune non-essential metadata and strictly bound AI ingestion profiles. |
| **VCS Workspace Persistence** | Pending | `vox-vcs` | Snapshot rollback boundaries across failed sub-tasks and comprehensive artifact persistence layers are targeted for future sweeps. |
| **Codex Telemetry Streaming** | Pending | `vox-db` | Exposing reliable Server-Sent Event (SSE) pipelines back to the end-users via the internal `vox-codex-api`. |
