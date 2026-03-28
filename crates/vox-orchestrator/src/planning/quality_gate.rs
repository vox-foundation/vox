use crate::orchestrator::OrchestratorError;
use crate::planning::PlanNode;

fn danger_keywords() -> &'static [&'static str] {
    &[
        "rm -rf",
        "delete all",
        "drop database",
        "truncate table",
        "force push",
        "chmod 777",
        "format c:",
        "mkfs",
        "dd if=",
    ]
}

fn vague_phrases() -> &'static [&'static str] {
    &[
        "fix stuff",
        "clean up",
        "misc",
        "various",
        "todo",
        "something",
        "somehow",
        "refactor everything",
        "placeholder",
        "tbd",
        "stub",
    ]
}

fn node_findings(node: &PlanNode) -> Vec<&'static str> {
    let mut out = Vec::new();
    let dl = node.description.to_ascii_lowercase();
    if node.description.trim().len() < 12 {
        out.push("vague_short");
    }
    for phrase in vague_phrases() {
        if dl.contains(phrase) {
            out.push("vague_phrase");
            break;
        }
    }
    for kw in danger_keywords() {
        if dl.contains(kw) {
            out.push("risk_destructive");
            break;
        }
    }
    out
}

pub fn validate_plan_nodes(nodes: &[PlanNode]) -> Result<(), OrchestratorError> {
    for node in nodes {
        let findings = node_findings(node);
        if findings.is_empty() {
            continue;
        }
        if node.execution_policy.force_risky
            && node
                .execution_policy
                .force_risky_reason
                .as_ref()
                .is_some_and(|r| !r.trim().is_empty())
        {
            tracing::warn!(
                node_id = %node.node_id,
                reason_codes = ?findings,
                reason = %node.execution_policy.force_risky_reason.as_deref().unwrap_or(""),
                "plan quality gate override accepted"
            );
            continue;
        }
        if node.execution_policy.force_risky {
            return Err(OrchestratorError::ScopeDenied(format!(
                "Plan node {} requested force_risky but missing force_risky_reason",
                node.node_id
            )));
        }
        return Err(OrchestratorError::ScopeDenied(format!(
            "Plan node {} blocked by quality gate: {:?}. Set execution_policy.force_risky=true with force_risky_reason to override",
            node.node_id, findings
        )));
    }
    Ok(())
}
