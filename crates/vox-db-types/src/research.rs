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

/// Defect classification for code generated from misleading external research.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchDefectClass {
    /// Inelegant, bloated, or over-engineered code (violates ponytail doctrine).
    InelegantCode,
    /// Code fails to compile, build, or run.
    FailsToRun,
    /// Human developer had to manually correct agent-authored code.
    UserCorrection,
    /// Model hallucinated a non-existent API or method from obsolete research.
    HallucinatedApi,
}

impl ResearchDefectClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InelegantCode => "inelegant_code",
            Self::FailsToRun => "fails_to_run",
            Self::UserCorrection => "user_correction",
            Self::HallucinatedApi => "hallucinated_api",
        }
    }
}

impl std::fmt::Display for ResearchDefectClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for ResearchDefectClass {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "inelegant_code" => Ok(Self::InelegantCode),
            "fails_to_run" => Ok(Self::FailsToRun),
            "user_correction" => Ok(Self::UserCorrection),
            "hallucinated_api" => Ok(Self::HallucinatedApi),
            other => Err(format!("unknown research defect class: {other}")),
        }
    }
}

/// Actor or pipeline stage that reported research misguidance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MisguidanceReporter {
    /// Compiler diagnostic or build failure.
    Compiler,
    /// Unit test, integration test, or runtime test runner.
    TestRunner,
    /// Linter or static analysis tool.
    Linter,
    /// Human developer via GUI or CLI.
    User,
    /// Ponytail audit finding over-engineering or unnecessary dependencies.
    PonytailAudit,
}

impl MisguidanceReporter {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Compiler => "compiler",
            Self::TestRunner => "test_runner",
            Self::Linter => "linter",
            Self::User => "user",
            Self::PonytailAudit => "ponytail_audit",
        }
    }
}

impl std::fmt::Display for MisguidanceReporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for MisguidanceReporter {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "compiler" => Ok(Self::Compiler),
            "test_runner" => Ok(Self::TestRunner),
            "linter" => Ok(Self::Linter),
            "user" => Ok(Self::User),
            "ponytail_audit" => Ok(Self::PonytailAudit),
            other => Err(format!("unknown misguidance reporter: {other}")),
        }
    }
}

/// Parameter block for recording a research misguidance event into VoxDb.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordMisguidanceParams {
    pub session_id: Option<i64>,
    pub defect_class: ResearchDefectClass,
    pub culprit_url: Option<String>,
    pub culprit_domain: String,
    pub claim_id: Option<i64>,
    pub research_query: String,
    pub misleading_excerpt: Option<String>,
    pub generated_code_snippet: Option<String>,
    pub failure_diagnostic: Option<String>,
    pub correction_diff: Option<String>,
    pub reporter: MisguidanceReporter,
    pub domain_penalty: f64,
}

/// Persisted research misguidance event row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchMisguidanceRecord {
    pub id: i64,
    pub session_id: Option<i64>,
    pub defect_class: ResearchDefectClass,
    pub culprit_url: Option<String>,
    pub culprit_domain: String,
    pub claim_id: Option<i64>,
    pub research_query: String,
    pub misleading_excerpt: Option<String>,
    pub generated_code_snippet: Option<String>,
    pub failure_diagnostic: Option<String>,
    pub correction_diff: Option<String>,
    pub reporter: MisguidanceReporter,
    pub domain_penalty: f64,
    pub status: String,
    pub resolved_at_ms: Option<i64>,
    pub created_at_ms: i64,
}

/// Persisted domain reputation summary row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainReputationRecord {
    pub domain: String,
    pub incident_count: i64,
    pub penalty_score: f64,
    pub last_incident_at_ms: i64,
    pub is_blacklisted: bool,
    pub updated_at_ms: i64,
}

/// Input parameters for inserting or updating a discovered source in a research session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NewResearchSessionSource {
    pub session_id: i64,
    pub wave: i64,
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub provider: String,
    pub domain: String,
    pub relevance_score: f64,
    pub created_at_ms: i64,
}

/// Persisted source row for a research session (`research_session_sources`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchSessionSourceRecord {
    pub id: i64,
    pub session_id: i64,
    pub wave: i64,
    pub url: String,
    pub title: String,
    pub snippet: String,
    pub provider: String,
    pub domain: String,
    pub relevance_score: f64,
    pub created_at_ms: i64,
}

/// One cursor-paginated claim row joined to its latest non-span verdict.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionClaimPageItem {
    pub row_id: i64,
    pub claim_id: i64,
    pub session_id: i64,
    pub text: String,
    pub verdict: Option<String>,
    pub confidence: Option<f64>,
    pub verifier_model: Option<String>,
    pub created_at_ms: i64,
}

/// Deterministic cursor-paginated page of claims for a research session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionClaimsPage {
    pub items: Vec<SessionClaimPageItem>,
    pub next_cursor: Option<i64>,
    pub total_count: i64,
}

/// Deterministic cursor-paginated page of sources for a research session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionSourcesPage {
    pub items: Vec<ResearchSessionSourceRecord>,
    pub next_cursor: Option<i64>,
    pub total_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_research_defect_class_as_str_and_from_str() {
        let classes = [
            (ResearchDefectClass::InelegantCode, "inelegant_code"),
            (ResearchDefectClass::FailsToRun, "fails_to_run"),
            (ResearchDefectClass::UserCorrection, "user_correction"),
            (ResearchDefectClass::HallucinatedApi, "hallucinated_api"),
        ];
        for (defect_class, s) in classes {
            assert_eq!(defect_class.as_str(), s);
            assert_eq!(defect_class.to_string(), s);
            assert_eq!(ResearchDefectClass::from_str(s).unwrap(), defect_class);
        }
    }

    #[test]
    fn test_misguidance_reporter_as_str_and_from_str() {
        let reporters = [
            (MisguidanceReporter::Compiler, "compiler"),
            (MisguidanceReporter::TestRunner, "test_runner"),
            (MisguidanceReporter::Linter, "linter"),
            (MisguidanceReporter::User, "user"),
            (MisguidanceReporter::PonytailAudit, "ponytail_audit"),
        ];
        for (reporter, s) in reporters {
            assert_eq!(reporter.as_str(), s);
            assert_eq!(reporter.to_string(), s);
            assert_eq!(MisguidanceReporter::from_str(s).unwrap(), reporter);
        }
    }

    #[test]
    fn test_session_page_dtos_serde_roundtrip() {
        let src = NewResearchSessionSource {
            session_id: 7,
            wave: 2,
            url: "https://arxiv.org/abs/2601.00001".to_string(),
            title: "Paper Title".to_string(),
            snippet: "Abstract snippet".to_string(),
            provider: "arxiv".to_string(),
            domain: "arxiv.org".to_string(),
            relevance_score: 0.91,
            created_at_ms: 1700000000000,
        };
        let rec = ResearchSessionSourceRecord {
            id: 42,
            session_id: src.session_id,
            wave: src.wave,
            url: src.url.clone(),
            title: src.title.clone(),
            snippet: src.snippet.clone(),
            provider: src.provider.clone(),
            domain: src.domain.clone(),
            relevance_score: src.relevance_score,
            created_at_ms: src.created_at_ms,
        };
        let sources_page = SessionSourcesPage {
            items: vec![rec],
            next_cursor: Some(42),
            total_count: 100,
        };
        let json = serde_json::to_string(&sources_page).expect("serialize sources_page");
        let decoded: SessionSourcesPage =
            serde_json::from_str(&json).expect("deserialize sources_page");
        assert_eq!(decoded, sources_page);

        let claims_page = SessionClaimsPage {
            items: vec![SessionClaimPageItem {
                row_id: 11,
                claim_id: 998877,
                session_id: 7,
                text: "Latency dropped by 35%.".to_string(),
                verdict: Some("Supported".to_string()),
                confidence: Some(0.88),
                verifier_model: Some("minicheck-v1".to_string()),
                created_at_ms: 1700000000123,
            }],
            next_cursor: None,
            total_count: 1,
        };
        let cjson = serde_json::to_string(&claims_page).expect("serialize claims_page");
        let cdecoded: SessionClaimsPage =
            serde_json::from_str(&cjson).expect("deserialize claims_page");
        assert_eq!(cdecoded, claims_page);
    }
}
