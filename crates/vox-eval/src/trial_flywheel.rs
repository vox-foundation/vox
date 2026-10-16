//! Research Trial Flywheel, Race Harness, Verified Knowledge Loop, Two-Campaign Promotion
//! Governance, Offline Replay & Triggered Code-Gen Benchmark Engine (Phases 08–11 & 42:
//! `RACE-01..04`, `SCORE-01..03`, `REPORT-01`, `KNOW-01..05`, `REPLAY-01`, `PROMO-01..05`,
//! `HARD-01..04`, `BENCH-07..12`).

use crate::provenance::RunProvenance;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use xxhash_rust::xxh3::xxh3_64;

/// Campaign stage role in the two-campaign confirmation protocol (`PROMO-02`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignKind {
    Exploratory,
    HoldoutConfirmation,
}

/// Enforced campaign lifecycle (`TRIAL-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignLifecycle {
    Draft,
    Signed,
    Running,
    Completed,
    Failed,
    Stopped,
}

impl CampaignLifecycle {
    /// Enforce legal transitions (`Draft -> Signed -> Running -> Completed|Failed|Stopped`).
    pub fn transition_to(self, next: Self) -> Result<Self, String> {
        let valid = matches!(
            (self, next),
            (Self::Draft, Self::Signed)
                | (Self::Signed, Self::Running)
                | (
                    Self::Running,
                    Self::Completed | Self::Failed | Self::Stopped
                )
        );
        if valid {
            Ok(next)
        } else {
            Err(format!(
                "Illegal campaign lifecycle transition: {self:?} -> {next:?}"
            ))
        }
    }
}

/// Content-addressed isolation key preventing cross-arm cache or policy contamination (`RACE-02`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArmIsolationKey {
    pub campaign_id: String,
    pub arm_id: String,
    pub digest_hex: String,
}

impl ArmIsolationKey {
    #[must_use]
    pub fn derive(
        campaign_id: &str,
        arm_id: &str,
        config_hash: &str,
        model_id: &str,
        threshold_bps: u32,
        context_manifest_hash: &str,
    ) -> Self {
        let payload = format!(
            "{campaign_id}|{arm_id}|{config_hash}|{model_id}|{threshold_bps}|{context_manifest_hash}"
        );
        let digest = xxh3_64(payload.as_bytes());
        Self {
            campaign_id: campaign_id.to_string(),
            arm_id: arm_id.to_string(),
            digest_hex: format!("{digest:016x}"),
        }
    }
}

/// Atomic worst-case spend reservation ledger (`RACE-01`, `RACE-03`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendReservationLedger {
    pub max_concurrency: usize,
    pub budget_tokens_cap: u64,
    pub budget_calls_cap: u32,
    pub budget_cost_micros_cap: u64,
    pub reserved_tokens: u64,
    pub reserved_calls: u32,
    pub reserved_cost_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReservationReceipt {
    pub arm_id: String,
    pub worst_case_tokens: u64,
    pub worst_case_calls: u32,
    pub worst_case_cost_micros: u64,
}

impl SpendReservationLedger {
    pub fn new(
        max_concurrency: usize,
        budget_tokens_cap: u64,
        budget_calls_cap: u32,
        budget_cost_micros_cap: u64,
    ) -> Result<Self, String> {
        if max_concurrency == 0 || max_concurrency > 64 {
            return Err(format!(
                "Invalid race harness concurrency {max_concurrency}: must be in 1..=64"
            ));
        }
        Ok(Self {
            max_concurrency,
            budget_tokens_cap,
            budget_calls_cap,
            budget_cost_micros_cap,
            reserved_tokens: 0,
            reserved_calls: 0,
            reserved_cost_micros: 0,
        })
    }

    /// Admit a live call only if its worst-case liability (multiplied by `cascade_fanout`)
    /// fits within the campaign's remaining token, call, and cost budget (`RACE-03`).
    pub fn reserve_worst_case(
        &mut self,
        arm_id: &str,
        max_tokens_per_call: u64,
        calls: u32,
        cost_per_1k_tokens_usd: f64,
        cascade_fanout: u32,
    ) -> Result<ReservationReceipt, String> {
        if !cost_per_1k_tokens_usd.is_finite() || cost_per_1k_tokens_usd < 0.0 {
            return Err("Provider lacks finite pre-dispatch liability bound".to_string());
        }
        let fanout = cascade_fanout.max(1);
        let worst_calls = calls
            .checked_mul(fanout)
            .ok_or_else(|| "Call count overflow".to_string())?;
        let worst_tokens = max_tokens_per_call
            .checked_mul(u64::from(worst_calls))
            .ok_or_else(|| "Token reservation overflow".to_string())?;
        let worst_cost_micros =
            ((worst_tokens as f64 / 1000.0) * cost_per_1k_tokens_usd * 1_000_000.0).ceil() as u64;

        let next_tokens = self.reserved_tokens.saturating_add(worst_tokens);
        let next_calls = self.reserved_calls.saturating_add(worst_calls);
        let next_cost = self.reserved_cost_micros.saturating_add(worst_cost_micros);

        if next_tokens > self.budget_tokens_cap
            || next_calls > self.budget_calls_cap
            || next_cost > self.budget_cost_micros_cap
        {
            return Err(format!(
                "Spend reservation rejected for arm '{arm_id}': would exceed campaign cap (tokens {next_tokens}/{}, calls {next_calls}/{}, cost_micros {next_cost}/{})",
                self.budget_tokens_cap, self.budget_calls_cap, self.budget_cost_micros_cap
            ));
        }

        self.reserved_tokens = next_tokens;
        self.reserved_calls = next_calls;
        self.reserved_cost_micros = next_cost;

        Ok(ReservationReceipt {
            arm_id: arm_id.to_string(),
            worst_case_tokens: worst_tokens,
            worst_case_calls: worst_calls,
            worst_case_cost_micros: worst_cost_micros,
        })
    }
}

/// Group-sequential stopping rule decision (`RACE-04`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoppingDecision {
    Continue,
    StopImmediateFailClosed(&'static str),
    StopAtBatchBoundary(&'static str),
}

/// Evaluate group-sequential stopping rules (`RACE-04`).
/// Safety, spend, and wall-time breaches stop immediately; statistical futility/efficacy
/// boundaries are evaluated ONLY at completed batch multiples.
#[must_use]
pub fn evaluate_stopping_rule(
    completed_samples: usize,
    batch_size: usize,
    safety_breach: bool,
    spend_exhausted: bool,
    wall_time_exceeded: bool,
    observed_effect_delta: f64,
    futility_floor: f64,
    efficacy_ceiling: f64,
) -> StoppingDecision {
    if safety_breach {
        return StoppingDecision::StopImmediateFailClosed("safety_breach");
    }
    if spend_exhausted {
        return StoppingDecision::StopImmediateFailClosed("spend_exhausted");
    }
    if wall_time_exceeded {
        return StoppingDecision::StopImmediateFailClosed("wall_time_exceeded");
    }
    if batch_size == 0 || completed_samples == 0 || completed_samples % batch_size != 0 {
        return StoppingDecision::Continue;
    }
    if observed_effect_delta < futility_floor {
        StoppingDecision::StopAtBatchBoundary("statistical_futility")
    } else if observed_effect_delta >= efficacy_ceiling {
        StoppingDecision::StopAtBatchBoundary("statistical_efficacy")
    } else {
        StoppingDecision::Continue
    }
}

/// Candidate arm metrics for hard-gate and Pareto frontier evaluation (`SCORE-01..03`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmScorecardInput {
    pub arm_id: String,
    pub is_baseline: bool,
    pub is_champion: bool,
    pub groundedness: f64,
    pub citation_precision: f64,
    pub completion_rate: f64,
    pub cost_usd: f64,
    pub latency_p50_ms: f64,
    pub hallucinated_no_evidence_count: usize,
    /// Display-only legacy aggregate; never used for eligibility or Pareto dominance (`SCORE-02`).
    pub display_only_quality_score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArmScorecardEvaluation {
    pub arm_id: String,
    pub hard_gate_passed: bool,
    pub hard_gate_failures: Vec<String>,
    pub on_pareto_frontier: bool,
    pub weighted_view_score: f64,
}

/// Validate that metric view weights sum to 1.00 (`SCORE-01`).
pub fn validate_metric_weights(weights: &[(&str, f64)]) -> Result<(), String> {
    if weights.is_empty() {
        return Err("Metric weights cannot be empty".to_string());
    }
    for (name, w) in weights {
        if !w.is_finite() || *w < 0.0 {
            return Err(format!("Invalid weight for metric '{name}': {w}"));
        }
    }
    let sum: f64 = weights.iter().map(|(_, w)| *w).sum();
    if (sum - 1.0).abs() > 1e-6 {
        return Err(format!("Metric weights must sum to 1.00 (got {sum:.6})"));
    }
    Ok(())
}

/// Evaluate hard gates and Pareto dominance across campaign arms (`SCORE-02`, `REPORT-01`).
#[must_use]
pub fn evaluate_pareto_scorecards(
    arms: &[ArmScorecardInput],
    min_groundedness: f64,
    min_completion_rate: f64,
) -> Vec<ArmScorecardEvaluation> {
    let mut gate_status: Vec<(bool, Vec<String>)> = Vec::with_capacity(arms.len());
    for arm in arms {
        let mut failures = Vec::new();
        if arm.hallucinated_no_evidence_count > 0 {
            failures.push(format!(
                "hallucinated_no_evidence_count={} (must be 0)",
                arm.hallucinated_no_evidence_count
            ));
        }
        if arm.groundedness < min_groundedness {
            failures.push(format!(
                "groundedness {:.3} < {:.3}",
                arm.groundedness, min_groundedness
            ));
        }
        if arm.completion_rate < min_completion_rate {
            failures.push(format!(
                "completion_rate {:.3} < {:.3}",
                arm.completion_rate, min_completion_rate
            ));
        }
        gate_status.push((failures.is_empty(), failures));
    }

    let mut out = Vec::with_capacity(arms.len());
    for (idx, arm) in arms.iter().enumerate() {
        let (passed, ref failures) = gate_status[idx];
        let mut dominated = false;
        if passed {
            for (other_idx, other) in arms.iter().enumerate() {
                if idx == other_idx || !gate_status[other_idx].0 {
                    continue;
                }
                let no_worse = other.groundedness >= arm.groundedness
                    && other.citation_precision >= arm.citation_precision
                    && other.cost_usd <= arm.cost_usd
                    && other.latency_p50_ms <= arm.latency_p50_ms;
                let strictly_better = other.groundedness > arm.groundedness
                    || other.citation_precision > arm.citation_precision
                    || other.cost_usd < arm.cost_usd
                    || other.latency_p50_ms < arm.latency_p50_ms;
                if no_worse && strictly_better {
                    dominated = true;
                    break;
                }
            }
        }
        let cost_norm = (1.0 / (1.0 + arm.cost_usd.max(0.0))).clamp(0.0, 1.0);
        let lat_norm = (1.0 / (1.0 + (arm.latency_p50_ms.max(0.0) / 1000.0))).clamp(0.0, 1.0);
        let weighted_view_score = 0.40 * arm.groundedness
            + 0.30 * arm.citation_precision
            + 0.15 * cost_norm
            + 0.15 * lat_norm;

        out.push(ArmScorecardEvaluation {
            arm_id: arm.arm_id.clone(),
            hard_gate_passed: passed,
            hard_gate_failures: failures.clone(),
            on_pareto_frontier: passed && !dominated,
            weighted_view_score,
        });
    }
    out
}

/// Frozen per-arm knowledge manifest (`KNOW-01..04`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenKnowledgeManifest {
    pub manifest_hash: String,
    pub campaign_signed_at_ms: u64,
    pub allowlisted_doc_paths: BTreeSet<String>,
    pub preregistered_treatment_finding_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateFinding {
    pub finding_id: String,
    pub doc_path: String,
    pub created_at_ms: u64,
    pub is_negative_finding: bool,
    pub content_hash: String,
}

impl FrozenKnowledgeManifest {
    /// Enforce that `docs/src/archive/` is never ingested (`KNOW-03`) and that findings created
    /// after campaign signing cannot enter an arm unless preregistered as a treatment (`KNOW-04`).
    pub fn admit_finding(&self, finding: &CandidateFinding) -> Result<(), String> {
        if finding.doc_path.starts_with("docs/src/archive/")
            || finding.doc_path.contains("/archive/")
        {
            return Err(format!(
                "Archival path '{}' is forbidden in KB retrieval (KNOW-03)",
                finding.doc_path
            ));
        }
        if finding.created_at_ms > self.campaign_signed_at_ms
            && !self
                .preregistered_treatment_finding_ids
                .contains(&finding.finding_id)
        {
            return Err(format!(
                "Finding '{}' created at {}ms post-dates campaign signature ({}ms) and is not in preregistered treatment set (KNOW-04)",
                finding.finding_id, finding.created_at_ms, self.campaign_signed_at_ms
            ));
        }
        if !self.allowlisted_doc_paths.is_empty()
            && !self.allowlisted_doc_paths.contains(&finding.doc_path)
            && !self
                .preregistered_treatment_finding_ids
                .contains(&finding.finding_id)
        {
            return Err(format!(
                "Doc path '{}' is outside frozen manifest allowlist (KNOW-04)",
                finding.doc_path
            ));
        }
        Ok(())
    }
}

/// Projection reconciliation status between Tier A authoritative findings and memory/search (`KNOW-02`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionStatus {
    Synced,
    MissingProjection,
    HashMismatch,
}

#[must_use]
pub fn reconcile_projection_hashes(
    authoritative_hash: &str,
    memory_projection_hash: Option<&str>,
    search_projection_hash: Option<&str>,
) -> ProjectionStatus {
    match (memory_projection_hash, search_projection_hash) {
        (Some(m), Some(s)) if m == authoritative_hash && s == authoritative_hash => {
            ProjectionStatus::Synced
        }
        (None, _) | (_, None) => ProjectionStatus::MissingProjection,
        _ => ProjectionStatus::HashMismatch,
    }
}

/// Four-way memory ablation modes (`KNOW-05`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAblationMode {
    Empty,
    Current,
    StaleConflicting,
    CuratedVerified,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryAblationReport {
    pub scores_by_mode: BTreeMap<MemoryAblationMode, f64>,
    pub curated_vs_empty_lift: f64,
    pub curated_vs_stale_resilience: f64,
}

/// Evaluate all four required memory ablations (`KNOW-05`).
pub fn evaluate_memory_ablations(
    scores: &BTreeMap<MemoryAblationMode, f64>,
) -> Result<MemoryAblationReport, String> {
    for required in [
        MemoryAblationMode::Empty,
        MemoryAblationMode::Current,
        MemoryAblationMode::StaleConflicting,
        MemoryAblationMode::CuratedVerified,
    ] {
        if !scores.contains_key(&required) {
            return Err(format!(
                "Missing required memory ablation arm: {required:?}"
            ));
        }
    }
    let empty = scores[&MemoryAblationMode::Empty];
    let stale = scores[&MemoryAblationMode::StaleConflicting];
    let curated = scores[&MemoryAblationMode::CuratedVerified];
    Ok(MemoryAblationReport {
        scores_by_mode: scores.clone(),
        curated_vs_empty_lift: curated - empty,
        curated_vs_stale_resilience: curated - stale,
    })
}

/// Offline replay verification bundle (`REPLAY-01`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineReplayBundle {
    pub campaign_id: String,
    pub provider_network_disabled: bool,
    pub expected_bundle_digest: String,
    pub sample_payloads: Vec<String>,
    pub recorded_pass_rate: f64,
}

pub fn verify_offline_replay(bundle: &OfflineReplayBundle) -> Result<f64, String> {
    if !bundle.provider_network_disabled {
        return Err(
            "Offline replay refused: provider_network_disabled must be true (REPLAY-01)"
                .to_string(),
        );
    }
    if bundle.sample_payloads.is_empty() {
        return Err("Offline replay bundle contains zero captured samples".to_string());
    }
    let joined = bundle.sample_payloads.join("\n");
    let digest = format!("{:016x}", xxh3_64(joined.as_bytes()));
    if digest != bundle.expected_bundle_digest {
        return Err(format!(
            "Offline replay digest mismatch: computed {digest} != expected {}",
            bundle.expected_bundle_digest
        ));
    }
    Ok(bundle.recorded_pass_rate)
}

/// Two-campaign promotion governance & dual-control state machine (`PROMO-01..05`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionProposalState {
    AwaitingConfirmation,
    AwaitingApproval,
    Designated,
    Suspended,
    Revoked,
    EvidenceExpired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionCandidateInput {
    pub exploratory_campaign_id: String,
    pub exploratory_kind: CampaignKind,
    pub confirmation_campaign_id: Option<String>,
    pub confirmation_kind: Option<CampaignKind>,
    pub hard_gates_passed: bool,
    pub holdout_concordant: bool,
    pub provenance: RunProvenance,
    pub replay_verified: bool,
    pub evidence_expired: bool,
}

/// Check promotion eligibility under the two-campaign confirmation rule (`PROMO-02`).
pub fn evaluate_promotion_eligibility(
    input: &PromotionCandidateInput,
) -> Result<PromotionProposalState, String> {
    if input.evidence_expired {
        return Ok(PromotionProposalState::EvidenceExpired);
    }
    if input.exploratory_kind != CampaignKind::Exploratory {
        return Err("Nomination campaign must have CampaignKind::Exploratory".to_string());
    }
    if !input.hard_gates_passed {
        return Err("Challenger failed hard gates".to_string());
    }
    if !input.provenance.missing_required().is_empty() {
        return Err(format!(
            "Challenger provenance incomplete: missing {:?}",
            input.provenance.missing_required()
        ));
    }
    if !input.replay_verified {
        return Err("Challenger offline replay has not been verified".to_string());
    }
    let Some(ref conf_id) = input.confirmation_campaign_id else {
        return Ok(PromotionProposalState::AwaitingConfirmation);
    };
    if conf_id == &input.exploratory_campaign_id {
        return Err(
            "Holdout confirmation campaign must be distinct from exploratory campaign".to_string(),
        );
    }
    if input.confirmation_kind != Some(CampaignKind::HoldoutConfirmation) {
        return Err(
            "Confirmation campaign must have CampaignKind::HoldoutConfirmation".to_string(),
        );
    }
    if !input.holdout_concordant {
        return Err("Holdout confirmation campaign did not confirm exploratory gain".to_string());
    }
    Ok(PromotionProposalState::AwaitingApproval)
}

/// Enforce dual-control human designation independent of campaign actors (`PROMO-03`, `PROMO-05`).
pub fn authorize_champion_designation(
    current_state: PromotionProposalState,
    approver_1: &str,
    approver_2: &str,
    campaign_actors: &BTreeSet<String>,
) -> Result<PromotionProposalState, String> {
    if current_state != PromotionProposalState::AwaitingApproval {
        return Err(format!(
            "Cannot designate champion from state {current_state:?}"
        ));
    }
    let a1 = approver_1.trim();
    let a2 = approver_2.trim();
    if a1.is_empty() || a2.is_empty() || a1 == a2 {
        return Err(
            "Dual-control designation requires two distinct non-empty human approvers".to_string(),
        );
    }
    if campaign_actors.contains(a1) || campaign_actors.contains(a2) {
        return Err(
            "Role conflict: approvers must be independent of campaign signing, authorship, execution, and curation (PROMO-03)"
                .to_string(),
        );
    }
    Ok(PromotionProposalState::Designated)
}

/// Emergency suspension by a single conflict-free safety principal (`PROMO-05`).
pub fn suspend_champion_by_safety_principal(
    current_state: PromotionProposalState,
    safety_principal: &str,
    campaign_actors: &BTreeSet<String>,
) -> Result<PromotionProposalState, String> {
    if current_state != PromotionProposalState::Designated {
        return Err(format!(
            "Cannot suspend non-designated state {current_state:?}"
        ));
    }
    if safety_principal.trim().is_empty() || campaign_actors.contains(safety_principal.trim()) {
        return Err("Safety principal must be non-empty and conflict-free".to_string());
    }
    Ok(PromotionProposalState::Suspended)
}

/// Scoped champion resolution where exact workload domain beats broader wildcard scope (`PROMO-04`).
#[must_use]
pub fn resolve_scoped_champion<'a>(
    champions_by_scope: &'a BTreeMap<String, String>,
    workload_domain: &str,
) -> Option<&'a str> {
    if let Some(exact) = champions_by_scope.get(workload_domain) {
        return Some(exact.as_str());
    }
    if let Some((prefix, _)) = workload_domain.rsplit_once('/') {
        let wildcard = format!("{prefix}/*");
        if let Some(broad) = champions_by_scope.get(&wildcard) {
            return Some(broad.as_str());
        }
    }
    champions_by_scope.get("*").map(String::as_str)
}

/// Adversarial hardening trial scenarios (`HARD-01..04` / `AUDIT-01..03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdversarialScenario {
    HallucinatedNoEvidence,
    ConflictingMemory,
    ProviderFailure,
    BudgetExhaustion,
    ReservationRace,
    CancellationMidBatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdversarialTrialOutcome {
    pub scenario: AdversarialScenario,
    pub fail_closed: bool,
    pub terminal_reason: &'static str,
}

/// Run the 6 mandatory adversarial trials and verify every scenario fails closed (`AUDIT-01`).
#[must_use]
pub fn run_adversarial_trial_suite() -> Vec<AdversarialTrialOutcome> {
    vec![
        AdversarialTrialOutcome {
            scenario: AdversarialScenario::HallucinatedNoEvidence,
            fail_closed: !evaluate_pareto_scorecards(
                &[ArmScorecardInput {
                    arm_id: "adv-halluc".to_string(),
                    is_baseline: false,
                    is_champion: false,
                    groundedness: 0.95,
                    citation_precision: 0.95,
                    completion_rate: 1.0,
                    cost_usd: 0.01,
                    latency_p50_ms: 100.0,
                    hallucinated_no_evidence_count: 1,
                    display_only_quality_score: 0.99,
                }],
                0.7,
                0.8,
            )[0]
            .hard_gate_passed,
            terminal_reason: "hard_gate_rejected_hallucinated_no_evidence",
        },
        AdversarialTrialOutcome {
            scenario: AdversarialScenario::ConflictingMemory,
            fail_closed: reconcile_projection_hashes("hash-a", Some("hash-a"), Some("hash-b"))
                == ProjectionStatus::HashMismatch,
            terminal_reason: "projection_reconciler_flagged_conflict",
        },
        AdversarialTrialOutcome {
            scenario: AdversarialScenario::ProviderFailure,
            fail_closed: evaluate_stopping_rule(3, 5, true, false, false, 0.1, 0.0, 0.5)
                == StoppingDecision::StopImmediateFailClosed("safety_breach"),
            terminal_reason: "immediate_fail_closed_on_provider_safety_fault",
        },
        AdversarialTrialOutcome {
            scenario: AdversarialScenario::BudgetExhaustion,
            fail_closed: {
                let mut ledger = SpendReservationLedger::new(2, 1000, 5, 10_000).unwrap();
                ledger
                    .reserve_worst_case("arm-a", 600, 1, 0.001, 2)
                    .is_err()
            },
            terminal_reason: "atomic_reservation_refused_overspend",
        },
        AdversarialTrialOutcome {
            scenario: AdversarialScenario::ReservationRace,
            fail_closed: {
                let mut ledger = SpendReservationLedger::new(2, 1000, 5, 10_000).unwrap();
                let first = ledger.reserve_worst_case("arm-1", 600, 1, 0.001, 1);
                let second = ledger.reserve_worst_case("arm-2", 600, 1, 0.001, 1);
                first.is_ok() && second.is_err()
            },
            terminal_reason: "concurrent_reservation_serialized_under_cap",
        },
        AdversarialTrialOutcome {
            scenario: AdversarialScenario::CancellationMidBatch,
            fail_closed: CampaignLifecycle::Running
                .transition_to(CampaignLifecycle::Stopped)
                .is_ok()
                && CampaignLifecycle::Stopped
                    .transition_to(CampaignLifecycle::Running)
                    .is_err(),
            terminal_reason: "cancelled_run_terminalized_no_same_id_resume",
        },
    ]
}

/// Phase 42 Triggered `.vox` Language Code-Gen Benchmark Campaign (`BENCH-07..12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkTriggerEvent {
    ReleaseTag,
    MainPush,
    MensPromotion,
    ManualDispatch,
    PullRequest,
    MergeGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignDispatchDecision {
    DispatchCampaign,
    SkipNoTrigger,
    SkipForbiddenOnPrOrMergeQueue,
    ReferenceMigrationRequired,
}

/// Decide whether a triggered language benchmark campaign runs (`BENCH-08`).
/// Never runs on `PullRequest` or `MergeGroup`, and fails closed with `ReferenceMigrationRequired`
/// before calling any model if any reference solution fails under the new syntax.
#[must_use]
pub fn should_dispatch_benchmark_campaign(
    event: BenchmarkTriggerEvent,
    old_syntax_fingerprint: &str,
    new_syntax_fingerprint: &str,
    all_reference_solutions_compile: bool,
) -> CampaignDispatchDecision {
    match event {
        BenchmarkTriggerEvent::PullRequest | BenchmarkTriggerEvent::MergeGroup => {
            return CampaignDispatchDecision::SkipForbiddenOnPrOrMergeQueue;
        }
        BenchmarkTriggerEvent::MainPush => {
            if old_syntax_fingerprint == new_syntax_fingerprint {
                return CampaignDispatchDecision::SkipNoTrigger;
            }
        }
        BenchmarkTriggerEvent::ReleaseTag
        | BenchmarkTriggerEvent::MensPromotion
        | BenchmarkTriggerEvent::ManualDispatch => {}
    }
    if !all_reference_solutions_compile {
        CampaignDispatchDecision::ReferenceMigrationRequired
    } else {
        CampaignDispatchDecision::DispatchCampaign
    }
}

/// Estimand labels E1–E6 (`BENCH-12`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BenchmarkEstimand {
    E1ZeroShotPassAt1,
    E2CompilerFeedbackRepairPassAtK,
    E3RagContextLift,
    E4MensAdapterVsBase,
    E5CrossLanguageFreshTier,
    E6ContaminatedPreCutoffTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggeredBenchmarkReport {
    pub syntax_fingerprint: String,
    pub coi_disclosure: String,
    pub anchor_drift_detected: bool,
    /// Separate columns per condition (`C0..C3`) and estimand (`E1..E6`); E5 is suppressed on anchor drift (`BENCH-12`).
    pub estimand_columns: BTreeMap<BenchmarkEstimand, BTreeMap<String, f64>>,
    pub suppressed_estimands: Vec<BenchmarkEstimand>,
}

#[must_use]
pub fn assemble_triggered_benchmark_report(
    syntax_fingerprint: &str,
    anchor_drift_detected: bool,
    mut raw_estimands: BTreeMap<BenchmarkEstimand, BTreeMap<String, f64>>,
) -> TriggeredBenchmarkReport {
    let mut suppressed = Vec::new();
    if anchor_drift_detected {
        if raw_estimands
            .remove(&BenchmarkEstimand::E5CrossLanguageFreshTier)
            .is_some()
        {
            suppressed.push(BenchmarkEstimand::E5CrossLanguageFreshTier);
        }
    }
    TriggeredBenchmarkReport {
        syntax_fingerprint: syntax_fingerprint.to_string(),
        coi_disclosure: "Conflict-of-Interest Disclosure: Benchmark authored and executed by the Vox language team; reference solutions and verifier are pinned and offline-verifiable.".to_string(),
        anchor_drift_detected,
        estimand_columns: raw_estimands,
        suppressed_estimands: suppressed,
    }
}

/// Versioned longitudinal multi-spoke evaluation scorecard record (`EVAL-03`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MensLongitudinalScorecardV1 {
    /// Schema discriminator (`"mens_longitudinal_scorecard_v1"`).
    pub schema_version: String,
    /// Stable evaluation run ID.
    pub run_id: String,
    /// Flywheel cycle index (`0, 1, 2, ...`).
    pub cycle_index: usize,
    /// Linked dataset `snapshot_uri` (`trusty://vox-dataset/...`).
    pub snapshot_uri: String,
    /// Linked adapter revision Trusty URI (`trusty://blake3/...`).
    pub adapter_trusty_uri: String,
    /// 5-spoke empirical scorecard (`EVAL-01`).
    pub scorecard: crate::mens::MensFiveSpokeScorecardV1,
    /// Deterministic XXH3-64 hex digest over `(run_id, cycle_index, snapshot_uri, adapter_trusty_uri, overall_pass_at_1)`.
    pub content_digest: String,
    /// Timestamp (epoch ms).
    pub recorded_at_ms: i64,
}

impl MensLongitudinalScorecardV1 {
    /// Canonical schema discriminator.
    pub const SCHEMA_VERSION: &'static str = "mens_longitudinal_scorecard_v1";

    /// Construct a new longitudinal scorecard entry with computed digest.
    #[must_use]
    pub fn new(
        run_id: impl Into<String>,
        cycle_index: usize,
        scorecard: crate::mens::MensFiveSpokeScorecardV1,
        recorded_at_ms: i64,
    ) -> Self {
        let run_id = run_id.into();
        let snapshot_uri = scorecard.snapshot_uri.clone();
        let adapter_trusty_uri = scorecard.adapter_trusty_uri.clone();
        let payload = format!(
            "{run_id}|{cycle_index}|{snapshot_uri}|{adapter_trusty_uri}|{:.6}",
            scorecard.overall_pass_at_1
        );
        let content_digest = format!("{:016x}", xxh3_64(payload.as_bytes()));
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            run_id,
            cycle_index,
            snapshot_uri,
            adapter_trusty_uri,
            scorecard,
            content_digest,
            recorded_at_ms,
        }
    }

    /// Verify schema discriminator, non-empty lineage URIs, 5-spoke coverage, and digest integrity.
    pub fn verify_integrity(&self) -> Result<(), String> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(format!("invalid schema_version `{}`", self.schema_version));
        }
        if !self.snapshot_uri.starts_with("trusty://") {
            return Err(format!(
                "snapshot_uri `{}` must start with `trusty://`",
                self.snapshot_uri
            ));
        }
        if !self.adapter_trusty_uri.starts_with("trusty://") {
            return Err(format!(
                "adapter_trusty_uri `{}` must start with `trusty://`",
                self.adapter_trusty_uri
            ));
        }
        if !self.scorecard.all_five_spokes_covered {
            return Err("scorecard must cover all 5 spokes".to_string());
        }
        let expected_payload = format!(
            "{}|{}|{}|{}|{:.6}",
            self.run_id,
            self.cycle_index,
            self.snapshot_uri,
            self.adapter_trusty_uri,
            self.scorecard.overall_pass_at_1
        );
        let expected_digest = format!("{:016x}", xxh3_64(expected_payload.as_bytes()));
        if self.content_digest != expected_digest {
            return Err(format!(
                "content_digest mismatch: got {}, expected {}",
                self.content_digest, expected_digest
            ));
        }
        Ok(())
    }
}

/// Longitudinal ledger persisting ordered [`MensLongitudinalScorecardV1`] runs (`EVAL-03`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MensLongitudinalLedger {
    /// Ordered longitudinal scorecard runs.
    pub runs: Vec<MensLongitudinalScorecardV1>,
}

impl MensLongitudinalLedger {
    /// Append a verified longitudinal scorecard run.
    pub fn append_run(&mut self, run: MensLongitudinalScorecardV1) -> Result<(), String> {
        run.verify_integrity()?;
        if self.runs.iter().any(|r| r.run_id == run.run_id) {
            return Err(format!("duplicate run_id `{}`", run.run_id));
        }
        self.runs.push(run);
        Ok(())
    }

    /// Verify that across consecutive cycles no spoke regressed and overall `pass@1` is non-decreasing.
    pub fn verify_monotonic_progression(&self) -> Result<(), String> {
        for pair in self.runs.windows(2) {
            let prev = &pair[0];
            let next = &pair[1];
            if next.scorecard.overall_pass_at_1 + 1e-9 < prev.scorecard.overall_pass_at_1 {
                return Err(format!(
                    "overall_pass_at_1 regressed from {:.4} (cycle {}) to {:.4} (cycle {})",
                    prev.scorecard.overall_pass_at_1,
                    prev.cycle_index,
                    next.scorecard.overall_pass_at_1,
                    next.cycle_index
                ));
            }
            for s_next in &next.scorecard.spokes {
                if let Some(s_prev) = prev
                    .scorecard
                    .spokes
                    .iter()
                    .find(|s| s.spoke == s_next.spoke)
                    && s_next.pass_at_1 + 1e-9 < s_prev.pass_at_1
                {
                    return Err(format!(
                        "spoke `{}` regressed from {:.4} to {:.4}",
                        s_next.spoke.as_str(),
                        s_prev.pass_at_1,
                        s_next.pass_at_1
                    ));
                }
            }
        }
        Ok(())
    }

    /// Serialize ledger as canonical JSONL for `VoxDb` / artifact storage (`EVAL-03`).
    pub fn to_jsonl(&self) -> Result<String, String> {
        let mut lines = Vec::with_capacity(self.runs.len());
        for r in &self.runs {
            lines.push(serde_json::to_string(r).map_err(|e| e.to_string())?);
        }
        Ok(lines.join("\n"))
    }

    /// Rehydrate ledger from JSONL and verify every record's integrity (`EVAL-03`).
    pub fn from_jsonl(jsonl: &str) -> Result<Self, String> {
        let mut ledger = Self::default();
        for line in jsonl.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let run: MensLongitudinalScorecardV1 =
                serde_json::from_str(line).map_err(|e| e.to_string())?;
            ledger.append_run(run)?;
        }
        Ok(ledger)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::{RunIdentity, SamplingRecord};

    fn sample_complete_provenance() -> RunProvenance {
        RunProvenance {
            syntax_fingerprint: Some("blake3:abc".to_string()),
            syntax_fingerprint_kind: Some("behavioral_ast_v1".to_string()),
            cargo_version: Some("1.85.0".to_string()),
            vox_git_sha: Some("deadbeef".to_string()),
            vox_binary_sha256: Some("cafebabe".to_string()),
            verifier_version: Some("0.6.0".to_string()),
            verifier_matches_harness: Some(true),
            tier: Some("smoke".to_string()),
            problem_set_hash: Some("pset-1".to_string()),
            corpus_manifest_hash: None,
            prompt_template_hash: Some("pt-1".to_string()),
            condition: Some("C0_zero_shot".to_string()),
            context_hash: Some("ctx-0".to_string()),
            harness_version: Some("2.0".to_string()),
            model_id: Some("openrouter/test".to_string()),
            provider: Some("openrouter".to_string()),
            request_date: Some("2026-10-15".to_string()),
            provider_reported_models: vec!["openrouter/test".to_string()],
            mens: None,
            sampling: Some(SamplingRecord {
                temperature: 0.2,
                top_p: 0.95,
                n: 5,
                k: 5,
                seed: 42,
                max_tokens: 1024,
                seed_enforced: true,
                feedback_turns: 2,
            }),
            runner: Some("darwin-arm64".to_string()),
            os: Some("macos".to_string()),
            arch: Some("aarch64".to_string()),
            verifier_timeout_secs: Some(30),
            identity: Some(RunIdentity {
                campaign_id: "camp-1".to_string(),
                run_id: "run-1".to_string(),
                replicate_id: 0,
                attempt_index: 0,
                retry_of_run_id: None,
            }),
            preregistration_id: Some("prereg-1".to_string()),
        }
    }

    #[test]
    fn phase08_race_harness_reservations_stopping_and_pareto_scorecards() {
        let k1 = ArmIsolationKey::derive("c1", "armA", "cfg1", "m1", 8000, "kb1");
        let k2 = ArmIsolationKey::derive("c1", "armA", "cfg2", "m1", 8000, "kb1");
        assert_ne!(k1.digest_hex, k2.digest_hex);

        assert!(SpendReservationLedger::new(0, 1000, 10, 10_000).is_err());
        let mut ledger = SpendReservationLedger::new(4, 10_000, 10, 50_000).unwrap();
        let r1 = ledger
            .reserve_worst_case("armA", 2000, 2, 0.002, 2)
            .expect("fits budget (8000 tokens)");
        assert_eq!(r1.worst_case_tokens, 8000);
        assert!(
            ledger
                .reserve_worst_case("armB", 2000, 1, 0.002, 2)
                .is_err()
        );

        // Stopping rules only fire at batch boundary unless safety/spend/wall-time breach
        assert_eq!(
            evaluate_stopping_rule(3, 5, false, false, false, -0.5, 0.0, 0.2),
            StoppingDecision::Continue
        );
        assert_eq!(
            evaluate_stopping_rule(5, 5, false, false, false, -0.5, 0.0, 0.2),
            StoppingDecision::StopAtBatchBoundary("statistical_futility")
        );

        let evals = evaluate_pareto_scorecards(
            &[
                ArmScorecardInput {
                    arm_id: "fast-cheap".to_string(),
                    is_baseline: true,
                    is_champion: false,
                    groundedness: 0.82,
                    citation_precision: 0.85,
                    completion_rate: 0.95,
                    cost_usd: 0.01,
                    latency_p50_ms: 400.0,
                    hallucinated_no_evidence_count: 0,
                    display_only_quality_score: 0.50,
                },
                ArmScorecardInput {
                    arm_id: "high-acc".to_string(),
                    is_baseline: false,
                    is_champion: true,
                    groundedness: 0.94,
                    citation_precision: 0.92,
                    completion_rate: 0.98,
                    cost_usd: 0.08,
                    latency_p50_ms: 1200.0,
                    hallucinated_no_evidence_count: 0,
                    display_only_quality_score: 0.90,
                },
                ArmScorecardInput {
                    arm_id: "hallucinated".to_string(),
                    is_baseline: false,
                    is_champion: false,
                    groundedness: 0.99,
                    citation_precision: 0.99,
                    completion_rate: 1.0,
                    cost_usd: 0.001,
                    latency_p50_ms: 100.0,
                    hallucinated_no_evidence_count: 1,
                    display_only_quality_score: 1.0,
                },
            ],
            0.75,
            0.90,
        );
        assert!(evals[0].on_pareto_frontier);
        assert!(evals[1].on_pareto_frontier);
        assert!(!evals[2].hard_gate_passed);
        assert!(!evals[2].on_pareto_frontier);
    }

    #[test]
    fn phase09_frozen_knowledge_manifest_and_four_way_memory_ablations() {
        let manifest = FrozenKnowledgeManifest {
            manifest_hash: "m-1".to_string(),
            campaign_signed_at_ms: 1_000_000,
            allowlisted_doc_paths: BTreeSet::from(["docs/src/architecture/ssot.md".to_string()]),
            preregistered_treatment_finding_ids: BTreeSet::from(["treat-1".to_string()]),
        };

        assert!(
            manifest
                .admit_finding(&CandidateFinding {
                    finding_id: "f-archive".to_string(),
                    doc_path: "docs/src/archive/old.md".to_string(),
                    created_at_ms: 500_000,
                    is_negative_finding: false,
                    content_hash: "h1".to_string(),
                })
                .is_err()
        );

        assert!(
            manifest
                .admit_finding(&CandidateFinding {
                    finding_id: "f-late".to_string(),
                    doc_path: "docs/src/architecture/ssot.md".to_string(),
                    created_at_ms: 1_000_001,
                    is_negative_finding: false,
                    content_hash: "h2".to_string(),
                })
                .is_err()
        );

        assert!(
            manifest
                .admit_finding(&CandidateFinding {
                    finding_id: "treat-1".to_string(),
                    doc_path: "docs/src/architecture/ssot.md".to_string(),
                    created_at_ms: 1_000_500,
                    is_negative_finding: true,
                    content_hash: "h3".to_string(),
                })
                .is_ok()
        );

        let mut ablations = BTreeMap::new();
        ablations.insert(MemoryAblationMode::Empty, 0.65);
        ablations.insert(MemoryAblationMode::Current, 0.74);
        ablations.insert(MemoryAblationMode::StaleConflicting, 0.58);
        ablations.insert(MemoryAblationMode::CuratedVerified, 0.84);
        let report = evaluate_memory_ablations(&ablations).expect("4 ablations");
        assert!((report.curated_vs_empty_lift - 0.19).abs() < 1e-6);
        assert!((report.curated_vs_stale_resilience - 0.26).abs() < 1e-6);
    }

    #[test]
    fn phase10_two_campaign_promotion_quorum_and_offline_replay() {
        let samples = vec!["sample-1:pass".to_string(), "sample-2:pass".to_string()];
        let expected_digest = format!("{:016x}", xxh3_64(samples.join("\n").as_bytes()));
        let bundle = OfflineReplayBundle {
            campaign_id: "camp-1".to_string(),
            provider_network_disabled: true,
            expected_bundle_digest: expected_digest,
            sample_payloads: samples,
            recorded_pass_rate: 1.0,
        };
        assert_eq!(verify_offline_replay(&bundle).unwrap(), 1.0);

        let mut cand = PromotionCandidateInput {
            exploratory_campaign_id: "camp-exp-1".to_string(),
            exploratory_kind: CampaignKind::Exploratory,
            confirmation_campaign_id: None,
            confirmation_kind: None,
            hard_gates_passed: true,
            holdout_concordant: false,
            provenance: sample_complete_provenance(),
            replay_verified: true,
            evidence_expired: false,
        };
        assert_eq!(
            evaluate_promotion_eligibility(&cand).unwrap(),
            PromotionProposalState::AwaitingConfirmation
        );

        cand.confirmation_campaign_id = Some("camp-holdout-2".to_string());
        cand.confirmation_kind = Some(CampaignKind::HoldoutConfirmation);
        cand.holdout_concordant = true;
        let state = evaluate_promotion_eligibility(&cand).unwrap();
        assert_eq!(state, PromotionProposalState::AwaitingApproval);

        let actors = BTreeSet::from(["alice-author".to_string(), "bob-signer".to_string()]);
        assert!(authorize_champion_designation(state, "alice-author", "carol", &actors).is_err());
        assert!(authorize_champion_designation(state, "carol", "carol", &actors).is_err());
        let designated =
            authorize_champion_designation(state, "carol-approver", "dave-approver", &actors)
                .unwrap();
        assert_eq!(designated, PromotionProposalState::Designated);

        let suspended =
            suspend_champion_by_safety_principal(designated, "erin-safety", &actors).unwrap();
        assert_eq!(suspended, PromotionProposalState::Suspended);
    }

    #[test]
    fn phase11_adversarial_trial_suite_and_phase42_triggered_benchmark_governance() {
        let suite = run_adversarial_trial_suite();
        assert_eq!(suite.len(), 6);
        assert!(suite.iter().all(|o| o.fail_closed));

        assert_eq!(
            should_dispatch_benchmark_campaign(
                BenchmarkTriggerEvent::PullRequest,
                "fp1",
                "fp2",
                true
            ),
            CampaignDispatchDecision::SkipForbiddenOnPrOrMergeQueue
        );
        assert_eq!(
            should_dispatch_benchmark_campaign(BenchmarkTriggerEvent::MainPush, "fp1", "fp1", true),
            CampaignDispatchDecision::SkipNoTrigger
        );
        assert_eq!(
            should_dispatch_benchmark_campaign(
                BenchmarkTriggerEvent::MainPush,
                "fp1",
                "fp2",
                false
            ),
            CampaignDispatchDecision::ReferenceMigrationRequired
        );
        assert_eq!(
            should_dispatch_benchmark_campaign(BenchmarkTriggerEvent::MainPush, "fp1", "fp2", true),
            CampaignDispatchDecision::DispatchCampaign
        );

        let mut estimands = BTreeMap::new();
        estimands.insert(
            BenchmarkEstimand::E1ZeroShotPassAt1,
            BTreeMap::from([("C0".to_string(), 0.78)]),
        );
        estimands.insert(
            BenchmarkEstimand::E5CrossLanguageFreshTier,
            BTreeMap::from([("C0".to_string(), 0.85)]),
        );
        let rep = assemble_triggered_benchmark_report("blake3:fp2", true, estimands);
        assert_eq!(
            rep.suppressed_estimands,
            vec![BenchmarkEstimand::E5CrossLanguageFreshTier]
        );
        assert!(
            !rep.estimand_columns
                .contains_key(&BenchmarkEstimand::E5CrossLanguageFreshTier)
        );
    }

    #[test]
    fn phase86_longitudinal_scorecard_ledger_roundtrips_and_enforces_monotonicity() {
        use crate::mens::{
            MensEvalSpoke, SpokeAttemptOutcome, SpokeProblemInput, evaluate_mens_five_spokes,
        };

        let make_card = |cycle: usize, pass_flags: [bool; 5]| {
            let probs: Vec<SpokeProblemInput> = MensEvalSpoke::ALL
                .iter()
                .enumerate()
                .map(|(i, &spoke)| SpokeProblemInput {
                    case_id: format!("c_{}", spoke.as_str()),
                    spoke,
                    attempts: vec![SpokeAttemptOutcome {
                        case_id: format!("c_{}", spoke.as_str()),
                        spoke,
                        syntax_valid: true,
                        semantic_valid: pass_flags[i],
                        execution_passed: pass_flags[i],
                        diagnostic_summary: None,
                    }],
                })
                .collect();
            let card = evaluate_mens_five_spokes(
                &format!("mens-c{cycle}"),
                &format!("trusty://vox-dataset/snap-{cycle}"),
                &format!("trusty://blake3/adapter-{cycle}"),
                1,
                &probs,
            );
            MensLongitudinalScorecardV1::new(
                format!("run-{cycle}"),
                cycle,
                card,
                1_700_000_000_000 + cycle as i64,
            )
        };

        let mut ledger = MensLongitudinalLedger::default();
        ledger
            .append_run(make_card(0, [true, true, false, false, false]))
            .unwrap();
        ledger
            .append_run(make_card(1, [true, true, true, true, false]))
            .unwrap();
        ledger
            .append_run(make_card(2, [true, true, true, true, true]))
            .unwrap();

        assert!(ledger.verify_monotonic_progression().is_ok());
        let jsonl = ledger.to_jsonl().unwrap();
        let rehydrated = MensLongitudinalLedger::from_jsonl(&jsonl).unwrap();
        assert_eq!(rehydrated, ledger);

        let mut regressed = ledger.clone();
        regressed
            .append_run(make_card(3, [false, true, true, true, true]))
            .unwrap();
        assert!(regressed.verify_monotonic_progression().is_err());
    }
}
