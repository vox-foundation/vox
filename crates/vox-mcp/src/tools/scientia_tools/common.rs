use crate::{ServerState, ToolResult};
use vox_publisher::publication::PublicationManifest;
use vox_publisher::types::UnifiedNewsItem;

pub const REM_VOXDB: &str = "Attach Turso/VoxDb to the MCP server, or run the same flow via `vox db` / `vox scientia` in a configured shell.";
pub const REM_PUBLICATION_ID: &str =
    "Run `vox scientia publication-prepare` (or verify the publication id) before this step.";
pub const REM_SCIENTIA_DB: &str =
    "Verify Turso/VoxDb connectivity and vox-db publication/scientia table migrations.";
pub const REM_SCIENTIA_APPROVER: &str =
    "Pass a non-empty `approver` string when recording publication approvals.";
pub const REM_SCIENTIA_ARXIV: &str =
    "When using `published` stage metadata, include a non-empty `arxiv_id`.";
pub const REM_SCIENTIA_OUTPUT_DIR: &str =
    "Set `output_dir` to a writable directory for scholarly pipeline artifacts.";
pub const REM_SCIENTIA_METADATA: &str =
    "Fix `scholarly_metadata` / manifest JSON to match SCIENTIA contracts (see scientia handbook).";
pub const REM_SCIENTIA_SIMULATE: &str =
    "Inspect simulate/gate output and manifest state; resolve blockers then retry.";
pub const REM_SCIENTIA_PUBLISH: &str = "Check syndication channel config, dry-run flags, approvals, and publisher credentials for live paths.";
pub const REM_SCIENTIA_REMOTE: &str = "Ensure `publication_id` has scholarly submission rows; pass `external_submission_id` when disambiguating.";
pub const REM_SCIENTIA_EXT_SUBMIT: &str =
    "When provided, `external_submission_id` must be a non-empty id from the submissions table.";
pub const REM_WORTHINESS_CONTRACT: &str = "Ensure SCIENTIA worthiness YAML under the repo root is readable and schema-valid (see `contracts/scientia/`).";
pub const REM_SCIENTIA_STAGE: &str = "Use stage tokens `staging_exported`, `operator_ack`, `bundle_validated`, `submitted`, or `published` (see arXiv handoff tool docs).";
pub const REM_SCIENTIA_ATTEMPTS: &str = "Run `publication_publish` or `route_simulate` first so syndication attempts exist for the current manifest digest.";

#[inline]
pub fn no_voxdb_tool_string() -> String {
    ToolResult::<String>::err_with_remediation("VoxDb is not connected", REM_VOXDB).to_json()
}

#[inline]
pub fn no_voxdb_syndication(compact: bool) -> String {
    ToolResult::<vox_publisher::SyndicationResult>::err_with_remediation(
        "VoxDb is not connected",
        REM_VOXDB,
    )
    .to_json_styled(compact)
}

#[inline]
pub fn no_voxdb_json_envelope(compact: bool) -> String {
    ToolResult::<serde_json::Value>::err_with_remediation("VoxDb is not connected", REM_VOXDB)
        .to_json_styled(compact)
}

pub fn publication_manifest_from_row(row: &vox_db::PublicationManifestRow) -> PublicationManifest {
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

pub fn worthiness_score_for_row(row: &vox_db::PublicationManifestRow) -> Option<f64> {
    let m = publication_manifest_from_row(row);
    let root = vox_repository::resolve_repo_root_for_ci();
    vox_publisher::publication_worthiness::worthiness_score_for_publication_manifest(&m, &root).ok()
}

pub fn mcp_social_worthiness_enforce(state: &ServerState) -> bool {
    state.orchestrator_config.news.worthiness_enforce
        || std::env::var("VOX_SOCIAL_WORTHINESS_ENFORCE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
}

pub fn mcp_social_worthiness_score_min(state: &ServerState) -> f64 {
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

pub fn operator_publisher_config(
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

pub fn unified_news_item_from_manifest_row(
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

pub fn default_one_u32() -> u32 {
    1
}
