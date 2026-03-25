use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PublicationApprovalMode {
    #[default]
    DigestBound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateReason {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub struct PublishGateInputs {
    pub orchestrator_dry_run: bool,
    pub item_dry_run: bool,
    pub publish_armed_config: bool,
    pub publish_armed_env: bool,
    pub db_present: bool,
    pub dual_approval_met: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishGateDecision {
    pub would_be_live_without_dry_run: bool,
    pub armed: bool,
    pub db_present: bool,
    pub dual_approval_met: bool,
    pub approval_mode: PublicationApprovalMode,
    pub live_publish_allowed: bool,
    pub blocking_reasons: Vec<GateReason>,
}

impl PublishGateDecision {
    #[must_use]
    pub fn has_blockers(&self) -> bool {
        !self.blocking_reasons.is_empty()
    }
}

fn reason(code: &str, message: &str) -> GateReason {
    GateReason {
        code: code.to_string(),
        message: message.to_string(),
    }
}

#[must_use]
pub fn evaluate_publish_gate(inputs: PublishGateInputs) -> PublishGateDecision {
    evaluate_publication_gate(PublicationGateInputs {
        orchestrator_dry_run: inputs.orchestrator_dry_run,
        item_dry_run: inputs.item_dry_run,
        publish_armed_config: inputs.publish_armed_config,
        publish_armed_env: inputs.publish_armed_env,
        db_present: inputs.db_present,
        dual_approval_met: inputs.dual_approval_met,
        approval_mode: PublicationApprovalMode::DigestBound,
    })
}

#[derive(Debug, Clone, Copy)]
pub struct PublicationGateInputs {
    pub orchestrator_dry_run: bool,
    pub item_dry_run: bool,
    pub publish_armed_config: bool,
    pub publish_armed_env: bool,
    pub db_present: bool,
    pub dual_approval_met: bool,
    pub approval_mode: PublicationApprovalMode,
}

#[must_use]
pub fn evaluate_publication_gate(inputs: PublicationGateInputs) -> PublishGateDecision {
    let would_be_live_without_dry_run = !inputs.orchestrator_dry_run && !inputs.item_dry_run;
    let armed = inputs.publish_armed_config || inputs.publish_armed_env;
    let mut blocking_reasons = Vec::new();

    if would_be_live_without_dry_run {
        if !inputs.db_present {
            blocking_reasons.push(reason(
                "missing_db",
                "Live publish requires VoxDb so approvals can be verified and audited.",
            ));
        }
        if !inputs.dual_approval_met {
            blocking_reasons.push(reason(
                "missing_dual_approval",
                "Live publish requires two distinct approvers for this content digest.",
            ));
        }
        if !armed {
            blocking_reasons.push(reason(
                "publish_not_armed",
                "Live publish is not armed. Set [orchestrator.news].publish_armed=true or VOX_NEWS_PUBLISH_ARMED=1.",
            ));
        }
    }

    PublishGateDecision {
        would_be_live_without_dry_run,
        armed,
        db_present: inputs.db_present,
        dual_approval_met: inputs.dual_approval_met,
        approval_mode: inputs.approval_mode,
        live_publish_allowed: would_be_live_without_dry_run && blocking_reasons.is_empty(),
        blocking_reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::{PublishGateInputs, evaluate_publish_gate};

    #[test]
    fn live_publish_allowed_when_all_guards_met() {
        let out = evaluate_publish_gate(PublishGateInputs {
            orchestrator_dry_run: false,
            item_dry_run: false,
            publish_armed_config: true,
            publish_armed_env: false,
            db_present: true,
            dual_approval_met: true,
        });
        assert!(out.live_publish_allowed);
        assert!(!out.has_blockers());
    }

    #[test]
    fn gate_reports_stable_reason_codes() {
        let out = evaluate_publish_gate(PublishGateInputs {
            orchestrator_dry_run: false,
            item_dry_run: false,
            publish_armed_config: false,
            publish_armed_env: false,
            db_present: false,
            dual_approval_met: false,
        });
        let codes: Vec<String> = out
            .blocking_reasons
            .iter()
            .map(|r| r.code.clone())
            .collect();
        assert!(codes.contains(&"missing_db".to_string()));
        assert!(codes.contains(&"missing_dual_approval".to_string()));
        assert!(codes.contains(&"publish_not_armed".to_string()));
    }
}
