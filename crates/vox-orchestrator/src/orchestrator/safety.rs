use crate::budget::DriftDecision;
use crate::bulletin::BulletinBoard;
use crate::events::EventBus;
use crate::hopper::types::IntakeItem;
use crate::locks::{ResourceLockKind, ResourceLockManager};
use crate::orchestrator::Orchestrator;
use crate::types::{AgentId, TaskId};

impl Orchestrator {
    /// Issues a cryptographic tool receipt for an agent to perform a specific tool call.
    /// This prevents agents from hallucinating tool outputs that were never executed.
    ///
    /// Returns `Err(ToolReceiptError::UnknownTool)` for names absent from
    /// `vox_mcp_registry::TOOL_REGISTRY`.
    pub fn issue_tool_receipt(
        &self,
        agent_id: AgentId,
        tool_name: &str,
        args_json: &str,
    ) -> Result<String, crate::tool_receipt::ToolReceiptError> {
        let ledger = crate::sync_lock::rw_read(&*self.tool_ledger);
        ledger
            .issue_intent(agent_id, tool_name, args_json)
            .map(|r| r.receipt_id)
    }

    /// Records the result of a tool execution in an existing receipt.
    pub fn fulfill_tool_receipt(&self, receipt_id: &str, result_json: &str) -> bool {
        let ledger = crate::sync_lock::rw_read(&*self.tool_ledger);
        ledger.fulfill_intent(receipt_id, result_json).is_ok()
    }

    /// Verifies that a tool receipt was indeed issued by this orchestrator.
    pub fn verify_tool_receipt(&self, receipt_id: &str) -> bool {
        let ledger = crate::sync_lock::rw_read(&*self.tool_ledger);
        ledger.verify(receipt_id).is_ok()
    }

    /// Records an agent's output iteration for semantic drift detection.
    /// If drift is detected (a "doom-loop"), returns a decision to halt or warn.
    pub fn record_agent_iteration(
        &self,
        agent_id: AgentId,
        output_text: &str,
        is_tool_call: bool,
    ) -> DriftDecision {
        let budget = crate::sync_lock::rw_read(&*self.budget_manager);
        budget.record_iteration_output(agent_id, output_text, is_tool_call)
    }

    /// Checks if a task should be routed to a specific model based on privacy requirements.
    pub fn privacy_check_routing(
        &self,
        _task_id: TaskId,
        pii_detected: bool,
    ) -> crate::privacy_router::PrivacyRoutingDecision {
        let router = crate::sync_lock::rw_read(&*self.privacy_router);
        router.route(pii_detected)
    }

    /// Performs a consensus check via a second "Judge" model for a high-stakes task.
    pub async fn judge_consensus(
        &self,
        _task_id: TaskId,
        _input: &str,
        _output: &str,
    ) -> crate::judge_model::JudgeVerdict {
        let judge = crate::sync_lock::rw_read(&*self.judge_model);
        // In a real implementation, this would trigger an actual LLM call.
        // For now, we use the policy-driven default.
        judge.policy.to_verdict()
    }

    /// Acquires a generic resource lock and broadcasts the event to the bulletin board.
    ///
    /// Optional `session_id` and `task_id` correlate the lock acquisition to a chat/workflow
    /// session and task (Phase 5 D-13).
    /// Returns a [`ResourceGate`] sharing this orchestrator's lock manager, bulletin, event bus, and task timeout.
    pub(crate) fn resource_gate(&self) -> ResourceGate {
        let task_ttl_ms = self
            .config
            .read()
            .map(|c| c.task_timeout_ms)
            .unwrap_or(1_800_000);
        ResourceGate::new(
            self.resource_locks.clone(),
            self.bulletin.clone(),
            self.event_bus.clone(),
            task_ttl_ms,
        )
    }

    /// Acquires a generic resource lock and broadcasts the event to the bulletin board.
    ///
    /// Optional `session_id` and `task_id` correlate the lock acquisition to a chat/workflow
    /// session and task (Phase 5 D-13).
    pub fn acquire_resource_lock(
        &self,
        agent_id: AgentId,
        resource_id: &str,
        kind: crate::locks::ResourceLockKind,
        ttl_ms: u64,
        session_id: Option<&str>,
        task_id: Option<TaskId>,
    ) -> bool {
        self.resource_gate()
            .acquire(agent_id, resource_id, kind, ttl_ms, session_id, task_id)
    }

    /// Releases a generic resource lock and broadcasts the event to the bulletin board.
    ///
    /// Optional `session_id` and `task_id` correlate the lock release to a chat/workflow
    /// session and task (Phase 5 D-13).
    pub fn release_resource_lock(
        &self,
        agent_id: AgentId,
        resource_id: &str,
        session_id: Option<&str>,
        task_id: Option<TaskId>,
    ) {
        self.resource_gate()
            .release(agent_id, resource_id, session_id, task_id);
    }

    /// Looks up any resource lock declared by the given task on this agent's queue (Phase 5 D-08).
    ///
    /// Reads the agent's queue (current task if its id matches, else the queued task with that id)
    /// and returns `(resource_id, session_id)` when a resource is declared.
    pub(crate) fn task_resource_lock(
        &self,
        agent_id: AgentId,
        task_id: TaskId,
    ) -> Option<(String, Option<String>)> {
        let agents = crate::sync_lock::rw_read(&*self.agents);
        let queue_lock = agents.get(&agent_id)?;
        let queue = crate::sync_lock::rw_read(&**queue_lock);
        if let Some(current) = queue.current_task() {
            if current.id == task_id {
                return current
                    .resource_id
                    .as_ref()
                    .map(|res| (res.clone(), current.session_id.clone()));
            }
        }
        queue
            .tasks()
            .iter()
            .find(|t| t.id == task_id)
            .and_then(|t| {
                t.resource_id
                    .as_ref()
                    .map(|res| (res.clone(), t.session_id.clone()))
            })
    }

    /// Releases a task's resource lock if one was held (Phase 5 D-08).
    ///
    /// No-op if `held` is `None`. The queue lock must never be held when calling this.
    pub(crate) fn release_task_resource_lock(
        &self,
        agent_id: AgentId,
        task_id: TaskId,
        held: Option<(String, Option<String>)>,
    ) {
        if let Some((resource, session)) = held {
            self.resource_gate()
                .release(agent_id, &resource, session.as_deref(), Some(task_id));
        }
    }
}

/// Shared gate for resource lock acquisition, release, parking, and holding.
///
/// Exists because the hopper dispatcher is spawned in `Orchestrator::new` before
/// `Orchestrator` (`Self`) exists and needs the exact same lock management,
/// bulletin board notification, and event bus publish path.
#[derive(Clone)]
pub struct ResourceGate {
    locks: ResourceLockManager,
    bulletin: BulletinBoard,
    events: EventBus,
    task_ttl_ms: u64,
    /// Items that already announced `LockWaiting`, so the dispatcher's retry loop announces
    /// each wait once. Cleared when the item stops waiting (its resource is free again).
    announced: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl ResourceGate {
    /// Creates a new `ResourceGate` over the shared lock manager, bulletin board, event bus, and task timeout.
    pub fn new(
        locks: ResourceLockManager,
        bulletin: BulletinBoard,
        events: EventBus,
        task_ttl_ms: u64,
    ) -> Self {
        Self {
            locks,
            bulletin,
            events,
            task_ttl_ms,
            announced: Default::default(),
        }
    }

    /// Acquires a generic resource lock and broadcasts the event to the bulletin board and event bus.
    pub fn acquire(
        &self,
        agent_id: AgentId,
        resource_id: &str,
        kind: ResourceLockKind,
        ttl_ms: u64,
        session_id: Option<&str>,
        task_id: Option<TaskId>,
    ) -> bool {
        match self.locks.try_acquire(resource_id, agent_id, kind, ttl_ms) {
            Ok(_) => {
                self.bulletin
                    .publish(crate::types::AgentMessage::ResourceLockAcquired {
                        agent_id,
                        resource_id: resource_id.to_string(),
                    });
                // Wire LockAcquired to the event bus so activity_log receives it.
                self.events
                    .emit(crate::events::AgentEventKind::LockAcquired {
                        agent_id,
                        path: std::path::PathBuf::from(resource_id),
                        exclusive: matches!(kind, ResourceLockKind::Exclusive),
                        session_id: session_id.map(str::to_string),
                        task_id,
                    });
                true
            }
            Err(_) => false,
        }
    }

    /// Releases a generic resource lock and broadcasts the event to the bulletin board and event bus.
    pub fn release(
        &self,
        agent_id: AgentId,
        resource_id: &str,
        session_id: Option<&str>,
        task_id: Option<TaskId>,
    ) {
        self.locks.release(resource_id, agent_id);
        self.bulletin
            .publish(crate::types::AgentMessage::ResourceLockReleased {
                agent_id,
                resource_id: resource_id.to_string(),
            });
        // Wire LockReleased to the event bus so activity_log receives it.
        self.events
            .emit(crate::events::AgentEventKind::LockReleased {
                agent_id,
                path: std::path::PathBuf::from(resource_id),
                session_id: session_id.map(str::to_string),
                task_id,
            });
    }

    /// True when the item declares a resource that `locks.is_locked` reports held (the caller leaves the item in the inbox).
    pub fn park(&self, item: &IntakeItem) -> bool {
        if let Some(ref res) = item.resource_id {
            if self.locks.is_locked(res) {
                let first = self
                    .announced
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(item.item_id.0.clone());
                if first {
                    let task_id =
                        TaskId(crate::orchestrator::dispatch::stable_hash(&item.item_id.0));
                    self.events
                        .emit(crate::events::AgentEventKind::LockWaiting {
                            resource_id: res.clone(),
                            task_id,
                            session_id: item.session_id.clone(),
                        });
                }
                true
            } else {
                self.announced
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .remove(&item.item_id.0);
                false
            }
        } else {
            false
        }
    }

    /// True when the item declares no resource; otherwise acquires an exclusive lock for `self.task_ttl_ms`.
    pub fn hold(&self, agent_id: AgentId, item: &IntakeItem) -> bool {
        if let Some(ref res) = item.resource_id {
            let task_id = TaskId(crate::orchestrator::dispatch::stable_hash(&item.item_id.0));
            self.acquire(
                agent_id,
                res,
                ResourceLockKind::Exclusive,
                self.task_ttl_ms,
                item.session_id.as_deref(),
                Some(task_id),
            )
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::OrchestratorConfig;
    use crate::tool_receipt::ToolReceiptError;

    #[test]
    fn issue_tool_receipt_propagates_unknown_tool_error() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        let res = orch.issue_tool_receipt(AgentId(1), "definitely_not_a_registered_tool", "{}");
        assert!(matches!(res, Err(ToolReceiptError::UnknownTool { .. })));
        assert!(orch.tool_ledger_handle().read().unwrap().is_empty());
    }

    #[test]
    fn issue_tool_receipt_issues_verifiable_receipt_for_registered_tool() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        let id = orch
            .issue_tool_receipt(AgentId(1), "vox_git_status", "{}")
            .unwrap();
        assert!(orch.verify_tool_receipt(&id));
        assert!(orch.fulfill_tool_receipt(&id, "{}"));
        assert!(orch.verify_tool_receipt(&id));
    }

    #[tokio::test]
    async fn lock_events_carry_session_and_task_into_the_activity_row() {
        use crate::activity::project::project;
        use crate::events::AgentEventKind;
        use crate::locks::ResourceLockKind;

        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        let mut rx = orch.event_bus.subscribe();

        let ok = orch.acquire_resource_lock(
            AgentId(3),
            "db://orders/1",
            ResourceLockKind::Exclusive,
            60_000,
            Some("chat-s1"),
            Some(TaskId(77)),
        );
        assert!(ok);

        let event = tokio::time::timeout(vox_config::timeouts::D_1S, rx.recv())
            .await
            .expect("timeout waiting for LockAcquired")
            .expect("recv");
        match &event.kind {
            AgentEventKind::LockAcquired {
                agent_id,
                path,
                exclusive,
                session_id,
                task_id,
            } => {
                assert_eq!(*agent_id, AgentId(3));
                assert_eq!(path.to_str().unwrap(), "db://orders/1");
                assert!(*exclusive);
                assert_eq!(session_id.as_deref(), Some("chat-s1"));
                assert_eq!(*task_id, Some(TaskId(77)));

                let row = project(&event.kind);
                assert_eq!(row.session_id.as_deref(), Some("chat-s1"));
                assert_eq!(row.kind, "LockAcquired");
                assert!(row.detail_json.contains("\"task_id\":77"));
                assert!(row.detail_json.contains("db://orders/1"));
            }
            other => panic!("expected LockAcquired, got {other:?}"),
        }

        orch.release_resource_lock(
            AgentId(3),
            "db://orders/1",
            Some("chat-s1"),
            Some(TaskId(77)),
        );

        let event = tokio::time::timeout(vox_config::timeouts::D_1S, rx.recv())
            .await
            .expect("timeout waiting for LockReleased")
            .expect("recv");
        match &event.kind {
            AgentEventKind::LockReleased {
                agent_id,
                path,
                session_id,
                task_id,
            } => {
                assert_eq!(*agent_id, AgentId(3));
                assert_eq!(path.to_str().unwrap(), "db://orders/1");
                assert_eq!(session_id.as_deref(), Some("chat-s1"));
                assert_eq!(*task_id, Some(TaskId(77)));

                let row = project(&event.kind);
                assert_eq!(row.session_id.as_deref(), Some("chat-s1"));
                assert_eq!(row.kind, "LockReleased");
                assert!(row.detail_json.contains("\"task_id\":77"));
                assert!(row.detail_json.contains("db://orders/1"));
            }
            other => panic!("expected LockReleased, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn lock_events_without_context_serialize_without_the_new_keys() {
        use crate::locks::ResourceLockKind;

        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        let mut rx = orch.event_bus.subscribe();

        let ok = orch.acquire_resource_lock(
            AgentId(3),
            "db://orders/2",
            ResourceLockKind::Shared,
            60_000,
            None,
            None,
        );
        assert!(ok);

        let event = tokio::time::timeout(vox_config::timeouts::D_1S, rx.recv())
            .await
            .expect("timeout waiting for LockAcquired")
            .expect("recv");
        let json = serde_json::to_string(&event.kind).unwrap();
        assert!(!json.contains("session_id"));
        assert!(!json.contains("task_id"));

        orch.release_resource_lock(AgentId(3), "db://orders/2", None, None);

        let event = tokio::time::timeout(vox_config::timeouts::D_1S, rx.recv())
            .await
            .expect("timeout waiting for LockReleased")
            .expect("recv");
        let json = serde_json::to_string(&event.kind).unwrap();
        assert!(!json.contains("session_id"));
        assert!(!json.contains("task_id"));
    }

    #[tokio::test]
    async fn resource_gate_park_and_hold_unit_tests() {
        use crate::hopper::types::{IntakeItem, IntakeSource, PriorityHint};
        use crate::locks::ResourceLockManager;

        let manager = ResourceLockManager::new();
        let bulletin = crate::bulletin::BulletinBoard::new(10);
        let bus = crate::events::EventBus::new(16);
        let gate = ResourceGate::new(manager.clone(), bulletin, bus, 60_000);

        let mut item = IntakeItem::new(
            "intent".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s1".into()),
        );
        item.resource_id = Some("db://orders/1".into());

        // Initially not locked, so park returns false
        assert!(!gate.park(&item));

        // Hold acquires the lock
        assert!(gate.hold(AgentId(1), &item));
        assert!(manager.is_locked("db://orders/1"));

        // Now park returns true for a contender on the same resource
        let mut item2 = IntakeItem::new(
            "intent 2".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s2".into()),
        );
        item2.resource_id = Some("db://orders/1".into());
        assert!(gate.park(&item2));

        // An item without a resource never parks and hold always returns true
        let mut no_res_item = IntakeItem::new(
            "no res".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            None,
        );
        no_res_item.resource_id = None;
        assert!(!gate.park(&no_res_item));
        assert!(gate.hold(AgentId(2), &no_res_item));
    }

    #[tokio::test]
    async fn park_announces_waiting_with_session_and_task() {
        use crate::hopper::types::{IntakeItem, IntakeSource, PriorityHint};
        use crate::locks::ResourceLockManager;

        let manager = ResourceLockManager::new();
        let bulletin = crate::bulletin::BulletinBoard::new(10);
        let bus = crate::events::EventBus::new(16);
        let gate = ResourceGate::new(manager.clone(), bulletin, bus.clone(), 60_000);
        let mut rx = bus.subscribe();

        let mut item = IntakeItem::new(
            "intent".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s2".into()),
        );
        item.resource_id = Some("db://orders/1".into());

        // With resource free: park returns false and emits nothing
        assert!(!gate.park(&item));
        assert!(rx.try_recv().is_err());

        // Hold the lock
        assert!(gate.hold(AgentId(1), &item));
        // Discard the LockAcquired event from hold
        let _ = rx.recv().await;

        // Contender item with session chat-s2 on db://orders/1
        let mut item2 = IntakeItem::new(
            "intent 2".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            Some("chat-s2".into()),
        );
        item2.resource_id = Some("db://orders/1".into());
        let expected_task_id = TaskId(crate::orchestrator::dispatch::stable_hash(&item2.item_id.0));

        // When locked: park returns true and emits LockWaiting
        assert!(gate.park(&item2));

        let event = tokio::time::timeout(vox_config::timeouts::D_1S, rx.recv())
            .await
            .expect("timeout waiting for LockWaiting")
            .expect("recv");
        match &event.kind {
            crate::events::AgentEventKind::LockWaiting {
                resource_id,
                task_id,
                session_id,
            } => {
                assert_eq!(resource_id, "db://orders/1");
                assert_eq!(*task_id, expected_task_id);
                assert_eq!(session_id.as_deref(), Some("chat-s2"));
            }
            other => panic!("expected LockWaiting, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn park_announces_each_wait_once() {
        use crate::hopper::types::{IntakeItem, IntakeSource, PriorityHint};
        use crate::locks::ResourceLockManager;

        let manager = ResourceLockManager::new();
        let bus = crate::events::EventBus::new(16);
        let gate = ResourceGate::new(
            manager.clone(),
            crate::bulletin::BulletinBoard::new(10),
            bus.clone(),
            60_000,
        );
        let mut rx = bus.subscribe();
        let mk = |intent: &str| {
            let mut i = IntakeItem::new(
                intent.into(),
                vec![],
                PriorityHint::Normal,
                IntakeSource::Developer,
                Some("chat-s3".into()),
            );
            i.resource_id = Some("db://orders/9".into());
            i
        };
        let holder = mk("holder");
        let waiter = mk("waiter");
        let waiting_count =
            |rx: &mut tokio::sync::broadcast::Receiver<crate::events::AgentEvent>| {
                let mut n = 0;
                while let Ok(ev) = rx.try_recv() {
                    if matches!(ev.kind, crate::events::AgentEventKind::LockWaiting { .. }) {
                        n += 1;
                    }
                }
                n
            };

        assert!(gate.hold(AgentId(1), &holder));
        assert!(gate.park(&waiter));
        assert!(gate.park(&waiter));
        assert!(gate.park(&waiter));
        assert_eq!(waiting_count(&mut rx), 1, "retries must not re-announce");

        // The wait ends (lock freed), then a later, separate wait announces again.
        gate.release(AgentId(1), "db://orders/9", None, None);
        assert!(!gate.park(&waiter));
        assert!(gate.hold(AgentId(2), &holder));
        assert!(gate.park(&waiter));
        assert_eq!(waiting_count(&mut rx), 1, "a new wait announces once more");
    }

    #[test]
    fn task_resource_lock_reads_the_queued_or_current_task() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        let agent_id = orch.spawn_agent("agent-1").expect("spawn_agent");

        let mut task1 = crate::types::AgentTask::new(
            TaskId(101),
            "task with resource",
            crate::types::TaskPriority::Normal,
            vec![],
        );
        task1.resource_id = Some("db://orders/10".to_string());
        task1.session_id = Some("sess-1".to_string());

        let mut task2 = crate::types::AgentTask::new(
            TaskId(102),
            "task queued",
            crate::types::TaskPriority::Normal,
            vec![],
        );
        task2.resource_id = Some("db://orders/11".to_string());
        task2.session_id = Some("sess-2".to_string());

        // Enqueue task1 and task2
        {
            let queue_arc = orch.agent_queue(agent_id).expect("queue");
            let mut q = queue_arc.write().unwrap();
            q.enqueue(task1);
            q.enqueue(task2);
        }

        // Before dequeue, task1 is in queued tasks
        assert_eq!(
            orch.task_resource_lock(agent_id, TaskId(101)),
            Some(("db://orders/10".to_string(), Some("sess-1".to_string())))
        );
        assert_eq!(
            orch.task_resource_lock(agent_id, TaskId(102)),
            Some(("db://orders/11".to_string(), Some("sess-2".to_string())))
        );
        assert_eq!(orch.task_resource_lock(agent_id, TaskId(999)), None);

        // Dequeue task1 -> now task1 is current_task (in_progress)
        {
            let queue_arc = orch.agent_queue(agent_id).expect("queue");
            let mut q = queue_arc.write().unwrap();
            let deq = q.dequeue();
            assert_eq!(deq.map(|t| t.id), Some(TaskId(101)));
        }

        // Now task1 is in_progress, task2 is still queued
        assert_eq!(
            orch.task_resource_lock(agent_id, TaskId(101)),
            Some(("db://orders/10".to_string(), Some("sess-1".to_string())))
        );
        assert_eq!(
            orch.task_resource_lock(agent_id, TaskId(102)),
            Some(("db://orders/11".to_string(), Some("sess-2".to_string())))
        );
    }

    #[test]
    fn release_task_resource_lock_is_a_no_op_without_a_resource() {
        let orch = Orchestrator::new(OrchestratorConfig::for_testing());
        let mut rx = orch.event_bus.subscribe();

        orch.release_task_resource_lock(AgentId(1), TaskId(101), None);

        assert!(rx.try_recv().is_err());
    }
}
