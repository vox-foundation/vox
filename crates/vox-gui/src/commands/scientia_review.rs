//! DiscoveryReview Tauri commands — a THIN layer over the daemon's Scientia review surface
//! (`GET /api/v2/scientia/*` and the `vox_scientia_*` review ops), which in turn delegates to the
//! SSOT in `vox_scientia::review_flow`. Works against a local or a remote Vox server.
//!
//! Like the CLI guard, this file carries NO production-network publishing symbols (asserted by
//! the guard test below); nanopublications stay local and offline-validated.

use std::sync::Arc;

use serde_json::json;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::research::{
    ArchiveStatusDto, AutofillResultDto, ClaimAwaitingReviewDto, CompletionReportDto,
    DiscoveryInboxDto, NanopubResultDto, NoveltyAssessmentDto, ReviewDecisionDto,
};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, encode_component, path_with_query, tool_data};

const CLAIM_REVIEW_TOOL: &str = "vox_scientia_publication_claim_review";
const NANOPUBLISH_TOOL: &str = "vox_scientia_publication_nanopublish";
const AUTOFILL_TOOL: &str = "vox_scientia_publication_autofill";
const DISCOVERY_ACK_TOOL: &str = "vox_scientia_discovery_ack";

fn publication_path(publication_id: &str, rest: &str) -> String {
    format!(
        "/scientia/publications/{}/{rest}",
        encode_component(publication_id)
    )
}

pub(crate) async fn review_queue_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
) -> Result<Vec<ClaimAwaitingReviewDto>, String> {
    api_data(
        b.api_v2_get(&publication_path(publication_id, "review-queue"))
            .await?,
    )
}

pub(crate) async fn claim_review_impl<B: BackendCalls>(
    b: &B,
    publication_id: String,
    claim_id: i64,
    decision: String,
    reason: Option<String>,
) -> Result<ReviewDecisionDto, String> {
    let args = json!({
        "publication_id": publication_id,
        "claim_id": claim_id,
        "decision": decision,
        "reason": reason,
    });
    tool_data(
        CLAIM_REVIEW_TOOL,
        b.call_tool(CLAIM_REVIEW_TOOL, args).await?,
    )
}

pub(crate) async fn nanopublish_impl<B: BackendCalls>(
    b: &B,
    publication_id: String,
    claim_id: i64,
    orcid: Option<String>,
) -> Result<NanopubResultDto, String> {
    let args = json!({ "publication_id": publication_id, "claim_id": claim_id, "orcid": orcid });
    tool_data(NANOPUBLISH_TOOL, b.call_tool(NANOPUBLISH_TOOL, args).await?)
}

pub(crate) async fn evidence_impl<B: BackendCalls>(
    b: &B,
    publication_id: &str,
    claim_id: i64,
) -> Result<Vec<vox_scientia::evidence_assist::EvidenceSuggestion>, String> {
    let rest = format!("claims/{claim_id}/evidence");
    api_data(
        b.api_v2_get(&publication_path(publication_id, &rest))
            .await?,
    )
}

pub(crate) async fn publication_get_impl<B: BackendCalls, T: serde::de::DeserializeOwned>(
    b: &B,
    publication_id: &str,
    rest: &str,
) -> Result<T, String> {
    api_data(
        b.api_v2_get(&publication_path(publication_id, rest))
            .await?,
    )
}

pub(crate) async fn discovery_inbox_impl<B: BackendCalls>(
    b: &B,
    limit: Option<i64>,
) -> Result<Vec<DiscoveryInboxDto>, String> {
    let path = path_with_query(
        "/scientia/discoveries",
        &[("limit", limit.map(|l| l.to_string()))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn discovery_ack_impl<B: BackendCalls>(b: &B, id: i64) -> Result<(), String> {
    tool_data(
        DISCOVERY_ACK_TOOL,
        b.call_tool(DISCOVERY_ACK_TOOL, json!({ "id": id })).await?,
    )
}

pub(crate) async fn autofill_impl<B: BackendCalls>(
    b: &B,
    publication_id: String,
    apply: bool,
) -> Result<AutofillResultDto, String> {
    let args = json!({ "publication_id": publication_id, "apply": apply });
    tool_data(AUTOFILL_TOOL, b.call_tool(AUTOFILL_TOOL, args).await?)
}

/// List the claims awaiting review for a publication.
#[tauri::command]
pub async fn list_publication_review_queue(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<Vec<ClaimAwaitingReviewDto>, String> {
    review_queue_impl(&backend.current().await, &publication_id).await
}

/// Record a human review decision for ONE claim, bound to the publication's current content
/// digest (a later content edit invalidates it).
#[tauri::command]
pub async fn record_publication_claim_review(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
    claim_id: i64,
    decision: String,
    reason: Option<String>,
) -> Result<ReviewDecisionDto, String> {
    claim_review_impl(
        &backend.current().await,
        publication_id,
        claim_id,
        decision,
        reason,
    )
    .await
}

/// Build + sign + offline-validate a nanopublication for an APPROVED claim; refused without a
/// recorded approval. Nothing is published to the network.
#[tauri::command]
pub async fn nanopublish_approved_claim(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
    claim_id: i64,
    orcid: Option<String>,
) -> Result<NanopubResultDto, String> {
    nanopublish_impl(&backend.current().await, publication_id, claim_id, orcid).await
}

/// ADVISORY LLM evidence suggestions for one claim; never mutates.
#[tauri::command]
pub async fn suggest_evidence_improvements(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
    claim_id: i64,
) -> Result<Vec<vox_scientia::evidence_assist::EvidenceSuggestion>, String> {
    evidence_impl(&backend.current().await, &publication_id, claim_id).await
}

#[tauri::command]
pub async fn get_novelty_assessment(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<NoveltyAssessmentDto, String> {
    publication_get_impl(&backend.current().await, &publication_id, "novelty").await
}

#[tauri::command]
pub async fn list_discovery_inbox(
    backend: State<'_, Arc<BackendState>>,
    limit: Option<i64>,
) -> Result<Vec<DiscoveryInboxDto>, String> {
    discovery_inbox_impl(&backend.current().await, limit).await
}

#[tauri::command]
pub async fn acknowledge_discovery(
    backend: State<'_, Arc<BackendState>>,
    id: i64,
) -> Result<(), String> {
    discovery_ack_impl(&backend.current().await, id).await
}

#[tauri::command]
pub async fn get_completion_report(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<CompletionReportDto, String> {
    publication_get_impl(&backend.current().await, &publication_id, "completion").await
}

/// Plan (and with `apply`, persist) metadata autofill from the server workspace's provenance.
#[tauri::command]
pub async fn run_autofill(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
    apply: bool,
) -> Result<AutofillResultDto, String> {
    autofill_impl(&backend.current().await, publication_id, apply).await
}

#[tauri::command]
pub async fn get_archive_status(
    backend: State<'_, Arc<BackendState>>,
    publication_id: String,
) -> Result<ArchiveStatusDto, String> {
    publication_get_impl(&backend.current().await, &publication_id, "archive").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    /// Guard: this GUI review surface must carry NO production-network publishing
    /// symbols (no network-publish toggle, no test-server toggle). The forbidden
    /// needles are assembled from fragments at runtime so this file cannot trip
    /// its own assertion. Mirrors the CLI guard in
    /// `crates/vox-cli/src/commands/scientia_nanopub.rs`.
    #[test]
    fn no_network_publish_symbol_in_gui_review_commands() {
        let src = include_str!("scientia_review.rs");
        let publish = format!("{}{}", "publish_to_", "network");
        let test_server = format!("{}{}", "use_test_", "server");
        assert!(!src.to_lowercase().contains(&publish));
        assert!(!src.contains(&test_server));
    }

    #[tokio::test]
    async fn publication_reads_encode_the_id_and_hit_the_scientia_routes() {
        let b = FakeBackend::new()
            .on_get(
                "/scientia/publications/pub%2F1/review-queue",
                Ok(json!({ "v": 1, "data": [] })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/claims/3/evidence",
                Ok(json!({ "v": 1, "data": [] })),
            )
            .on_get(
                "/scientia/discoveries?limit=4",
                Ok(json!({ "v": 1, "data": [] })),
            )
            .on_get(
                "/scientia/publications/pub%2F1/archive",
                Ok(json!({ "v": 1, "error": { "code": "not_found", "message": "x" } })),
            );
        assert!(review_queue_impl(&b, "pub/1").await.unwrap().is_empty());
        assert!(evidence_impl(&b, "pub/1", 3).await.unwrap().is_empty());
        assert!(discovery_inbox_impl(&b, Some(4)).await.unwrap().is_empty());
        let archive: Result<ArchiveStatusDto, String> =
            publication_get_impl(&b, "pub/1", "archive").await;
        assert!(archive.unwrap_err().starts_with("not_found"));
    }

    #[tokio::test]
    async fn review_writes_go_to_the_operator_ops() {
        let b = FakeBackend::new()
            .on_tool(
                CLAIM_REVIEW_TOOL,
                Ok(json!({ "success": true, "data": {
                    "claim_id": 3, "publication_id": "p", "decision": "approved",
                    "bound_digest": "d", "decided_at_ms": 1
                }})),
            )
            .on_tool(
                NANOPUBLISH_TOOL,
                Ok(json!({ "success": false, "error": "no review decision recorded" })),
            )
            .on_tool(
                DISCOVERY_ACK_TOOL,
                Ok(json!({ "success": true, "data": null })),
            );
        let decided = claim_review_impl(&b, "p".into(), 3, "approved".into(), None)
            .await
            .unwrap();
        assert_eq!(decided.bound_digest, "d");
        let refused = nanopublish_impl(&b, "p".into(), 3, None).await.unwrap_err();
        assert!(refused.contains("no review decision"), "{refused}");
        discovery_ack_impl(&b, 9).await.unwrap();
        let calls = b.calls();
        assert_eq!(calls[0].1["decision"], "approved");
        assert_eq!(
            calls[2],
            (format!("tool:{DISCOVERY_ACK_TOOL}"), json!({ "id": 9 }))
        );
    }
}
