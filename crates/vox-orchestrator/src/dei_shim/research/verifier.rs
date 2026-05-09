//! Claim verification. Phase 0a STUB — returns empty Vec.
//!
//! Phase 1 replaces this with MiniCheck-FT5 (770M T5) wrapped as a Vox plugin,
//! plus calibrated abstention (temperature-scale logits → ABSTAIN below τ).
//! See: docs/src/architecture/scientia-self-publication-finalization-plan-2026.md §3.2

use std::fmt;

use serde::{Deserialize, Serialize};

use super::claims::Claim;
use super::provider::ProviderRegistry;

/// Verifier configuration. Phase 0a — fields are placeholders; Phase 1
/// adds calibration parameters (`abstain_threshold`, `temperature`,
/// `escalation_endpoint`, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VerifierConfig {
    pub abstain_threshold: Option<f32>,
    pub model: Option<String>,
    /// NLI model ID used for claim classification.
    /// Defaults to the registry FALLBACK_NLI_MODEL_ID constant; overridden
    /// in `verifier_config_for_research_run` when registry resolves a better model.
    pub nli_model_id: String,
}

/// Verdict label per SciFact taxonomy (as used in stages.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Supported,
    Contradicted,
    Contested,
    Unverified,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Supported => write!(f, "supported"),
            Self::Contradicted => write!(f, "contradicted"),
            Self::Contested => write!(f, "contested"),
            Self::Unverified => write!(f, "unverified"),
        }
    }
}

/// Type of evidence span linkage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanType {
    Supporting,
    Contradicting,
    Background,
}

impl fmt::Display for SpanType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Supporting => write!(f, "supporting"),
            Self::Contradicting => write!(f, "contradicting"),
            Self::Background => write!(f, "background"),
        }
    }
}

/// One evidence span linking a claim to a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSpan {
    pub source_id: i64,
    pub text: String,
    pub span_type: SpanType,
}

/// Per-claim verification verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimVerdict {
    pub claim: Claim,
    pub verdict: Verdict,
    pub confidence: f64,
    pub supporting_count: usize,
    pub contradicting_count: usize,
    pub evidence_spans: Vec<EvidenceSpan>,
}

/// Verify a batch of claims against retrieved evidence.
///
/// **PHASE_0a_STUB**: returns `Vec::new()`. Phase 1 wires this to
/// `vox-claim-extractor`'s MiniCheck-backed verifier.
pub async fn verify_claims_with_config(
    _claims: &[Claim],
    _query: &str,
    _registry: &ProviderRegistry,
    _config: &VerifierConfig,
    _endpoint: Option<&str>,
    _api_key: Option<&str>,
) -> Vec<ClaimVerdict> {
    // PHASE_0a_STUB: replaced by vox-claim-extractor in Phase 1.
    Vec::new()
}
