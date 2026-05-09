use crate::params::ToolResult;
use crate::server_state::ServerState;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vox_publisher::publication::PublicationManifest;
use vox_publisher::publication_preflight::PreflightProfile;
use vox_publisher::scientia_discovery::DiscoveryIntakeGate;
use vox_publisher::scientia_evidence::ScientiaEvidenceContext;
use vox_publisher::scientific_metadata::ScientificPublicationMetadata;

use vox_publisher::scholarly_external_jobs::publication_scholarly_submit_with_ledger;

use super::common::{
    REM_PUBLICATION_ID, REM_SCIENTIA_APPROVER, REM_SCIENTIA_DB, REM_SCIENTIA_METADATA,
    no_voxdb_tool_string, publication_manifest_from_row,
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
    /// Optional [`ScientiaEvidenceContext`] merged under `metadata_json.scientia_evidence`.
    #[serde(default)]
    pub scientia_evidence: Option<serde_json::Value>,
    #[serde(default)]
    pub discovery_intake_gate: DiscoveryIntakeGateParam,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryIntakeGateParam {
    #[default]
    None,
    StrongSignalsOnly,
    AllowReviewSuggested,
}

impl From<DiscoveryIntakeGateParam> for DiscoveryIntakeGate {
    fn from(p: DiscoveryIntakeGateParam) -> Self {
        match p {
            DiscoveryIntakeGateParam::None => Self::None,
            DiscoveryIntakeGateParam::StrongSignalsOnly => Self::StrongSignalsOnly,
            DiscoveryIntakeGateParam::AllowReviewSuggested => Self::AllowReviewSuggested,
        }
    }
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreflightProfileParam {
    #[default]
    Default,
    DoubleBlind,
    MetadataComplete,
    ArxivAssist,
}

impl From<PreflightProfileParam> for PreflightProfile {
    fn from(p: PreflightProfileParam) -> Self {
        match p {
            PreflightProfileParam::Default => Self::Default,
            PreflightProfileParam::DoubleBlind => Self::DoubleBlind,
            PreflightProfileParam::MetadataComplete => Self::MetadataComplete,
            PreflightProfileParam::ArxivAssist => Self::ArxivAssist,
        }
    }
}

async fn publication_attention_inputs_for_row(
    db: &vox_db::VoxDb,
    row: &vox_db::PublicationManifestRow,
    item: &vox_publisher::types::UnifiedNewsItem,
    orchestrator_dry_run: bool,
    publish_armed: bool,
) -> Result<vox_publisher::publication_preflight::PreflightAttentionInputs, String> {
    let dual = db
        .has_dual_publication_approval_for_digest(
            row.publication_id.as_str(),
            row.content_sha3_256.as_str(),
        )
        .await
        .map_err(|e| e.to_string())?;
    let gate = vox_publisher::gate::evaluate_publish_gate(
        vox_publisher::gate::publish_gate_inputs_for_mcp(
            false,
            orchestrator_dry_run,
            publish_armed,
            true,
            dual,
            item,
        ),
    );
    Ok(vox_publisher::publication_preflight::PreflightAttentionInputs { gate: Some(gate) })
}

pub(super) async fn publication_preflight_report_for_row(
    db: &vox_db::VoxDb,
    row: &vox_db::PublicationManifestRow,
    manifest: &PublicationManifest,
    profile: PreflightProfile,
    orchestrator_dry_run: bool,
    publish_armed: bool,
    repo_root: &std::path::Path,
    repository_id_fallback: Option<&str>,
    with_worthiness: bool,
) -> Result<vox_publisher::publication_preflight::PreflightReport, String> {
    let item = vox_publisher::switching::unified_news_item_from_manifest_parts(
        row.publication_id.as_str(),
        row.title.as_str(),
        row.author.as_str(),
        row.body_markdown.as_str(),
        row.metadata_json.as_deref(),
    )
    .map_err(|e| format!("parse metadata_json for gate: {e}"))?;
    let attention =
        publication_attention_inputs_for_row(db, row, &item, orchestrator_dry_run, publish_armed)
            .await?;
    if with_worthiness {
        let mut manifest = manifest.clone();
        let rid = manifest
            .metadata_json
            .as_deref()
            .and_then(|raw| {
                let v: serde_json::Value = serde_json::from_str(raw).ok()?;
                v.get("repository_id")
                    .and_then(|x| x.as_str())
                    .map(std::string::ToString::to_string)
            })
            .or_else(|| repository_id_fallback.map(std::string::ToString::to_string));
        if let Some(rid) = rid {
            let merged = db
                .merge_scientia_live_socrates_into_metadata_json(
                    manifest.metadata_json.as_deref(),
                    rid.as_str(),
                )
                .await
                .map_err(|e| format!("socrates telemetry merge: {e}"))?;
            manifest.metadata_json = Some(merged);
        }
        if let Some(updated) =
            vox_publisher::scientia_evidence::enrich_metadata_json_with_repo_files(
                manifest.metadata_json.as_deref(),
                repo_root,
            )
            .map_err(|e| format!("scientia_evidence file hydration: {e}"))?
        {
            manifest.metadata_json = Some(updated);
        }
        let contract_path =
            repo_root.join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH);
        let yaml = std::fs::read_to_string(&contract_path)
            .map_err(|e| format!("read worthiness contract {}: {e}", contract_path.display()))?;
        let contract = vox_publisher::publication_worthiness::load_contract_from_str(&yaml)
            .map_err(|e| format!("parse worthiness contract: {e}"))?;
        vox_publisher::publication_worthiness::validate_contract_invariants(&contract)
            .map_err(|e| format!("worthiness contract invariants: {e}"))?;
        Ok(
            vox_publisher::publication_preflight::run_preflight_with_worthiness_attention(
                &manifest,
                profile,
                &contract,
                Some(attention),
            ),
        )
    } else {
        Ok(
            vox_publisher::publication_preflight::run_preflight_with_attention(
                manifest,
                profile,
                Some(attention),
            ),
        )
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
    let scientia_evidence: Option<ScientiaEvidenceContext> = match params.scientia_evidence.as_ref()
    {
        None => None,
        Some(v) => match serde_json::from_value::<ScientiaEvidenceContext>(v.clone()) {
            Ok(e) => Some(e),
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(
                    format!("scientia_evidence: {e}"),
                    REM_SCIENTIA_METADATA,
                )
                .to_json();
            }
        },
    };
    let intake_gate: DiscoveryIntakeGate = params.discovery_intake_gate.into();
    if intake_gate != DiscoveryIntakeGate::None {
        let empty_rank_evidence = ScientiaEvidenceContext::default();
        let ev_ref = scientia_evidence.as_ref().unwrap_or(&empty_rank_evidence);
        let scientia_h =
            vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(
                &state.repository.root,
            );
        let rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
            params.publication_id.as_str(),
            Some("mcp://vox_scientia_publication_prepare"),
            ev_ref,
            &scientia_h,
            None,
        );
        if !vox_publisher::scientia_discovery::intake_gate_allows(intake_gate, &rank) {
            return ToolResult::<()>::err_with_remediation(
                format!(
                    "discovery_intake_gate rejected prepare: gate={intake_gate:?} rank={}",
                    serde_json::to_string(&rank).unwrap_or_default()
                ),
                "Relax `discovery_intake_gate`, attach a stronger `scientia_evidence` block, or use `vox db publication-prepare` with repo-local eval/benchmark sidecars.",
            )
            .to_json();
        }
    }
    let metadata_json = match vox_publisher::scientific_metadata::build_scientia_metadata_json(
        "vox_scientia_publication_prepare",
        Some(state.repository.repository_id.as_str()),
        scientific.as_ref(),
        scientia_evidence.as_ref(),
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
            revision_history_json: None,
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
            "Verify `VOX_SCHOLARLY_*` flags, adapter credentials (Secrets / env), and that live adapters are not disabled.",
        )
        .to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationStatusParams {
    pub publication_id: String,
    #[serde(default)]
    pub with_worthiness: bool,
}

#[derive(Debug, Serialize)]
struct ScientiaPublicationStatusBody {
    publication_id: String,
    content_type: String,
    state: String,
    digest: String,
    version: i64,
    approvals_for_digest: i64,
    preflight_report: vox_publisher::publication_preflight::PreflightReport,
    discovery_rank: vox_publisher::scientia_discovery::DiscoveryCandidateRank,
    manifest_completion: vox_publisher::scientia_discovery::ManifestCompletionReport,
    evidence_completeness_0_100: u8,
    transform_preview: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    impact_readership_projection:
        Option<vox_publisher::scientia_finding_ledger::ImpactReadershipProjectionV1>,
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
    let manifest = publication_manifest_from_row(&row);
    let preflight_report = match publication_preflight_report_for_row(
        db,
        &row,
        &manifest,
        PreflightProfile::Default,
        state.orchestrator_config.news.dry_run,
        state.orchestrator_config.news.publish_armed,
        &state.repository.root,
        Some(state.repository.repository_id.as_str()),
        params.with_worthiness,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(e, REM_SCIENTIA_METADATA).to_json();
        }
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
    let evidence =
        vox_publisher::scientia_evidence::parse_scientia_evidence(row.metadata_json.as_deref());
    let evidence_fallback = vox_publisher::scientia_evidence::ScientiaEvidenceContext::default();
    let evidence_ref = evidence.as_ref().unwrap_or(&evidence_fallback);
    let scientia_h = vox_publisher::scientia_heuristics::ScientiaHeuristics::load_from_repo_root(
        &state.repository.root,
    );
    let discovery_rank = vox_publisher::scientia_discovery::rank_candidate_heuristics(
        params.publication_id.as_str(),
        row.source_ref.as_deref(),
        evidence_ref,
        &scientia_h,
        None,
    );
    let manifest_completion =
        vox_publisher::scientia_discovery::manifest_completion_report(&manifest);
    let evidence_completeness_0_100 =
        vox_publisher::scientia_discovery::evidence_completeness_score(evidence_ref, &scientia_h);
    let transform_preview = vox_publisher::scientia_discovery::destination_transform_previews(
        &manifest,
        evidence.as_ref(),
    );
    let impact_readership_projection =
        vox_publisher::scientia_prior_art::parse_novelty_bundle_from_metadata_json(
            row.metadata_json.as_deref(),
        )
        .map(|b| {
            vox_publisher::scientia_finding_ledger::impact_readership_projection_v1(&b, &scientia_h)
        });
    ToolResult::ok(ScientiaPublicationStatusBody {
        publication_id: row.publication_id,
        content_type: row.content_type,
        state: row.state,
        digest: row.content_sha3_256,
        version: row.version,
        approvals_for_digest: approvals,
        preflight_report,
        discovery_rank,
        manifest_completion,
        evidence_completeness_0_100,
        transform_preview,
        impact_readership_projection,
        scholarly_submissions: submissions,
        media_assets,
        publication_attempts,
        publication_status_events,
    })
    .to_json()
}
