---
title: "Explanation: Durable Execution"
description: "Official documentation for workflow durability in Vox's interpreted runtime, including journal semantics, replay limits, and idempotency guidance."
category: "Concepts"
training_eligible: true

schema_type: "TechArticle"
---
# Explanation: Durable Execution

> [!NOTE]
> **What is true as of Phase 6.** The interpreted runner executes activity
> bodies and journals their real return values. Resume replays completed
> `activity_id`s. `workflow_wait` parks the run with a persisted `wake_at_ms`
> (not an in-process sleep). `workflow_wait_signal` parks until a signal
> arrives. `with { }` and `workflow.version` are implemented. Generated
> workflows (`vox build`) use the same `VoxDbTracker` engine as
> `vox mens workflow run`. Interp is **Stable** for the body-first subset
> (`if` / `match` / `for` / `while` / `?`). Nested workflow calls and mesh /
> `PopuliActivity` remain unsupported (loud error, not a silent drop). See
> [True workflow durability](../architecture/true-workflow-durability-design-2026.md).

Understand the current durability boundary in Vox. Durable execution is a workflow feature of one engine — the interpreted runner used by `vox mens workflow ...` and by generated wrappers (`vox build`) — not a blanket guarantee for every compiled Vox program.

> [!NOTE]
> **One engine, two entry points.** `vox mens workflow run` and generated
> Rust wrappers both call `interpret_workflow_durable` with a SQL
> `VoxDbTracker`. Generated code is not a second linearizer and is not an
> in-memory tracker.

## 1. The Journal System

In the interpreted workflow runtime, Vox records workflow progress as activity steps complete. The durable truth today is step-oriented: the runtime tracks which `activity_id` values have already completed for a workflow run and stores the completed step result payload so it can replay that result after a restart.

```mermaid
graph TD
    A[Start Workflow] --> B{Activity Finished?}
    B -- No --> C[Execute Activity]
    C --> D[Write to Journal]
    D --> B
    B -- Yes --> E[End Workflow]
```

## 2. Recovery via Replay

If the interpreted runtime crashes mid-workflow, recovery currently works like this:

1. Restart the workflow runner with the same workflow, durable `run_id`, and stable activity ids.
2. Read durable workflow tracking data from Codex / `VoxDb`.
3. Load stored results for activities that were already recorded as completed for that run.
4. For completed steps, replay the journalled activity return value so the
   workflow body continues without re-running that activity.

This is replay-from-the-top, not a process/VM checkpoint. Local variables are
not snapshotted; they are recomputed as the body re-executes and completed
activities return their stored results. Nested workflow calls and mesh /
`PopuliActivity` are rejected with a loud error.

## 3. Exactly-Once Semantics

Treat the current model as **durable step deduplication**, not a universal exactly-once guarantee.

- If an activity step was already recorded as completed for the same run, the interpreted runtime can skip it on resume.
- For linear interpreted workflows, the runtime can also replay the stored step result payload into the new journal stream.
- External side effects are only safe when the activity itself is **idempotent**, meaning it can tolerate retries without corrupting state.
- If you need a stronger guarantee, design the activity to accept an explicit idempotency key such as `activity_id`.

## 4. Determinism Requirements

For replay to work, the workflow body should stay deterministic.

- **BAD**: `let d = Date.now()` (Time changes on replay)
- **GOOD**: `let d = get_current_time()` (Wrap non-deterministic calls in an `activity`)

## 5. Storage Backend

The current durable workflow tracking path uses Codex / `VoxDb` tables such as `workflow_activity_log` and `workflow_run_log`. These tables store durable run identity, step completion status, replayable result payloads, and run lifecycle state for the interpreted workflow path, including single-owner run lease fields used to avoid split-brain execution on the same `run_id`.

Older docs referenced `_vox_journal`, `sqlite_vox_journal`, PostgreSQL, or DynamoDB; treat those as stale unless a newer implementation page says otherwise.

## 6. Journal Contract (v1)

The interpreted workflow journal now carries **`journal_version: 1`** on event objects emitted by the workflow runtime.

Current event families:

- Lifecycle: `WorkflowStarted`, `WorkflowCompleted`
- Step execution: `ActivityStarted`, `ActivityCompleted`
- Step replay: `ActivityReplayed`, followed by the stored step payload
- Retry support: `ActivityAttemptRecovered`, `ActivityAttemptFailed`, `ActivityRetryScheduled`
- Step payloads: `LocalActivity`, `MeshActivity`, `MeshActivitySkipped`
- Legacy fallback: `ActivitySkipped` when a step is marked complete but no replayable result payload is available

The current SSOT for this contract is the interpreted workflow runtime in:

- [crates/vox-workflow-runtime/src/workflow/run.rs](../../../crates/vox-workflow-runtime/src/workflow/run.rs)
- [crates/vox-db/src/facade/workflow.rs](../../../crates/vox-db/src/facade/workflow.rs)
- [crates/vox-db/src/workflow_journal.rs](../../../crates/vox-db/src/workflow_journal.rs)
- [contracts/workflow/workflow-journal.v1.schema.json](../../../contracts/workflow/workflow-journal.v1.schema.json)
- [docs/src/adr/019-durable-workflow-journal-contract-v1.md](../adr/019-durable-workflow-journal-contract-v1.md)
- [docs/src/adr/021-generated-workflow-durability-parity.md](../adr/021-generated-workflow-durability-parity.md)

Codex append for interpreted workflow journals is **enabled by default when DB config resolves** and can be disabled with `VOX_WORKFLOW_JOURNAL_CODEX_OFF=1`.

## 7. Durability Taxonomy

Use these terms distinctly:

- **Durable execution**: workflow step replay in the interpreted workflow runtime
- **Durable state**: actor persistence through `state_load` / `state_save`
- **Durable delivery**: inbox/outbox, queue, and lease/ack message semantics
- **Durable jobs**: background workers or scheduled work surviving restarts
- **Durable history / audit**: oplogs, lineage, and analytics journals

This keeps Vox from accidentally using one word for several different guarantees.

## 8. Current Scope

- **Supported durable path today:** interpreted workflows through `vox mens workflow ...` and generated wrappers (`vox build`) that call the same runner
- **Supported today:** real activity-body execution, journalled return values, `if` / `match` / `for` / `while` / `?`, durable `workflow_wait` (park + `wake_at_ms`) and `workflow_wait_signal` (park until signal), `with { }` / `workflow.version`, crash-resume via `VoxDbTracker`
- **Out of subset (loud error):** nested workflow calls; mesh / `PopuliActivity`
- **Not offered:** process/VM checkpointing, exactly-once for arbitrary I/O, durable execution for every compiled Vox `async fn`

---

**Related Reference**:

- [Workflow Tutorial](../tutorials/tut-workflow-durability.md) — Build your first durable process.
- [Actors & Workflows](expl-actors-workflows.md) — Current implementation boundary and supported workflow semantics.
- [Vox Language Reference](../reference/ref-syntax.md) — Syntax for workflows and activities.


