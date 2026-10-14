//! Research, Scientia review and research-engine wire DTOs (plan 07 Task 7). Field names are part
//! of the Axis UI contract — do NOT rename.
use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ─── Research sessions ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSessionDto {
    pub id: i64,
    pub status: String,
    pub query_text: String,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchClaimDto {
    pub claim_id: String,
    pub text: String,
    pub verdict: String,
    pub confidence: f64,
    pub resample_stability: f64,
    pub citation_urls: Vec<String>,
    pub corroboration_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchDetailDto {
    pub session: ResearchSessionDto,
    pub report_markdown: Option<String>,
    pub artifact_json: Option<String>,
    pub confidence_tier: Option<String>,
    pub claims: Vec<ResearchClaimDto>,
    pub source_count: Option<usize>,
    pub citation_precision: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocDraftPreview {
    pub slug: String,
    pub title: String,
    pub filename: String,
    pub markdown_content: String,
    pub is_valid: bool,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishDocResult {
    pub file_path: String,
    pub relative_path: String,
    pub indexed: bool,
}

// ─── Scientia publications ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationManifestDto {
    pub publication_id: String,
    pub content_type: String,
    pub state: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// One extracted claim awaiting human review, with its latest verdict (if any).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimAwaitingReviewDto {
    pub claim_id: i64,
    pub text: String,
    pub is_numeric: bool,
    pub verdict: Option<String>,
    pub confidence: Option<f64>,
    pub verifier_model: Option<String>,
    pub created_at_ms: i64,
}

/// The persisted human review decision, content-bound to the publication's current manifest
/// digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewDecisionDto {
    pub claim_id: i64,
    pub publication_id: String,
    pub decision: String,
    pub bound_digest: String,
    pub decided_at_ms: i64,
}

/// A local, offline-validated nanopublication. NOTHING is published to the network:
/// `published_state` is always `"local"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NanopubResultDto {
    pub trusty_uri: String,
    pub published_state: String,
    pub validated_offline: bool,
}

/// One prior-art hit, trimmed to the fields the evidence panel renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorArtHitDto {
    pub work_uri: String,
    pub title: String,
    pub year: Option<i32>,
    pub cited_by_count: Option<u64>,
    pub semantic_score: Option<f64>,
}

/// One supporting-vs-contradicting conflict among high-similarity hits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDto {
    pub claim_text: String,
    pub conflict_score: f64,
    pub supporting: Vec<ConflictHitDto>,
    pub contradicting: Vec<ConflictHitDto>,
}

/// One side of a conflict: the hit's uri + the excerpt that justified its polarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictHitDto {
    pub work_uri: String,
    pub excerpt: Option<String>,
}

/// Explainable signal breakdown mirrored from `NoveltySignalBreakdown`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalsDto {
    pub max_semantic: Option<f64>,
    pub max_lexical: Option<f64>,
    pub near_hit_count: usize,
    pub top_hit_citations: Option<u64>,
    pub sources_succeeded: usize,
}

/// The full novelty assessment for one publication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoveltyAssessmentDto {
    /// `"insufficient_evidence"` | `"novel"` | `"possibly_novel"` | `"not_novel"` | `"contradicted"`.
    pub verdict_kind: String,
    pub closest_hit_uri: Option<String>,
    pub closest_score: Option<f64>,
    pub excluded_future_hits: usize,
    pub conflicts: Vec<ConflictDto>,
    pub signals: SignalsDto,
    /// Top-5 prior-art hits by semantic score (desc).
    pub prior_art: Vec<PriorArtHitDto>,
    /// Present when `verdict_kind` is `insufficient_evidence`.
    pub insufficient_evidence_reason: Option<String>,
}

impl NoveltyAssessmentDto {
    /// No bundle has been fetched yet: the panel renders the insufficient-evidence banner.
    pub fn insufficient_evidence() -> Self {
        Self {
            verdict_kind: "insufficient_evidence".into(),
            closest_hit_uri: None,
            closest_score: None,
            excluded_future_hits: 0,
            conflicts: vec![],
            signals: SignalsDto {
                max_semantic: None,
                max_lexical: None,
                near_hit_count: 0,
                top_hit_citations: None,
                sources_succeeded: 0,
            },
            prior_art: vec![],
            insufficient_evidence_reason: Some("prior-art retrieval produced no evidence".into()),
        }
    }
}

/// One unacknowledged discovery-inbox row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryInboxDto {
    pub id: i64,
    pub publication_id: String,
    pub surfaced_at_ms: i64,
    pub intake_tier: String,
    pub signal_codes: Vec<String>,
    /// `"research"` when surfaced from the research pipeline; otherwise `"commit_watcher"`.
    pub origin: String,
}

/// `"research"` when any signal came from the research pipeline, else `"commit_watcher"`.
pub fn discovery_inbox_origin(signal_codes: &[String]) -> &'static str {
    if signal_codes
        .iter()
        .any(|code| code.starts_with("research_pipeline."))
    {
        "research"
    } else {
        "commit_watcher"
    }
}

impl From<vox_db::DiscoveryInboxRow> for DiscoveryInboxDto {
    fn from(r: vox_db::DiscoveryInboxRow) -> Self {
        let origin = discovery_inbox_origin(&r.signal_codes).to_string();
        Self {
            id: r.id,
            publication_id: r.publication_id,
            surfaced_at_ms: r.surfaced_at_ms,
            intake_tier: r.intake_tier,
            signal_codes: r.signal_codes,
            origin,
        }
    }
}

/// One provenance entry: which field, where its value came from, optional note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldProvenanceDto {
    pub field: String,
    pub origin: String,
    pub notes: Option<String>,
}

/// Metadata-completeness report for one publication's manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionReportDto {
    pub completeness_0_100: u8,
    pub required_missing: Vec<String>,
    pub inferred_ok: Vec<String>,
    pub human_only_pending: Vec<String>,
    pub field_provenance: Vec<FieldProvenanceDto>,
}

/// One proposed (or applied) field fill; `value` is the JSON value as a compact string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedFillDto {
    pub field: String,
    pub value: String,
    pub origin: String,
    pub notes: Option<String>,
}

/// Result of computing (and optionally applying) the deterministic autofill plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutofillResultDto {
    pub fills: Vec<PlannedFillDto>,
    pub human_only_remaining: Vec<String>,
    pub completeness_before: u8,
    /// Equals `completeness_before` when `apply == false`.
    pub completeness_after: u8,
}

/// Deposit status surfaced from whatever is persisted; any field is `None` when nothing is on
/// record ("not yet deposited").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveStatusDto {
    pub swhid: Option<String>,
    pub swh_task_status: Option<String>,
    pub zenodo_doi: Option<String>,
    pub zenodo_state: Option<String>,
}

// ─── Research engine (search providers) ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaUsageDto {
    #[serde(alias = "unitsSpent")]
    pub units_spent: i64,
    #[serde(alias = "unitsLimit")]
    pub units_limit: i64,
    #[serde(alias = "lastSyncedAt")]
    pub last_synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusDto {
    pub id: String,
    pub name: String,
    #[serde(alias = "isKeyless")]
    pub is_keyless: bool,
    #[serde(alias = "isEnabled")]
    pub is_enabled: bool,
    #[serde(alias = "hasKey")]
    pub has_key: bool,
    #[serde(alias = "quotaUsage")]
    pub quota_usage: Option<QuotaUsageDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchEngineStatusDto {
    #[serde(alias = "activeLane")]
    pub active_lane: String,
    #[serde(alias = "fastTimeoutMs")]
    pub fast_timeout_ms: u64,
    #[serde(alias = "deepTimeoutMs")]
    pub deep_timeout_ms: u64,
    pub providers: Vec<ProviderStatusDto>,
    #[serde(alias = "freeKeyOffers")]
    pub free_key_offers: Vec<vox_secrets::FreeTierOffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct ResearchEngineConfigDto {
    #[serde(default, alias = "activeLane")]
    pub active_lane: Option<String>,
    #[serde(default, alias = "fastTimeoutMs")]
    pub fast_timeout_ms: Option<u64>,
    #[serde(default, alias = "deepTimeoutMs")]
    pub deep_timeout_ms: Option<u64>,
    #[serde(default, alias = "enabledProviders")]
    pub enabled_providers: Option<Vec<String>>,
    /// Provider id → API key, stored through Clavis on the server. Blank values are skipped.
    #[serde(default, alias = "providerApiKeys")]
    pub provider_api_keys: Option<HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_config_accepts_camel_case_from_the_ui() {
        let cfg: ResearchEngineConfigDto = serde_json::from_str(
            r#"{"activeLane":"deep","fastTimeoutMs":2200,"enabledProviders":["arxiv"]}"#,
        )
        .unwrap();
        assert_eq!(cfg.active_lane.as_deref(), Some("deep"));
        assert_eq!(cfg.fast_timeout_ms, Some(2200));
        assert_eq!(cfg.deep_timeout_ms, None);
        assert_eq!(cfg.enabled_providers, Some(vec!["arxiv".to_string()]));
    }

    #[test]
    fn discovery_origin_flags_research_pipeline_signals() {
        assert_eq!(
            discovery_inbox_origin(&["research_pipeline.supported_claims".into()]),
            "research"
        );
        assert_eq!(
            discovery_inbox_origin(&["perf_claim".into()]),
            "commit_watcher"
        );
    }
}
