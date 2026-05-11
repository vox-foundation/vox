//! Unified orchestrator policy façade (D1–D10).
//!
//! [`OrchestratorPolicy::evaluate`] composes all ten decision axes into a single
//! [`PolicyDecision`] from a [`PolicyContext`]. Callers update context after each
//! loop iteration and pass it here; the policy returns all decisions atomically.
//!
//! All checks are pure: no async, no I/O.

use serde::{Deserialize, Serialize};

use crate::budget_gate::{BudgetDecision, BudgetGateConfig, OrchestratorBudgetGate};
use crate::cache_predictor::{CachePrediction, CachePredictor, CachePredictorConfig, CacheSignal};
use crate::calibration::{CalibrationConfig, CalibrationLoop};
use crate::circuit_breaker::{
    AlarmTier, CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState, TripReason,
};
use crate::compaction::CompactionStrategy;
use crate::compaction_trigger::{CompactionTrigger, CompactionTriggerConfig};
use crate::confidence_fusion::{ConfidenceFuser, FusionConfig, FusionDecision, FusionInputs};
use crate::planning::plan_mode_trigger::{
    PlanModeDecision, PlanModeSignal, PlanModeTrigger, PlanModeTriggerConfig,
};
use crate::privacy_classifier::{
    ClassificationSignals, PrivacyClassifier, PrivacyClassifierConfig, route_for_level,
};
use crate::privacy_router::{
    PrivacyLevel, PrivacyRouter, PrivacyRoutingDecision, PrivacyRoutingPolicy,
};
use crate::risk_matrix::{
    HitlAction, RiskDimensions, RiskGrade, RiskMatrix, RiskMatrixConfig,
    apply_agentos_mutation_risk,
};
use crate::subagent_dispatch::{DispatchConfig, DispatchDecision, DispatchRouter, DispatchSignal};
use crate::tier_cascade::{
    AlarmLevel, CompositeSignal, RoutingTier, TierCascadeConfig, TierCascadeRouter,
};

// ── Policy context ────────────────────────────────────────────────────────────

/// All signals the policy reads in a single evaluation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    // D6: circuit breaker state
    pub circuit_breaker: CircuitBreakerState,
    // D3: confidence fusion inputs (+ optional entropy from completion text)
    pub fusion_inputs: FusionInputs,
    // D1: task complexity 0–10
    pub complexity: u8,
    // D1: true when session budget is exhausted
    pub budget_token_fraction: f64,
    pub budget_cost_fraction: f64,
    // D2: plan mode signals
    pub plan_mode: PlanModeSignal,
    // D5+D9: risk dimensions for HITL
    pub risk: RiskDimensions,
    // D8: privacy classification signals
    pub privacy: ClassificationSignals,
    // D7: cache signal
    pub cache: CacheSignal,
    // D7: context utilization ratio for compaction
    pub context_utilization: f64,
    // D4: dispatch signal
    pub dispatch: DispatchSignal,
    /// When set (e.g. last MCP tool `aci.mutation_kind`), merges AgentOS signals into risk scoring.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agentos_last_mutation_kind: Option<String>,
}

impl Default for PolicyContext {
    fn default() -> Self {
        Self {
            circuit_breaker: CircuitBreakerState::default(),
            fusion_inputs: FusionInputs {
                evidence_quality: 0.75,
                citation_coverage: 0.75,
                source_diversity_norm: 0.4,
                contradiction_ratio: 0.0,
                entropy_score: 0.7,
            },
            complexity: 5,
            budget_token_fraction: 0.0,
            budget_cost_fraction: 0.0,
            plan_mode: PlanModeSignal::default(),
            risk: RiskDimensions::default(),
            privacy: ClassificationSignals::default(),
            cache: CacheSignal {
                prefix_overlap_tokens: 700,
                total_context_tokens: 1000,
            },
            context_utilization: 0.5,
            dispatch: DispatchSignal::default(),
            agentos_last_mutation_kind: None,
        }
    }
}

// ── Policy decision ───────────────────────────────────────────────────────────

/// Atomic bundle of all D1–D10 decisions for one evaluation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    // D6
    pub circuit_trip: Option<TripReason>,
    pub alarm_tier: AlarmTier,
    // D3
    pub fusion_score: f64,
    pub fusion_decision: FusionDecision,
    // D1
    pub routing_tier: RoutingTier,
    // D7 budget
    pub budget_decision: BudgetDecision,
    // D2
    pub plan_mode: PlanModeDecision,
    // D5+D9
    pub risk_score: f64,
    pub risk_grade: RiskGrade,
    pub hitl_action: HitlAction,
    // D8
    pub privacy_level: PrivacyLevel,
    pub privacy_routing: PrivacyRoutingDecision,
    // D7 cache
    pub cache_prediction: CachePrediction,
    // D7 compaction
    pub compaction_strategy: CompactionStrategy,
    // D4
    pub dispatch_decision: DispatchDecision,
}

// ── Config bundle ─────────────────────────────────────────────────────────────

/// Aggregated config for all D1–D10 modules. Each field uses the module's own Default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorPolicyConfig {
    pub circuit_breaker: CircuitBreakerConfig,
    pub fusion: FusionConfig,
    pub tier_cascade: TierCascadeConfig,
    pub plan_mode_trigger: PlanModeTriggerConfig,
    pub risk_matrix: RiskMatrixConfig,
    pub budget_gate: BudgetGateConfig,
    pub cache_predictor: CachePredictorConfig,
    pub compaction_trigger: CompactionTriggerConfig,
    pub calibration: CalibrationConfig,
    pub dispatch: DispatchConfig,
}

impl Default for OrchestratorPolicyConfig {
    fn default() -> Self {
        Self {
            circuit_breaker: CircuitBreakerConfig::default(),
            fusion: FusionConfig::default(),
            tier_cascade: TierCascadeConfig::default(),
            plan_mode_trigger: PlanModeTriggerConfig::default(),
            risk_matrix: RiskMatrixConfig::default(),
            budget_gate: BudgetGateConfig::default(),
            cache_predictor: CachePredictorConfig::default(),
            compaction_trigger: CompactionTriggerConfig::default(),
            calibration: CalibrationConfig::default(),
            dispatch: DispatchConfig::default(),
        }
    }
}

// ── Façade ────────────────────────────────────────────────────────────────────

/// Single entry-point for all orchestrator policy decisions.
///
/// Construct once and call [`evaluate`] after each loop iteration.
/// The [`CalibrationLoop`] is stateful — it accumulates observations across calls.
pub struct OrchestratorPolicy {
    cb: CircuitBreaker,
    fuser: ConfidenceFuser,
    tier: TierCascadeRouter,
    plan_trigger: PlanModeTrigger,
    risk: RiskMatrix,
    budget: OrchestratorBudgetGate,
    cache: CachePredictor,
    compaction: CompactionTrigger,
    calibration: CalibrationLoop,
    dispatch: DispatchRouter,
    privacy_classifier: PrivacyClassifier,
    privacy_router: PrivacyRouter,
}

impl OrchestratorPolicy {
    pub fn new(config: OrchestratorPolicyConfig) -> Self {
        Self {
            cb: CircuitBreaker::new(config.circuit_breaker),
            fuser: ConfidenceFuser::new(config.fusion),
            tier: TierCascadeRouter::new(config.tier_cascade),
            plan_trigger: PlanModeTrigger::new(config.plan_mode_trigger),
            risk: RiskMatrix::new(config.risk_matrix),
            budget: OrchestratorBudgetGate::new(config.budget_gate),
            cache: CachePredictor::new(config.cache_predictor),
            compaction: CompactionTrigger::new(config.compaction_trigger),
            calibration: CalibrationLoop::new(config.calibration),
            dispatch: DispatchRouter::new(config.dispatch),
            privacy_classifier: PrivacyClassifier::new(PrivacyClassifierConfig),
            privacy_router: PrivacyRouter::new(PrivacyRoutingPolicy::default()),
        }
    }

    /// Evaluate all D1–D10 decisions from `ctx` and return them as a [`PolicyDecision`].
    ///
    /// Also records the fusion score into the calibration loop so drift detection
    /// accumulates across loop iterations automatically.
    #[must_use]
    pub fn evaluate(&mut self, ctx: &PolicyContext) -> PolicyDecision {
        // D6 — circuit breaker
        let circuit_trip = self.cb.should_trip(&ctx.circuit_breaker);
        let alarm_tier = self.cb.check_tier(&ctx.circuit_breaker);

        // D3 — confidence fusion
        let (fusion_score, fusion_decision) = self.fuser.evaluate(&ctx.fusion_inputs);

        // D1 — tier cascade (needs budget + alarm)
        let budget_decision = self
            .budget
            .evaluate(ctx.budget_token_fraction, ctx.budget_cost_fraction);
        let tier_signal = CompositeSignal {
            complexity: ctx.complexity,
            alarm_level: AlarmLevel::from(alarm_tier),
            confidence: fusion_score,
            budget_exhausted: budget_decision.is_exhausted(),
        };
        let routing_tier = self.tier.select(&tier_signal);

        // D2 — plan mode trigger
        let plan_mode = self.plan_trigger.decide(&ctx.plan_mode);

        // D5+D9 — risk matrix (optional AgentOS mutation_kind overlay)
        let mut risk_dims = ctx.risk.clone();
        if let Some(ref mk) = ctx.agentos_last_mutation_kind {
            apply_agentos_mutation_risk(&mut risk_dims, mk.as_str());
        }
        let (risk_score, risk_grade, hitl_action) = self.risk.evaluate(&risk_dims);

        // D8 — privacy
        let privacy_level = self.privacy_classifier.classify(&ctx.privacy);
        let privacy_routing = route_for_level(&self.privacy_router, privacy_level);

        // D7 cache + compaction
        let cache_prediction = self.cache.predict(&ctx.cache);
        let compaction_strategy = self.compaction.select(ctx.context_utilization);

        // D4 — dispatch
        let mut dispatch_sig = ctx.dispatch.clone();
        dispatch_sig.budget_exhausted = budget_decision.is_exhausted();
        let dispatch_decision = self.dispatch.route(&dispatch_sig);

        // D10 — calibration (record fusion score for drift tracking)
        let _ = self.calibration.observe(fusion_score);

        PolicyDecision {
            circuit_trip,
            alarm_tier,
            fusion_score,
            fusion_decision,
            routing_tier,
            budget_decision,
            plan_mode,
            risk_score,
            risk_grade,
            hitl_action,
            privacy_level,
            privacy_routing,
            cache_prediction,
            compaction_strategy,
            dispatch_decision,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> OrchestratorPolicy {
        OrchestratorPolicy::new(OrchestratorPolicyConfig::default())
    }

    #[test]
    fn default_context_produces_sane_decisions() {
        let mut p = policy();
        let d = p.evaluate(&PolicyContext::default());
        assert!(d.circuit_trip.is_none());
        assert_eq!(d.alarm_tier, AlarmTier::None);
        assert!(d.fusion_score > 0.0 && d.fusion_score <= 1.0);
        assert_eq!(d.routing_tier, RoutingTier::Standard);
        assert_eq!(d.plan_mode, PlanModeDecision::React);
        assert_eq!(d.hitl_action, HitlAction::Proceed);
        assert_eq!(d.privacy_routing, PrivacyRoutingDecision::Redact); // default = Internal
        assert_eq!(d.dispatch_decision, DispatchDecision::Inline);
    }

    #[test]
    fn doom_loop_state_trips_circuit_breaker() {
        let mut p = policy();
        let ctx = PolicyContext {
            circuit_breaker: CircuitBreakerState {
                no_progress_loops: 3,
                ..Default::default()
            },
            ..Default::default()
        };
        let d = p.evaluate(&ctx);
        assert_eq!(d.circuit_trip, Some(TripReason::NoProgress));
    }

    #[test]
    fn exhausted_budget_forces_economy_tier() {
        let mut p = policy();
        let ctx = PolicyContext {
            budget_token_fraction: 0.97,
            ..Default::default()
        };
        let d = p.evaluate(&ctx);
        assert_eq!(d.routing_tier, RoutingTier::Economy);
        assert!(d.budget_decision.is_exhausted());
    }

    #[test]
    fn regulated_content_routes_local_only() {
        let mut p = policy();
        let ctx = PolicyContext {
            privacy: ClassificationSignals {
                regulated_marker_detected: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let d = p.evaluate(&ctx);
        assert_eq!(d.privacy_level, PrivacyLevel::Regulated);
        assert_eq!(d.privacy_routing, PrivacyRoutingDecision::LocalOnly);
    }

    #[test]
    fn high_complexity_spawns_subagent() {
        let mut p = policy();
        let ctx = PolicyContext {
            complexity: 8,
            dispatch: DispatchSignal {
                complexity: 8,
                ..Default::default()
            },
            ..Default::default()
        };
        let d = p.evaluate(&ctx);
        assert_eq!(d.dispatch_decision, DispatchDecision::Spawn);
    }

    #[test]
    fn critical_risk_blocks_and_escalates() {
        let mut p = policy();
        let ctx = PolicyContext {
            risk: RiskDimensions {
                irreversibility: 0.95,
                blast_radius: 0.0,
                compliance_exposure: 0.0,
                confidence_deficit: 0.0,
            },
            ..Default::default()
        };
        let d = p.evaluate(&ctx);
        assert_eq!(d.risk_grade, RiskGrade::Critical);
        assert_eq!(d.hitl_action, HitlAction::BlockAndEscalate);
    }

    #[test]
    fn agentos_external_mutation_boosts_risk_score_over_read_only() {
        let mut p_base = policy();
        let base = p_base.evaluate(&PolicyContext {
            agentos_last_mutation_kind: Some("read_only".into()),
            ..Default::default()
        });
        let mut p_ext = policy();
        let boosted = p_ext.evaluate(&PolicyContext {
            agentos_last_mutation_kind: Some("external_side_effect".into()),
            ..Default::default()
        });
        assert!(
            boosted.risk_score > base.risk_score,
            "base={} boosted={}",
            base.risk_score,
            boosted.risk_score
        );
    }

    #[test]
    fn evaluate_is_stateful_across_calls() {
        let mut p = policy();
        // First call
        let _ = p.evaluate(&PolicyContext::default());
        // Second call — calibration loop now has 1 observation, no crash
        let d2 = p.evaluate(&PolicyContext::default());
        assert!(d2.fusion_score >= 0.0);
    }
}
