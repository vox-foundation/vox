//! Machine-readable publication-worthiness policy (`contracts/scientia/*.yaml`) and evaluation.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

/// Default contract path relative to repository root.
pub const DEFAULT_CONTRACT_REL_PATH: &str =
    "contracts/scientia/publication-worthiness.default.yaml";

/// JSON Schema path relative to repository root (validated by `vox ci scientia-worthiness-contract`).
pub const CONTRACT_SCHEMA_REL_PATH: &str = "contracts/scientia/publication-worthiness.schema.json";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct PublicationWorthinessContract {
    pub version: u32,
    pub decision_labels: DecisionLabels,
    pub hard_red_lines: Vec<HardRedLine>,
    pub thresholds: Thresholds,
    pub weights: Weights,
    pub venue_profiles: std::collections::BTreeMap<String, VenueProfile>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DecisionLabels {
    pub publish: String,
    pub ask_for_evidence: String,
    #[serde(rename = "abstain_do_not_publish")]
    pub abstain_do_not_publish: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct HardRedLine {
    pub id: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Thresholds {
    pub claim_evidence_coverage_min: f64,
    pub artifact_replayability_min: f64,
    pub before_after_pair_integrity_min: f64,
    pub metadata_completeness_min: f64,
    pub ai_disclosure_compliance_exact: f64,
    pub publish_score_min: f64,
    pub abstain_score_max: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Weights {
    pub epistemic: f64,
    pub reproducibility: f64,
    pub novelty: f64,
    pub reliability: f64,
    pub metadata_policy: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct VenueProfile {
    pub description: String,
    pub required_checks: Vec<String>,
}

/// Inputs for [`evaluate_worthiness`]; typically deserialized from JSON.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct WorthinessInputs {
    /// Red-line ids that the caller attests were violated (must match enabled contract rows).
    #[serde(default)]
    pub red_line_violation_ids: Vec<String>,
    #[serde(default)]
    pub repeated_unresolved_contradiction: bool,
    pub claim_evidence_coverage: f64,
    pub artifact_replayability: f64,
    pub before_after_pair_integrity: f64,
    pub metadata_completeness: f64,
    pub ai_disclosure_compliance: f64,
    pub epistemic: f64,
    pub reproducibility: f64,
    pub novelty: f64,
    pub reliability: f64,
    pub metadata_policy: f64,
    /// When true, `mdl_gain_proxy` / `delta_signal_to_noise` (or human review) supports a real advance.
    #[serde(default)]
    pub meaningful_advance: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorthinessDecision {
    Publish,
    AskForEvidence,
    AbstainDoNotPublish,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorthinessEvaluation {
    pub decision: WorthinessDecision,
    pub decision_label: String,
    pub worthiness_score: f64,
    pub hard_metrics_ok: bool,
    pub reasons: Vec<String>,
}

/// Parse the worthiness contract YAML (repo file body).
pub fn load_contract_from_str(yaml: &str) -> Result<PublicationWorthinessContract> {
    serde_yaml::from_str(yaml).context("parse publication worthiness YAML")
}

/// Structural checks beyond JSON Schema (weights, ordering).
pub fn validate_contract_invariants(c: &PublicationWorthinessContract) -> Result<()> {
    let sum = c.weights.epistemic
        + c.weights.reproducibility
        + c.weights.novelty
        + c.weights.reliability
        + c.weights.metadata_policy;
    if (sum - 1.0).abs() > 1e-5 {
        return Err(anyhow!(
            "weights must sum to 1.0 (got {sum}; epistemic={}, repro={}, novelty={}, reliability={}, metadata_policy={})",
            c.weights.epistemic,
            c.weights.reproducibility,
            c.weights.novelty,
            c.weights.reliability,
            c.weights.metadata_policy
        ));
    }
    if c.thresholds.publish_score_min <= c.thresholds.abstain_score_max {
        return Err(anyhow!(
            "publish_score_min ({}) must be greater than abstain_score_max ({})",
            c.thresholds.publish_score_min,
            c.thresholds.abstain_score_max
        ));
    }
    for (name, v) in [
        (
            "claim_evidence_coverage_min",
            c.thresholds.claim_evidence_coverage_min,
        ),
        (
            "artifact_replayability_min",
            c.thresholds.artifact_replayability_min,
        ),
        (
            "before_after_pair_integrity_min",
            c.thresholds.before_after_pair_integrity_min,
        ),
        (
            "metadata_completeness_min",
            c.thresholds.metadata_completeness_min,
        ),
        (
            "ai_disclosure_compliance_exact",
            c.thresholds.ai_disclosure_compliance_exact,
        ),
        ("publish_score_min", c.thresholds.publish_score_min),
        ("abstain_score_max", c.thresholds.abstain_score_max),
    ] {
        range_01(name, v)?;
    }
    for (name, v) in [
        ("weight.epistemic", c.weights.epistemic),
        ("weight.reproducibility", c.weights.reproducibility),
        ("weight.novelty", c.weights.novelty),
        ("weight.reliability", c.weights.reliability),
        ("weight.metadata_policy", c.weights.metadata_policy),
    ] {
        range_01(name, v)?;
    }
    Ok(())
}

fn range_01(name: &str, v: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&v) {
        return Err(anyhow!("{name} must be in [0,1] (got {v})"));
    }
    Ok(())
}

fn label_for(decision: WorthinessDecision, c: &PublicationWorthinessContract) -> String {
    match decision {
        WorthinessDecision::Publish => c.decision_labels.publish.clone(),
        WorthinessDecision::AskForEvidence => c.decision_labels.ask_for_evidence.clone(),
        WorthinessDecision::AbstainDoNotPublish => c.decision_labels.abstain_do_not_publish.clone(),
    }
}

/// Apply the default rubric: red lines and low aggregate abstain; metric floors gate publish; else ask.
pub fn evaluate_worthiness(
    c: &PublicationWorthinessContract,
    inputs: &WorthinessInputs,
) -> WorthinessEvaluation {
    let mut reasons: Vec<String> = Vec::new();

    let enabled_ids: std::collections::HashSet<&str> = c
        .hard_red_lines
        .iter()
        .filter(|r| r.enabled)
        .map(|r| r.id.as_str())
        .collect();

    let mut active_violations: Vec<&str> = Vec::new();
    for id in &inputs.red_line_violation_ids {
        if enabled_ids.contains(id.as_str()) {
            active_violations.push(id.as_str());
        }
    }
    if !active_violations.is_empty() {
        reasons.push(format!(
            "enabled hard red-line violations: {}",
            active_violations.join(", ")
        ));
        return WorthinessEvaluation {
            decision: WorthinessDecision::AbstainDoNotPublish,
            decision_label: label_for(WorthinessDecision::AbstainDoNotPublish, c),
            worthiness_score: aggregate_score(c, inputs),
            hard_metrics_ok: hard_metrics_ok(c, inputs),
            reasons,
        };
    }

    if inputs.repeated_unresolved_contradiction {
        reasons.push("repeated_unresolved_contradiction".to_string());
        return WorthinessEvaluation {
            decision: WorthinessDecision::AbstainDoNotPublish,
            decision_label: label_for(WorthinessDecision::AbstainDoNotPublish, c),
            worthiness_score: aggregate_score(c, inputs),
            hard_metrics_ok: hard_metrics_ok(c, inputs),
            reasons,
        };
    }

    let score = aggregate_score(c, inputs);
    if score < c.thresholds.abstain_score_max {
        reasons.push(format!(
            "worthiness_score {score:.4} < abstain_score_max {}",
            c.thresholds.abstain_score_max
        ));
        return WorthinessEvaluation {
            decision: WorthinessDecision::AbstainDoNotPublish,
            decision_label: label_for(WorthinessDecision::AbstainDoNotPublish, c),
            worthiness_score: score,
            hard_metrics_ok: hard_metrics_ok(c, inputs),
            reasons,
        };
    }

    let hard_ok = hard_metrics_ok(c, inputs);
    if !hard_ok {
        reasons.push("one_or_more_hard_metric_minimums_not_met".to_string());
        return WorthinessEvaluation {
            decision: WorthinessDecision::AskForEvidence,
            decision_label: label_for(WorthinessDecision::AskForEvidence, c),
            worthiness_score: score,
            hard_metrics_ok: false,
            reasons,
        };
    }

    if score >= c.thresholds.publish_score_min && inputs.meaningful_advance {
        reasons.push("hard_metrics_ok_and_publish_score_with_meaningful_advance".to_string());
        WorthinessEvaluation {
            decision: WorthinessDecision::Publish,
            decision_label: label_for(WorthinessDecision::Publish, c),
            worthiness_score: score,
            hard_metrics_ok: true,
            reasons,
        }
    } else {
        if score < c.thresholds.publish_score_min {
            reasons.push(format!(
                "worthiness_score {score:.4} < publish_score_min {}",
                c.thresholds.publish_score_min
            ));
        }
        if !inputs.meaningful_advance {
            reasons.push("meaningful_advance_required_for_publish".to_string());
        }
        WorthinessEvaluation {
            decision: WorthinessDecision::AskForEvidence,
            decision_label: label_for(WorthinessDecision::AskForEvidence, c),
            worthiness_score: score,
            hard_metrics_ok: true,
            reasons,
        }
    }
}

fn hard_metrics_ok(c: &PublicationWorthinessContract, inputs: &WorthinessInputs) -> bool {
    inputs.claim_evidence_coverage >= c.thresholds.claim_evidence_coverage_min
        && inputs.artifact_replayability >= c.thresholds.artifact_replayability_min
        && inputs.before_after_pair_integrity >= c.thresholds.before_after_pair_integrity_min
        && inputs.metadata_completeness >= c.thresholds.metadata_completeness_min
        && (inputs.ai_disclosure_compliance - c.thresholds.ai_disclosure_compliance_exact).abs()
            < 1e-9
}

fn aggregate_score(c: &PublicationWorthinessContract, inputs: &WorthinessInputs) -> f64 {
    c.weights.epistemic * inputs.epistemic
        + c.weights.reproducibility * inputs.reproducibility
        + c.weights.novelty * inputs.novelty
        + c.weights.reliability * inputs.reliability
        + c.weights.metadata_policy * inputs.metadata_policy
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_contract() -> PublicationWorthinessContract {
        let yaml = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../contracts/scientia/publication-worthiness.default.yaml"
        ));
        let c = load_contract_from_str(yaml).expect("default contract");
        validate_contract_invariants(&c).expect("invariants");
        c
    }

    fn sample_inputs_publish_ready() -> WorthinessInputs {
        WorthinessInputs {
            red_line_violation_ids: vec![],
            repeated_unresolved_contradiction: false,
            claim_evidence_coverage: 0.95,
            artifact_replayability: 0.9,
            before_after_pair_integrity: 0.95,
            metadata_completeness: 0.95,
            ai_disclosure_compliance: 1.0,
            epistemic: 0.9,
            reproducibility: 0.9,
            novelty: 0.88,
            reliability: 0.9,
            metadata_policy: 0.95,
            meaningful_advance: true,
        }
    }

    #[test]
    fn default_contract_loads_and_evaluates_publish() {
        let c = sample_contract();
        let r = evaluate_worthiness(&c, &sample_inputs_publish_ready());
        assert_eq!(r.decision, WorthinessDecision::Publish);
        assert!(r.hard_metrics_ok);
    }

    #[test]
    fn red_line_abstains() {
        let c = sample_contract();
        let mut i = sample_inputs_publish_ready();
        i.red_line_violation_ids = vec!["fabricated_citation".to_string()];
        let r = evaluate_worthiness(&c, &i);
        assert_eq!(r.decision, WorthinessDecision::AbstainDoNotPublish);
    }

    #[test]
    fn low_score_abstains() {
        let c = sample_contract();
        let mut i = sample_inputs_publish_ready();
        i.epistemic = 0.1;
        i.reproducibility = 0.1;
        i.novelty = 0.1;
        i.reliability = 0.1;
        i.metadata_policy = 0.1;
        let r = evaluate_worthiness(&c, &i);
        assert_eq!(r.decision, WorthinessDecision::AbstainDoNotPublish);
    }

    #[test]
    fn missing_metric_floor_asks() {
        let c = sample_contract();
        let mut i = sample_inputs_publish_ready();
        i.claim_evidence_coverage = 0.5;
        let r = evaluate_worthiness(&c, &i);
        assert_eq!(r.decision, WorthinessDecision::AskForEvidence);
    }

    #[test]
    fn no_meaningful_advance_asks() {
        let c = sample_contract();
        let mut i = sample_inputs_publish_ready();
        i.meaningful_advance = false;
        let r = evaluate_worthiness(&c, &i);
        assert_eq!(r.decision, WorthinessDecision::AskForEvidence);
    }
}
