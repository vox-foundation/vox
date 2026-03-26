use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vox_publisher::publication::PublicationManifest;
use vox_publisher::publication_preflight::PreflightProfile;
use vox_publisher::scholarly::submit_with_configured_adapter;
use vox_publisher::scientific_metadata::ScientificPublicationMetadata;
use vox_publisher::types::UnifiedNewsItem;

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
    let mut site = vox_publisher::NewsSiteConfig::default();
    site.base_url = state
        .orchestrator_config
        .news
        .site_base_url
        .clone()
        .unwrap_or_else(|| {
            vox_publisher::contract::DEFAULT_SITE_BASE_URL
                .trim_end_matches('/')
                .to_string()
        });
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
    let receipt = match submit_with_configured_adapter(&manifest) {
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
    media_assets: Vec<vox_db::PublicationMediaAssetRow>,
    publication_attempts: Vec<vox_db::PublicationAttemptRow>,
    publication_status_events: Vec<vox_db::PublicationStatusEventRow>,
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
    let media_assets = match db
        .list_publication_media_assets(&params.publication_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    let publication_attempts = match db.list_publication_attempts(&params.publication_id).await {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    let publication_status_events = match db
        .list_publication_status_events(&params.publication_id)
        .await
    {
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
        media_assets,
        publication_attempts,
        publication_status_events,
    })
    .to_json()
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
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
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
        return ToolResult::<String>::err(format!("DB error: {e}")).to_json();
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
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    let rows = match db
        .list_publication_media_assets(&params.publication_id)
        .await
    {
        Ok(v) => v,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
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
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    if let Err(e) = db
        .delete_publication_media_asset(&params.publication_id, &params.asset_ref)
        .await
    {
        return ToolResult::<String>::err(format!("DB error: {e}")).to_json();
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
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    let Some(row) = row else {
        return ToolResult::<String>::err("publication not found".to_string()).to_json();
    };
    let item = match unified_news_item_from_manifest_row(&row) {
        Ok(i) => i,
        Err(e) => {
            return ToolResult::<String>::err(format!("parse metadata_json: {e}")).to_json();
        }
    };
    let worthiness = worthiness_score_for_row(&row);
    let publisher = vox_publisher::Publisher::new(operator_publisher_config(state, true, worthiness));
    match publisher.publish_all(&item).await {
        Ok(r) => ToolResult::ok(r).to_json(),
        Err(e) => ToolResult::<String>::err(format!("simulate failed: {e}")).to_json(),
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
        return ToolResult::<vox_publisher::SyndicationResult>::err(
            "VoxDb is not connected".to_string(),
        )
        .to_json_styled(compact);
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<vox_publisher::SyndicationResult>::err(format!("DB error: {e}"))
                .to_json_styled(compact);
        }
    };
    let Some(row) = row else {
        return ToolResult::<vox_publisher::SyndicationResult>::err(
            "publication not found".to_string(),
        )
        .to_json_styled(compact);
    };
    let digest = row.content_sha3_256.clone();
    let mut item = match unified_news_item_from_manifest_row(&row) {
        Ok(i) => i,
        Err(e) => {
            return ToolResult::<vox_publisher::SyndicationResult>::err(format!(
                "parse metadata_json: {e}"
            ))
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
            return ToolResult::<vox_publisher::SyndicationResult>::err(format!("DB error: {e}"))
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
        return ToolResult::<vox_publisher::SyndicationResult>::err(msg).to_json_styled(compact);
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
            return ToolResult::<vox_publisher::SyndicationResult>::err(msg).to_json_styled(compact);
        }
    }
    let publisher =
        vox_publisher::Publisher::new(operator_publisher_config(state, params.dry_run, worthiness));
    let out = match publisher.publish_all(&item).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<vox_publisher::SyndicationResult>::err(format!(
                "publish failed: {e}"
            ))
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
        return ToolResult::<serde_json::Value>::err("VoxDb is not connected".to_string())
            .to_json_styled(compact);
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!("DB error: {e}"))
                .to_json_styled(compact);
        }
    };
    let Some(row) = row else {
        return ToolResult::<serde_json::Value>::err("publication not found".to_string())
            .to_json_styled(compact);
    };
    let digest = row.content_sha3_256;
    let attempts = match db.list_publication_attempts(&params.publication_id).await {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!("DB error: {e}"))
                .to_json_styled(compact);
        }
    };
    if attempts.is_empty() {
        return ToolResult::<serde_json::Value>::err("no attempts found".to_string())
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
            return ToolResult::<serde_json::Value>::err(
                "no syndication attempt outcome for current manifest digest".to_string(),
            )
            .to_json_styled(compact);
        }
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!("attempt parse: {e}"))
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
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    let row = match db.get_publication_manifest(&params.publication_id).await {
        Ok(r) => r,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {e}")).to_json(),
    };
    let Some(row) = row else {
        return ToolResult::<String>::err("publication not found".to_string()).to_json();
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
                return ToolResult::<String>::err(format!("socrates telemetry merge: {e}"))
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
                return ToolResult::<String>::err(format!("scientia_evidence file hydration: {e}"))
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
    let yaml = match crate::bounded_fs::read_utf8_path_capped(&contract_path) {
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
