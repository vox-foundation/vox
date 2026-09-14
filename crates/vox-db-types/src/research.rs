use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchSessionRecord {
    pub id: i64,
    pub session_key: String,
    pub status: String,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub query_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchSessionSummary {
    pub id: i64,
    pub session_key: String,
    pub status: String,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub query_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchArtifactRecord {
    pub session_id: i64,
    pub artifact_json: String,
    pub report_markdown: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// An extracted SCIENTIA claim joined to its latest (non-span) verdict, for a
/// publication's claim ledger view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScientiaClaimWithVerdict {
    pub claim_id: i64,
    pub text: String,
    pub is_numeric: bool,
    pub verifiability_score: Option<f64>,
    /// Latest verdict label (`Supported` / `Contested` / `Contradicted` /
    /// `Abstain`), or `None` when extraction ran but no verdict is recorded yet.
    pub verdict: Option<String>,
    pub confidence: Option<f64>,
    pub verifier_model: Option<String>,
    pub created_at_ms: i64,
}

/// Global claims-pending counts for the SCIENTIA dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaimsPendingCounts {
    /// Claims whose latest verdict is `Supported`.
    pub verifiable: i64,
    /// Claims whose latest verdict is `Abstain`.
    pub abstained: i64,
    /// Claims with no non-span verdict row yet (verification pending).
    pub extraction_running: i64,
}

/// Result hit from full-text or pattern search across research artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchSearchResult {
    pub session_id: i64,
    pub query_text: String,
    pub snippet: String,
    pub created_at_ms: i64,
}

/// Cached claim verification verdict from historical research sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CachedClaimVerdict {
    pub claim_id: u64,
    pub verdict: String,
    pub confidence: f64,
    pub verifier_model: Option<String>,
    pub created_at_ms: i64,
}
