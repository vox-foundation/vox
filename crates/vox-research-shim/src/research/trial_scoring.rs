//! Four-axis trial scoring for [`super::types::ResearchResult`] (`07-12`, `SCORE-01..04`).
//!
//! Computes:
//! 1. **Factuality (`SCORE-01`)**: verified-supported claim fraction (`Option<f64>` — `None` when
//!    zero claims were verified).
//! 2. **Citation Precision (`SCORE-02`)**: fraction of emitted citations backed by verifier
//!    evidence spans (`Option<f64>` — `None` when no citation audit ran or zero citations checked).
//! 3. **Completeness / Judge Quality (`SCORE-03`, `SCORE-04`)**: normalized `[0.0, 1.0]` judge
//!    score, strictly `None` when `judge_error.is_some()` so a failed judge call is recorded as
//!    missing rather than a synthetic `0.0`.
//! 4. **Abstention Calibration (`SCORE-04`)**: whether the run appropriately abstained via
//!    [`super::orchestrator::ABSTENTION_MARKER`] on unanswerable/low-evidence queries vs answered
//!    on answerable queries.

use serde::{Deserialize, Serialize};

use super::orchestrator::ABSTENTION_MARKER;
use super::types::{ResearchResult, Verdict};

/// Four-axis evaluation of a single research trial sample (`07-12`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrialSampleEvaluation {
    /// Fraction of verified claims with [`Verdict::Supported`] (`None` if 0 verified claims).
    pub factuality: Option<f64>,
    /// Fraction of checked citations backed by evidence spans (`None` if 0 checked citations).
    pub citation_precision: Option<f64>,
    /// Judge quality normalized to `[0.0, 1.0]` (`None` when `judge_error.is_some()`).
    pub completeness_quality: Option<f64>,
    /// Whether the answer began with [`ABSTENTION_MARKER`].
    pub abstained: bool,
    /// `Some(1.0)` if `abstained == should_abstain`, `Some(0.0)` otherwise.
    pub abstention_calibration: f64,
    /// Composite primary score across available non-missing axes (`None` if all axes missing).
    pub composite_score: Option<f64>,
}

/// Score a [`ResearchResult`] across the 4 trial axes (`07-12`).
///
/// Missing axes (`judge_error.is_some()`, zero verified claims, or zero checked citations) are
/// represented as `None` — never coerced to `0.0`.
#[must_use]
pub fn score_research_result(
    result: &ResearchResult,
    should_abstain: bool,
) -> TrialSampleEvaluation {
    let meta = &result.research_metadata;

    // 1. Factuality: Supported / (Supported + Contradicted + Unsupported) among verified claims
    let mut verified_denom = 0usize;
    let mut supported_num = 0usize;
    for cv in &meta.claim_verdicts {
        match cv.verdict {
            Verdict::Supported => {
                verified_denom += 1;
                supported_num += 1;
            }
            Verdict::Contradicted | Verdict::Contested | Verdict::Unverified => {
                verified_denom += 1;
            }
        }
    }
    let factuality = if verified_denom > 0 {
        Some((supported_num as f64) / (verified_denom as f64))
    } else {
        None
    };

    // 2. Citation precision
    let citation_precision = meta.citation_audit.as_ref().and_then(|audit| {
        if audit.checked_citations > 0 {
            Some(audit.precision.clamp(0.0, 1.0))
        } else {
            None
        }
    });

    // 3. Completeness / Judge quality (None when judge_error is present)
    let completeness_quality = if meta.judge_error.is_some() {
        None
    } else {
        Some(((meta.quality_score as f64) / 100.0).clamp(0.0, 1.0))
    };

    // 4. Abstention calibration
    let abstained = result.answer.trim_start().starts_with(ABSTENTION_MARKER);
    let abstention_calibration = if abstained == should_abstain {
        1.0
    } else {
        0.0
    };

    // Composite over present axes
    let mut sum = 0.0;
    let mut count = 0usize;
    for axis in [factuality, citation_precision, completeness_quality]
        .into_iter()
        .flatten()
    {
        sum += axis;
        count += 1;
    }
    let composite_score = if should_abstain {
        Some(abstention_calibration)
    } else if count > 0 {
        Some((sum / (count as f64)) * abstention_calibration)
    } else {
        None
    };

    TrialSampleEvaluation {
        factuality,
        citation_precision,
        completeness_quality,
        abstained,
        abstention_calibration,
        composite_score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::claims::Claim;
    use crate::research::metering::RunUsage;
    use crate::research::types::{
        CitationAuditResult, ClaimVerdict, ResearchMetadata, RetrievalDiagnostics, RoutingTier,
    };

    fn fixture_result() -> ResearchResult {
        ResearchResult {
            answer: "Vox actors communicate over typed channels [1].".to_string(),
            sources: Vec::new(),
            citations: Vec::new(),
            research_metadata: ResearchMetadata {
                session_id: 1,
                duration_ms: 400,
                provider: "test".to_string(),
                routing_tier: RoutingTier::DeepResearch,
                confidence: 0.85,
                subquery_count: 2,
                source_count: 3,
                claim_verdicts: vec![
                    ClaimVerdict {
                        claim: Claim {
                            text: "c1".to_string(),
                            claim_id: 1,
                            is_numeric: false,
                            is_recent: false,
                            is_named_event: false,
                            salience_score: 0.5,
                        },
                        verdict: Verdict::Supported,
                        confidence: 0.9,
                        supporting_count: 1,
                        contradicting_count: 0,
                        evidence_spans: Vec::new(),
                        resample_stability: 1.0,
                    },
                    ClaimVerdict {
                        claim: Claim {
                            text: "c2".to_string(),
                            claim_id: 2,
                            is_numeric: false,
                            is_recent: false,
                            is_named_event: false,
                            salience_score: 0.5,
                        },
                        verdict: Verdict::Contradicted,
                        confidence: 0.8,
                        supporting_count: 0,
                        contradicting_count: 1,
                        evidence_spans: Vec::new(),
                        resample_stability: 1.0,
                    },
                ],
                retrieval_diagnostics: RetrievalDiagnostics::default(),
                quality_score: 80,
                planner_degraded: false,
                competence: None,
                self_verification: None,
                citation_audit: Some(CitationAuditResult {
                    checked_citations: 4,
                    supported_citations: 3,
                    unsupported_citation_indices: vec![3],
                    precision: 0.75,
                    supports: Vec::new(),
                }),
                corroboration_counts: Vec::new(),
                wave_count: 1,
                wave_stability: Some(1.0),
                low_grounding_evidence: false,
                subqueries: Vec::new(),
                synthesis_model: "test-model".to_string(),
                judge_error: None,
                served_from_cache: false,
                claims_extracted_count: 2,
                claims_verified_count: 2,
                usage: RunUsage::default(),
            },
        }
    }

    #[test]
    fn four_axis_scoring_computes_present_axes_and_preserves_missing_on_judge_error() {
        let res = fixture_result();
        let eval = score_research_result(&res, false);
        assert_eq!(eval.factuality, Some(0.5));
        assert_eq!(eval.citation_precision, Some(0.75));
        assert_eq!(eval.completeness_quality, Some(0.80));
        assert!(!eval.abstained);
        assert_eq!(eval.abstention_calibration, 1.0);

        // When judge_error is set, completeness_quality must be None, not 0.0
        let mut judge_failed = res.clone();
        judge_failed.research_metadata.quality_score = 0;
        judge_failed.research_metadata.judge_error = Some("timeout".to_string());
        let eval_missing = score_research_result(&judge_failed, false);
        assert_eq!(eval_missing.completeness_quality, None);
        assert_eq!(eval_missing.composite_score, Some((0.5 + 0.75) / 2.0));
    }

    #[test]
    fn abstention_marker_calibrates_unanswerable_queries() {
        let mut res = fixture_result();
        res.answer = format!("{ABSTENTION_MARKER}\nNo primary sources exist for this query.");
        let eval = score_research_result(&res, true);
        assert!(eval.abstained);
        assert_eq!(eval.abstention_calibration, 1.0);
        assert_eq!(eval.composite_score, Some(1.0));
    }
}
