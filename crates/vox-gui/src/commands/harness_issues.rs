//! Tauri commands for harness issue discovery: listing/deciding issues, the golden-corpus
//! staleness scanner, and LLM fix proposals. Served by the daemon (`GET /api/v2/harness/*` and
//! the `vox_harness_*` ops), so scans and approved fixes act on the **server** workspace.

use std::sync::Arc;

use serde_json::json;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::harness::{HarnessFixProposalDto, HarnessIssueDto};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, encode_component, path_with_query, tool_data};

const CORPUS_SCAN_TOOL: &str = "vox_harness_corpus_scan";
const FIX_PROPOSE_TOOL: &str = "vox_harness_fix_propose";
const FIX_RESOLVE_TOOL: &str = "vox_harness_fix_resolve";
const ISSUE_DECIDE_TOOL: &str = "vox_harness_issue_decide";

pub(crate) async fn scan_impl<B: BackendCalls>(b: &B) -> Result<usize, String> {
    tool_data(
        CORPUS_SCAN_TOOL,
        b.call_tool(CORPUS_SCAN_TOOL, json!({})).await?,
    )
}

pub(crate) async fn propose_impl<B: BackendCalls>(
    b: &B,
    issue_id: i64,
    target_path: String,
) -> Result<i64, String> {
    let args = json!({ "issue_id": issue_id, "target_path": target_path });
    tool_data(FIX_PROPOSE_TOOL, b.call_tool(FIX_PROPOSE_TOOL, args).await?)
}

pub(crate) async fn list_proposals_impl<B: BackendCalls>(
    b: &B,
    status: Option<String>,
) -> Result<Vec<HarnessFixProposalDto>, String> {
    let path = path_with_query("/harness/fix-proposals", &[("status", status)]);
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn resolve_impl<B: BackendCalls>(
    b: &B,
    proposal_id: i64,
    approve: bool,
) -> Result<(), String> {
    let args = json!({ "proposal_id": proposal_id, "approve": approve });
    tool_data(FIX_RESOLVE_TOOL, b.call_tool(FIX_RESOLVE_TOOL, args).await?)
}

pub(crate) async fn list_issues_impl<B: BackendCalls>(
    b: &B,
    status: Option<String>,
    source: Option<String>,
) -> Result<Vec<HarnessIssueDto>, String> {
    let path = path_with_query("/harness/issues", &[("status", status), ("source", source)]);
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn issues_for_session_impl<B: BackendCalls>(
    b: &B,
    session_key: &str,
) -> Result<Vec<HarnessIssueDto>, String> {
    let path = format!(
        "/harness/issues/by-session/{}",
        encode_component(session_key)
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn decide_impl<B: BackendCalls>(
    b: &B,
    issue_id: i64,
    decision: String,
    reason: Option<String>,
) -> Result<(), String> {
    let args = json!({ "issue_id": issue_id, "decision": decision, "reason": reason });
    tool_data(
        ISSUE_DECIDE_TOOL,
        b.call_tool(ISSUE_DECIDE_TOOL, args).await?,
    )
}

/// Scan the server workspace's `examples/golden/*.vox` for staleness and return how many NEW
/// harness issues were recorded.
#[tauri::command]
pub async fn scan_training_corpus(backend: State<'_, Arc<BackendState>>) -> Result<usize, String> {
    scan_impl(&backend.current().await).await
}

/// Ask the LLM facade for a corrected version of the issue's target file; returns the new
/// pending proposal's id.
#[tauri::command]
pub async fn propose_harness_issue_fix(
    backend: State<'_, Arc<BackendState>>,
    issue_id: i64,
    target_path: String,
) -> Result<i64, String> {
    propose_impl(&backend.current().await, issue_id, target_path).await
}

/// List fix proposals, optionally filtered by status.
#[tauri::command]
pub async fn list_harness_fix_proposals(
    backend: State<'_, Arc<BackendState>>,
    status: Option<String>,
) -> Result<Vec<HarnessFixProposalDto>, String> {
    list_proposals_impl(&backend.current().await, status).await
}

/// Approve (write `proposed_content` to `target_path` on the server) or reject a proposal.
#[tauri::command]
pub async fn resolve_harness_fix_proposal(
    backend: State<'_, Arc<BackendState>>,
    proposal_id: i64,
    approve: bool,
) -> Result<(), String> {
    resolve_impl(&backend.current().await, proposal_id, approve).await
}

/// List harness issues, optionally filtered by status/source.
#[tauri::command]
pub async fn list_harness_issues(
    backend: State<'_, Arc<BackendState>>,
    status: Option<String>,
    source: Option<String>,
) -> Result<Vec<HarnessIssueDto>, String> {
    list_issues_impl(&backend.current().await, status, source).await
}

/// List harness issues for one chat session (used by the inline transcript summary).
#[tauri::command]
pub async fn list_harness_issues_for_session(
    backend: State<'_, Arc<BackendState>>,
    session_key: String,
) -> Result<Vec<HarnessIssueDto>, String> {
    issues_for_session_impl(&backend.current().await, &session_key).await
}

/// Record a human decision (confirm/dismiss) for a harness issue.
#[tauri::command]
pub async fn record_harness_issue_decision(
    backend: State<'_, Arc<BackendState>>,
    issue_id: i64,
    decision: String,
    reason: Option<String>,
) -> Result<(), String> {
    decide_impl(&backend.current().await, issue_id, decision, reason).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    fn ok_null() -> Result<serde_json::Value, String> {
        Ok(json!({ "success": true, "data": null }))
    }

    #[tokio::test]
    async fn reads_go_to_the_harness_routes() {
        let b = FakeBackend::new()
            .on_get(
                "/harness/issues?status=pending&source=corpus_scan",
                Ok(json!({ "v": 1, "data": [] })),
            )
            .on_get(
                "/harness/issues/by-session/chat%3A1",
                Ok(json!({ "v": 1, "data": [] })),
            )
            .on_get("/harness/fix-proposals", Ok(json!({ "v": 1, "data": [] })));
        assert!(
            list_issues_impl(&b, Some("pending".into()), Some("corpus_scan".into()))
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            issues_for_session_impl(&b, "chat:1")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(list_proposals_impl(&b, None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn writes_go_to_the_operator_ops_with_the_tauri_arguments() {
        let b = FakeBackend::new()
            .on_tool(CORPUS_SCAN_TOOL, Ok(json!({ "success": true, "data": 2 })))
            .on_tool(FIX_PROPOSE_TOOL, Ok(json!({ "success": true, "data": 5 })))
            .on_tool(FIX_RESOLVE_TOOL, ok_null())
            .on_tool(ISSUE_DECIDE_TOOL, ok_null());
        assert_eq!(scan_impl(&b).await.unwrap(), 2);
        assert_eq!(
            propose_impl(&b, 1, "examples/golden/x.vox".into())
                .await
                .unwrap(),
            5
        );
        resolve_impl(&b, 5, true).await.unwrap();
        decide_impl(&b, 1, "confirmed".into(), None).await.unwrap();
        let calls = b.calls();
        assert_eq!(calls[1].1["target_path"], "examples/golden/x.vox");
        assert_eq!(calls[2].1, json!({ "proposal_id": 5, "approve": true }));
        assert_eq!(calls[3].1["decision"], "confirmed");
    }
}
