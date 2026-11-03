//! Evaluation and false-positive measurement for mention extraction and resolution.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::resolver::GroundTruthRegistry;
use super::types::{ExtractedMention, MentionKind, MentionRole, ResolutionStatus};

#[derive(Debug, Deserialize, Serialize)]
pub struct EvaluationSample {
    pub id: String,
    pub doc_path: String,
    pub raw_text: String,
    pub kind: MentionKind,
    pub expected_role: MentionRole,
    pub expected_status: ResolutionStatus,
    pub is_true_deficit: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EvaluationDataset {
    pub schema_version: u32,
    pub description: String,
    pub samples: Vec<EvaluationSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalReport {
    pub total_samples: usize,
    pub true_positives: usize,
    pub false_positives: usize,
    pub true_negatives: usize,
    pub false_negatives: usize,
    pub precision: f64,
    pub recall: f64,
    pub false_positive_rate: f64,
}

impl EvalReport {
    pub fn is_acceptable(&self) -> bool {
        self.false_positive_rate < 0.05
    }
}

/// Run evaluation against a hand-labelled sample dataset.
pub fn evaluate_sample_dataset(
    dataset_path: &Path,
    registry: &GroundTruthRegistry,
) -> Result<EvalReport> {
    let content = fs::read_to_string(dataset_path).with_context(|| {
        format!(
            "Failed to read evaluation dataset at {}",
            dataset_path.display()
        )
    })?;
    let dataset: EvaluationDataset = serde_json::from_str(&content)
        .with_context(|| "Failed to parse evaluation dataset JSON")?;

    let mut tp = 0;
    let mut fp = 0;
    let mut tn = 0;
    let mut fn_count = 0;

    for sample in &dataset.samples {
        let mention = ExtractedMention {
            kind: sample.kind,
            role: sample.expected_role,
            raw_text: sample.raw_text.clone(),
            normalized_target: sample.raw_text.clone(),
            source_file: sample.doc_path.clone(),
            line_number: 1,
            byte_offset: 0,
            fence_info: None,
        };

        let resolved = registry.resolve_mention(&mention);
        let predicted_deficit = resolved.is_actionable_deficit();

        match (predicted_deficit, sample.is_true_deficit) {
            (true, true) => tp += 1,
            (true, false) => fp += 1,
            (false, false) => tn += 1,
            (false, true) => fn_count += 1,
        }
    }

    let total = dataset.samples.len();
    let precision = if tp + fp > 0 {
        tp as f64 / (tp + fp) as f64
    } else {
        1.0
    };
    let recall = if tp + fn_count > 0 {
        tp as f64 / (tp + fn_count) as f64
    } else {
        1.0
    };
    let false_positive_rate = if fp + tn > 0 {
        fp as f64 / (fp + tn) as f64
    } else {
        0.0
    };

    Ok(EvalReport {
        total_samples: total,
        true_positives: tp,
        false_positives: fp,
        true_negatives: tn,
        false_negatives: fn_count,
        precision,
        recall,
        false_positive_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_sample_accuracy() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();

        let registry = GroundTruthRegistry::load(repo_root).expect("load registry");
        let sample_path =
            repo_root.join("contracts/reports/docs-reality-audit/evaluation-sample.v1.json");

        let report = evaluate_sample_dataset(&sample_path, &registry).expect("evaluate dataset");
        assert_eq!(report.false_positives, 0);
        assert!(
            report.is_acceptable(),
            "False positive rate {} must be < 0.05",
            report.false_positive_rate
        );
    }
}
