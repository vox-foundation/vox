use super::{Orchestrator, OrchestratorError};
use crate::planning::PlanningTaskMeta;
use crate::types::{FileAffinity, TaskEnqueueHints, TaskId, TaskPriority};

impl Orchestrator {
    /// Bridge planner workflow handoff decisions back into orchestrator execution.
    ///
    /// Current implementation schedules a workflow-tagged task and records handoff telemetry.
    pub async fn submit_workflow_handoff_goal(
        &self,
        goal: String,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        session_id: Option<String>,
        enqueue_hints: Option<TaskEnqueueHints>,
    ) -> Result<TaskId, OrchestratorError> {
        let plan_session_id = format!("wf-{}", uuid::Uuid::new_v4());
        let meta = PlanningTaskMeta {
            plan_session_id: plan_session_id.clone(),
            plan_node_id: "workflow_handoff".to_string(),
            plan_version: 1,
            execution_policy_json: Some(
                serde_json::json!({
                    "workflow_handoff": true,
                    "goal": goal,
                })
                .to_string(),
            ),
            campaign_id: enqueue_hints.as_ref().and_then(|h| h.campaign_id.clone()),
            benchmark_tier: enqueue_hints.as_ref().and_then(|h| h.benchmark_tier),
            execution_role: enqueue_hints.as_ref().and_then(|h| h.execution_role),
        };
        self.event_bus
            .emit(crate::events::AgentEventKind::WorkflowHandoffRequested {
                plan_session_id: plan_session_id.clone(),
                workflow_name: "auto".to_string(),
            });
        if crate::lineage::orchestration_lineage_persist_enabled() {
            if let Some(db) = self.db() {
                let repo = crate::lineage::repository_id();
                let mut payload = serde_json::json!({
                    "plan_session_id": plan_session_id,
                    "goal_preview": goal.chars().take(240).collect::<String>(),
                });
                if let Some(cid) = crate::lineage::orchestration_campaign_id() {
                    payload["campaign_id"] = serde_json::Value::String(cid);
                }
                let payload_str = payload.to_string();
                let _ = db
                    .append_orchestration_lineage_event(
                        &repo,
                        "workflow_handoff_started",
                        0_i64,
                        None,
                        session_id.as_deref(),
                        None,
                        Some(plan_session_id.as_str()),
                        Some("workflow_handoff"),
                        Some(payload_str.as_str()),
                    )
                    .await;
            }
        }
        let task_id = self
            .submit_task_with_agent_planned(
                format!("[workflow-handoff] {}", goal),
                file_manifest,
                priority,
                None,
                None,
                session_id,
                enqueue_hints,
                Some(meta),
            )
            .await?;
        self.event_bus
            .emit(crate::events::AgentEventKind::WorkflowHandoffCompleted {
                plan_session_id,
                task_id: task_id.0,
            });
        Ok(task_id)
    }
}
