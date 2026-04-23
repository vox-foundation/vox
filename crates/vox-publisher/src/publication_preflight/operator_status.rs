use super::*;
/// `contracts/scientia/operator-status-surface.v1.schema.json` compatible summary.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperatorStatusSurfaceV1 {
    pub publication_id: String,
    pub profile: String,
    pub snapshot_summary: OperatorStatusSnapshotSummary,
    pub next_actions: Vec<OperatorStatusAction>,
    pub route_readiness: Vec<OperatorStatusRouteReadiness>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperatorStatusSnapshotSummary {
    pub hard_gate_failures: u32,
    pub soft_gate_failures: u32,
    pub diagnostic_count: u32,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperatorStatusAction {
    pub priority: u16,
    pub action: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OperatorStatusRouteReadiness {
    pub route: String,
    pub ready: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_required: Vec<String>,
}
pub(crate) struct OperatorCredentialPresence {
    pub twitter: bool,
    pub github: bool,
    pub open_collective: bool,
    pub reddit: bool,
    pub youtube: bool,
}
/// Derive a stable operator status surface from preflight output for CLI/MCP parity.
pub fn operator_status_surface_v1(
    publication_id: &str,
    profile: PreflightProfile,
    report: &PreflightReport,
) -> OperatorStatusSurfaceV1 {
    let mut hard_gate_failures = 0_u32;
    let mut soft_gate_failures = 0_u32;
    for f in &report.findings {
        match f.severity {
            PreflightSeverity::Error => hard_gate_failures += 1,
            PreflightSeverity::Warning => soft_gate_failures += 1,
        }
    }
    let next_actions = report
        .next_actions
        .iter()
        .enumerate()
        .map(|(idx, a)| OperatorStatusAction {
            priority: (idx + 1) as u16,
            action: a.summary.clone(),
        })
        .collect::<Vec<_>>();
    let route_readiness = report
        .destination_readiness
        .iter()
        .map(|d| {
            let missing_required = if d.ready {
                Vec::new()
            } else if d.remediation.trim().is_empty() {
                vec!["manual_operator_review".to_string()]
            } else {
                vec![d.remediation.clone()]
            };
            OperatorStatusRouteReadiness {
                route: d.destination.to_string(),
                ready: d.ready,
                missing_required,
            }
        })
        .collect::<Vec<_>>();
    OperatorStatusSurfaceV1 {
        publication_id: publication_id.to_string(),
        profile: profile_label(profile).to_string(),
        snapshot_summary: OperatorStatusSnapshotSummary {
            hard_gate_failures,
            soft_gate_failures,
            diagnostic_count: report.findings.len() as u32,
        },
        next_actions,
        route_readiness,
    }
}
pub(crate) fn operator_credential_presence() -> OperatorCredentialPresence {
    let cfg = crate::PublisherConfig::from_operator_environment(
        true,
        None,
        crate::NewsSiteConfig::default(),
    );
    OperatorCredentialPresence {
        twitter: cfg.twitter_bearer_token.is_some(),
        github: cfg.forge_token.is_some(),
        open_collective: cfg.open_collective_token.is_some(),
        reddit: cfg.reddit_client_id.is_some()
            && cfg.reddit_client_secret.is_some()
            && cfg.reddit_refresh_token.is_some()
            && cfg.reddit_user_agent.is_some(),
        youtube: cfg.youtube_client_id.is_some()
            && cfg.youtube_client_secret.is_some()
            && cfg.youtube_refresh_token.is_some(),
    }
}
