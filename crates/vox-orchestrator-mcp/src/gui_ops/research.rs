//! Research-profile routes and ops for Axis (plan 07 Task 7): research sessions and their trust
//! summary, Architecture-SSOT doc drafts and publishing, misguidance flags, and the research
//! engine's provider status / config / probes. Scientia publication review lives in
//! [`super::scientia`]; its router is merged here.
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_secrets::{SecretId, list_free_tier_offers, resolve_secret, store_secret};

use crate::gui_ops::dto::research::{
    DocDraftPreview, ProviderStatusDto, PublishDocResult, QuotaUsageDto, ResearchClaimDto,
    ResearchDetailDto, ResearchEngineConfigDto, ResearchEngineStatusDto, ResearchSessionDto,
};
use crate::gui_ops::{GUI_USER_ID, api_db, enforce_route, require_db};
use crate::http_gateway::GatewayState;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::services::routes::{err, ok};

const RESEARCH_SURFACES: &[&str] = &["research"];
const DEFAULT_SESSION_LIMIT: u32 = 20;

// ─── Artifact summary ────────────────────────────────────────────────────────

// Minimal mirror of `vox_research_shim::research::types::ResearchRunArtifact` used only to parse
// `artifact_json`. The real `ResearchMetadata` lacks `#[serde(default)]` on fields such as
// `corroboration_counts`, so importing it would make an older persisted artifact fail to parse
// instead of degrading; these mirrors tolerate missing/unknown fields and must be kept in sync by
// hand when the real types change.
mod artifact_mirror {
    use serde::Deserialize;

    #[derive(Debug, Deserialize, Default)]
    pub struct Citation {
        pub source_id: i64,
        pub url: String,
    }

    #[derive(Debug, Deserialize, Default)]
    pub struct EvidenceSpan {
        pub source_id: i64,
    }

    #[derive(Debug, Deserialize, Default)]
    pub struct Claim {
        pub text: String,
        pub claim_id: u64,
    }

    #[derive(Debug, Deserialize, Default)]
    pub struct ClaimVerdict {
        pub claim: Claim,
        pub verdict: String,
        pub confidence: f64,
        #[serde(default)]
        pub evidence_spans: Vec<EvidenceSpan>,
        #[serde(default)]
        pub resample_stability: f64,
    }

    #[derive(Debug, Deserialize, Default)]
    pub struct ResearchMetadata {
        pub routing_tier: String,
        #[serde(default)]
        pub source_count: usize,
        #[serde(default)]
        pub claim_verdicts: Vec<ClaimVerdict>,
        #[serde(default)]
        pub citation_audit: Option<CitationAudit>,
        /// `(claim_id, distinct_supporting_domain_count)` pairs (`vox_search::corroboration`).
        #[serde(default)]
        pub corroboration_counts: Vec<(u64, usize)>,
    }

    #[derive(Debug, Deserialize, Default)]
    pub struct CitationAudit {
        #[serde(default)]
        pub precision: f64,
    }

    #[derive(Debug, Deserialize, Default)]
    pub struct ResearchResult {
        #[serde(default)]
        pub citations: Vec<Citation>,
        pub research_metadata: ResearchMetadata,
    }

    #[derive(Debug, Deserialize, Default)]
    pub struct ResearchRunArtifact {
        pub result: ResearchResult,
    }
}

/// Summary fields the ResearchView trust UI (headline banner + claim accordion) needs.
#[derive(Debug, Default)]
pub struct ResearchSummary {
    pub confidence_tier: Option<String>,
    pub claims: Vec<ResearchClaimDto>,
    pub source_count: Option<usize>,
    pub citation_precision: Option<f64>,
}

/// `Verdict`'s wire form is snake_case; the GUI's verdict vocabulary is capitalized.
fn capitalize_verdict(wire: &str) -> String {
    match wire {
        "supported" => "Supported".to_string(),
        "contradicted" => "Contradicted".to_string(),
        "contested" => "Contested".to_string(),
        "unverified" => "Unverified".to_string(),
        other => other.to_string(),
    }
}

/// Parse `artifact_json` into the trust-UI summary. Fail-open: a parse failure yields an empty
/// summary (logged) rather than failing the whole session-detail read.
pub fn extract_research_summary(artifact_json: &str) -> ResearchSummary {
    use std::collections::HashMap;

    let artifact: artifact_mirror::ResearchRunArtifact = match serde_json::from_str(artifact_json) {
        Ok(a) => a,
        Err(e) => {
            tracing::warn!("failed to parse research artifact_json for trust UI: {e}");
            return ResearchSummary::default();
        }
    };
    let result = artifact.result;
    let meta = result.research_metadata;

    let url_by_source_id: HashMap<i64, String> = result
        .citations
        .iter()
        .map(|c| (c.source_id, c.url.clone()))
        .collect();
    let corroboration_by_claim_id: HashMap<u64, usize> =
        meta.corroboration_counts.into_iter().collect();

    let claims: Vec<ResearchClaimDto> = meta
        .claim_verdicts
        .into_iter()
        .map(|cv| {
            let mut citation_urls: Vec<String> = cv
                .evidence_spans
                .iter()
                .filter_map(|span| url_by_source_id.get(&span.source_id).cloned())
                .collect();
            citation_urls.sort();
            citation_urls.dedup();
            let corroboration_count = corroboration_by_claim_id
                .get(&cv.claim.claim_id)
                .copied()
                .unwrap_or(0);
            ResearchClaimDto {
                claim_id: cv.claim.claim_id.to_string(),
                text: cv.claim.text,
                verdict: capitalize_verdict(&cv.verdict),
                confidence: cv.confidence,
                resample_stability: cv.resample_stability,
                citation_urls,
                corroboration_count,
            }
        })
        .collect();

    ResearchSummary {
        confidence_tier: Some(meta.routing_tier),
        claims,
        source_count: Some(meta.source_count),
        citation_precision: meta.citation_audit.map(|a| a.precision),
    }
}

fn session_dto(
    id: i64,
    status: &str,
    query_text: &str,
    started_at_ms: i64,
    finished_at_ms: Option<i64>,
) -> ResearchSessionDto {
    ResearchSessionDto {
        id,
        status: status.to_string(),
        query_text: query_text.to_string(),
        started_at_ms,
        finished_at_ms,
    }
}

// ─── Doc drafts and publishing ───────────────────────────────────────────────

/// Render the Architecture-SSOT preview draft for a research session (`vox-scientia` scaffold).
pub async fn render_doc_draft(session_id: i64) -> Result<DocDraftPreview, String> {
    let title = format!("Research Session #{session_id} Architecture SSOT (2026)");
    let slug = format!("session-{session_id}-research");
    let filename = format!("{slug}-2026.md");
    let (title_in, slug_in) = (title.clone(), slug.clone());
    let markdown_content = tokio::task::spawn_blocking(move || {
        let input = vox_scientia::manuscript::scaffold::architecture_ssot::ArchitectureSsotInput {
            title: title_in,
            description: "Empirically verified architecture findings and benchmarks.".into(),
            category: "Architecture SSOTs".into(),
            status: "current".into(),
            training_eligible: true,
            training_rationale: Some("Empirically verified architecture findings.".into()),
            sort_order: None,
            session_id,
            stability_score: 0.90,
            slug: slug_in,
            executive_summary: "Empirical investigation findings.".into(),
            hypothesis: "Initial architectural hypothesis.".into(),
            empirical_outcome_summary: "Measured system performance.".into(),
            codebase_refs: vec![],
            competitive_matrix: vec![],
            verified_claims: vec![],
            sandbox_probes: vec![],
            gaps_and_recommendations: vec![],
            roadmap_phases: vec![],
        };
        vox_scientia::manuscript::scaffold::architecture_ssot::render_architecture_ssot(&input)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(DocDraftPreview {
        slug,
        title,
        filename,
        markdown_content,
        is_valid: true,
        validation_errors: vec![],
    })
}

/// `docs/src/architecture/<slug>-2026.md` under `repo_root`; rejects path-traversal slugs.
fn research_doc_target(repo_root: &FsPath, slug: &str) -> Result<(PathBuf, String), String> {
    if slug.contains('/') || slug.contains('\\') || slug.contains("..") || slug.starts_with('.') {
        return Err("Invalid slug: path traversal characters are not permitted".to_string());
    }
    let filename = if slug.ends_with("-2026.md") {
        slug.to_string()
    } else {
        format!("{slug}-2026.md")
    };
    let path = repo_root
        .join("docs")
        .join("src")
        .join("architecture")
        .join(&filename);
    Ok((path, filename))
}

/// Atomically write a research doc under `repo_root` and index it in the research-artifact FTS.
/// `research-index.md` is retired: the Starlight sidebar derives from frontmatter.
pub async fn write_research_doc(
    db: &vox_db::VoxDb,
    repo_root: &FsPath,
    session_id: i64,
    slug: &str,
    content: &str,
) -> Result<PublishDocResult, String> {
    let (target_path, filename) = research_doc_target(repo_root, slug)?;
    let (path, body) = (target_path.clone(), content.to_string());
    tokio::task::spawn_blocking(move || {
        vox_db::research_doc_io::atomic_write_secure(&path, body.as_bytes())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    let indexed = db
        .store_research_artifact(session_id, "{}", content)
        .await
        .is_ok();
    Ok(PublishDocResult {
        file_path: target_path.to_string_lossy().to_string(),
        relative_path: format!("docs/src/architecture/{filename}"),
        indexed,
    })
}

// ─── Research engine ─────────────────────────────────────────────────────────

fn apply_lane(policy: &mut SearchPolicy, lane: &str) {
    if lane.eq_ignore_ascii_case("deep") {
        policy.default_lane = ResearchLane::Deep;
    } else if lane.eq_ignore_ascii_case("fast") {
        policy.default_lane = ResearchLane::Fast;
    }
}

/// Provider status for the research engine: `Vox.toml [search]` under `repo_root` first, then the
/// GUI user preferences in `db`, then `SearchPolicy::from_env`.
pub async fn engine_status(
    repo_root: &FsPath,
    db: Option<&vox_db::VoxDb>,
) -> ResearchEngineStatusDto {
    let mut policy = SearchPolicy::from_env();
    let mut configured_providers: Option<Vec<String>> = None;
    if let Ok((manifest, _)) = vox_package::VoxManifest::discover(repo_root)
        && let Some(search_table) = manifest.search
    {
        if let Some(lane) = search_table
            .get("active_lane")
            .or_else(|| search_table.get("default_lane"))
            .and_then(|v| v.as_str())
        {
            apply_lane(&mut policy, lane);
        }
        if let Some(ft) = search_table
            .get("fast_timeout_ms")
            .and_then(|v| v.as_integer())
            && ft > 0
        {
            policy.fast_timeout_ms = ft as u64;
        }
        if let Some(dt) = search_table
            .get("deep_timeout_ms")
            .and_then(|v| v.as_integer())
            && dt > 0
        {
            policy.deep_timeout_ms = dt as u64;
        }
        if let Some(arr) = search_table
            .get("enabled_providers")
            .and_then(|v| v.as_array())
        {
            let list: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
            if !list.is_empty() {
                configured_providers = Some(list);
            }
        }
    }

    if let Some(db) = db {
        if let Ok(Some(lane)) = db
            .get_user_preference(GUI_USER_ID, "research.active_lane")
            .await
        {
            apply_lane(&mut policy, &lane);
        }
        if let Ok(Some(ft)) = db
            .get_user_preference(GUI_USER_ID, "research.fast_timeout_ms")
            .await
            && let Ok(ft) = ft.parse::<u64>()
            && ft > 0
        {
            policy.fast_timeout_ms = ft;
        }
        if let Ok(Some(dt)) = db
            .get_user_preference(GUI_USER_ID, "research.deep_timeout_ms")
            .await
            && let Ok(dt) = dt.parse::<u64>()
            && dt > 0
        {
            policy.deep_timeout_ms = dt;
        }
        if configured_providers.is_none()
            && let Ok(Some(prov_json)) = db
                .get_user_preference(GUI_USER_ID, "research.enabled_providers")
                .await
            && let Ok(list) = serde_json::from_str::<Vec<String>>(&prov_json)
            && !list.is_empty()
        {
            configured_providers = Some(list);
        }
    }

    let is_provider_enabled = |id: &str, default_val: bool| -> bool {
        match &configured_providers {
            Some(list) => list.iter().any(|item| item.eq_ignore_ascii_case(id)),
            None => default_val,
        }
    };

    let period = vox_db::store::ops_quota::current_period_key();
    let budget_db = vox_search::tavily_budget::get_budget_db();
    let quota_db = budget_db.as_deref().or(db);
    let tavily_quota = match quota_db {
        Some(qdb) => match qdb.get_quota_usage("tavily", &period).await {
            Ok(Some(usage)) => Some(QuotaUsageDto {
                units_spent: usage.units_spent,
                units_limit: usage.units_limit,
                last_synced_at: usage.last_synced_at,
            }),
            _ => None,
        },
        None => None,
    };

    let provider =
        |id: &str, name: &str, keyless: bool, enabled: bool, has_key: bool| ProviderStatusDto {
            id: id.to_string(),
            name: name.to_string(),
            is_keyless: keyless,
            is_enabled: enabled,
            has_key,
            quota_usage: None,
        };
    let mut tavily = provider(
        "tavily",
        "Tavily Search",
        false,
        is_provider_enabled("tavily", policy.tavily_enabled),
        resolve_secret(SecretId::TavilyApiKey).is_present(),
    );
    tavily.quota_usage = tavily_quota;
    let providers = vec![
        provider(
            "wikipedia",
            "Wikipedia",
            true,
            is_provider_enabled("wikipedia", policy.enable_wikipedia),
            false,
        ),
        provider(
            "openalex",
            "OpenAlex",
            true,
            is_provider_enabled("openalex", policy.enable_openalex),
            resolve_secret(SecretId::VoxOpenAlexEmail).is_present(),
        ),
        provider(
            "arxiv",
            "arXiv",
            true,
            is_provider_enabled("arxiv", policy.enable_arxiv),
            resolve_secret(SecretId::VoxArxivAccessToken).is_present(),
        ),
        tavily,
        provider(
            "searxng",
            "SearXNG",
            true,
            is_provider_enabled("searxng", policy.searxng_url.is_some()),
            false,
        ),
    ];

    ResearchEngineStatusDto {
        active_lane: match policy.default_lane {
            ResearchLane::Fast => "fast".to_string(),
            ResearchLane::Deep => "deep".to_string(),
        },
        fast_timeout_ms: policy.fast_timeout_ms,
        deep_timeout_ms: policy.deep_timeout_ms,
        providers,
        free_key_offers: list_free_tier_offers(),
    }
}

fn provider_secret_id(provider: &str) -> Option<SecretId> {
    match provider.to_ascii_lowercase().as_str() {
        "tavily" | "tavily_api_key" | "tavilyapikey" => Some(SecretId::TavilyApiKey),
        "openalex" => Some(SecretId::VoxOpenAlexEmail),
        "arxiv" => Some(SecretId::VoxArxivAccessToken),
        "gemini" => Some(SecretId::GeminiApiKey),
        "openrouter" => Some(SecretId::OpenRouterApiKey),
        "semantic_scholar" => Some(SecretId::VoxSemanticScholarApiKey),
        other => vox_secrets::secret_id_for_canonical_env(other),
    }
}

/// Merge the lane / timeouts / providers into `Vox.toml [search]` under `repo_root` (no-op when
/// no manifest is found).
fn write_search_table(repo_root: &FsPath, config: &ResearchEngineConfigDto) {
    let Ok((_manifest, path)) = vox_package::VoxManifest::discover(repo_root) else {
        return;
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(mut doc) = content.parse::<toml::Table>() else {
        return;
    };
    let mut search_table = doc
        .remove("search")
        .and_then(|v| match v {
            toml::Value::Table(t) => Some(t),
            _ => None,
        })
        .unwrap_or_default();
    if let Some(lane) = &config.active_lane {
        search_table.insert("active_lane".to_string(), toml::Value::String(lane.clone()));
    }
    if let Some(ft) = config.fast_timeout_ms {
        search_table.insert(
            "fast_timeout_ms".to_string(),
            toml::Value::Integer(ft as i64),
        );
    }
    if let Some(dt) = config.deep_timeout_ms {
        search_table.insert(
            "deep_timeout_ms".to_string(),
            toml::Value::Integer(dt as i64),
        );
    }
    if let Some(providers) = &config.enabled_providers {
        let arr = providers
            .iter()
            .map(|p| toml::Value::String(p.clone()))
            .collect();
        search_table.insert("enabled_providers".to_string(), toml::Value::Array(arr));
    }
    doc.insert("search".to_string(), toml::Value::Table(search_table));
    if let Ok(toml_str) = toml::to_string_pretty(&doc) {
        let _ = std::fs::write(&path, toml_str);
    }
}

/// Persist the engine config: provider keys through Clavis, the lane / timeouts / providers into
/// `Vox.toml` and the GUI user preferences.
pub async fn save_engine_config(
    repo_root: &FsPath,
    db: Option<&vox_db::VoxDb>,
    config: &ResearchEngineConfigDto,
) -> Result<(), String> {
    if let Some(keys) = &config.provider_api_keys {
        for (provider, key_value) in keys {
            let trimmed = key_value.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(id) = provider_secret_id(provider) {
                store_secret(id, trimmed, None)
                    .map_err(|e| format!("Failed to write secret for {provider}: {e}"))?;
            }
        }
    }
    write_search_table(repo_root, config);
    if let Some(db) = db {
        if let Some(lane) = &config.active_lane {
            let _ = db
                .set_user_preference(GUI_USER_ID, "research.active_lane", lane)
                .await;
        }
        if let Some(ft) = config.fast_timeout_ms {
            let _ = db
                .set_user_preference(GUI_USER_ID, "research.fast_timeout_ms", &ft.to_string())
                .await;
        }
        if let Some(dt) = config.deep_timeout_ms {
            let _ = db
                .set_user_preference(GUI_USER_ID, "research.deep_timeout_ms", &dt.to_string())
                .await;
        }
        if let Some(providers) = &config.enabled_providers
            && let Ok(prov_json) = serde_json::to_string(providers)
        {
            let _ = db
                .set_user_preference(GUI_USER_ID, "research.enabled_providers", &prov_json)
                .await;
        }
    }
    Ok(())
}

// ─── Ops ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResearchFlagParams {
    /// `RecordMisguidanceParams` (vox-db-types): session_id, defect_class, culprit_url,
    /// culprit_domain, claim_id, research_query, misleading_excerpt, generated_code_snippet,
    /// failure_diagnostic, correction_diff, reporter, domain_penalty.
    #[schemars(with = "serde_json::Value")]
    pub params: vox_db::RecordMisguidanceParams,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResearchPublishDocParams {
    pub session_id: i64,
    pub slug: String,
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResearchEngineConfigSetParams {
    pub config: ResearchEngineConfigDto,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResearchEngineProbeParams {
    /// Provider id; omit to probe every provider.
    #[serde(default)]
    pub provider: Option<String>,
    pub query: String,
}

pub async fn flag_misleading(state: &ServerState, p: ResearchFlagParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<i64>::err(e).to_json(),
    };
    match db.record_research_misguidance(&p.params).await {
        Ok(id) => ToolResult::ok(id).to_json(),
        Err(e) => ToolResult::<i64>::err(e.to_string()).to_json(),
    }
}

pub async fn publish_doc(state: &ServerState, p: ResearchPublishDocParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<PublishDocResult>::err(e).to_json(),
    };
    match write_research_doc(
        db,
        &state.repository.root,
        p.session_id,
        &p.slug,
        &p.content,
    )
    .await
    {
        Ok(r) => ToolResult::ok(r).to_json(),
        Err(e) => ToolResult::<PublishDocResult>::err(e).to_json(),
    }
}

pub async fn engine_config_set(state: &ServerState, p: ResearchEngineConfigSetParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<()>::err(e).to_json(),
    };
    match save_engine_config(&state.repository.root, Some(db), &p.config).await {
        Ok(()) => ToolResult::ok(()).to_json(),
        Err(e) => ToolResult::<()>::err(e).to_json(),
    }
}

pub async fn engine_probe(_state: &ServerState, p: ResearchEngineProbeParams) -> String {
    let out = match p.provider {
        Some(provider) => vox_search::probe::probe_search_provider(provider, p.query)
            .await
            .map(|r| serde_json::to_value(r).unwrap_or(Value::Null)),
        None => vox_search::probe::probe_all_search_providers(p.query)
            .await
            .map(|r| serde_json::to_value(r).unwrap_or(Value::Null)),
    };
    match out {
        Ok(v) => ToolResult::ok(v).to_json(),
        Err(e) => ToolResult::<Value>::err(e).to_json(),
    }
}

// ─── Routes ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<u32>,
}

/// GET /api/v2/research/sessions?limit=
pub async fn get_research_sessions(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<LimitQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, RESEARCH_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db
        .list_recent_research_sessions(q.limit.unwrap_or(DEFAULT_SESSION_LIMIT))
        .await
    {
        Ok(rows) => ok(rows
            .iter()
            .map(|r| {
                session_dto(
                    r.id,
                    &r.status,
                    &r.query_text,
                    r.started_at_ms,
                    r.finished_at_ms,
                )
            })
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/research/sessions/{id}
pub async fn get_research_session_detail(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(session_id): Path<i64>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, RESEARCH_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let session = match db.get_research_session(session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return err(
                "not_found",
                &format!("research session {session_id} not found"),
            );
        }
        Err(e) => return err("db_error", &e.to_string()),
    };
    let artifact = match db.get_research_artifact(session_id).await {
        Ok(a) => a,
        Err(e) => return err("db_error", &e.to_string()),
    };
    let summary = artifact
        .as_ref()
        .map(|a| extract_research_summary(&a.artifact_json))
        .unwrap_or_default();
    ok(ResearchDetailDto {
        session: session_dto(
            session.id,
            &session.status,
            &session.query_text,
            session.started_at_ms,
            session.finished_at_ms,
        ),
        report_markdown: artifact.as_ref().map(|a| a.report_markdown.clone()),
        artifact_json: artifact.as_ref().map(|a| a.artifact_json.clone()),
        confidence_tier: summary.confidence_tier,
        claims: summary.claims,
        source_count: summary.source_count,
        citation_precision: summary.citation_precision,
    })
}

/// GET /api/v2/research/sessions/{id}/doc-draft
pub async fn get_research_doc_draft(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(session_id): Path<i64>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, RESEARCH_SURFACES) {
        return e;
    }
    match render_doc_draft(session_id).await {
        Ok(d) => ok(d),
        Err(e) => err("render_error", &e),
    }
}

/// GET /api/v2/research/engine-status
pub async fn get_research_engine_status(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, RESEARCH_SURFACES) {
        return e;
    }
    let state = gs.server_state();
    ok(engine_status(&state.repository.root, state.db.as_deref()).await)
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
        .route("/research/sessions", get(get_research_sessions))
        .route("/research/sessions/{id}", get(get_research_session_detail))
        .route(
            "/research/sessions/{id}/doc-draft",
            get(get_research_doc_draft),
        )
        .route("/research/engine-status", get(get_research_engine_status))
        .merge(super::scientia::api_v2_router())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer, tool_json};

    #[test]
    fn router_builds() {
        let _ = api_v2_router();
    }

    fn fake_artifact_json() -> String {
        r#"{
            "schema_version": 1,
            "result": {
                "answer": "x",
                "citations": [
                    { "source_id": 1, "url": "https://example.com/a" },
                    { "source_id": 2, "url": "https://example.org/b" }
                ],
                "research_metadata": {
                    "routing_tier": "DeepResearch",
                    "source_count": 2,
                    "claim_verdicts": [
                        {
                            "claim": { "text": "The sky is blue.", "claim_id": 42 },
                            "verdict": "supported",
                            "confidence": 0.95,
                            "evidence_spans": [ { "source_id": 1 }, { "source_id": 2 } ],
                            "resample_stability": 1.0
                        }
                    ],
                    "citation_audit": { "precision": 1.0 },
                    "corroboration_counts": [[42, 2]]
                }
            }
        }"#
        .to_string()
    }

    #[test]
    fn extracts_confidence_tier_claims_and_source_count() {
        let summary = extract_research_summary(&fake_artifact_json());
        assert_eq!(summary.confidence_tier.as_deref(), Some("DeepResearch"));
        assert_eq!(summary.source_count, Some(2));
        assert_eq!(summary.citation_precision, Some(1.0));
        let claim = &summary.claims[0];
        assert_eq!(claim.claim_id, "42");
        assert_eq!(claim.verdict, "Supported");
        assert_eq!(
            claim.citation_urls,
            vec!["https://example.com/a", "https://example.org/b"]
        );
        assert_eq!(claim.corroboration_count, 2);
    }

    #[test]
    fn summary_fails_open_on_malformed_json() {
        let summary = extract_research_summary("not json");
        assert!(summary.confidence_tier.is_none());
        assert!(summary.claims.is_empty());
    }

    #[tokio::test]
    async fn draft_then_publish_writes_the_doc_and_leaves_the_retired_index_alone() {
        let dir = tempfile::tempdir().expect("tempdir");
        let arch = dir.path().join("docs/src/architecture");
        std::fs::create_dir_all(&arch).unwrap();
        let index = arch.join("research-index.md");
        std::fs::write(&index, "# Research Index\n").unwrap();
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .unwrap();

        let draft = render_doc_draft(42).await.expect("draft");
        assert_eq!(draft.slug, "session-42-research");
        assert_eq!(draft.filename, "session-42-research-2026.md");
        assert!(
            draft
                .markdown_content
                .contains("title: \"Research Session #42 Architecture SSOT (2026)\"")
        );

        let r = write_research_doc(&db, dir.path(), 42, &draft.slug, &draft.markdown_content)
            .await
            .expect("publish");
        assert_eq!(
            r.relative_path,
            "docs/src/architecture/session-42-research-2026.md"
        );
        assert!(r.indexed);
        assert_eq!(
            std::fs::read_to_string(&r.file_path).unwrap(),
            draft.markdown_content
        );
        assert_eq!(
            std::fs::read_to_string(&index).unwrap(),
            "# Research Index\n"
        );
        let again = write_research_doc(&db, dir.path(), 42, &draft.filename, "x")
            .await
            .expect("re-publish with .md slug");
        assert_eq!(again.relative_path, r.relative_path);
    }

    #[tokio::test]
    async fn publish_rejects_path_traversal_slugs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .unwrap();
        for bad in ["../../etc/passwd", "..\\..\\w", "nested/path", ".hidden"] {
            let e = write_research_doc(&db, dir.path(), 1, bad, "# x")
                .await
                .unwrap_err();
            assert!(e.contains("Invalid slug"), "{bad}: {e}");
        }
    }

    #[tokio::test]
    async fn sessions_routes_report_missing_db_and_are_profile_gated() {
        let gs = GatewayState::for_test().await;
        let q = Query(LimitQuery { limit: None });
        let missing =
            body(get_research_sessions(State(gs.clone()), peer(), HeaderMap::new(), q).await).await;
        assert_eq!(missing["error"]["code"], "db_unavailable", "{missing}");

        let named = ServerState::new_test().await.with_profile(
            crate::profiles::ActiveProfile::resolve(&["mesh"]).expect("mesh profile"),
        );
        let gs = GatewayState::for_test_with_state(named, HashSet::new()).await;
        let denied =
            body(get_research_engine_status(State(gs.clone()), peer(), HeaderMap::new()).await)
                .await;
        assert_eq!(denied["error"]["code"], "tool_not_in_profile", "{denied}");
    }

    #[tokio::test]
    async fn session_detail_and_flag_round_trip_on_a_memory_db() {
        let (gs, db) = gateway_with_memory_db().await;
        let sid = db
            .create_research_session("s-detail", "why is the sky blue")
            .await
            .expect("session");
        let detail = body(
            get_research_session_detail(State(gs.clone()), peer(), HeaderMap::new(), Path(sid))
                .await,
        )
        .await;
        assert_eq!(
            detail["data"]["session"]["query_text"],
            "why is the sky blue"
        );
        let listed = body(
            get_research_sessions(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(LimitQuery { limit: Some(5) }),
            )
            .await,
        )
        .await;
        assert_eq!(listed["data"].as_array().unwrap().len(), 1, "{listed}");
        let missing = body(
            get_research_session_detail(State(gs.clone()), peer(), HeaderMap::new(), Path(999))
                .await,
        )
        .await;
        assert_eq!(missing["error"]["code"], "not_found");

        let params: vox_db::RecordMisguidanceParams = serde_json::from_value(serde_json::json!({
            "session_id": sid,
            "defect_class": "inelegant_code",
            "culprit_url": "https://flawed-docs.com",
            "culprit_domain": "flawed-docs.com",
            "claim_id": null,
            "research_query": "q",
            "misleading_excerpt": "x",
            "generated_code_snippet": null,
            "failure_diagnostic": null,
            "correction_diff": null,
            "reporter": "user",
            "domain_penalty": 0.2
        }))
        .expect("params");
        let flagged =
            tool_json(&flag_misleading(gs.server_state(), ResearchFlagParams { params }).await);
        assert_eq!(flagged["success"], true, "{flagged}");
    }

    #[tokio::test]
    async fn probe_op_delegates_input_validation_to_vox_search() {
        let st = ServerState::new_test().await;
        let one = tool_json(
            &engine_probe(
                &st,
                ResearchEngineProbeParams {
                    provider: Some("wikipedia".into()),
                    query: "  ".into(),
                },
            )
            .await,
        );
        assert_eq!(one["error"], "Query cannot be empty", "{one}");
        let all = tool_json(
            &engine_probe(
                &st,
                ResearchEngineProbeParams {
                    provider: None,
                    query: "a".repeat(1001),
                },
            )
            .await,
        );
        assert_eq!(all["success"], false, "{all}");
    }

    #[tokio::test]
    async fn engine_config_round_trips_through_vox_toml_and_prefs() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Vox.toml"),
            "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .unwrap();
        let cfg = ResearchEngineConfigDto {
            active_lane: Some("deep".into()),
            fast_timeout_ms: Some(2200),
            enabled_providers: Some(vec!["arxiv".into()]),
            ..Default::default()
        };
        save_engine_config(dir.path(), Some(&db), &cfg)
            .await
            .expect("save");
        let toml_text = std::fs::read_to_string(dir.path().join("Vox.toml")).unwrap();
        assert!(toml_text.contains("active_lane = \"deep\""), "{toml_text}");
        let status = engine_status(dir.path(), Some(&db)).await;
        assert_eq!(status.active_lane, "deep");
        assert_eq!(status.fast_timeout_ms, 2200);
        let arxiv = status.providers.iter().find(|p| p.id == "arxiv").unwrap();
        assert!(arxiv.is_enabled);
        let wiki = status
            .providers
            .iter()
            .find(|p| p.id == "wikipedia")
            .unwrap();
        assert!(!wiki.is_enabled);
        assert!(
            status
                .providers
                .iter()
                .any(|p| p.id == "openalex" && p.is_keyless)
        );
        assert!(
            status
                .free_key_offers
                .iter()
                .any(|o| o.provider_id == "tavily" && o.signup_url.contains("tavily.com"))
        );
    }
}
