use crate::orchestrator::OrchestratorError;
use crate::planning::PlanNode;
use crate::planning::plan_adequacy::orchestrator_node_text_findings;
use crate::types::FileAffinity;

fn push_unique(out: &mut Vec<&'static str>, code: &'static str) {
    if !out.contains(&code) {
        out.push(code);
    }
}

/// Placeholder paths in `file_manifest` (aligned with MCP task `files` / plan adequacy cues).
fn file_manifest_placeholder_findings(manifest: &[FileAffinity]) -> Vec<&'static str> {
    let mut out = Vec::new();
    for fa in manifest {
        let lossy = fa.path.to_string_lossy();
        if lossy.trim().is_empty() {
            push_unique(&mut out, "manifest_empty_path");
            continue;
        }
        let stem_is_tbd = lossy.trim().eq_ignore_ascii_case("tbd");
        let name_tbd = fa
            .path
            .file_name()
            .is_some_and(|n| n.eq_ignore_ascii_case("tbd"));
        if stem_is_tbd || name_tbd {
            push_unique(&mut out, "tbd_placeholder");
        }
    }
    out
}

fn node_findings(node: &PlanNode) -> Vec<&'static str> {
    let mut v = orchestrator_node_text_findings(&node.description);
    for code in file_manifest_placeholder_findings(&node.execution_policy.file_manifest) {
        push_unique(&mut v, code);
    }
    v
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::{ExecutionPolicy, PlanStatus};
    use crate::types::FileAffinity;

    fn sample_node(desc: &str, manifest: Vec<FileAffinity>) -> PlanNode {
        PlanNode {
            node_id: "n1".into(),
            description: desc.into(),
            depends_on: vec![],
            status: PlanStatus::Pending,
            execution_policy: ExecutionPolicy {
                file_manifest: manifest,
                ..Default::default()
            },
            workflow_invocation: None,
        }
    }

    #[test]
    fn manifest_tbd_path_blocked() {
        let n = sample_node(
            "Implement the requested feature with tests and verification steps included.",
            vec![FileAffinity::write("crates/demo/tbd")],
        );
        assert!(validate_plan_nodes(std::slice::from_ref(&n)).is_err());
    }

    #[test]
    fn manifest_empty_path_blocked() {
        let n = sample_node(
            "Implement the requested feature with tests and verification steps included.",
            vec![FileAffinity::read("")],
        );
        assert!(validate_plan_nodes(std::slice::from_ref(&n)).is_err());
    }

    #[test]
    fn manifest_tbd_allowed_with_force_risky_reason() {
        let mut n = sample_node(
            "Implement the requested feature with tests and verification steps included.",
            vec![FileAffinity::write("tbd")],
        );
        n.execution_policy.force_risky = true;
        n.execution_policy.force_risky_reason =
            Some("Reviewer approved TBD path for spike.".into());
        assert!(validate_plan_nodes(std::slice::from_ref(&n)).is_ok());
    }
}
