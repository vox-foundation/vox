//! Socrates task gate: evidence-weighted confidence against shared [`vox_socrates_policy`] thresholds.

use serde::{Deserialize, Serialize};
use vox_socrates_policy::{ConfidencePolicy, RiskBand, RiskDecision};

/// Context-store key prefix shared with MCP (`vox_chat_message` stores JSON here per session).
#[must_use]
pub fn session_retrieval_envelope_key(session_id: &str) -> String {
    format!("retrieval_envelope:{session_id}")
}

/// Retrieval telemetry shape persisted by MCP (extra fields ignored on deserialize).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRetrievalEnvelope {
    #[serde(default)]
    pub retrieval_tier: String,
    #[serde(default)]
    pub memory_hit_count: usize,
    #[serde(default)]
    pub knowledge_hit_count: usize,
    /// Ingested `search_document_chunks` rows (RAG corpus) surfaced by MCP retrieval.
    #[serde(default)]
    pub chunk_hit_count: usize,
    #[serde(default)]
    pub used_vector: bool,
    #[serde(default)]
    pub used_bm25: bool,
    #[serde(default)]
    pub used_lexical_fallback: bool,
    #[serde(default)]
    pub contradiction_count: usize,
}

impl SessionRetrievalEnvelope {
    /// Refreshes evidence fields from a new retrieval pass while preserving budget hints
    /// (`risk_budget`, `factual_mode`) from a prior task context.
    #[must_use]
    pub fn merge_into(self, prev: Option<SocratesTaskContext>) -> SocratesTaskContext {
        let fresh = self.to_task_context();
        let Some(mut base) = prev else {
            return fresh;
        };
        base.retrieval_tier = fresh.retrieval_tier;
        base.retrieval_used_vector = fresh.retrieval_used_vector;
        base.retrieval_used_lexical_fallback = fresh.retrieval_used_lexical_fallback;
        base.required_citations = fresh.required_citations;
        base.evidence_count = fresh.evidence_count;
        base.contradiction_hints = fresh.contradiction_hints;
        base
    }

    /// Maps envelope fields into task-level Socrates evidence (same contract as MCP bridging).
    #[must_use]
    pub fn to_task_context(&self) -> SocratesTaskContext {
        let doc_graph_hits = self
            .knowledge_hit_count
            .saturating_add(self.chunk_hit_count);
        let required_citations = if self.memory_hit_count == 0 && doc_graph_hits == 0 {
            1_u8
        } else {
            0_u8
        };
        let evidence_total = self
            .memory_hit_count
            .saturating_add(doc_graph_hits)
            .min(u8::MAX as usize) as u8;
        SocratesTaskContext {
            risk_budget: "normal".to_string(),
            factual_mode: true,
            required_citations,
            evidence_count: evidence_total,
            contradiction_hints: self.contradiction_count.min(u8::MAX as usize) as u8,
            retrieval_tier: Some(self.retrieval_tier.clone()),
            retrieval_used_vector: self.used_vector,
            retrieval_used_lexical_fallback: self.used_lexical_fallback,
        }
    }
}

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
    /// Retrieval tier label (`hybrid`, `bm25`, `lexical_fallback`, ...), when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval_tier: Option<String>,
    /// True when vector evidence contributed to the retrieval context.
    #[serde(default)]
    pub retrieval_used_vector: bool,
    /// True when retrieval had to fall back to lexical substring matching.
    #[serde(default)]
    pub retrieval_used_lexical_fallback: bool,
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
    if matches!(ctx.retrieval_tier.as_deref(), Some("hybrid") | Some("bm25")) {
        confidence = (confidence + 0.05).clamp(0.0, 1.0);
    }
    if ctx.retrieval_used_lexical_fallback {
        confidence = (confidence - 0.08).clamp(0.0, 1.0);
    }
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
