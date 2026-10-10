//! Harness issue and fix-proposal wire DTOs (plan 07 Task 7). Field names are part of the Axis UI
//! contract — do NOT rename.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessIssueDto {
    pub id: i64,
    pub source: String,
    pub session_key: Option<String>,
    pub target_path: Option<String>,
    pub detected_at_ms: i64,
    pub category: String,
    pub severity: String,
    pub summary: String,
    pub evidence_json: String,
    pub status: String,
}

impl From<vox_db::HarnessIssueRow> for HarnessIssueDto {
    fn from(r: vox_db::HarnessIssueRow) -> Self {
        Self {
            id: r.id,
            source: r.source,
            session_key: r.session_key,
            target_path: r.target_path,
            detected_at_ms: r.detected_at_ms,
            category: r.category,
            severity: r.severity,
            summary: r.summary,
            evidence_json: r.evidence_json,
            status: r.status,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessFixProposalDto {
    pub id: i64,
    pub issue_id: i64,
    pub target_path: String,
    pub proposed_content: String,
    pub proposed_diff: String,
    pub status: String,
    pub proposed_at_ms: i64,
    pub resolved_at_ms: Option<i64>,
}

impl From<vox_db::HarnessFixProposalRow> for HarnessFixProposalDto {
    fn from(r: vox_db::HarnessFixProposalRow) -> Self {
        Self {
            id: r.id,
            issue_id: r.issue_id,
            target_path: r.target_path,
            proposed_content: r.proposed_content,
            proposed_diff: r.proposed_diff,
            status: r.status,
            proposed_at_ms: r.proposed_at_ms,
            resolved_at_ms: r.resolved_at_ms,
        }
    }
}
