//! Hopper → agent-queue dispatch: pure mapping + the dispatcher loop.

use crate::hopper::types::IntakeItem;
use crate::types::{AgentId, AgentTask, TaskId, TaskPriority};
use std::sync::Arc;

/// Stable hash function to map string IDs deterministically to u64 TaskIds.
pub fn stable_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for c in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(c as u64);
    }
    hash
}

/// Convert an admitted hopper item into the task an `AgentQueue` enqueues.
/// Pure + deterministic so it is unit-testable in isolation.
pub fn intake_to_task(item: &IntakeItem) -> AgentTask {
    let task_id = TaskId(stable_hash(&item.item_id.0));
    let mut task = AgentTask::new(
        task_id,
        item.intent.clone(),
        item.classified_priority,
        vec![], // file_manifest
    );
    task.session_id = item.session_id.clone();
    task.resource_id = item.resource_id.clone();
    task
}

/// Runs the admit→enqueue loop: every HopperItemAdmitted becomes an enqueued task.
/// Returns after `max_events` for test determinism (None = run forever).
///
/// T1.1 (hopper wiring, harness reliability spec Phase 1): this is the real
/// production admit→dispatch site (spawned from `Orchestrator::new`,
/// `orchestrator/core/mod.rs`). `enqueue` now returns the `AgentId` the task
/// was actually routed to (or `None` if no agent was available), which lets
/// this loop call the real `HopperIntake::assign` — previously unreachable
/// from any production call site — and durably record `HopperAssign`
/// alongside it. `oplog` is optional so existing unit tests that construct
/// `run_dispatcher` without a durable log keep working unchanged.
pub async fn run_dispatcher(
    rx: tokio::sync::broadcast::Receiver<crate::events::AgentEvent>,
    hopper: Arc<dyn crate::hopper::store::HopperIntake>,
    enqueue: impl Fn(AgentTask) -> Option<AgentId> + Send + 'static,
    max_events: Option<usize>,
) {
    run_dispatcher_with_oplog(rx, hopper, enqueue, max_events, None, None).await
}

/// Same as [`run_dispatcher`], additionally recording `HopperAssign` (and, on
/// first sight of an item, `HopperAdmit`) to the durable op-log when `oplog`
/// is `Some`. Split out so production wiring (which has an oplog handle) and
/// existing tests (which don't) share one implementation.
///
/// `gate: None` keeps the pre-Phase-5 behaviour.
pub async fn run_dispatcher_with_oplog(
    mut rx: tokio::sync::broadcast::Receiver<crate::events::AgentEvent>,
    hopper: Arc<dyn crate::hopper::store::HopperIntake>,
    enqueue: impl Fn(AgentTask) -> Option<AgentId> + Send + 'static,
    max_events: Option<usize>,
    oplog: Option<Arc<std::sync::RwLock<crate::oplog::OpLog>>>,
    gate: Option<crate::orchestrator::safety::ResourceGate>,
) {
    let mut seen = 0usize;
    while let Ok(ev) = rx.recv().await {
        match ev.kind {
            crate::events::AgentEventKind::HopperItemAdmitted { item_id, .. } => {
                // Find the item in the hopper inbox/assigned list
                let mut found_item = None;
                for item in hopper.inbox().await {
                    if item.item_id == item_id {
                        found_item = Some(item);
                        break;
                    }
                }
                if found_item.is_none() {
                    for item in hopper.assigned().await {
                        if item.item_id == item_id {
                            found_item = Some(item);
                            break;
                        }
                    }
                }

                if let Some(item) = found_item {
                    if let Some(log) = oplog.as_ref() {
                        record_hopper_op(
                            log,
                            crate::oplog::OperationKind::HopperAdmit {
                                item_id: item.item_id.0.clone(),
                            },
                            format!("Hopper item {} admitted", item.item_id.0),
                        );
                    }

                    dispatch_item(item, &hopper, &enqueue, oplog.as_ref(), gate.as_ref()).await;
                }

                seen += 1;
                if Some(seen) == max_events {
                    break;
                }
            }
            crate::events::AgentEventKind::LockReleased { path, .. } => {
                if let Some(ref g) = gate {
                    let path_str = path.to_string_lossy();
                    for item in hopper.inbox().await {
                        if item.resource_id.as_deref() == Some(&*path_str) {
                            if dispatch_item(item, &hopper, &enqueue, oplog.as_ref(), Some(g)).await
                            {
                                break;
                            }
                        }
                    }
                    seen += 1;
                    if Some(seen) == max_events {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
}

// ponytail: check-then-acquire (park then hold) is race-free because this dispatcher
// loop is the only resource-lock acquirer and handles events sequentially. Move the acquire
// inside the enqueue closure if a second acquirer is ever added.
async fn dispatch_item(
    item: IntakeItem,
    hopper: &Arc<dyn crate::hopper::store::HopperIntake>,
    enqueue: &(impl Fn(AgentTask) -> Option<AgentId> + Send + 'static),
    oplog: Option<&Arc<std::sync::RwLock<crate::oplog::OpLog>>>,
    gate: Option<&crate::orchestrator::safety::ResourceGate>,
) -> bool {
    if gate.is_some_and(|g| g.park(&item)) {
        return false;
    }

    let task = intake_to_task(&item);
    if let Some(agent_id) = enqueue(task) {
        if let Some(g) = gate {
            if !g.hold(agent_id, &item) {
                tracing::error!(
                    resource = ?item.resource_id,
                    %agent_id,
                    "gate.hold failed to acquire resource lock"
                );
            }
        }

        // Real production caller of `HopperIntake::assign` (previously
        // unreachable outside tests — see hopper/store.rs). Only record
        // the `HopperAssign` oplog entry when the real assign actually
        // succeeded — mirrors the `HopperComplete` gating in
        // `task_dispatch/complete/success/mod.rs` (`hopper.complete(..)
        // .await.is_ok()`), so the oplog never claims a hopper state
        // transition that didn't really happen.
        if hopper
            .assign(&item.item_id, agent_id.to_string())
            .await
            .is_ok()
        {
            if let Some(log) = oplog {
                record_hopper_op(
                    log,
                    crate::oplog::OperationKind::HopperAssign {
                        item_id: item.item_id.0.clone(),
                        task_id: stable_hash(&item.item_id.0),
                    },
                    format!(
                        "Hopper item {} assigned to agent {}",
                        item.item_id.0, agent_id
                    ),
                );
            }
        } else {
            tracing::warn!(
                item_id = %item.item_id.0,
                %agent_id,
                "hopper.assign failed; not recording HopperAssign oplog entry"
            );
        }
        true
    } else {
        false
    }
}

/// Synchronous op-log write for the dispatcher loop: acquires the std
/// `RwLockWriteGuard`, records, and drops the guard — never held across an
/// `.await` (mirrors the non-`_persisted` `OpLog::record` used elsewhere for
/// in-process-only recording; write-through-to-db uses `record_persisted`,
/// which is async — the dispatcher loop stays sync-only here to avoid
/// threading agent_id/db-persist plumbing into a hot broadcast loop).
fn record_hopper_op(
    oplog: &Arc<std::sync::RwLock<crate::oplog::OpLog>>,
    kind: crate::oplog::OperationKind,
    description: String,
) {
    let mut log = match oplog.write() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    log.record(
        crate::types::AgentId(0),
        kind,
        description,
        None,
        None,
        None,
        None,
        None,
        None,
    );
}

/// Runs the priority/cancel cascade loop:
/// - HopperItemOverridden: updates the task's priority on agent queue.
/// - HopperItemCancelled: cancels the task on agent queue.
pub async fn run_cascade(
    mut rx: tokio::sync::broadcast::Receiver<crate::events::AgentEvent>,
    on_reprioritize: impl Fn(TaskId, TaskPriority) + Send + 'static,
    on_cancel: impl Fn(TaskId) + Send + 'static,
    max_events: Option<usize>,
) {
    let mut seen = 0usize;
    while let Ok(ev) = rx.recv().await {
        match ev.kind {
            crate::events::AgentEventKind::HopperItemOverridden {
                item_id,
                developer_priority,
                ..
            } => {
                let task_id = TaskId(stable_hash(&item_id.0));
                on_reprioritize(task_id, developer_priority);
                seen += 1;
            }
            crate::events::AgentEventKind::HopperItemCancelled { item_id } => {
                let task_id = TaskId(stable_hash(&item_id.0));
                on_cancel(task_id);
                seen += 1;
            }
            _ => {}
        }
        if Some(seen) == max_events {
            break;
        }
    }
}

#[cfg(test)]
#[path = "dispatch_tests.rs"]
mod tests;
