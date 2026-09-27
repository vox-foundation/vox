use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct VisualReviewConfig {
    pub schema_version: u32,
    /// Vision-model preference; defaults to the `visual_review` role in
    /// `contracts/orchestration/model-defaults.v1.yaml` (Task 14).
    #[serde(default = "default_model_preference")]
    pub model_preference: Vec<String>,
    /// Defaults to the `visual_review_escalation` role.
    #[serde(default = "default_escalation_model")]
    pub escalation_model: String,
    pub per_surface_review_budget_ms: u64,
    pub total_review_budget_ms: u64,
    pub max_concurrent_reviews: usize,
    pub max_image_edge_px: u32,
    pub spike_factor: f64,
}

pub(crate) fn default_model_preference() -> Vec<String> {
    vox_config::model_defaults::VISUAL_REVIEW
        .iter()
        .map(|m| (*m).to_string())
        .collect()
}

pub(crate) fn default_escalation_model() -> String {
    vox_config::model_defaults::VISUAL_REVIEW_ESCALATION.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    #[serde(default)]
    pub principle: String,
    #[serde(default)]
    pub severity: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub critique: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewVerdict {
    #[serde(default)]
    pub score: u32,
    #[serde(default)]
    pub verdict: String,
    #[serde(default)]
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SurfaceReport {
    pub view_key: String,
    pub screenshot_sha256: String,
    pub status: String,
    pub score: Option<u32>,
    pub verdict: Option<String>,
    pub findings: Vec<Finding>,
    pub model: Option<String>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    pub review_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunReport {
    pub schema_version: u32,
    pub generated_at: String,
    pub default_model: String,
    pub surfaces: Vec<SurfaceReport>,
    pub total_capture_ms: u64,
    pub total_review_ms: u64,
    pub surfaces_reviewed: usize,
    pub surfaces_cached: usize,
    pub surfaces_deferred: usize,
    pub spiked: bool,
    pub spike_detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub screenshot_sha256: String,
    pub score: u32,
    pub verdict: String,
    pub model: String,
    pub reviewed_at: String,
    /// Prompt version the verdict was produced under (empty on legacy entries).
    #[serde(default)]
    pub prompt_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheIndex {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    #[serde(default)]
    pub entries: BTreeMap<String, CacheEntry>,
}

impl Default for CacheIndex {
    fn default() -> Self {
        Self {
            schema_version: default_schema(),
            entries: BTreeMap::new(),
        }
    }
}
fn default_schema() -> u32 {
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Task 14: the visual-review contract no longer names model ids; they
    /// default to the `visual_review` / `visual_review_escalation` roles in
    /// model-defaults.v1.yaml, and the shipped contract parses that way.
    #[test]
    fn config_without_model_ids_uses_the_contract_defaults() {
        let json = r#"{ "schema_version":1, "per_surface_review_budget_ms":8000, "total_review_budget_ms":90000, "max_concurrent_reviews":3, "max_image_edge_px":2880, "spike_factor":1.5 }"#;
        let cfg: VisualReviewConfig = serde_json::from_str(json).unwrap();
        let want: Vec<String> = vox_config::model_defaults::VISUAL_REVIEW
            .iter()
            .map(|m| m.to_string())
            .collect();
        assert_eq!(cfg.model_preference, want);
        assert_eq!(
            cfg.escalation_model,
            vox_config::model_defaults::VISUAL_REVIEW_ESCALATION
        );

        let shipped: VisualReviewConfig = serde_json::from_str(include_str!(
            "../../../../contracts/orchestration/visual-review.config.v1.json"
        ))
        .expect("shipped visual-review contract parses");
        assert_eq!(shipped.model_preference, want);
    }

    #[test]
    fn config_parses_model_preference_and_budgets() {
        let json = r#"{ "schema_version":1, "model_preference":["google/gemini-3-flash-preview","google/gemini-2.5-flash"], "escalation_model":"anthropic/claude-opus-4.8", "per_surface_review_budget_ms":8000, "total_review_budget_ms":90000, "max_concurrent_reviews":3, "max_image_edge_px":2880, "spike_factor":1.5 }"#;
        let cfg: VisualReviewConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.model_preference[0], "google/gemini-3-flash-preview");
        assert_eq!(cfg.total_review_budget_ms, 90_000);
        assert_eq!(cfg.spike_factor, 1.5);
    }
    #[test]
    fn cache_roundtrips() {
        let mut idx = CacheIndex::default();
        idx.entries.insert(
            "dashboard".into(),
            CacheEntry {
                screenshot_sha256: "a".repeat(64),
                score: 82,
                verdict: "pass_with_notes".into(),
                model: "google/gemini-3-flash-preview".into(),
                reviewed_at: "2026-06-15T00:00:00Z".into(),
                prompt_version: "2026-07-16.1".into(),
            },
        );
        let s = serde_json::to_string(&idx).unwrap();
        let back: CacheIndex = serde_json::from_str(&s).unwrap();
        assert_eq!(back.entries["dashboard"].score, 82);
    }
    #[test]
    fn default_cache_index_is_schema_1_not_0() {
        assert_eq!(CacheIndex::default().schema_version, 1);
    }
    #[test]
    fn legacy_entry_without_prompt_version_deserializes_empty() {
        let json = r#"{ "screenshot_sha256":"aa", "score":90, "verdict":"pass", "model":"m", "reviewed_at":"t" }"#;
        let e: CacheEntry = serde_json::from_str(json).unwrap();
        assert_eq!(e.prompt_version, "");
    }
    #[test]
    fn verdict_tolerates_missing_finding_fields() {
        let json = r#"{ "score": 55, "verdict": "fail", "findings": [ { "severity": "high", "region": "top", "critique": "x" } ] }"#;
        let v: ReviewVerdict = serde_json::from_str(json).unwrap();
        assert_eq!(v.findings.len(), 1);
        assert_eq!(v.findings[0].principle, ""); // defaulted, not an error
        let empty: ReviewVerdict =
            serde_json::from_str(r#"{ "score": 90, "verdict": "pass" }"#).unwrap();
        assert!(empty.findings.is_empty());
    }
}
