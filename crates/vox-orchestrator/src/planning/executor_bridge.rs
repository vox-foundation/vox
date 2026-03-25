use crate::Orchestrator;
use crate::planning::{PlanNode, PlanningTaskMeta};
use crate::types::{FileAffinity, TaskId, TaskPriority};

pub async fn enqueue_plan_node(
    orch: &Orchestrator,
    node: &PlanNode,
    plan_session_id: &str,
    plan_version: u32,
    session_id: Option<String>,
) -> Result<TaskId, crate::orchestrator::OrchestratorError> {
    let meta = PlanningTaskMeta {
        plan_session_id: plan_session_id.to_string(),
        plan_node_id: node.node_id.clone(),
        plan_version,
        execution_policy_json: serde_json::to_string(&node.execution_policy).ok(),
    };
    // Planning is task-centric in orchestrator; this bridge just enriches task metadata.
    orch.submit_task_with_agent_planned(
        node.description.clone(),
        vec![FileAffinity::read("Cargo.toml")],
        Some(TaskPriority::Normal),
        None,
        None,
        session_id,
        Some(meta),
    )
    .await
}
