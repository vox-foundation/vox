use crate::Orchestrator;
use crate::planning::synthesizer::synthesize_plan_nodes;
use crate::planning::{PlanNode, PlanningTaskMeta};

pub fn trigger_matches(reason: &str, policy_json: Option<&str>) -> bool {
    let Some(policy_json) = policy_json else {
        return false;
    };
    let Ok(policy) = serde_json::from_str::<crate::planning::ExecutionPolicy>(policy_json) else {
        return false;
    };
    let r = reason.to_ascii_lowercase();
    policy.replan_triggers.iter().any(|t| {
        let s = match t {
            crate::planning::ReplanTrigger::CompilerErrorUnresolved => "compile",
            crate::planning::ReplanTrigger::TestFailureNewRegression => "test",
            crate::planning::ReplanTrigger::ScopeDenied => "scope",
            crate::planning::ReplanTrigger::LockConflictPersistent => "lock",
            crate::planning::ReplanTrigger::MissingCapability => "capability",
            crate::planning::ReplanTrigger::ExternalDependencyUnreachable => "dependency",
            crate::planning::ReplanTrigger::Custom(v) => v,
        };
        r.contains(s)
    })
}

pub async fn synthesize_recovery_nodes(reason: &str, failed_desc: &str) -> Vec<PlanNode> {
    let goal = format!(
        "diagnose failure `{}` and fix task `{}`",
        reason, failed_desc
    );
    synthesize_plan_nodes(&goal)
}

pub async fn enqueue_recovery_first_node(
    orch: &Orchestrator,
    meta: &PlanningTaskMeta,
    reason: &str,
    failed_desc: &str,
    session_id: Option<String>,
) -> Result<(), crate::orchestrator::OrchestratorError> {
    let nodes = synthesize_recovery_nodes(reason, failed_desc).await;
    if let Some(first) = nodes.first() {
        let next_version = meta.plan_version.saturating_add(1);
        let _ = crate::planning::executor_bridge::enqueue_plan_node(
            orch,
            first,
            &meta.plan_session_id,
            next_version,
            session_id,
        )
        .await?;
    }
    Ok(())
}
