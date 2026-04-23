//! Export external review findings from VoxDb into reproducible JSONL rows.

#[cfg(feature = "database")]
use vox_db::VoxDb;

/// Canonical replay row for review-derived training/eval artifacts.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExternalReviewReplayRow {
    pub prompt: String,
    pub response: String,
    pub category: String,
    pub severity: String,
    pub placement_kind: String,
    pub source_id: String,
    pub repository_id: String,
    pub pr_number: i64,
    pub file_path: Option<String>,
    pub line_start: Option<i64>,
    pub correctness_state: String,
    pub sample_kind: String, // review_fix_pairs | review_antipattern_memory | review_regression_challenges
}

#[cfg(feature = "database")]
pub async fn extract_external_review_rows(
    db: &VoxDb,
    repository_id: &str,
    limit: i64,
) -> anyhow::Result<Vec<ExternalReviewReplayRow>> {
    let findings = db
        .list_external_review_findings_for_training_window(repository_id, limit)
        .await?;
    let mut out = Vec::new();
    for f in findings {
        let prompt = format!(
            "Review finding in {}:{} category={} severity={}: {}",
            f.file_path.as_deref().unwrap_or("global"),
            f.line_start.unwrap_or(0),
            f.category,
            f.severity,
            f.title
        );
        let response = if let Some(s) = f.suggested_fix.clone() {
            s
        } else {
            f.details.clone()
        };
        out.push(ExternalReviewReplayRow {
            prompt: prompt.clone(),
            response: response.clone(),
            category: f.category.clone(),
            severity: f.severity.clone(),
            placement_kind: f.placement_kind.clone(),
            source_id: f.finding_identity.clone(),
            repository_id: f.repository_id.clone(),
            pr_number: f.pr_number,
            file_path: f.file_path.clone(),
            line_start: f.line_start,
            correctness_state: f.status.clone(),
            sample_kind: "review_fix_pairs".to_string(),
        });

        out.push(ExternalReviewReplayRow {
            prompt: format!("Anti-pattern memory: {}", f.title),
            response: f.details.clone(),
            category: f.category.clone(),
            severity: f.severity.clone(),
            placement_kind: f.placement_kind.clone(),
            source_id: f.finding_identity.clone(),
            repository_id: f.repository_id.clone(),
            pr_number: f.pr_number,
            file_path: f.file_path.clone(),
            line_start: f.line_start,
            correctness_state: f.status.clone(),
            sample_kind: "review_antipattern_memory".to_string(),
        });

        if f.severity == "error" || f.severity == "warning" {
            out.push(ExternalReviewReplayRow {
                prompt: format!("Regression challenge for {}", f.title),
                response: f.details.clone(),
                category: f.category.clone(),
                severity: f.severity.clone(),
                placement_kind: f.placement_kind.clone(),
                source_id: f.finding_identity.clone(),
                repository_id: f.repository_id.clone(),
                pr_number: f.pr_number,
                file_path: f.file_path.clone(),
                line_start: f.line_start,
                correctness_state: f.status.clone(),
                sample_kind: "review_regression_challenges".to_string(),
            });
        }
    }
    out.sort_by(|a, b| {
        a.source_id
            .cmp(&b.source_id)
            .then(a.sample_kind.cmp(&b.sample_kind))
    });
    Ok(out)
}

/// Basic validation for exported rows.
pub fn validate_external_review_rows(rows: &[ExternalReviewReplayRow]) -> anyhow::Result<()> {
    for (idx, row) in rows.iter().enumerate() {
        if row.prompt.trim().is_empty() {
            anyhow::bail!("row {idx}: prompt is empty");
        }
        if row.response.trim().is_empty() {
            anyhow::bail!("row {idx}: response is empty");
        }
        if row.source_id.trim().is_empty() {
            anyhow::bail!("row {idx}: source_id is empty");
        }
        if row.sample_kind.trim().is_empty() {
            anyhow::bail!("row {idx}: sample_kind is empty");
        }
    }
    Ok(())
}

/// Build repeated-error features and hard negatives from review rows.
pub fn build_repeated_error_features(rows: &[ExternalReviewReplayRow]) -> Vec<serde_json::Value> {
    let mut category_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut file_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for row in rows {
        *category_counts.entry(row.category.clone()).or_insert(0) += 1;
        if let Some(path) = &row.file_path {
            *file_counts.entry(path.clone()).or_insert(0) += 1;
        }
    }

    let mut out = Vec::new();
    for row in rows {
        let cat_repeat = category_counts.get(&row.category).copied().unwrap_or(0);
        let file_repeat = row
            .file_path
            .as_ref()
            .and_then(|p| file_counts.get(p))
            .copied()
            .unwrap_or(0);
        out.push(serde_json::json!({
            "source_id": row.source_id,
            "category_repeat_count": cat_repeat,
            "file_repeat_count": file_repeat,
            "hard_negative": row.correctness_state == "confirmed_false" || row.correctness_state == "likely_false",
        }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_empty_prompt() {
        let rows = vec![ExternalReviewReplayRow {
            prompt: String::new(),
            response: "x".to_string(),
            category: "style".to_string(),
            severity: "info".to_string(),
            placement_kind: "inline".to_string(),
            source_id: "id-1".to_string(),
            repository_id: "owner/repo".to_string(),
            pr_number: 1,
            file_path: Some("a.rs".to_string()),
            line_start: Some(1),
            correctness_state: "unverified".to_string(),
            sample_kind: "review_fix_pairs".to_string(),
        }];
        assert!(validate_external_review_rows(&rows).is_err());
    }

    #[test]
    fn validate_accepts_basic_row() {
        let rows = vec![ExternalReviewReplayRow {
            prompt: "Fix issue".to_string(),
            response: "Use checked math".to_string(),
            category: "logic".to_string(),
            severity: "warning".to_string(),
            placement_kind: "inline".to_string(),
            source_id: "id-1".to_string(),
            repository_id: "owner/repo".to_string(),
            pr_number: 1,
            file_path: Some("a.rs".to_string()),
            line_start: Some(1),
            correctness_state: "confirmed_true".to_string(),
            sample_kind: "review_fix_pairs".to_string(),
        }];
        assert!(validate_external_review_rows(&rows).is_ok());
    }

    #[test]
    fn repeated_error_features_include_hard_negatives() {
        let rows = vec![
            ExternalReviewReplayRow {
                prompt: "p1".to_string(),
                response: "r1".to_string(),
                category: "logic".to_string(),
                severity: "warning".to_string(),
                placement_kind: "inline".to_string(),
                source_id: "id-1".to_string(),
                repository_id: "owner/repo".to_string(),
                pr_number: 1,
                file_path: Some("a.rs".to_string()),
                line_start: Some(1),
                correctness_state: "confirmed_false".to_string(),
                sample_kind: "review_fix_pairs".to_string(),
            },
            ExternalReviewReplayRow {
                prompt: "p2".to_string(),
                response: "r2".to_string(),
                category: "logic".to_string(),
                severity: "warning".to_string(),
                placement_kind: "inline".to_string(),
                source_id: "id-2".to_string(),
                repository_id: "owner/repo".to_string(),
                pr_number: 1,
                file_path: Some("a.rs".to_string()),
                line_start: Some(2),
                correctness_state: "confirmed_true".to_string(),
                sample_kind: "review_fix_pairs".to_string(),
            },
        ];
        let features = build_repeated_error_features(&rows);
        assert_eq!(features.len(), 2);
        assert_eq!(features[0]["category_repeat_count"], 2);
        assert_eq!(features[0]["hard_negative"], true);
    }
}
