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

/// Per-category pass/fail rollup for one harness eval run (design spec §10.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategorySummaryDto {
    pub category: String,
    pub pass_count: i64,
    pub fail_count: i64,
}

/// One row of the Harness Health recent-runs table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarnessEvalRunDto {
    pub run_id: String,
    pub git_sha: String,
    pub triggered_by: String,
    pub pass_count: i64,
    pub fail_count: i64,
    pub skip_count: i64,
    pub total_cost_usd: f64,
    pub started_at_ms: i64,
    pub category_breakdown: Vec<CategorySummaryDto>,
}

/// One flagged regression for the Harness Health banner (design spec §10.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegressionFlagDto {
    pub kind: String,
    pub previous_run_id: String,
    pub current_run_id: String,
    pub previous_git_sha: String,
    pub current_git_sha: String,
    pub changed_files: Vec<String>,
    pub flipped_task_ids: Vec<String>,
    pub detail: String,
}
