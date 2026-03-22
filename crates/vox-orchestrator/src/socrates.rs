//! Socrates task gate: evidence-weighted confidence against shared [`vox_socrates_policy`] thresholds.

use serde::{Deserialize, Serialize};
use vox_socrates_policy::{ConfidencePolicy, RiskBand, RiskDecision};

/// Structured evidence / risk hints attached to an [`crate::types::AgentTask`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocratesTaskContext {
    /// Operational risk tier label (logged to traces only).
    #[serde(default)]
    pub risk_budget: String,
    /// When true, `required_citations` constrains completion confidence.
    #[serde(default)]
    pub factual_mode: bool,
    /// Minimum grounded citations expected before claiming completion.
    #[serde(default)]
    pub required_citations: u8,
    /// Citations the agent reports having satisfied.
    #[serde(default)]
    pub evidence_count: u8,
    /// Unresolved contradictions the agent is aware of.
    #[serde(default)]
    pub contradiction_hints: u8,
}

/// Result of applying the completion gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocratesGateOutcome {
    /// Triaged answer / ask / abstain.
    pub decision: RiskDecision,
    /// Normalized confidence in `[0, 1]`.
    pub confidence: f64,
    /// Contradiction mass in `[0, 1]`.
    pub contradiction_ratio: f64,
    /// Discrete band for dashboards.
    pub band: RiskBand,
}

/// Evaluate structured task metadata against `policy`.
#[must_use]
pub fn evaluate_socrates_gate(
    ctx: &SocratesTaskContext,
    policy: &ConfidencePolicy,
) -> SocratesGateOutcome {
    let contradiction_ratio = match ctx.contradiction_hints {
        0 => 0.0,
        1 => 0.15,
        2 => 0.28,
        n => ((n as f64) * 0.22).min(1.0),
    };

    let coverage = if ctx.required_citations == 0 {
        1.0
    } else {
        (f64::from(ctx.evidence_count) / f64::from(ctx.required_citations)).clamp(0.0, 1.0)
    };

    let mut confidence = coverage;
    if ctx.factual_mode && ctx.required_citations > 0 && ctx.evidence_count < ctx.required_citations
    {
        confidence *= policy.abstain_threshold;
    }

    let band = policy.classify_risk(confidence, contradiction_ratio);
    let decision = policy.evaluate_risk_decision(confidence, contradiction_ratio);

    SocratesGateOutcome {
        decision,
        confidence,
        contradiction_ratio,
        band,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factual_under_cited_abstains() {
        let p = ConfidencePolicy::default();
        let ctx = SocratesTaskContext {
            factual_mode: true,
            required_citations: 3,
            evidence_count: 0,
            ..Default::default()
        };
        let o = evaluate_socrates_gate(&ctx, &p);
        assert_eq!(o.decision, RiskDecision::Abstain);
    }
}
