use crate::budget::DriftDecision;
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
    pub fn acquire_resource_lock(
        &self,
        agent_id: AgentId,
        resource_id: &str,
        kind: crate::locks::ResourceLockKind,
        ttl_ms: u64,
        session_id: Option<&str>,
        task_id: Option<TaskId>,
    ) -> bool {
        match self
            .resource_locks
            .try_acquire(resource_id, agent_id, kind, ttl_ms)
        {
            Ok(_) => {
                self.bulletin
                    .publish(crate::types::AgentMessage::ResourceLockAcquired {
                        agent_id,
                        resource_id: resource_id.to_string(),
                    });
                // Wire LockAcquired to the event bus so activity_log receives it.
                self.event_bus
                    .emit(crate::events::AgentEventKind::LockAcquired {
                        agent_id,
                        path: std::path::PathBuf::from(resource_id),
                        exclusive: matches!(kind, crate::locks::ResourceLockKind::Exclusive),
                        session_id: session_id.map(str::to_string),
                        task_id,
                    });
                true
            }
            Err(_) => false,
        }
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
        self.resource_locks.release(resource_id, agent_id);
        self.bulletin
            .publish(crate::types::AgentMessage::ResourceLockReleased {
                agent_id,
                resource_id: resource_id.to_string(),
            });
        // Wire LockReleased to the event bus so activity_log receives it.
        self.event_bus
            .emit(crate::events::AgentEventKind::LockReleased {
                agent_id,
                path: std::path::PathBuf::from(resource_id),
                session_id: session_id.map(str::to_string),
                task_id,
            });
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
}
