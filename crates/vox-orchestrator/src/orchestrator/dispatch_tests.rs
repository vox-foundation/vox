use super::*;
use crate::events::EventBus;
use crate::hopper::store::{HopperIntake, InMemoryHopper};
use crate::hopper::types::{IntakeSource, PriorityHint};
use std::sync::Mutex;

#[tokio::test]
async fn maps_intent_and_priority() {
    let hopper = InMemoryHopper::headless();
    let item = hopper
        .submit(
            "fix login bug".into(),
            vec!["crates/auth".into()],
            PriorityHint::Urgent,
            IntakeSource::Developer,
            None,
        )
        .await;
    let task = intake_to_task(&item);
    assert!(task.description.contains("fix login bug"));
}

#[tokio::test]
async fn admit_enqueues_one_task() {
    let bus = Arc::new(EventBus::new(16));
    let rx = bus.subscribe();
    let hopper = Arc::new(InMemoryHopper::new(bus.clone()));
    let enqueued = Arc::new(Mutex::new(Vec::new()));
    let sink = enqueued.clone();

    let handle = tokio::spawn(run_dispatcher(
        rx,
        hopper.clone(),
        move |t| {
            sink.lock().unwrap().push(t);
            Some(crate::types::AgentId(1))
        },
        Some(1),
    ));

    hopper
        .submit(
            "t".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            None,
        )
        .await;
    handle.await.unwrap();
    assert_eq!(enqueued.lock().unwrap().len(), 1);
}

/// T1.1: the dispatcher, given an oplog handle, records `HopperAdmit` +
/// `HopperAssign` for an admitted item, and calls the real
/// `HopperIntake::assign` (previously unreachable from production code).
#[tokio::test]
async fn dispatcher_with_oplog_records_admit_and_assign_and_calls_hopper_assign() {
    use crate::hopper::types::ItemState;

    let bus = Arc::new(EventBus::new(16));
    let rx = bus.subscribe();
    let hopper = Arc::new(InMemoryHopper::new(bus.clone()));
    let oplog = Arc::new(std::sync::RwLock::new(crate::oplog::OpLog::new(100)));

    let handle = tokio::spawn(run_dispatcher_with_oplog(
        rx,
        hopper.clone(),
        move |_t| Some(crate::types::AgentId(7)),
        Some(1),
        Some(oplog.clone()),
        None,
    ));

    let item = hopper
        .submit(
            "t".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            None,
        )
        .await;
    handle.await.unwrap();

    // The real HopperIntake::assign was called: the item transitioned to Assigned.
    let assigned = hopper.assigned().await;
    assert!(
        assigned
            .iter()
            .any(|i| i.item_id == item.item_id && matches!(i.state, ItemState::Assigned { .. })),
        "hopper.assign must have been called by the dispatcher"
    );

    let entries = {
        let log = oplog.read().unwrap();
        log.list(None, 100).into_iter().cloned().collect::<Vec<_>>()
    };
    assert!(
        entries.iter().any(|e| matches!(
            &e.kind,
            crate::oplog::OperationKind::HopperAdmit { item_id } if item_id == &item.item_id.0
        )),
        "expected a HopperAdmit oplog entry"
    );
    assert!(
        entries.iter().any(|e| matches!(
            &e.kind,
            crate::oplog::OperationKind::HopperAssign { item_id, task_id }
                if item_id == &item.item_id.0 && *task_id == stable_hash(&item.item_id.0)
        )),
        "expected a HopperAssign oplog entry"
    );
}

/// Test-only decorator that delegates everything to an inner `InMemoryHopper`
/// except `assign`, which always fails — used to prove the dispatcher does
/// NOT record `HopperAssign` when the real assign call errors (Issue 1).
struct AssignFailingHopper {
    inner: InMemoryHopper,
}

#[async_trait::async_trait]
impl HopperIntake for AssignFailingHopper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    async fn submit_with_resource(
        &self,
        intent: String,
        affinity_hints: Vec<String>,
        priority_hint: PriorityHint,
        source: IntakeSource,
        session_id: Option<String>,
        resource_id: Option<String>,
    ) -> crate::hopper::types::IntakeItem {
        self.inner
            .submit_with_resource(
                intent,
                affinity_hints,
                priority_hint,
                source,
                session_id,
                resource_id,
            )
            .await
    }
    async fn inbox(&self) -> Vec<crate::hopper::types::IntakeItem> {
        self.inner.inbox().await
    }
    async fn assigned(&self) -> Vec<crate::hopper::types::IntakeItem> {
        self.inner.assigned().await
    }
    async fn history(&self) -> Vec<crate::hopper::types::IntakeItem> {
        self.inner.history().await
    }
    async fn reprioritize(
        &self,
        item_id: &crate::events::HopperItemId,
        new_priority: TaskPriority,
        cap: crate::hopper::capability::DeveloperOverride,
    ) -> Result<crate::hopper::types::IntakeItem, crate::hopper::store::HopperError> {
        self.inner.reprioritize(item_id, new_priority, cap).await
    }
    async fn assign(
        &self,
        _item_id: &crate::events::HopperItemId,
        _agent_id: String,
    ) -> Result<crate::hopper::types::IntakeItem, crate::hopper::store::HopperError> {
        Err(crate::hopper::store::HopperError::NotFound(
            "synthetic assign failure (test)".into(),
        ))
    }
    async fn complete(
        &self,
        item_id: &crate::events::HopperItemId,
    ) -> Result<crate::hopper::types::IntakeItem, crate::hopper::store::HopperError> {
        self.inner.complete(item_id).await
    }
    async fn cancel(
        &self,
        item_id: &crate::events::HopperItemId,
    ) -> Result<crate::hopper::types::IntakeItem, crate::hopper::store::HopperError> {
        self.inner.cancel(item_id).await
    }
    async fn replay_admitted(
        &self,
        op: crate::hopper::store::AdmittedReplay,
    ) -> crate::hopper::types::IntakeItem {
        self.inner.replay_admitted(op).await
    }
    async fn replay_overridden(
        &self,
        item_id: &crate::events::HopperItemId,
        new_priority: TaskPriority,
        override_at_unix_ms: u64,
        override_by_node_id: String,
    ) -> Result<crate::hopper::types::IntakeItem, crate::hopper::store::HopperError> {
        self.inner
            .replay_overridden(
                item_id,
                new_priority,
                override_at_unix_ms,
                override_by_node_id,
            )
            .await
    }
    async fn replay_transitioned(
        &self,
        item_id: &crate::events::HopperItemId,
        new_state: crate::hopper::types::ItemState,
    ) -> Result<crate::hopper::types::IntakeItem, crate::hopper::store::HopperError> {
        self.inner.replay_transitioned(item_id, new_state).await
    }
}

/// Issue 1 fix: when the real `hopper.assign()` call fails, the dispatcher
/// must NOT record a `HopperAssign` oplog entry (it would otherwise claim a
/// hopper state transition that never actually happened).
#[tokio::test]
async fn dispatcher_does_not_record_hopper_assign_when_assign_fails() {
    let bus = Arc::new(EventBus::new(16));
    let rx = bus.subscribe();
    let inner = InMemoryHopper::new(bus.clone());
    let hopper: Arc<dyn HopperIntake> = Arc::new(AssignFailingHopper { inner });
    let oplog = Arc::new(std::sync::RwLock::new(crate::oplog::OpLog::new(100)));

    let handle = tokio::spawn(run_dispatcher_with_oplog(
        rx,
        hopper.clone(),
        move |_t| Some(crate::types::AgentId(7)),
        Some(1),
        Some(oplog.clone()),
        None,
    ));

    let item = hopper
        .submit(
            "t".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            None,
        )
        .await;
    handle.await.unwrap();

    let entries = {
        let log = oplog.read().unwrap();
        log.list(None, 100).into_iter().cloned().collect::<Vec<_>>()
    };
    // HopperAdmit should still be recorded (assign failure only gates the
    // HopperAssign write).
    assert!(
        entries.iter().any(|e| matches!(
            &e.kind,
            crate::oplog::OperationKind::HopperAdmit { item_id } if item_id == &item.item_id.0
        )),
        "expected a HopperAdmit oplog entry even though assign later failed"
    );
    assert!(
        !entries.iter().any(|e| matches!(
            &e.kind,
            crate::oplog::OperationKind::HopperAssign { item_id, .. } if item_id == &item.item_id.0
        )),
        "must NOT record HopperAssign when the real hopper.assign() call failed; entries: {entries:?}"
    );
}

#[tokio::test]
async fn override_event_triggers_reprioritize_callback() {
    use crate::events::AgentEventKind;
    let bus = Arc::new(EventBus::new(16));
    let rx = bus.subscribe();
    let reprioritized = Arc::new(Mutex::new(Vec::new()));
    let sink = reprioritized.clone();
    let handle = tokio::spawn(run_cascade(
        rx,
        move |id, _p| sink.lock().unwrap().push(id),
        |_| {},
        Some(1),
    ));
    bus.emit(AgentEventKind::HopperItemOverridden {
        item_id: crate::events::HopperItemId("test_item".to_string()),
        original_priority: TaskPriority::Normal,
        developer_priority: TaskPriority::Urgent,
        delta_seconds_since_admit: 0,
    });
    handle.await.unwrap();
    assert_eq!(reprioritized.lock().unwrap().len(), 1);
    assert_eq!(
        reprioritized.lock().unwrap()[0],
        TaskId(stable_hash("test_item"))
    );
}

#[tokio::test]
async fn cancel_event_triggers_cancel_callback() {
    use crate::events::AgentEventKind;
    let bus = Arc::new(EventBus::new(16));
    let rx = bus.subscribe();
    let cancelled = Arc::new(Mutex::new(Vec::new()));
    let sink = cancelled.clone();
    let handle = tokio::spawn(run_cascade(
        rx,
        |_id, _p| {},
        move |id| sink.lock().unwrap().push(id),
        Some(1),
    ));
    bus.emit(AgentEventKind::HopperItemCancelled {
        item_id: crate::events::HopperItemId("test_item".to_string()),
    });
    handle.await.unwrap();
    assert_eq!(cancelled.lock().unwrap().len(), 1);
    assert_eq!(
        cancelled.lock().unwrap()[0],
        TaskId(stable_hash("test_item"))
    );
}

#[tokio::test]
async fn resource_item_is_dispatched_holding_the_lock() {
    use crate::events::AgentEventKind;
    use crate::locks::ResourceLockManager;
    use crate::orchestrator::safety::ResourceGate;

    let bus = Arc::new(EventBus::new(16));
    let mut rx_events = bus.subscribe();
    let hopper = Arc::new(InMemoryHopper::new(bus.clone()));
    let manager = ResourceLockManager::new();
    let bulletin = crate::bulletin::BulletinBoard::new(10);
    let gate = ResourceGate::new(manager.clone(), bulletin, (*bus).clone(), 60_000);

    let enqueued = Arc::new(Mutex::new(Vec::new()));
    let sink = enqueued.clone();

    let rx = bus.subscribe();
    let handle = tokio::spawn(run_dispatcher_with_oplog(
        rx,
        hopper.clone(),
        move |t| {
            sink.lock().unwrap().push(t);
            Some(crate::types::AgentId(7))
        },
        Some(1),
        None,
        Some(gate),
    ));

    let item = hopper
        .submit_with_resource(
            "task with resource".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s1".into()),
            Some("db://orders/1".into()),
        )
        .await;

    handle.await.unwrap();

    let snap = manager.snapshot();
    let lock_entry = snap.iter().find(|l| l.resource_id == "db://orders/1");
    assert!(lock_entry.is_some(), "expected lock held for db://orders/1");
    assert_eq!(lock_entry.unwrap().holder, crate::types::AgentId(7));

    let tasks = enqueued.lock().unwrap().clone();
    assert_eq!(tasks.len(), 1);

    let mut saw_lock_acquired = false;
    while let Ok(ev) = rx_events.try_recv() {
        if let AgentEventKind::LockAcquired {
            agent_id,
            path,
            session_id,
            task_id,
            ..
        } = ev.kind
        {
            if agent_id == crate::types::AgentId(7)
                && path.to_str() == Some("db://orders/1")
                && session_id.as_deref() == Some("chat-s1")
                && task_id == Some(TaskId(stable_hash(&item.item_id.0)))
            {
                saw_lock_acquired = true;
                break;
            }
        }
    }
    assert!(
        saw_lock_acquired,
        "expected LockAcquired event with matching session and task"
    );
}

#[tokio::test]
async fn second_item_on_a_held_resource_waits_then_runs_after_release() {
    use crate::locks::ResourceLockManager;
    use crate::orchestrator::safety::ResourceGate;

    let bus = Arc::new(EventBus::new(16));
    let hopper = Arc::new(InMemoryHopper::new(bus.clone()));
    let manager = ResourceLockManager::new();
    let bulletin = crate::bulletin::BulletinBoard::new(10);
    let gate = ResourceGate::new(manager.clone(), bulletin, (*bus).clone(), 60_000);

    let enqueued = Arc::new(Mutex::new(Vec::new()));
    let sink = enqueued.clone();

    // Phase 1 — dispatcher (max_events: Some(2)) processes items A and B on the same resource
    let rx = bus.subscribe();
    let handle = tokio::spawn(run_dispatcher_with_oplog(
        rx,
        hopper.clone(),
        move |t| {
            sink.lock().unwrap().push(t);
            Some(crate::types::AgentId(7))
        },
        Some(2),
        None,
        Some(gate.clone()),
    ));

    let _item_a = hopper
        .submit_with_resource(
            "item A".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s1".into()),
            Some("db://orders/1".into()),
        )
        .await;

    let item_b = hopper
        .submit_with_resource(
            "item B".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s2".into()),
            Some("db://orders/1".into()),
        )
        .await;

    handle.await.unwrap();

    // Exactly one enqueue (A), B is still in inbox(), the lock is held.
    assert_eq!(enqueued.lock().unwrap().len(), 1);
    let inbox = hopper.inbox().await;
    assert!(
        inbox.iter().any(|i| i.item_id == item_b.item_id),
        "item B must still be in inbox"
    );
    assert!(manager.is_locked("db://orders/1"));

    // Phase 2 — spawn a fresh dispatcher (max_events: Some(1), subscribed before the release)
    let rx2 = bus.subscribe();
    let sink2 = enqueued.clone();
    let handle2 = tokio::spawn(run_dispatcher_with_oplog(
        rx2,
        hopper.clone(),
        move |t| {
            sink2.lock().unwrap().push(t);
            Some(crate::types::AgentId(7))
        },
        Some(1),
        None,
        Some(gate.clone()),
    ));

    gate.release(crate::types::AgentId(7), "db://orders/1", None, None);

    handle2.await.unwrap();

    // B is enqueued (two captured tasks total) and the lock is held again.
    assert_eq!(enqueued.lock().unwrap().len(), 2);
    assert!(manager.is_locked("db://orders/1"));
}

#[tokio::test]
async fn item_without_resource_takes_no_lock() {
    use crate::locks::ResourceLockManager;
    use crate::orchestrator::safety::ResourceGate;

    let bus = Arc::new(EventBus::new(16));
    let hopper = Arc::new(InMemoryHopper::new(bus.clone()));
    let manager = ResourceLockManager::new();
    let bulletin = crate::bulletin::BulletinBoard::new(10);
    let gate = ResourceGate::new(manager.clone(), bulletin, (*bus).clone(), 60_000);

    let enqueued = Arc::new(Mutex::new(Vec::new()));
    let sink = enqueued.clone();

    let rx = bus.subscribe();
    let handle = tokio::spawn(run_dispatcher_with_oplog(
        rx,
        hopper.clone(),
        move |t| {
            sink.lock().unwrap().push(t);
            Some(crate::types::AgentId(1))
        },
        Some(1),
        None,
        Some(gate),
    ));

    hopper
        .submit(
            "plain task".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            None,
        )
        .await;

    handle.await.unwrap();
    assert_eq!(enqueued.lock().unwrap().len(), 1);
    assert!(manager.snapshot().is_empty());
}
