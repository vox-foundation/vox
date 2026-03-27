use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vox_publisher::publication::PublicationManifest;
use vox_publisher::publication_preflight::PreflightProfile;
use vox_publisher::scientific_metadata::ScientificPublicationMetadata;

use vox_publisher::scholarly_external_jobs::publication_scholarly_submit_with_ledger;

use super::common::{
    REM_PUBLICATION_ID, REM_SCIENTIA_APPROVER, REM_SCIENTIA_DB, REM_SCIENTIA_METADATA,
    no_voxdb_tool_string,
};

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
    #[serde(default)]
    pub scholarly_metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub preflight: bool,
    #[serde(default)]
    pub preflight_profile: Option<PreflightProfileParam>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreflightProfileParam {
    #[default]
    Default,
    DoubleBlind,
    MetadataComplete,
}

impl From<PreflightProfileParam> for PreflightProfile {
    fn from(p: PreflightProfileParam) -> Self {
        match p {
            PreflightProfileParam::Default => Self::Default,
            PreflightProfileParam::DoubleBlind => Self::DoubleBlind,
            PreflightProfileParam::MetadataComplete => Self::MetadataComplete,
        }
    }
}

pub async fn vox_scientia_publication_prepare(
    state: &ServerState,
    params: VoxScientiaPublicationPrepareParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let citations_json = params
        .citations_json
        .as_ref()
        .map(serde_json::Value::to_string);
    let scientific = match params.scholarly_metadata.as_ref() {
        None => None,
        Some(v) => match serde_json::from_value::<ScientificPublicationMetadata>(v.clone()) {
            Ok(s) => Some(s),
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(
                    format!("scholarly_metadata: {e}"),
                    REM_SCIENTIA_METADATA,
                )
                .to_json();
            }
        },
    };
    let profile: PreflightProfile = params.preflight_profile.unwrap_or_default().into();
    let metadata_json = match vox_publisher::scientific_metadata::build_scientia_metadata_json(
        "vox_scientia_publication_prepare",
        Some(state.repository.repository_id.as_str()),
        scientific.as_ref(),
        None,
    ) {
        Ok(s) => s,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("metadata_json: {e}"),
                REM_SCIENTIA_METADATA,
            )
            .to_json();
        }
    };
    let manifest = PublicationManifest {
        publication_id: params.publication_id.clone(),
        content_type: "scientia".to_string(),
        source_ref: Some("mcp://vox_scientia_publication_prepare".to_string()),
        title: params.title,
        author: params.author,
        abstract_text: params.abstract_text,
        body_markdown: params.content,
        citations_json: citations_json.clone(),
        metadata_json: Some(metadata_json),
    };

    if params.preflight {
        let report = vox_publisher::publication_preflight::run_preflight(&manifest, profile);
        if !report.ok {
            return ToolResult::<()>::err_with_remediation(
                format!(
                    "preflight failed: {}",
                    serde_json::to_string(&report).unwrap_or_default()
                ),
                "Fix readiness findings on the manifest or pass a different `preflight_profile`; mirror check with `vox scientia publication-preflight`.",
            )
            .to_json();
        }
    }

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
        return ToolResult::<String>::err_with_remediation(
            format!("DB error: {e}"),
            REM_SCIENTIA_DB,
        )
        .to_json();
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
        return no_voxdb_tool_string();
    };
    let manifest = match db.get_publication_manifest(&params.publication_id).await {
        Ok(m) => m,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let Some(manifest) = manifest else {
        return ToolResult::<String>::err_with_remediation(
            "publication not found",
            REM_PUBLICATION_ID,
        )
        .to_json();
    };
    let approver = params.approver.trim();
    if approver.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "approver must not be empty".to_string(),
            REM_SCIENTIA_APPROVER,
        )
        .to_json();
    }
    if let Err(e) = db
        .record_publication_approval_for_digest(
            &params.publication_id,
            &manifest.content_sha3_256,
            approver,
        )
        .await
    {
        return ToolResult::<String>::err_with_remediation(
            format!("DB error: {e}"),
            REM_SCIENTIA_DB,
        )
        .to_json();
    }
    let count = match db
        .count_publication_approvers_for_digest(&params.publication_id, &manifest.content_sha3_256)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
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
    /// When set, submit with this adapter (`zenodo`, `openreview`, …) instead of `VOX_SCHOLARLY_ADAPTER`.
    #[serde(default)]
    pub adapter: Option<String>,
}

pub async fn vox_scientia_publication_submit_local(
    state: &ServerState,
    params: VoxScientiaPublicationSubmitLocalParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let adapter = params
        .adapter
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    match publication_scholarly_submit_with_ledger(
        db,
        params.publication_id.trim(),
        adapter,
    )
    .await
    {
        Ok(receipt) => ToolResult::ok(receipt).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(
            e.to_string(),
            "Verify `VOX_SCHOLARLY_*` flags, adapter credentials (Clavis / env), and that live adapters are not disabled.",
        )
        .to_json(),
    }
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
    media_assets: Vec<vox_db::PublicationMediaAssetRow>,
    publication_attempts: Vec<vox_db::PublicationAttemptRow>,
    publication_status_events: Vec<vox_db::PublicationStatusEventRow>,
}

pub async fn vox_scientia_publication_status(
    state: &ServerState,
    params: VoxScientiaPublicationStatusParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let Some(row) = row else {
        return ToolResult::<String>::err_with_remediation(
            "publication not found",
            REM_PUBLICATION_ID,
        )
        .to_json();
    };
    let approvals = match db
        .count_publication_approvers_for_digest(&params.publication_id, &row.content_sha3_256)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let submissions = match db.list_scholarly_submissions(&params.publication_id).await {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let media_assets = match db
        .list_publication_media_assets(&params.publication_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let publication_attempts = match db.list_publication_attempts(&params.publication_id).await {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    let publication_status_events = match db
        .list_publication_status_events(&params.publication_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    ToolResult::ok(ScientiaPublicationStatusBody {
        publication_id: row.publication_id,
        content_type: row.content_type,
        state: row.state,
        digest: row.content_sha3_256,
        version: row.version,
        approvals_for_digest: approvals,
        scholarly_submissions: submissions,
        media_assets,
        publication_attempts,
        publication_status_events,
    })
    .to_json()
}
