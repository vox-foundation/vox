use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vox_publisher::publication::PublicationManifest;
use vox_publisher::publication_preflight::PreflightProfile;
use vox_publisher::scholarly::{LocalLedgerAdapter, ScholarlyAdapter};
use vox_publisher::scientific_metadata::ScientificPublicationMetadata;

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
}

impl From<PreflightProfileParam> for PreflightProfile {
    fn from(p: PreflightProfileParam) -> Self {
        match p {
            PreflightProfileParam::Default => Self::Default,
            PreflightProfileParam::DoubleBlind => Self::DoubleBlind,
        }
    }
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
    let scientific = match params.scholarly_metadata.as_ref() {
        None => None,
        Some(v) => match serde_json::from_value::<ScientificPublicationMetadata>(v.clone()) {
            Ok(s) => Some(s),
            Err(e) => {
                return ToolResult::<String>::err(format!("scholarly_metadata: {e}")).to_json();
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
        Err(e) => return ToolResult::<String>::err(format!("metadata_json: {e}")).to_json(),
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
            return ToolResult::<()>::err(format!(
                "preflight failed: {}",
                serde_json::to_string(&report).unwrap_or_default()
            ))
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
    let manifest = match db.get_publication_manifest(&params.publication_id).await {
        Ok(m) => m,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    let Some(manifest) = manifest else {
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
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    let Some(row) = row else {
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
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    let Some(row) = row else {
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationPreflightParams {
    pub publication_id: String,
    #[serde(default)]
    pub profile: Option<PreflightProfileParam>,
    /// When true, attach [`vox_publisher::publication_worthiness::WorthinessEvaluation`] (`contracts/scientia/publication-worthiness.default.yaml` from repo root).
    #[serde(default)]
    pub with_worthiness: bool,
}

pub async fn vox_scientia_publication_preflight(
    state: &ServerState,
    params: VoxScientiaPublicationPreflightParams,
) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    let Some(row) = row else {
        return ToolResult::<String>::err("publication not found".to_string()).to_json();
    };
    let mut manifest = PublicationManifest {
        publication_id: row.publication_id,
        content_type: row.content_type,
        source_ref: row.source_ref,
        title: row.title,
        author: row.author,
        abstract_text: row.abstract_text,
        body_markdown: row.body_markdown,
        citations_json: row.citations_json,
        metadata_json: row.metadata_json,
    };
    let profile: PreflightProfile = params.profile.unwrap_or_default().into();
    let report = if params.with_worthiness {
        let rid = manifest
            .metadata_json
            .as_deref()
            .and_then(|raw| {
                let v: serde_json::Value = serde_json::from_str(raw).ok()?;
                v.get("repository_id")
                    .and_then(|x| x.as_str())
                    .map(std::string::ToString::to_string)
            })
            .unwrap_or_else(|| state.repository.repository_id.clone());
        match db
            .merge_scientia_live_socrates_into_metadata_json(
                manifest.metadata_json.as_deref(),
                rid.as_str(),
            )
            .await
        {
            Ok(s) => manifest.metadata_json = Some(s),
            Err(e) => {
                return ToolResult::<String>::err(format!("socrates telemetry merge: {e}"))
                    .to_json();
            }
        }
        let path = state
            .repository
            .root
            .join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH);
        let yaml = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                return ToolResult::<String>::err(format!(
                    "read worthiness contract {}: {e}",
                    path.display()
                ))
                .to_json();
            }
        };
        let contract = match vox_publisher::publication_worthiness::load_contract_from_str(&yaml) {
            Ok(c) => c,
            Err(e) => {
                return ToolResult::<String>::err(format!("parse worthiness contract: {e}"))
                    .to_json();
            }
        };
        if let Err(e) =
            vox_publisher::publication_worthiness::validate_contract_invariants(&contract)
        {
            return ToolResult::<String>::err(format!("worthiness contract invariants: {e}"))
                .to_json();
        }
        vox_publisher::publication_preflight::run_preflight_with_worthiness(
            &manifest, profile, &contract,
        )
    } else {
        vox_publisher::publication_preflight::run_preflight(&manifest, profile)
    };
    ToolResult::ok(report).to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaWorthinessEvaluateParams {
    /// Repo-relative contract YAML (defaults to `contracts/scientia/publication-worthiness.default.yaml`).
    #[serde(default)]
    pub contract_yaml_relative: Option<String>,
    /// [`vox_publisher::publication_worthiness::WorthinessInputs`] as a JSON object.
    pub metrics: serde_json::Value,
}

/// Local-only worthiness gate: load contract from the discovered repository root; no DB writes.
pub async fn vox_scientia_worthiness_evaluate(
    state: &ServerState,
    params: VoxScientiaWorthinessEvaluateParams,
) -> String {
    let root = &state.repository.root;
    let contract_path = match params.contract_yaml_relative {
        Some(rel) if !rel.trim().is_empty() => root.join(rel.trim()),
        _ => root.join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH),
    };
    let yaml = match std::fs::read_to_string(&contract_path) {
        Ok(s) => s,
        Err(e) => {
            return ToolResult::<String>::err(format!(
                "read contract {}: {e}",
                contract_path.display()
            ))
            .to_json();
        }
    };
    let contract = match vox_publisher::publication_worthiness::load_contract_from_str(&yaml) {
        Ok(c) => c,
        Err(e) => {
            return ToolResult::<String>::err(format!("parse contract YAML: {e}")).to_json();
        }
    };
    if let Err(e) = vox_publisher::publication_worthiness::validate_contract_invariants(&contract) {
        return ToolResult::<String>::err(format!("contract invariants: {e}")).to_json();
    }
    let inputs: vox_publisher::publication_worthiness::WorthinessInputs =
        match serde_json::from_value(params.metrics) {
            Ok(i) => i,
            Err(e) => return ToolResult::<String>::err(format!("metrics: {e}")).to_json(),
        };
    let out = vox_publisher::publication_worthiness::evaluate_worthiness(&contract, &inputs);
    ToolResult::ok(out).to_json()
}
