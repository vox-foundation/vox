use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vox_publisher::publication::PublicationManifest;
use vox_publisher::publication_preflight::PreflightProfile;
use vox_publisher::scholarly_external_jobs::{
    poll_scholarly_remote_status_all_submissions_for_publication,
    poll_scholarly_remote_status_batch, poll_scholarly_remote_status_persist,
    publication_scholarly_submit_with_ledger,
};
use vox_publisher::scientific_metadata::ScientificPublicationMetadata;
use vox_publisher::types::UnifiedNewsItem;

const REM_VOXDB: &str = "Attach Turso/VoxDb to the MCP server, or run the same flow via `vox db` / `vox scientia` in a configured shell.";
const REM_PUBLICATION_ID: &str = "Run `vox scientia publication-prepare` (or verify the publication id) before this step.";
const REM_SCIENTIA_DB: &str =
    "Verify Turso/VoxDb connectivity and vox-db publication/scientia table migrations.";
const REM_SCIENTIA_APPROVER: &str =
    "Pass a non-empty `approver` string when recording publication approvals.";
const REM_SCIENTIA_ARXIV: &str =
    "When using `published` stage metadata, include a non-empty `arxiv_id`.";
const REM_SCIENTIA_OUTPUT_DIR: &str =
    "Set `output_dir` to a writable directory for scholarly pipeline artifacts.";
const REM_SCIENTIA_METADATA: &str =
    "Fix `scholarly_metadata` / manifest JSON to match SCIENTIA contracts (see scientia handbook).";
const REM_SCIENTIA_SIMULATE: &str =
    "Inspect simulate/gate output and manifest state; resolve blockers then retry.";
const REM_SCIENTIA_PUBLISH: &str =
    "Check syndication channel config, dry-run flags, approvals, and publisher credentials for live paths.";
const REM_SCIENTIA_REMOTE: &str =
    "Ensure `publication_id` has scholarly submission rows; pass `external_submission_id` when disambiguating.";
const REM_SCIENTIA_EXT_SUBMIT: &str =
    "When provided, `external_submission_id` must be a non-empty id from the submissions table.";
const REM_WORTHINESS_CONTRACT: &str =
    "Ensure SCIENTIA worthiness YAML under the repo root is readable and schema-valid (see `contracts/scientia/`).";
const REM_SCIENTIA_STAGE: &str =
    "Use stage tokens `staging_exported`, `operator_ack`, `bundle_validated`, `submitted`, or `published` (see arXiv handoff tool docs).";
const REM_SCIENTIA_ATTEMPTS: &str =
    "Run `publication_publish` or `route_simulate` first so syndication attempts exist for the current manifest digest.";

#[inline]
fn no_voxdb_tool_string() -> String {
    ToolResult::<String>::err_with_remediation("VoxDb is not connected", REM_VOXDB).to_json()
}

#[inline]
fn no_voxdb_syndication(compact: bool) -> String {
    ToolResult::<vox_publisher::SyndicationResult>::err_with_remediation(
        "VoxDb is not connected",
        REM_VOXDB,
    )
    .to_json_styled(compact)
}

#[inline]
fn no_voxdb_json_envelope(compact: bool) -> String {
    ToolResult::<serde_json::Value>::err_with_remediation("VoxDb is not connected", REM_VOXDB)
        .to_json_styled(compact)
}

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

fn publication_manifest_from_row(row: &vox_db::PublicationManifestRow) -> PublicationManifest {
    PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    }
}

fn worthiness_score_for_row(row: &vox_db::PublicationManifestRow) -> Option<f64> {
    let m = publication_manifest_from_row(row);
    let root = vox_repository::resolve_repo_root_for_ci();
    vox_publisher::publication_worthiness::worthiness_score_for_publication_manifest(&m, &root).ok()
}

fn mcp_social_worthiness_enforce(state: &ServerState) -> bool {
    state.orchestrator_config.news.worthiness_enforce
        || std::env::var("VOX_SOCIAL_WORTHINESS_ENFORCE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
}

fn mcp_social_worthiness_score_min(state: &ServerState) -> f64 {
    state
        .orchestrator_config
        .news
        .worthiness_score_min
        .or_else(|| {
            std::env::var("VOX_SOCIAL_WORTHINESS_SCORE_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0.85)
}

fn operator_publisher_config(
    state: &ServerState,
    dry_run: bool,
    worthiness_score: Option<f64>,
) -> vox_publisher::PublisherConfig {
    let mut site = vox_publisher::NewsSiteConfig {
        base_url: state
            .orchestrator_config
            .news
            .site_base_url
            .clone()
            .unwrap_or_else(|| {
                vox_publisher::contract::DEFAULT_SITE_BASE_URL
                    .trim_end_matches('/')
                    .to_string()
            }),
        ..Default::default()
    };
    if let Some(ref p) = state.orchestrator_config.news.rss_feed_path {
        let t = p.trim();
        if !t.is_empty() {
            site.rss_feed_path = std::path::PathBuf::from(t);
        }
    }
    site.merge_operator_env_overrides();
    let mut cfg = vox_publisher::PublisherConfig::from_operator_environment(
        dry_run,
        Some(vox_repository::resolve_repo_root_for_ci()),
        site,
    );
    let news = &state.orchestrator_config.news;
    if cfg.twitter_text_chunk_max.is_none() {
        cfg.twitter_text_chunk_max = news.twitter_text_chunk_max;
    }
    if cfg.twitter_truncation_suffix.is_none() {
        if let Some(ref s) = news.twitter_truncation_suffix {
            let t = s.trim();
            if !t.is_empty() {
                cfg.twitter_truncation_suffix = Some(t.to_string());
            }
        }
    }
    if cfg.twitter_api_base.is_none() {
        if let Some(ref b) = news.twitter_api_base {
            let t = b.trim();
            if !t.is_empty() {
                cfg.twitter_api_base = Some(t.to_string());
            }
        }
    }
    if cfg.github_rest_base.is_none() {
        if let Some(ref b) = news.github_rest_base {
            let t = b.trim();
            if !t.is_empty() {
                cfg.github_rest_base = Some(t.to_string());
            }
        }
    }
    if cfg.github_graphql_url.is_none() {
        if let Some(ref u) = news.github_graphql_url {
            let t = u.trim();
            if !t.is_empty() {
                cfg.github_graphql_url = Some(t.to_string());
            }
        }
    }
    if cfg.opencollective_graphql_url.is_none() {
        if let Some(ref u) = news.opencollective_graphql_url {
            let t = u.trim();
            if !t.is_empty() {
                cfg.opencollective_graphql_url = Some(t.to_string());
            }
        }
    }
    cfg.worthiness_score = worthiness_score;
    cfg
}

fn unified_news_item_from_manifest_row(
    row: &vox_db::PublicationManifestRow,
) -> Result<UnifiedNewsItem, String> {
    vox_publisher::switching::unified_news_item_from_manifest_parts(
        &row.publication_id,
        &row.title,
        &row.author,
        &row.body_markdown,
        row.metadata_json.as_deref(),
    )
    .map_err(|e| e.to_string())
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
            return ToolResult::<String>::err_with_remediation(format!("metadata_json: {e}"), REM_SCIENTIA_METADATA)
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
        return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json();
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
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let Some(manifest) = manifest else {
        return ToolResult::<String>::err_with_remediation("publication not found", REM_PUBLICATION_ID)
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
        return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json();
    }
    let count = match db
        .count_publication_approvers_for_digest(&params.publication_id, &manifest.content_sha3_256)
        .await
    {
        Ok(c) => c,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
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
    let adapter = params.adapter.as_deref().map(str::trim).filter(|s| !s.is_empty());
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
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let Some(row) = row else {
        return ToolResult::<String>::err_with_remediation("publication not found", REM_PUBLICATION_ID)
            .to_json();
    };
    let approvals = match db
        .count_publication_approvers_for_digest(&params.publication_id, &row.content_sha3_256)
        .await
    {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let submissions = match db.list_scholarly_submissions(&params.publication_id).await {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let media_assets = match db
        .list_publication_media_assets(&params.publication_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let publication_attempts = match db.list_publication_attempts(&params.publication_id).await {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let publication_status_events = match db
        .list_publication_status_events(&params.publication_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyRemoteStatusParams {
    pub publication_id: String,
    #[serde(default)]
    pub external_submission_id: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyRemoteStatusSyncAllParams {
    pub publication_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyRemoteStatusSyncBatchParams {
    #[serde(default = "default_scholarly_remote_sync_batch_limit")]
    pub limit: i64,
    #[serde(default = "default_one_u32")]
    pub iterations: u32,
    #[serde(default)]
    pub interval_secs: u64,
    #[serde(default)]
    pub max_runtime_secs: Option<u64>,
    #[serde(default)]
    pub jitter_secs: u64,
}

fn default_scholarly_remote_sync_batch_limit() -> i64 {
    25
}

fn default_one_u32() -> u32 {
    1
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyStagingExportParams {
    pub publication_id: String,
    /// Absolute or process-relative directory; created if missing.
    pub output_dir: String,
    /// `zenodo`, `openreview`, or `arxiv-assist` (same tokens as `ScholarlyVenue::parse`).
    pub venue: String,
}

pub async fn vox_scientia_publication_scholarly_staging_export(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyStagingExportParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let publication_id = params.publication_id.trim();
    if publication_id.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "publication_id must not be empty".to_string(),
            REM_PUBLICATION_ID,
        )
        .to_json();
    }
    let out_s = params.output_dir.trim();
    if out_s.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "output_dir must not be empty".to_string(),
            REM_SCIENTIA_OUTPUT_DIR,
        )
        .to_json();
    }
    let venue_raw = params.venue.trim();
    let Some(venue) = vox_publisher::submission_package::ScholarlyVenue::parse(venue_raw) else {
        return ToolResult::<String>::err_with_remediation(
            format!("unknown venue {venue_raw:?}"),
            "Use `zenodo`, `openreview`, or `arxiv-assist` (see `ScholarlyVenue::parse` in vox-publisher).",
        )
        .to_json();
    };
    let output_dir = std::path::PathBuf::from(out_s);
    let row = match db.get_publication_manifest(publication_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return ToolResult::<String>::err_with_remediation(
                format!("publication not found: {publication_id}"),
                REM_PUBLICATION_ID,
            )
            .to_json();
        }
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let manifest = PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let written = match vox_publisher::submission_package::write_scholarly_staging(
        &manifest,
        venue,
        &output_dir,
    ) {
        Ok(w) => w,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_OUTPUT_DIR).to_json();
        }
    };
    if let Err(findings) =
        vox_publisher::submission_package::validate_scholarly_staging(&output_dir, venue, &manifest)
    {
        let msg: String = findings
            .iter()
            .map(|f| format!("{}: {}", f.code, f.message))
            .collect::<Vec<_>>()
            .join("; ");
        return ToolResult::<String>::err_with_remediation(
            format!("staging validation failed: {msg}"),
            "Inspect `written` paths under output_dir; re-run export or fix files to match the venue plan (see vox-publisher `submission_package::staging_artifacts`).",
        )
        .to_json();
    }
    ToolResult::ok(serde_json::json!({
        "publication_id": publication_id,
        "output_dir": output_dir,
        "venue": venue.as_str(),
        "written": written,
    }))
    .to_json()
}

pub async fn vox_scientia_publication_scholarly_remote_status(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyRemoteStatusParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let submissions = match db
        .list_scholarly_submissions(&params.publication_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let sub_row: &vox_db::ScholarlySubmissionRow = match params.external_submission_id.as_deref() {
        Some(e) => {
            let e = e.trim();
            if e.is_empty() {
                return ToolResult::<String>::err_with_remediation(
                    "external_submission_id must not be empty when provided".to_string(),
                    REM_SCIENTIA_EXT_SUBMIT,
                )
                .to_json();
            }
            let Some(row) = submissions.iter().find(|r| r.external_submission_id == e) else {
                return ToolResult::<String>::err_with_remediation(
                    format!("no scholarly submission with external_submission_id {e}"),
                    REM_SCIENTIA_REMOTE,
                )
                .to_json();
            };
            row
        }
        None => {
            let Some(row) = submissions.first() else {
                return ToolResult::<String>::err_with_remediation(
                    "no scholarly submissions for this publication".to_string(),
                    REM_SCIENTIA_REMOTE,
                )
                .to_json();
            };
            row
        }
    };
    match poll_scholarly_remote_status_persist(db, params.publication_id.as_str(), sub_row).await {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json()
        }
    }
}

pub async fn vox_scientia_publication_scholarly_remote_status_sync_all(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyRemoteStatusSyncAllParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let publication_id = params.publication_id.trim();
    if publication_id.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "publication_id must not be empty".to_string(),
            REM_PUBLICATION_ID,
        )
        .to_json();
    }
    match poll_scholarly_remote_status_all_submissions_for_publication(db, publication_id).await {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationScholarlyPipelineRunParams {
    pub publication_id: String,
    #[serde(default)]
    pub preflight_profile: Option<PreflightProfileParam>,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub staging_output_dir: Option<String>,
    /// When `staging_output_dir` is set: `zenodo`, `openreview`, or `arxiv-assist`.
    #[serde(default)]
    pub venue: Option<String>,
    #[serde(default)]
    pub adapter: Option<String>,
    /// When true, emit compact JSON in the tool result (single line).
    #[serde(default)]
    pub json_compact: bool,
}

pub async fn vox_scientia_publication_scholarly_pipeline_run(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyPipelineRunParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let publication_id = params.publication_id.trim();
    if publication_id.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "publication_id must not be empty".to_string(),
            REM_PUBLICATION_ID,
        )
        .to_json();
    }
    let profile: PreflightProfile = params.preflight_profile.unwrap_or_default().into();
    let row = match db.get_publication_manifest(publication_id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return ToolResult::<String>::err_with_remediation(
                format!("publication not found: {publication_id}"),
                REM_PUBLICATION_ID,
            )
            .to_json();
        }
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let manifest = PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let report = vox_publisher::publication_preflight::run_preflight(&manifest, profile);
    if !report.ok {
        return ToolResult::<String>::err_with_remediation(
            format!(
                "scholarly pipeline preflight failed (readiness {}): {}",
                report.readiness_score,
                serde_json::to_string(&report).unwrap_or_else(|_| "{}".into())
            ),
            "Fix preflight findings on the stored manifest or pass a different `preflight_profile`; compare with `vox scientia publication-preflight`.",
        )
        .to_json();
    }
    let digest = row.content_sha3_256.clone();
    let dual = match db
        .has_dual_publication_approval_for_digest(publication_id, &digest)
        .await
    {
        Ok(b) => b,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    if !dual {
        return ToolResult::<String>::err_with_remediation(
            "scholarly pipeline requires two distinct digest-bound approvers before staging export / submit",
            "Record two digest-bound approvers with `vox scientia publication-approve --publication-id ...` (distinct `--approver` values), then retry.",
        )
        .to_json();
    }
    let mut stages: Vec<String> = vec!["preflight_ok".into(), "dual_approval_ok".into()];

    let out_dir = params
        .staging_output_dir
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let venue_raw = params.venue.as_deref().map(str::trim).filter(|s| !s.is_empty());

    match (venue_raw, out_dir) {
        (Some(vs), Some(od)) => {
            let Some(venue) = vox_publisher::submission_package::ScholarlyVenue::parse(vs) else {
                return ToolResult::<String>::err_with_remediation(
                    format!("unknown venue {vs:?}"),
                    "Use `zenodo`, `openreview`, or `arxiv-assist` for `venue` when `staging_output_dir` is set.",
                )
                .to_json();
            };
            if params.dry_run {
                stages.push(format!("staging_skipped_dry_run venue={vs} dir={od}"));
            } else {
                let output_path = std::path::Path::new(od);
                if let Err(e) = vox_publisher::submission_package::write_scholarly_staging(
                    &manifest,
                    venue,
                    output_path,
                ) {
                    return ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_OUTPUT_DIR)
                        .to_json();
                }
                if let Err(findings) =
                    vox_publisher::submission_package::validate_scholarly_staging(
                        output_path,
                        venue,
                        &manifest,
                    )
                {
                    let msg: String = findings
                        .iter()
                        .map(|f| format!("{}: {}", f.code, f.message))
                        .collect::<Vec<_>>()
                        .join("; ");
                    return ToolResult::<String>::err_with_remediation(
                        format!("staging validation failed: {msg}"),
                        "Inspect staging under output_dir; re-export with matching venue or fix validation codes reported above.",
                    )
                    .to_json();
                }
                stages.push("staging_exported".into());
            }
        }
        (None, Some(_)) => {
            return ToolResult::<String>::err_with_remediation(
                "staging_output_dir requires venue",
                "Pass `venue` (`zenodo`, `openreview`, or `arxiv-assist`) whenever `staging_output_dir` is set (matches CLI).",
            )
            .to_json();
        }
        (Some(_), None) => {
            return ToolResult::<String>::err_with_remediation(
                "venue requires staging_output_dir",
                "Set `staging_output_dir` to the directory that should receive staging files, or omit both for submit-only.",
            )
            .to_json();
        }
        (None, None) => {}
    }

    let compact = params.json_compact;
    if params.dry_run {
        let tr = ToolResult::ok(serde_json::json!({
            "dry_run": true,
            "publication_id": publication_id,
            "digest": digest,
            "stages": stages,
            "preflight_report": report,
        }));
        return if compact {
            tr.to_json_compact()
        } else {
            tr.to_json()
        };
    }

    match publication_scholarly_submit_with_ledger(db, publication_id, params.adapter.as_deref()).await
    {
        Ok(receipt) => {
            let tr = ToolResult::ok(serde_json::json!({
                "pipeline_completed": true,
                "publication_id": publication_id,
                "digest": digest,
                "stages": stages,
                "submission": {
                    "adapter": receipt.adapter,
                    "external_submission_id": receipt.external_submission_id,
                    "status": receipt.status,
                }
            }));
            if compact {
                tr.to_json_compact()
            } else {
                tr.to_json()
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            e.to_string(),
            "Verify `VOX_SCHOLARLY_*` flags, adapter credentials (Clavis / env), dual approval, and that the stored digest matches the manifest.",
        )
        .to_json(),
    }
}

pub async fn vox_scientia_publication_scholarly_remote_status_sync_batch(
    state: &ServerState,
    params: VoxScientiaPublicationScholarlyRemoteStatusSyncBatchParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let res = if params.iterations <= 1
        && params.interval_secs == 0
        && params.max_runtime_secs.is_none()
        && params.jitter_secs == 0
    {
        poll_scholarly_remote_status_batch(db, params.limit).await
    } else {
        vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_batch_loop(
            db,
            params.limit,
            params.iterations,
            params.interval_secs,
            params.max_runtime_secs,
            params.jitter_secs,
        )
        .await
    };
    match res {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationArxivHandoffRecordParams {
    pub publication_id: String,
    /// One of: staging_exported, operator_ack, bundle_validated, submitted, published.
    pub stage: String,
    #[serde(default)]
    pub operator: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub arxiv_id: Option<String>,
}

pub async fn vox_scientia_publication_arxiv_handoff_record(
    state: &ServerState,
    params: VoxScientiaPublicationArxivHandoffRecordParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let publication_id = params.publication_id.trim();
    if publication_id.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "publication_id must not be empty".to_string(),
            REM_PUBLICATION_ID,
        )
        .to_json();
    }
    let stage = params.stage.trim().to_ascii_lowercase();
    let allowed = [
        "staging_exported",
        "operator_ack",
        "bundle_validated",
        "submitted",
        "published",
    ];
    if !allowed.contains(&stage.as_str()) {
        return ToolResult::<String>::err_with_remediation(
            format!(
                "invalid stage {stage:?}; expected one of {}",
                allowed.join(", ")
            ),
            REM_SCIENTIA_STAGE,
        )
        .to_json();
    }
    if stage == "published"
        && params
            .arxiv_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .is_none()
    {
        return ToolResult::<String>::err_with_remediation(
            "arxiv_id is required when stage is published".to_string(),
            REM_SCIENTIA_ARXIV,
        )
        .to_json();
    }
    match db.get_publication_manifest(publication_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return ToolResult::<String>::err_with_remediation(
                format!("publication not found: {publication_id}"),
                REM_PUBLICATION_ID,
            )
            .to_json();
        }
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json();
        }
    }

    let op_trim = params
        .operator
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let note_trim = params.note.as_deref().map(str::trim).filter(|s| !s.is_empty());
    let arxiv_trim = params
        .arxiv_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let status = format!("arxiv_handoff:{stage}");
    let detail = serde_json::json!({
        "schema_version": 1_u32,
        "workflow": "arxiv_operator_assist",
        "stage": stage,
        "operator": op_trim,
        "note": note_trim,
        "arxiv_id": arxiv_trim,
    });
    if let Err(e) = db
        .append_publication_status_event(
            publication_id,
            &status,
            Some(&detail.to_string()),
        )
        .await
    {
        return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json();
    }
    ToolResult::ok(serde_json::json!({
        "recorded": true,
        "publication_id": publication_id,
        "status": status,
        "detail": detail,
    }))
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalJobsDueParams {
    #[serde(default = "default_jobs_due_limit")]
    pub limit: i64,
}

fn default_jobs_due_limit() -> i64 {
    50
}

pub async fn vox_scientia_publication_external_jobs_due(
    state: &ServerState,
    params: VoxScientiaPublicationExternalJobsDueParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let before_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    match db
        .list_external_submission_jobs_due(before_ms, params.limit)
        .await
    {
        Ok(jobs) => ToolResult::ok(serde_json::json!({
            "due_before_ms_inclusive": before_ms,
            "jobs": jobs,
        }))
        .to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalJobsDeadLetterParams {
    #[serde(default = "default_jobs_dead_letter_limit")]
    pub limit: i64,
}

fn default_jobs_dead_letter_limit() -> i64 {
    50
}

pub async fn vox_scientia_publication_external_jobs_dead_letter(
    state: &ServerState,
    params: VoxScientiaPublicationExternalJobsDeadLetterParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    match db.list_external_submission_jobs_failed(params.limit).await {
        Ok(jobs) => ToolResult::ok(serde_json::json!({ "jobs": jobs })).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalJobsReplayParams {
    pub job_id: i64,
}

pub async fn vox_scientia_publication_external_jobs_replay(
    state: &ServerState,
    params: VoxScientiaPublicationExternalJobsReplayParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    match db
        .replay_failed_external_submission_job_to_queued(params.job_id)
        .await
    {
        Ok(job) => ToolResult::ok(serde_json::json!({
            "replayed": true,
            "job": job,
        }))
        .to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalJobsTickParams {
    #[serde(default = "default_jobs_tick_limit")]
    pub limit: i64,
    #[serde(default = "default_jobs_tick_lock_ttl_ms")]
    pub lock_ttl_ms: i64,
    #[serde(default)]
    pub lock_owner: Option<String>,
    #[serde(default = "default_one_u32")]
    pub iterations: u32,
    #[serde(default)]
    pub interval_secs: u64,
    #[serde(default)]
    pub max_runtime_secs: Option<u64>,
    #[serde(default)]
    pub jitter_secs: u64,
}

fn default_jobs_tick_limit() -> i64 {
    10
}

fn default_jobs_tick_lock_ttl_ms() -> i64 {
    120_000
}

pub async fn vox_scientia_publication_external_jobs_tick(
    state: &ServerState,
    params: VoxScientiaPublicationExternalJobsTickParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    if params.iterations <= 1
        && params.interval_secs == 0
        && params.max_runtime_secs.is_none()
        && params.jitter_secs == 0
    {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        return match vox_publisher::scholarly_external_jobs::run_external_submit_jobs_tick(
            db,
            params.limit,
            params.lock_ttl_ms,
            params.lock_owner.as_deref(),
            now_ms,
        )
        .await
        {
            Ok(out) => ToolResult::ok(serde_json::json!({
                "now_ms": now_ms,
                "lock_owner": out.lock_owner,
                "lock_ttl_ms": out.lock_ttl_ms,
                "results": out.results,
            }))
            .to_json(),
            Err(e) => ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json(),
        };
    }
    match vox_publisher::scholarly_external_jobs::run_external_submit_jobs_tick_loop(
        db,
        params.limit,
        params.lock_ttl_ms,
        params.lock_owner.as_deref(),
        params.iterations,
        params.interval_secs,
        params.max_runtime_secs,
        params.jitter_secs,
    )
    .await
    {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(e.to_string(), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationExternalPipelineMetricsParams {
    /// Hours of history for attempts, snapshots, terminal latencies, and publication_attempts (0 = all time). Clamped 0–8760.
    #[serde(default = "default_metrics_since_hours")]
    pub since_hours: i64,
}

fn default_metrics_since_hours() -> i64 {
    168
}

pub async fn vox_scientia_publication_external_pipeline_metrics(
    state: &ServerState,
    params: VoxScientiaPublicationExternalPipelineMetricsParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let hours = params.since_hours.clamp(0, 8_760);
    let since_ms = if hours == 0 {
        0_i64
    } else {
        now_ms.saturating_sub(hours.saturating_mul(3_600_000))
    };
    match db.summarize_scholarly_external_pipeline_metrics(since_ms).await {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationMediaUpsertParams {
    pub publication_id: String,
    pub asset_ref: String,
    pub media_type: String,
    #[serde(default)]
    pub storage_uri: Option<String>,
    pub status: String,
    #[serde(default)]
    pub metadata_json: Option<serde_json::Value>,
}

pub async fn vox_scientia_publication_media_upsert(
    state: &ServerState,
    params: VoxScientiaPublicationMediaUpsertParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let metadata_json = params
        .metadata_json
        .as_ref()
        .map(serde_json::Value::to_string);
    if let Err(e) = db
        .upsert_publication_media_asset(vox_db::PublicationMediaAssetParams {
            publication_id: params.publication_id.as_str(),
            asset_ref: params.asset_ref.as_str(),
            media_type: params.media_type.as_str(),
            storage_uri: params.storage_uri.as_deref(),
            status: params.status.as_str(),
            metadata_json: metadata_json.as_deref(),
        })
        .await
    {
        return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json();
    }
    ToolResult::ok(serde_json::json!({
        "publication_id": params.publication_id,
        "asset_ref": params.asset_ref,
        "media_type": params.media_type,
        "storage_uri": params.storage_uri,
        "status": params.status,
        "metadata_json_present": metadata_json.is_some()
    }))
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationMediaListParams {
    pub publication_id: String,
}

pub async fn vox_scientia_publication_media_list(
    state: &ServerState,
    params: VoxScientiaPublicationMediaListParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let rows = match db
        .list_publication_media_assets(&params.publication_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    ToolResult::ok(rows).to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationMediaDeleteParams {
    pub publication_id: String,
    pub asset_ref: String,
}

pub async fn vox_scientia_publication_media_delete(
    state: &ServerState,
    params: VoxScientiaPublicationMediaDeleteParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    if let Err(e) = db
        .delete_publication_media_asset(&params.publication_id, &params.asset_ref)
        .await
    {
        return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json();
    }
    ToolResult::ok(serde_json::json!({
        "deleted": true,
        "publication_id": params.publication_id,
        "asset_ref": params.asset_ref
    }))
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationRouteSimulateParams {
    pub publication_id: String,
}

pub async fn vox_scientia_publication_route_simulate(
    state: &ServerState,
    params: VoxScientiaPublicationRouteSimulateParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let Some(row) = row else {
        return ToolResult::<String>::err_with_remediation("publication not found", REM_PUBLICATION_ID)
            .to_json();
    };
    let item = match unified_news_item_from_manifest_row(&row) {
        Ok(i) => i,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("parse metadata_json: {e}"),
                REM_SCIENTIA_METADATA,
            )
            .to_json();
        }
    };
    let worthiness = worthiness_score_for_row(&row);
    let publisher = vox_publisher::Publisher::new(operator_publisher_config(state, true, worthiness));
    match publisher.publish_all(&item).await {
        Ok(r) => ToolResult::ok(r).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("simulate failed: {e}"),
            REM_SCIENTIA_SIMULATE,
        )
        .to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationPublishParams {
    pub publication_id: String,
    #[serde(default)]
    pub channels: Option<Vec<String>>,
    #[serde(default = "default_true")]
    pub dry_run: bool,
    /// When true, emit compact JSON in the tool text payload (single line).
    #[serde(default)]
    pub json: bool,
}

fn default_true() -> bool {
    true
}

pub async fn vox_scientia_publication_publish(
    state: &ServerState,
    params: VoxScientiaPublicationPublishParams,
) -> String {
    let compact = params.json;
    let Some(db) = &state.db else {
        return no_voxdb_syndication(compact);
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<vox_publisher::SyndicationResult>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB)
                .to_json_styled(compact);
        }
    };
    let Some(row) = row else {
        return ToolResult::<vox_publisher::SyndicationResult>::err_with_remediation(
            "publication not found",
            REM_PUBLICATION_ID,
        )
        .to_json_styled(compact);
    };
    let digest = row.content_sha3_256.clone();
    let mut item = match unified_news_item_from_manifest_row(&row) {
        Ok(i) => i,
        Err(e) => {
            return ToolResult::<vox_publisher::SyndicationResult>::err_with_remediation(
                format!("parse metadata_json: {e}"),
                REM_SCIENTIA_METADATA,
            )
            .to_json_styled(compact);
        }
    };
    if let Some(channels) = params.channels.as_ref() {
        let normalized = vox_publisher::switching::normalize_channels(channels);
        vox_publisher::switching::apply_channel_allowlist(&mut item, normalized.as_slice());
    }
    let dual = match db
        .has_dual_publication_approval_for_digest(&params.publication_id, &digest)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<vox_publisher::SyndicationResult>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB)
                .to_json_styled(compact);
        }
    };
    let gate = vox_publisher::gate::evaluate_publish_gate(
        vox_publisher::gate::publish_gate_inputs_for_mcp(
            params.dry_run,
            state.orchestrator_config.news.dry_run,
            state.orchestrator_config.news.publish_armed,
            true,
            dual,
            &item,
        ),
    );
    if gate.has_blockers() {
        let msg = serde_json::json!({
            "error": "live publish blocked by gate",
            "blocking_reasons": gate.blocking_reasons,
        })
        .to_string();
        return ToolResult::<vox_publisher::SyndicationResult>::err_with_remediation(
            msg,
            REM_SCIENTIA_SIMULATE,
        )
        .to_json_styled(compact);
    }
    let worthiness = worthiness_score_for_row(&row);
    if mcp_social_worthiness_enforce(state)
        && !params.dry_run
        && !state.orchestrator_config.news.dry_run
        && !item.syndication.dry_run
        && gate.live_publish_allowed
        && let Some(score) = worthiness
    {
        let floor = mcp_social_worthiness_score_min(state);
        if score < floor {
            let msg = serde_json::json!({
                "error": "live publish blocked by worthiness floor",
                "worthiness_score": score,
                "floor": floor,
            })
            .to_string();
            return ToolResult::<vox_publisher::SyndicationResult>::err_with_remediation(
                msg,
                REM_SCIENTIA_PUBLISH,
            )
            .to_json_styled(compact);
        }
    }
    let publisher =
        vox_publisher::Publisher::new(operator_publisher_config(state, params.dry_run, worthiness));
    let out = match publisher.publish_all(&item).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<vox_publisher::SyndicationResult>::err_with_remediation(
                format!("publish failed: {e}"),
                REM_SCIENTIA_PUBLISH,
            )
            .to_json_styled(compact);
        }
    };
    if let Ok(out_json) = serde_json::to_string(&out) {
        let _ = db
            .record_publication_attempt(
                &params.publication_id,
                &digest,
                "manual_mcp",
                out_json.as_str(),
            )
            .await;
    }
    if gate.live_publish_allowed {
        if out.all_enabled_channels_succeeded(&item) {
            let _ = db
                .set_publication_state(
                    &params.publication_id,
                    "published",
                    Some(
                        &serde_json::json!({ "channel_group": "manual_mcp" }).to_string(),
                    ),
                )
                .await;
        } else if out.has_failures() {
            let _ = db
                .set_publication_state(
                    &params.publication_id,
                    "publish_failed",
                    Some(
                        &serde_json::json!({ "channel_group": "manual_mcp" }).to_string(),
                    ),
                )
                .await;
        }
    }
    ToolResult::ok(out).to_json_styled(compact)
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationRetryFailedParams {
    pub publication_id: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default = "default_true")]
    pub dry_run: bool,
    /// When true, emit compact JSON (including nested publish responses).
    #[serde(default)]
    pub json: bool,
}

pub async fn vox_scientia_publication_retry_failed(
    state: &ServerState,
    params: VoxScientiaPublicationRetryFailedParams,
) -> String {
    if let Some(ch) = params.channel.as_ref() {
        return vox_scientia_publication_publish(
            state,
            VoxScientiaPublicationPublishParams {
                publication_id: params.publication_id,
                channels: Some(vec![ch.clone()]),
                dry_run: params.dry_run,
                json: params.json,
            },
        )
        .await;
    }
    let compact = params.json;
    let Some(db) = &state.db else {
        return no_voxdb_json_envelope(compact);
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB)
                .to_json_styled(compact);
        }
    };
    let Some(row) = row else {
        return ToolResult::<serde_json::Value>::err_with_remediation(
            "publication not found",
            REM_PUBLICATION_ID,
        )
        .to_json_styled(compact);
    };
    let digest = row.content_sha3_256;
    let attempts = match db.list_publication_attempts(&params.publication_id).await {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB)
                .to_json_styled(compact);
        }
    };
    if attempts.is_empty() {
        return ToolResult::<serde_json::Value>::err_with_remediation(
            "no attempts found".to_string(),
            REM_SCIENTIA_ATTEMPTS,
        )
        .to_json_styled(compact);
    }
    let attempt_refs: Vec<vox_publisher::switching::AttemptOutcome<'_>> = attempts
        .iter()
        .map(|a| vox_publisher::switching::AttemptOutcome {
            content_sha3_256: a.content_sha3_256.as_str(),
            outcome_json: a.outcome_json.as_str(),
        })
        .collect();
    let failed = match vox_publisher::switching::failed_channels_from_latest_digest_attempt(
        attempt_refs.as_slice(),
        digest.as_str(),
    ) {
        Ok(Some(v)) => v,
        Ok(None) => {
            return ToolResult::<serde_json::Value>::err_with_remediation(
                "no syndication attempt outcome for current manifest digest".to_string(),
                REM_SCIENTIA_ATTEMPTS,
            )
            .to_json_styled(compact);
        }
        Err(e) => {
            return ToolResult::<serde_json::Value>::err_with_remediation(
                format!("attempt parse: {e}"),
                REM_SCIENTIA_METADATA,
            )
            .to_json_styled(compact);
        }
    };
    if failed.is_empty() {
        return ToolResult::ok(serde_json::json!({
            "publication_id": params.publication_id,
            "retried": false,
            "reason": "no_failed_channels"
        }))
        .to_json_styled(compact);
    }
    vox_scientia_publication_publish(
        state,
        VoxScientiaPublicationPublishParams {
            publication_id: params.publication_id,
            channels: Some(failed),
            dry_run: params.dry_run,
            json: params.json,
        },
    )
    .await
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
        return no_voxdb_tool_string();
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err_with_remediation(format!("DB error: {e}"), REM_SCIENTIA_DB).to_json(),
    };
    let Some(row) = row else {
        return ToolResult::<String>::err_with_remediation("publication not found", REM_PUBLICATION_ID)
            .to_json();
    };
    let mut manifest = publication_manifest_from_row(&row);
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
                return ToolResult::<String>::err_with_remediation(
                    format!("socrates telemetry merge: {e}"),
                    REM_SCIENTIA_DB,
                )
                .to_json();
            }
        }
        match vox_publisher::scientia_evidence::enrich_metadata_json_with_repo_files(
            manifest.metadata_json.as_deref(),
            &state.repository.root,
        ) {
            Ok(Some(updated)) => manifest.metadata_json = Some(updated),
            Ok(None) => {}
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(
                    format!("scientia_evidence file hydration: {e}"),
                    REM_SCIENTIA_METADATA,
                )
                .to_json();
            }
        }
        let path = state
            .repository
            .root
            .join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH);
        let yaml = match crate::bounded_fs::read_utf8_path_capped(&path) {
            Ok(s) => s,
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(
                    format!("read worthiness contract {}: {e}", path.display()),
                    REM_WORTHINESS_CONTRACT,
                )
                .to_json();
            }
        };
        let contract = match vox_publisher::publication_worthiness::load_contract_from_str(&yaml) {
            Ok(c) => c,
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(
                    format!("parse worthiness contract: {e}"),
                    REM_WORTHINESS_CONTRACT,
                )
                .to_json();
            }
        };
        if let Err(e) =
            vox_publisher::publication_worthiness::validate_contract_invariants(&contract)
        {
            return ToolResult::<String>::err_with_remediation(
                format!("worthiness contract invariants: {e}"),
                REM_WORTHINESS_CONTRACT,
            )
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
    let yaml = match crate::bounded_fs::read_utf8_path_capped(&contract_path) {
        Ok(s) => s,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("read contract {}: {e}", contract_path.display()),
                REM_WORTHINESS_CONTRACT,
            )
            .to_json();
        }
    };
    let contract = match vox_publisher::publication_worthiness::load_contract_from_str(&yaml) {
        Ok(c) => c,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("parse contract YAML: {e}"),
                REM_WORTHINESS_CONTRACT,
            )
            .to_json();
        }
    };
    if let Err(e) = vox_publisher::publication_worthiness::validate_contract_invariants(&contract) {
        return ToolResult::<String>::err_with_remediation(
            format!("contract invariants: {e}"),
            REM_WORTHINESS_CONTRACT,
        )
        .to_json();
    }
    let inputs: vox_publisher::publication_worthiness::WorthinessInputs =
        match serde_json::from_value(params.metrics) {
            Ok(i) => i,
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(
                    format!("metrics: {e}"),
                    "Pass `metrics` as a JSON object matching `WorthinessInputs` (see publication_worthiness docs).",
                )
                .to_json();
            }
        };
    let out = vox_publisher::publication_worthiness::evaluate_worthiness(&contract, &inputs);
    ToolResult::ok(out).to_json()
}
