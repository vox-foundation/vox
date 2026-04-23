use super::*;
use crate::publication_worthiness::WorthinessInputs;

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

fn merge_socrates_aggregate(
    inputs: &mut WorthinessInputs,
    agg: &SocratesAggregateSnapshot,
    h: &crate::scientia_heuristics::ScientiaHeuristics,
) {
    if agg.parsed_metadata_rows == 0 && agg.sample_size == 0 {
        return;
    }
    let conf = agg.mean_confidence_estimate.clamp(0.0, 1.0);
    let risk = agg.mean_hallucination_risk_proxy.clamp(0.0, 1.0);
    let cr = agg.mean_contradiction_ratio.clamp(0.0, 1.0);

    let epistemic_signal = conf * (1.0 - 0.88 * risk);
    inputs.epistemic = clamp01(0.4 * inputs.epistemic + 0.6 * epistemic_signal);
    if inputs.claim_evidence_coverage >= h.worthiness_contradiction_coverage_gate {
        inputs.epistemic = clamp01(inputs.epistemic * (1.0 - 0.45 * cr));
    }

    let reliability_signal = (1.0 - risk).max(0.0);
    inputs.reliability = clamp01(0.35 * inputs.reliability + 0.65 * reliability_signal);

    if inputs.claim_evidence_coverage >= h.worthiness_contradiction_coverage_gate {
        if agg.abstain_count >= 2 && cr > 0.2 {
            inputs.repeated_unresolved_contradiction = true;
        }
        if cr > 0.38
            && !inputs
                .red_line_violation_ids
                .iter()
                .any(|x| x == "unresolved_socrates_contradiction")
        {
            inputs
                .red_line_violation_ids
                .push("unresolved_socrates_contradiction".to_string());
        }
    }
}

/// Fold structured evidence into heuristic [`WorthinessInputs`] (order: heuristic base first, then evidence).
#[must_use]
pub fn apply_scientia_evidence(
    mut inputs: WorthinessInputs,
    evidence: &ScientiaEvidenceContext,
    h: &crate::scientia_heuristics::ScientiaHeuristics,
) -> WorthinessInputs {
    if let Some(ref agg) = evidence.socrates_aggregate {
        merge_socrates_aggregate(&mut inputs, agg, h);
    }

    if let Some(ref g) = evidence.eval_gate {
        if g.passed {
            inputs.reliability = clamp01(inputs.reliability.max(0.9));
            inputs.metadata_policy = clamp01(inputs.metadata_policy.max(0.87));
        } else {
            inputs.reliability = clamp01(inputs.reliability * 0.68);
        }
    }

    if let Some(ref b) = evidence.benchmark
        && b.pair_complete
        && b.baseline_run_id
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
        && b.candidate_run_id
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
    {
        inputs.before_after_pair_integrity = clamp01(inputs.before_after_pair_integrity.max(0.9));
    }

    if evidence.human_ai_disclosure_complete {
        inputs.ai_disclosure_compliance = 1.0;
    }

    if evidence.human_meaningful_advance {
        let socrates_ok = evidence.socrates_aggregate.as_ref().is_some_and(|a| {
            a.parsed_metadata_rows > 0
                && a.mean_hallucination_risk_proxy < 0.36
                && a.mean_contradiction_ratio < 0.26
        });
        let gate_ok = evidence.eval_gate.as_ref().is_none_or(|g| g.passed);
        let bench_ok = evidence.benchmark.as_ref().is_none_or(|b| {
            b.pair_complete
                && b.baseline_run_id
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
                && b.candidate_run_id
                    .as_ref()
                    .is_some_and(|s| !s.trim().is_empty())
        });
        if socrates_ok && gate_ok && bench_ok {
            inputs.meaningful_advance = true;
        }
    }

    inputs
}
