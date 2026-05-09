//! Claim extraction. Phase 0a STUB — returns empty Vec.
//!
//! Phase 1 replaces this with `vox-claim-extractor` crate calls
//! (SciClaims architecture: VeriScore atomicity gate → atomic decomposition
//! → XGrammar-constrained emission → MiniCheck verification → calibrated
//! ABSTAIN). See:
//!   docs/src/architecture/scientia-self-publication-finalization-plan-2026.md §3.2

use serde::{Deserialize, Serialize};

/// One extracted research claim.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    /// The claim text itself.
    pub text: String,
    /// Stable hash assigned downstream (FNV-1a of `text`).
    pub claim_id: u64,
    /// Heuristic flag: claim contains a numeric value.
    pub is_numeric: bool,
    /// Heuristic flag: claim mentions a recent date or "recently" / "latest".
    pub is_recent: bool,
    /// Heuristic flag: claim mentions a named entity / event.
    pub is_named_event: bool,
}

/// Extract claims from a query.
///
/// **PHASE_0a_STUB**: returns `Vec::new()`. No LLM invocation. Phase 1 wires
/// this to `vox-claim-extractor`.
///
/// # Parameters
/// - `_query`: the source text (in Phase 0a, this is the user query; Phase 1
///   will accept arbitrary documents).
/// - `_endpoint`, `_api_key`, `_model`, `_max_tokens`: ignored in Phase 0a.
pub async fn extract_claims_with_model(
    _query: &str,
    _endpoint: Option<&str>,
    _api_key: Option<&str>,
    _model: Option<&str>,
    _max_tokens: Option<u32>,
) -> Vec<Claim> {
    // PHASE_0a_STUB: replaced by vox-claim-extractor in Phase 1.
    Vec::new()
}
