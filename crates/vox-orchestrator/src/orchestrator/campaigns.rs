//! Reconstruction campaign lifecycle helpers (start + score).

use super::{Orchestrator, OrchestratorError};

fn build_campaign_scored_payload(
    campaign_id: &str,
    tier: crate::reconstruction::ReconstructionBenchmarkTier,
    evidence: &crate::reconstruction::ReconstructionEvidence,
    evidence_source: &'static str,
    allow_tier_promotion: bool,
) -> serde_json::Value {
    let next_tier = if allow_tier_promotion && evidence.passes_gate() {
        tier.next().map(|t| t.as_str().to_string())
    } else {
        None
    };
    serde_json::json!({
        "campaign_id": campaign_id,
        "benchmark_tier": tier.as_str(),
        "score": evidence.score(),
        "passes_gate": evidence.passes_gate(),
        "compile_ok": evidence.compile_ok,
        "targeted_tests_ok": evidence.targeted_tests_ok,
        "contract_checks_ok": evidence.contract_checks_ok,
        "docs_ssot_ok": evidence.docs_ssot_ok,
        "regression_checks_ok": evidence.regression_checks_ok,
        "evidence_source": evidence_source,
        "allow_tier_promotion": allow_tier_promotion,
        "next_tier": next_tier,
    })
}

impl Orchestrator {
    /// Start (or refresh) a reconstruction campaign snapshot.
    pub async fn begin_reconstruction_campaign(
        &self,
        campaign_id: impl Into<String>,
        tier: crate::reconstruction::ReconstructionBenchmarkTier,
        goal_preview: impl Into<String>,
        session_id: Option<&str>,
    ) -> Result<(), OrchestratorError> {
        let campaign_id = campaign_id.into();
        if campaign_id.trim().is_empty() {
            return Err(OrchestratorError::DatabaseError(
                "campaign_id cannot be empty".to_string(),
            ));
        }

        let snapshot = crate::reconstruction::CampaignMemorySnapshot {
            campaign_id: campaign_id.clone(),
            milestone_summary: Some(goal_preview.into().chars().take(500).collect()),
            ..Default::default()
        };
        let key = format!(
            "{}snapshot",
            crate::reconstruction::campaign_context_prefix(campaign_id.as_str())
        );
        crate::sync_lock::rw_read(&*self.context_store).set(
            crate::types::AgentId(0),
            key,
            serde_json::to_string(&snapshot).unwrap_or_else(|e| {
                tracing::error!(error = %e, "campaign snapshot serialization failed");
                format!(
                    "{{\"campaign_id\":\"{}\",\"milestone_summary\":null}}",
                    snapshot.campaign_id
                )
            }),
            0,
        );

        if crate::lineage::orchestration_lineage_persist_enabled() {
            if let Some(db) = self.db() {
                let repo = crate::lineage::repository_id();
                let objective = snapshot.milestone_summary.clone().unwrap_or_default();
                let spec = crate::reconstruction::RepoReconstructionSpec {
                    campaign_id: campaign_id.clone(),
                    objective: objective.clone(),
                    constraints: Vec::new(),
                    acceptance_tests: Vec::new(),
                    architecture_assumptions: Vec::new(),
                    shard_boundaries: Vec::new(),
                };
                let _ = db
                    .upsert_reconstruction_campaign_spec(
                        &campaign_id,
                        tier.as_str(),
                        objective.as_str(),
                        &serde_json::json!(spec),
                    )
                    .await;
                let _ = db
                    .upsert_reconstruction_artifact(
                        &campaign_id,
                        "campaign_snapshot",
                        crate::reconstruction::ReconstructionArtifactKind::PlannerBrief.as_str(),
                        &serde_json::json!(snapshot),
                        &["campaign".to_string(), "snapshot".to_string()],
                        Some("begin_reconstruction_campaign"),
                    )
                    .await;
                let payload = serde_json::json!({
                    "campaign_id": campaign_id,
                    "benchmark_tier": tier.as_str(),
                    "goal_preview": snapshot.milestone_summary,
                });
                let payload_str = payload.to_string();
                let _ = db
                    .append_orchestration_lineage_event(
                        &repo,
                        "reconstruction_campaign_started",
                        0,
                        None,
                        session_id,
                        None,
                        None,
                        None,
                        Some(payload_str.as_str()),
                    )
                    .await;
            }
        }
        Ok(())
    }

    /// Score campaign evidence and persist a lineage event.
    pub async fn record_reconstruction_campaign_result(
        &self,
        campaign_id: impl Into<String>,
        tier: crate::reconstruction::ReconstructionBenchmarkTier,
        evidence: crate::reconstruction::ReconstructionEvidence,
        evidence_source: &'static str,
        allow_tier_promotion: bool,
        session_id: Option<&str>,
        task_id: Option<crate::types::TaskId>,
        agent_id: Option<crate::types::AgentId>,
    ) {
        let campaign_id = campaign_id.into();
        if campaign_id.trim().is_empty() || !crate::lineage::orchestration_lineage_persist_enabled()
        {
            return;
        }
        if let Some(db) = self.db() {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let payload = build_campaign_scored_payload(
                campaign_id.as_str(),
                tier,
                &evidence,
                evidence_source,
                allow_tier_promotion,
            );
            let payload_str = payload.to_string();
            let repo = crate::lineage::repository_id();
            let _ = db
                .upsert_reconstruction_artifact(
                    campaign_id.as_str(),
                    &format!("verification:{now_ms}"),
                    crate::reconstruction::ReconstructionArtifactKind::VerificationEvidence
                        .as_str(),
                    &serde_json::json!({
                        "tier": tier.as_str(),
                        "source": evidence_source,
                        "evidence": evidence,
                    }),
                    &[
                        format!("tier:{}", tier.as_str()),
                        "verification".to_string(),
                    ],
                    Some("record_reconstruction_campaign_result"),
                )
                .await;
            let _ = db
                .upsert_reconstruction_benchmark_kpis(
                    campaign_id.as_str(),
                    tier.as_str(),
                    0,
                    if evidence.passes_gate() { 1.0 } else { 0.0 },
                    evidence.score(),
                    0.0,
                )
                .await;
            let _ = db
                .append_orchestration_lineage_event(
                    &repo,
                    "reconstruction_campaign_scored",
                    task_id.map(|t| t.0 as i64).unwrap_or(0),
                    agent_id.map(|a| a.0 as i64),
                    session_id,
                    None,
                    None,
                    None,
                    Some(payload_str.as_str()),
                )
                .await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_scored_payload_includes_next_tier_when_allowed_and_passed() {
        let payload = build_campaign_scored_payload(
            "camp-1",
            crate::reconstruction::ReconstructionBenchmarkTier::IssueRepair,
            &crate::reconstruction::ReconstructionEvidence {
                compile_ok: true,
                targeted_tests_ok: true,
                contract_checks_ok: true,
                docs_ssot_ok: true,
                regression_checks_ok: true,
                failures: Vec::new(),
            },
            "verified",
            true,
        );
        assert_eq!(payload["next_tier"], "subsystem_regen");
    }

    #[test]
    fn campaign_scored_payload_omits_next_tier_when_promotion_disabled() {
        let payload = build_campaign_scored_payload(
            "camp-1",
            crate::reconstruction::ReconstructionBenchmarkTier::IssueRepair,
            &crate::reconstruction::ReconstructionEvidence {
                compile_ok: true,
                targeted_tests_ok: true,
                contract_checks_ok: true,
                docs_ssot_ok: true,
                regression_checks_ok: true,
                failures: Vec::new(),
            },
            "heuristic",
            false,
        );
        assert!(payload["next_tier"].is_null());
    }
}
