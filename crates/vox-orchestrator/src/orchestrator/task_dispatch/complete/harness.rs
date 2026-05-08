use crate::ApprovalTier;
use crate::types::{AgentTask, CompletionAttestation};

pub(crate) fn completion_attestation_satisfies_tier(
    tier: ApprovalTier,
    attestation: Option<&CompletionAttestation>,
) -> bool {
    match tier {
        ApprovalTier::AutoApprove => true,
        ApprovalTier::Confirm => attestation
            .map(|a| !a.checks_passed.is_empty() || a.declared_non_placeholder)
            .unwrap_or(false),
        ApprovalTier::Review => attestation
            .map(|a| {
                a.checks_passed.iter().any(|c| {
                    let normalized = c.trim().to_ascii_lowercase();
                    normalized == "human_review_approved"
                        || normalized == "peer_review_approved"
                        || normalized == "approval_reviewed"
                })
            })
            .unwrap_or(false),
        ApprovalTier::Blocked => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HarnessCompletionGuardMode {
    Off,
    Shadow,
    Enforce,
}

pub(crate) fn harness_completion_guard_mode() -> HarnessCompletionGuardMode {
    let secret =
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxOrchestratorHarnessCompletionGuard);
    let raw = secret.expose().unwrap_or("off");
    match raw.trim().to_ascii_lowercase().as_str() {
        "shadow" | "warn" => HarnessCompletionGuardMode::Shadow,
        "enforce" | "strict" => HarnessCompletionGuardMode::Enforce,
        _ => HarnessCompletionGuardMode::Off,
    }
}

pub(crate) fn normalize_path_for_match(raw: &str) -> String {
    raw.trim().replace('\\', "/")
}

pub(crate) fn harness_completion_issues(
    task: &AgentTask,
    completion_attestation: Option<&CompletionAttestation>,
) -> Result<(Option<String>, Vec<String>), String> {
    let Some(raw_harness) = task
        .harness_spec_json
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Ok((None, Vec::new()));
    };
    let harness = serde_json::from_str::<crate::AgentHarnessSpec>(raw_harness)
        .map_err(|e| format!("task harness_spec_json parse failed: {e}"))?;
    let mut issues = Vec::new();
    if harness.contracts.requires_artifact_backed_completion {
        let provided_artifacts = completion_attestation
            .map(|a| {
                a.artifact_paths
                    .iter()
                    .map(|p| normalize_path_for_match(&p.to_string_lossy()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for output in harness
            .contracts
            .required_outputs
            .iter()
            .filter(|x| x.required)
        {
            let Some(path_hint) = output.path_hint.as_deref() else {
                continue;
            };
            let expected = normalize_path_for_match(path_hint);
            let present = provided_artifacts
                .iter()
                .any(|p| p == &expected || p.ends_with(&expected));
            if !present {
                issues.push(format!(
                    "missing required harness artifact path {:?} for output {:?}",
                    path_hint, output.artifact_id
                ));
            }
        }
    }
    let evidence_required = harness
        .contracts
        .completion_gates
        .iter()
        .any(|g| g.evidence_required.unwrap_or(false));
    if evidence_required {
        let evidence_present = completion_attestation
            .map(|a| !a.evidence_citations.is_empty())
            .unwrap_or(false);
        if !evidence_present {
            issues.push(
                "harness completion gate requires evidence_citations[] in completion attestation"
                    .to_string(),
            );
        }
    }
    Ok((Some(harness.harness_id), issues))
}

#[cfg(test)]
mod harness_completion_tests {
    use super::harness_completion_issues;
    use crate::types::{AgentTask, CompletionAttestation, FileAffinity, TaskId, TaskPriority};

    fn task_with_harness(harness_json: &str) -> AgentTask {
        let mut task = AgentTask::new(
            TaskId(1),
            "harness completion test",
            TaskPriority::Normal,
            vec![FileAffinity::write("src/lib.rs")],
        );
        task.harness_spec_json = Some(harness_json.to_string());
        task
    }

    #[test]
    fn harness_completion_issues_empty_when_not_required() {
        let harness = crate::AgentHarnessSpec::minimal_contract_first(
            "repo-test",
            "no required outputs",
            None,
            None,
            &[],
        );
        let mut harness = harness;
        harness.contracts.requires_artifact_backed_completion = false;
        harness.contracts.required_outputs.clear();
        harness.contracts.completion_gates.clear();
        let raw = serde_json::to_string(&harness).expect("serialize harness");
        let task = task_with_harness(&raw);
        let (_hid, issues) = harness_completion_issues(&task, None).expect("evaluate");
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn harness_completion_issues_reports_missing_required_artifact() {
        let harness = crate::AgentHarnessSpec::minimal_contract_first(
            "repo-test",
            "artifact required",
            None,
            None,
            &["artifacts/out.md".to_string()],
        );
        let raw = serde_json::to_string(&harness).expect("serialize harness");
        let task = task_with_harness(&raw);
        let att = CompletionAttestation::default();
        let (_hid, issues) = harness_completion_issues(&task, Some(&att)).expect("evaluate");
        assert!(
            issues
                .iter()
                .any(|x| x.contains("missing required harness artifact")),
            "{issues:?}"
        );
    }
}
