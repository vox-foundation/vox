use super::{Orchestrator, OrchestratorError};
use crate::planning::PlanningTaskMeta;
use crate::types::{FileAffinity, TaskId, TaskPriority};

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
        };
        self.event_bus
            .emit(crate::events::AgentEventKind::WorkflowHandoffRequested {
                plan_session_id: plan_session_id.clone(),
                workflow_name: "auto".to_string(),
            });
        let task_id = self
            .submit_task_with_agent_planned(
                format!("[workflow-handoff] {}", goal),
                file_manifest,
                priority,
                None,
                None,
                session_id,
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
