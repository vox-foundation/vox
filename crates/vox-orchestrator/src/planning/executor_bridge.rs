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
    crate::planning::quality_gate::validate_plan_nodes(std::slice::from_ref(node))?;
    let meta = PlanningTaskMeta {
        plan_session_id: plan_session_id.to_string(),
        plan_node_id: node.node_id.clone(),
        plan_version,
        execution_policy_json: serde_json::to_string(&node.execution_policy).ok(),
        campaign_id: node
            .execution_policy
            .enqueue_hints
            .as_ref()
            .and_then(|h| h.campaign_id.clone()),
        benchmark_tier: node
            .execution_policy
            .enqueue_hints
            .as_ref()
            .and_then(|h| h.benchmark_tier),
        execution_role: node
            .execution_policy
            .enqueue_hints
            .as_ref()
            .and_then(|h| h.execution_role),
    };
    let manifest = if node.execution_policy.file_manifest.is_empty() {
        vec![FileAffinity::read("Cargo.toml")]
    } else {
        node.execution_policy.file_manifest.clone()
    };
    // Planning is task-centric in orchestrator; this bridge just enriches task metadata.
    orch.submit_task_with_agent_planned(
        node.description.clone(),
        manifest,
        Some(TaskPriority::Normal),
        None,
        None,
        session_id,
        node.execution_policy.enqueue_hints.clone(),
        Some(meta),
    )
    .await
}
