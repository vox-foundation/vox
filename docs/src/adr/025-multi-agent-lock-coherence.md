---
title: "ADR-025: Multi-Agent Lock Coherence and Lease Propagation"
description: "Architecture Decision Record for multi-agent lock coherence and lease propagation in the Vox ecosystem."
category: "Architecture Decisions (ADRs)"
status: "current"
---
# ADR 025: Multi-Agent Lock Coherence and Lease Propagation

## Status
Accepted (2026-09-29) — proposed 2026-04-23, ratified after Phase 5 implemented it.

## Context
As Vox moves toward multi-agent environments (multiple agents working on the same task or in the same workspace), resource contention (not just file-level) becomes a risk. We need a way to lock generic resources (URIs, DB rows) and propagate these locks across the bulletin board.

## Decision
We extend the `locks` subsystem with a `ResourceLockManager`:
1. **Generic Resource IDs**: Locks can be held on arbitrary strings (URIs).
2. **Lease-based Expiration**: All locks have a mandatory TTL to prevent deadlocks from crashed agents.
3. **Bulletin Synchronization**: Lock acquisition and release events are broadcast as `AgentMessage` variants to ensure all agents are aware of the coherence state.

## Consequences
- Agents can safely coordinate on non-file resources.
- The system is resilient to agent failures via lease expiration.
- Real-time visualization of resource contention is possible via the bulletin board.

## Implementation (Phase 5)

- **Lock manager**: `ResourceLockManager` (`crates/vox-orchestrator-queue/src/locks/resource.rs`) holds leases with an expiry; `try_acquire`, `release`, and `is_locked` sweep every expired lease lazily (D-06), so a crashed holder cannot wedge a resource. There is no background sweeper.
- **Resource on the work item** (D-05): a hopper intake item and an `AgentTask` carry an optional `resource_id` (`crates/vox-orchestrator/src/hopper/types.rs`, validated by `validate_resource_id`); it is persisted in the SQLite hopper (schema 94) and accepted by `POST /api/v2/hopper/submit`.
- **Gate at dispatch** (D-06): `ResourceGate` (`crates/vox-orchestrator/src/orchestrator/safety.rs`) holds the lock for the task's lifetime, using the task timeout as the lease. `dispatch_item` (`crates/vox-orchestrator/src/orchestrator/dispatch.rs`) parks an item whose resource is held and retries it when a `LockReleased` event arrives.
- **Release** (D-06): completing, failing, or cancelling the holder (directly or through the hopper) releases the lock, and the waiter is dispatched (`crates/vox-orchestrator/tests/resource_lock_lifecycle.rs`).
- **Events instead of new `AgentMessage` variants**: acquire, release, and waiting are `LockAcquired`, `LockReleased`, and `LockWaiting` events (`crates/vox-orchestrator/src/events.rs`), carrying `session_id` and `task_id`, and are visible in the activity feed.
- **Chat surface** (D-10, D-11): `ChatExecutionRail.tsx` shows a holding or "waiting on <resource>" chip, fed by `lockStatesFromActivity` in `crates/vox-gui/ui/src/hooks/useChatExecutionData.ts`.

### Known limitations

- Single-process scope (D-07): the lock manager is in memory; two orchestrator processes do not see each other's locks.
- A waiter behind a lease that expires without a release event waits for the next release of that resource; a background retry is deferred.
- `LockWaiting` is emitted again on every park retry, so a long wait produces repeated events.
- The chat `vox_submit_task` path does not go through the hopper, so resource-locked work arrives via `POST /api/v2/hopper/submit`.
- The lock is enqueued before it is held, and the check-then-acquire is not atomic across concurrent dispatchers.
