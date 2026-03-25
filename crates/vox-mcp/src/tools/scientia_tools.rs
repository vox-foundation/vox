use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vox_publisher::publication::PublicationManifest;
use vox_publisher::scholarly::{LocalLedgerAdapter, ScholarlyAdapter};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationPrepareParams {
    pub publication_id: String,
    pub title: String,
    pub author: String,
    pub content: String,
    #[serde(default)]
    pub abstract_text: Option<String>,
    #[serde(default)]
    pub citations_json: Option<serde_json::Value>,
}

pub async fn vox_scientia_publication_prepare(
    state: &ServerState,
    params: VoxScientiaPublicationPrepareParams,
) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    let citations_json = params
        .citations_json
        .as_ref()
        .map(serde_json::Value::to_string);
    let manifest = PublicationManifest {
        publication_id: params.publication_id.clone(),
        content_type: "scientia".to_string(),
        source_ref: Some("mcp://vox_scientia_publication_prepare".to_string()),
        title: params.title,
        author: params.author,
        abstract_text: params.abstract_text,
        body_markdown: params.content,
        citations_json: citations_json.clone(),
        metadata_json: Some(
            serde_json::json!({
                "prepared_by": "vox_scientia_publication_prepare",
                "repository_id": state.repository.repository_id
            })
            .to_string(),
        ),
    };
    let digest = manifest.content_sha3_256();
    if let Err(e) = db
        .upsert_publication_manifest(vox_db::PublicationManifestParams {
            publication_id: &manifest.publication_id,
            content_type: &manifest.content_type,
            source_ref: manifest.source_ref.as_deref(),
            title: &manifest.title,
            author: &manifest.author,
            abstract_text: manifest.abstract_text.as_deref(),
            body_markdown: &manifest.body_markdown,
            citations_json: citations_json.as_deref(),
            metadata_json: manifest.metadata_json.as_deref(),
            content_sha3_256: &digest,
            state: "draft",
        })
        .await
    {
        return ToolResult::<String>::err(format!("DB error: {e}")).to_json();
    }
    ToolResult::ok(serde_json::json!({
        "publication_id": manifest.publication_id,
        "content_type": manifest.content_type,
        "digest": digest,
        "state": "draft",
    }))
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationApproveParams {
    pub publication_id: String,
    pub approver: String,
}

pub async fn vox_scientia_publication_approve(
    state: &ServerState,
    params: VoxScientiaPublicationApproveParams,
) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    let Some(manifest) = (match db.get_publication_manifest(&params.publication_id).await {
        Ok(m) => m,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    }) else {
        return ToolResult::<String>::err("publication not found".to_string()).to_json();
    };
    let approver = params.approver.trim();
    if approver.is_empty() {
        return ToolResult::<String>::err("approver must not be empty".to_string()).to_json();
    }
    if let Err(e) = db
        .record_publication_approval_for_digest(
            &params.publication_id,
            &manifest.content_sha3_256,
            approver,
        )
        .await
    {
        return ToolResult::<String>::err(format!("DB error: {e}")).to_json();
    }
    let count = match db
        .count_publication_approvers_for_digest(&params.publication_id, &manifest.content_sha3_256)
        .await
    {
        Ok(c) => c,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    if count >= 2 {
        let _ = db
            .set_publication_state(&params.publication_id, "approved", None)
            .await;
    }
    ToolResult::ok(serde_json::json!({
        "publication_id": params.publication_id,
        "digest": manifest.content_sha3_256,
        "distinct_approver_count": count,
        "dual_approval_met": count >= 2
    }))
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationSubmitLocalParams {
    pub publication_id: String,
}

pub async fn vox_scientia_publication_submit_local(
    state: &ServerState,
    params: VoxScientiaPublicationSubmitLocalParams,
) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    let Some(row) = (match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    }) else {
        return ToolResult::<String>::err("publication not found".to_string()).to_json();
    };
    let dual = match db
        .has_dual_publication_approval_for_digest(&params.publication_id, &row.content_sha3_256)
        .await
    {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    if !dual {
        return ToolResult::<String>::err(
            "publication requires two distinct digest-bound approvals before submission"
                .to_string(),
        )
        .to_json();
    }
    let manifest = PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type,
        source_ref: row.source_ref,
        title: row.title,
        author: row.author,
        abstract_text: row.abstract_text,
        body_markdown: row.body_markdown,
        citations_json: row.citations_json,
        metadata_json: row.metadata_json,
    };
    let adapter = LocalLedgerAdapter;
    let receipt = match adapter.submit(&manifest) {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("submit error: {e}")).to_json(),
    };
    if let Err(e) = db
        .upsert_scholarly_submission(
            &params.publication_id,
            &row.content_sha3_256,
            &receipt.adapter,
            &receipt.external_submission_id,
            &receipt.status,
            receipt.response_fingerprint.as_deref(),
            receipt.metadata_json.as_deref(),
        )
        .await
    {
        return ToolResult::<String>::err(format!("DB error: {e}")).to_json();
    }
    ToolResult::ok(receipt).to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationStatusParams {
    pub publication_id: String,
}

#[derive(Debug, Serialize)]
struct ScientiaPublicationStatusBody {
    publication_id: String,
    content_type: String,
    state: String,
    digest: String,
    version: i64,
    approvals_for_digest: i64,
    scholarly_submissions: Vec<vox_db::ScholarlySubmissionRow>,
}

pub async fn vox_scientia_publication_status(
    state: &ServerState,
    params: VoxScientiaPublicationStatusParams,
) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    let Some(row) = (match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    }) else {
        return ToolResult::<String>::err("publication not found".to_string()).to_json();
    };
    let approvals = match db
        .count_publication_approvers_for_digest(&params.publication_id, &row.content_sha3_256)
        .await
    {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    let submissions = match db.list_scholarly_submissions(&params.publication_id).await {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    ToolResult::ok(ScientiaPublicationStatusBody {
        publication_id: row.publication_id,
        content_type: row.content_type,
        state: row.state,
        digest: row.content_sha3_256,
        version: row.version,
        approvals_for_digest: approvals,
        scholarly_submissions: submissions,
    })
    .to_json()
}
