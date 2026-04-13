---
status: archived
archived_date: 2026-04-13
training_eligible: false
schema_type: "TechArticle"
title: "Archived Plan: ai_developer_experience_implementation.plan"
---

> [!WARNING]
> **ARCHIVED COMPONENT**: This file was archived on 2026-04-13. It is intentionally excluded from active AI context. It must not be referenced for contemporary development.


# AI Developer Experience Implementation Plan & Correction Matrix

Based on the research synthesizing the behaviors of Cursor Composer, Devin, and Copilot Workspaces, the Vox platform requires four major structural upgrades to realize its intent as the ultimate AI developer tool.

## Execution Blueprint

### Phase 1: Human-in-the-Loop Visuals
Instead of relying strictly on command-line stdout, agents must emit state.
1. Wire `vox-dei` to emit `TelemetryEvent::AgentProgress` via the standard telemetry pipe.
2. Surface this in the VS Code extension's Webview, showing a node-graph of current file locks and reasoning states.
3. Establish hard "Review Gates" for destruct edits (e.g. deleting files requires human IDE acknowledgment).

### Phase 2: State & Context Persistence (Durable Execution)
1. Implement the `AgentJournal` table in `vox-arc/src/schema.rs`.
2. Ensure every step loop in the LLM chat completion writes a delta to the database.
3. Implement `vox dei resume <task-id>` to reconstruct the prompt context from the database if the IDE reloads.

### Phase 3: Sandbox Terminal Virtualization
1. Audit current shell command usages across agent scripts.
2. Implement `vox_dei::sandbox::execute(...)` which intercepts commands.
3. Apply a blacklist (or whitelist) to prevent destructive ops (`rm -rf`, network calls outside allowed domains).
4. Allow seamless, unprompted execution of `vox test` and `cargo check` within loop budgets.

### Phase 4: Multi-Agent Parallelism (Architect / Implementer)
1. Split the unified `Orchestrator` into `PlannerNode` and `WorkerNode`.
2. Add support to parse a `plan.md` representation into distinct JSON `SubTask` structures.
3. Spin up concurrent LLM calls for `WorkerNode`s restricted to their active sub-task file path constraint.
4. Merge results using a map-reduce style Git patch applier.

## Verification
- Success is defined when a developer can assign Vox a Jira ticket and view a durable, resumable agent process in real-time, safely sandboxed from destroying the OS environment, utilizing multiple models concurrently.

