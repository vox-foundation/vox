use crate::planning::policy::default_policy_for_goal;
use crate::planning::{PlanNode, PlanStatus};

pub fn synthesize_plan_nodes(goal: &str) -> Vec<PlanNode> {
    let parts: Vec<String> = goal
        .split(" and ")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    if parts.is_empty() {
        return vec![PlanNode {
            node_id: "n1".to_string(),
            description: goal.to_string(),
            depends_on: vec![],
            status: PlanStatus::Pending,
            execution_policy: default_policy_for_goal(goal),
            workflow_invocation: None,
        }];
    }

    let mut nodes = Vec::with_capacity(parts.len());
    for (idx, part) in parts.iter().enumerate() {
        let node_id = format!("n{}", idx + 1);
        let depends_on = if idx == 0 {
            vec![]
        } else {
            vec![format!("n{}", idx)]
        };
        nodes.push(PlanNode {
            node_id,
            description: part.clone(),
            depends_on,
            status: PlanStatus::Pending,
            execution_policy: default_policy_for_goal(part),
            workflow_invocation: None,
        });
    }
    nodes
}
