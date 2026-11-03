//! Knowledge-profile routes and ops for Axis (plan 07 Task 6): knowledge-base browsing and
//! ingest, Memory-surface corpus counts and recall, and unified search.
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::gui_ops::dto::knowledge::{CorpusCountsDto, KbHealthDto, KnowledgeNodeRow, UiHitResult};
use crate::gui_ops::{api_db, enforce_dashboard_surface, require_db};
use crate::http_gateway::GatewayState;
use crate::http_gateway::dashboard_api::enforce_dashboard_read;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::services::routes::{err, ok};

const REM_KB_DB: &str = "Verify Turso connectivity and vox-db migrations for knowledge_nodes.";
const DEFAULT_NODE_LIMIT: usize = 50;
const DEFAULT_RECALL_LIMIT: usize = 10;

// ─── Ops ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KbNodeDeleteParams {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KbIngestTextParams {
    /// Node label; defaults to "Manual Ingest" when blank.
    #[serde(default)]
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KbIngestUrlParams {
    pub url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct KbImportResearchSessionParams {
    pub session_id: i64,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct MemoryReindexParams {}

fn db_err<T: serde::Serialize>(e: impl ToString) -> String {
    ToolResult::<T>::err_with_remediation(e.to_string(), REM_KB_DB).to_json()
}

pub async fn kb_node_delete(state: &ServerState, p: KbNodeDeleteParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<String>::err(e).to_json(),
    };
    match db.delete_knowledge_node(&p.id).await {
        Ok(()) => ToolResult::ok(p.id).to_json(),
        Err(e) => db_err::<String>(e),
    }
}

pub async fn kb_ingest_text(state: &ServerState, p: KbIngestTextParams) -> String {
    if p.content.trim().is_empty() {
        return ToolResult::<String>::err("Content cannot be empty").to_json();
    }
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<String>::err(e).to_json(),
    };
    let label = if p.title.trim().is_empty() {
        "Manual Ingest".to_string()
    } else {
        p.title.trim().to_string()
    };
    let content_hash = vox_crypto::hash_fast_hex(p.content.as_bytes());
    let node_id = format!("doc:{content_hash}");
    let meta = serde_json::json!({
        "title": label,
        "source_url": p.source_url,
        "ingest_type": "text",
    })
    .to_string();
    match db
        .upsert_knowledge_node(
            &node_id,
            &label,
            &p.content,
            Some("document"),
            Some(&meta),
            None,
        )
        .await
    {
        Ok(_) => ToolResult::ok(node_id).to_json(),
        Err(e) => db_err::<String>(e),
    }
}

fn validate_ingest_url(url: &str) -> Result<&str, String> {
    let clean = url.trim();
    if clean.is_empty() {
        return Err("URL cannot be empty".to_string());
    }
    if !clean.starts_with("http://") && !clean.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }
    Ok(clean)
}

#[cfg_attr(not(feature = "web-scrape"), allow(dead_code))]
fn get_robots_url(target_url: &str) -> Option<(String, String)> {
    let remainder = target_url
        .strip_prefix("http://")
        .map(|r| ("http://", r))
        .or_else(|| target_url.strip_prefix("https://").map(|r| ("https://", r)))?;
    let (scheme, rest) = remainder;
    let (host, path) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{}", p)),
        None => (rest, "/".to_string()),
    };
    if host.is_empty() {
        return None;
    }
    let robots_url = format!("{scheme}{host}/robots.txt");
    Some((robots_url, path))
}

#[cfg(feature = "web-scrape")]
async fn check_robots_allowed(client: &reqwest::Client, target_url: &str) -> bool {
    let Some((robots_url, target_path)) = get_robots_url(target_url) else {
        return true;
    };
    if let Ok(resp) = client.get(&robots_url).send().await
        && resp.status().is_success()
        && let Ok(text) = resp.text().await
    {
        let mut in_relevant_agent = false;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = trimmed.split_once(':') {
                let key = k.trim().to_lowercase();
                let val = v.trim();
                if key == "user-agent" {
                    in_relevant_agent = val == "*" || val.to_lowercase().contains("vox");
                } else if in_relevant_agent && key == "disallow" {
                    if val.is_empty() {
                        continue;
                    }
                    if val == "/" || target_path.starts_with(val) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

#[cfg(feature = "web-scrape")]
async fn scrape_into_kb(db: &vox_db::VoxDb, clean_url: &str) -> Result<String, String> {
    let policy = vox_search::policy::SearchPolicy::from_env();
    let timeout_ms = policy.scraper_timeout_ms.max(1000);

    if policy.scraper_robots_txt_respect {
        let client = vox_http_client::client_builder()
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .user_agent("VoxResearchBot/1.0 (+https://vox.dev/research-bot)")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;
        if !check_robots_allowed(&client, clean_url).await {
            return Err(format!("Scraping disallowed by robots.txt for {clean_url}"));
        }
    }

    let doc = vox_search::scraper::fetch_and_extract(clean_url, timeout_ms)
        .await
        .map_err(|e| format!("Failed to scrape URL {clean_url}: {e}"))?;

    let url_hash = vox_crypto::hash_fast_hex(clean_url.as_bytes());
    let node_id = format!("web:{url_hash}");
    let meta = serde_json::json!({
        "url": doc.url,
        "title": doc.title,
        "text_density": doc.text_density,
        "ingest_type": "url",
    })
    .to_string();
    let label = if doc.title.trim().is_empty() {
        clean_url
    } else {
        doc.title.trim()
    };
    db.upsert_knowledge_node(
        &node_id,
        label,
        &doc.markdown,
        Some("web_scrape"),
        Some(&meta),
        None,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(node_id)
}

#[cfg(not(feature = "web-scrape"))]
async fn scrape_into_kb(_db: &vox_db::VoxDb, _clean_url: &str) -> Result<String, String> {
    Err("URL ingest needs a server built with web scraping (vox-cli `heavy-retrieval`)".into())
}

pub async fn kb_ingest_url(state: &ServerState, p: KbIngestUrlParams) -> String {
    let clean_url = match validate_ingest_url(&p.url) {
        Ok(u) => u,
        Err(e) => return ToolResult::<String>::err(e).to_json(),
    };
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<String>::err(e).to_json(),
    };
    match scrape_into_kb(db, clean_url).await {
        Ok(id) => ToolResult::ok(id).to_json(),
        Err(e) => ToolResult::<String>::err(e).to_json(),
    }
}

#[derive(serde::Deserialize)]
struct MinimalArtifactCitation {
    #[serde(default)]
    url: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    snippet: String,
}

#[derive(serde::Deserialize)]
struct MinimalArtifactResult {
    #[serde(default)]
    citations: Vec<MinimalArtifactCitation>,
}

#[derive(serde::Deserialize)]
struct MinimalArtifact {
    #[serde(default)]
    result: Option<MinimalArtifactResult>,
}

async fn import_research_session(
    db: &vox_db::VoxDb,
    repo_root: &std::path::Path,
    session_id: i64,
) -> Result<usize, String> {
    let session = db
        .get_research_session(session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Research session {session_id} not found"))?;
    let artifact = db
        .get_research_artifact(session_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("No artifact found for research session {session_id}"))?;
    let parsed: MinimalArtifact = serde_json::from_str(&artifact.artifact_json)
        .map_err(|e| format!("Failed to parse session artifact JSON: {e}"))?;
    let citations = parsed.result.map(|r| r.citations).unwrap_or_default();
    let sources: Vec<crate::chat_tools::chat::research_turn::Source> = citations
        .into_iter()
        .enumerate()
        .map(|(idx, c)| crate::chat_tools::chat::research_turn::Source {
            n: idx + 1,
            url: c.url,
            title: c.title,
            engine: "research_session".to_string(),
            snippet: c.snippet,
        })
        .collect();
    use crate::chat_tools::chat::research_turn as rt;
    let origin = vox_db::kb_text::FindingOrigin::GuiSaveResearchSession;
    let mut saved =
        rt::capture_research_sources(db, repo_root, &session.query_text, &sources, origin).await;
    if !artifact.report_markdown.trim().is_empty()
        && rt::capture_research_synthesis(
            db,
            repo_root,
            &session.query_text,
            &sources,
            &artifact.report_markdown,
            origin,
        )
        .await
        .is_some()
    {
        saved += 1;
    }
    Ok(saved)
}

pub async fn kb_import_research_session(
    state: &ServerState,
    p: KbImportResearchSessionParams,
) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<usize>::err(e).to_json(),
    };
    match import_research_session(db, &state.repository.root, p.session_id).await {
        Ok(saved) => ToolResult::ok(saved).to_json(),
        Err(e) => ToolResult::<usize>::err(e).to_json(),
    }
}

/// Two-way sync between the daemon's `MEMORY.md` store and the database.
pub async fn memory_reindex(state: &ServerState, _p: MemoryReindexParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db.clone(),
        Err(e) => return ToolResult::<String>::err(e).to_json(),
    };
    let manager =
        vox_orchestrator::memory::MemoryManager::new(state.orchestrator_config.memory.clone());
    let mut manager = match manager {
        Ok(m) => m.with_db(db),
        Err(e) => return ToolResult::<String>::err(e.to_string()).to_json(),
    };
    if let Err(e) = manager.sync_from_db().await {
        return ToolResult::<String>::err(e.to_string()).to_json();
    }
    match manager.sync_to_db().await {
        Ok(_) => ToolResult::ok("reindexed".to_string()).to_json(),
        Err(e) => ToolResult::<String>::err(e.to_string()).to_json(),
    }
}

// ─── Routes ──────────────────────────────────────────────────────────────────

fn guard(
    gs: &GatewayState,
    peer: &SocketAddr,
    headers: &HeaderMap,
    surfaces: &[&str],
) -> Result<(), Json<Value>> {
    enforce_dashboard_read(gs, peer, headers)?;
    enforce_dashboard_surface(gs, surfaces)
}

#[derive(Debug, Deserialize)]
pub struct NodesQuery {
    pub node_type: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

/// GET /api/v2/knowledge/nodes?node_type=&page=&limit=
pub async fn get_knowledge_nodes(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<NodesQuery>,
) -> Json<Value> {
    if let Err(e) = guard(&gs, &connect.0, &headers, &["memory", "research"]) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db
        .list_knowledge_nodes(
            q.node_type.as_deref(),
            q.page.unwrap_or(0),
            q.limit.unwrap_or(DEFAULT_NODE_LIMIT),
        )
        .await
    {
        Ok(rows) => ok(rows
            .into_iter()
            .map(KnowledgeNodeRow::from_record)
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/knowledge/health
pub async fn get_knowledge_health(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = guard(&gs, &connect.0, &headers, &["memory", "research"]) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db.get_knowledge_health().await {
        Ok(h) => ok(KbHealthDto {
            node_count: h.node_count,
            edge_count: h.edge_count,
            fts_available: h.fts_available,
            corpus_counts: h.corpus_counts,
        }),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/knowledge/corpus-counts
pub async fn get_corpus_counts(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = guard(&gs, &connect.0, &headers, &["memory"]) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db.search_corpus_counts().await {
        Ok(c) => ok(CorpusCountsDto::from(c)),
        Err(e) => err("db_error", &e.to_string()),
    }
}

#[derive(Debug, Deserialize)]
pub struct RecallQuery {
    pub q: String,
    pub limit: Option<usize>,
}

fn hit_kind(path: &str) -> &'static str {
    if path.contains("memory.md") {
        "chat"
    } else if path.contains("docs") || path.contains("README") {
        "text"
    } else if path.contains("crates") || path.contains(".rs") {
        "code"
    } else {
        "text"
    }
}

/// GET /api/v2/knowledge/recall?q=&limit=
pub async fn get_recall(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<RecallQuery>,
) -> Json<Value> {
    if let Err(e) = guard(&gs, &connect.0, &headers, &["memory"]) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db.clone(),
        Err(e) => return e,
    };
    let engine = vox_search::memory_hybrid::MemorySearchEngine::new().with_db(db);
    let hits = engine
        .hybrid_search(&q.q, q.limit.unwrap_or(DEFAULT_RECALL_LIMIT), None, 0.5)
        .await;
    ok(hits
        .into_iter()
        .map(|h| UiHitResult {
            kind: hit_kind(&h.path).to_string(),
            src: h.path,
            line: 0,
            score: h.score,
            text: h.content_snippet,
        })
        .collect::<Vec<_>>())
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    /// Comma-separated corpus allow-list.
    pub scope: Option<String>,
    /// Comma-separated hit-kind filter.
    pub kinds: Option<String>,
    pub path_glob: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// `Some(vec![])` for an empty-but-present list keeps the "explicitly scoped" meaning.
fn split_list(s: Option<String>) -> Option<Vec<String>> {
    s.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    })
}

/// GET /api/v2/search/query?q=&scope=a,b&kinds=x,y&path_glob=&limit=&offset=
pub async fn get_search_query(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<SearchQuery>,
) -> Json<Value> {
    if let Err(e) = guard(&gs, &connect.0, &headers, &["memory", "vox-search"]) {
        return e;
    }
    match crate::gui_ops::search::search_query(
        gs.server_state(),
        q.q,
        split_list(q.scope),
        split_list(q.kinds),
        q.path_glob,
        q.limit,
        q.offset,
    )
    .await
    {
        Ok(resp) => ok(resp),
        Err(e) => err("search_failed", &e),
    }
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
        .route("/knowledge/nodes", get(get_knowledge_nodes))
        .route("/knowledge/health", get(get_knowledge_health))
        .route("/knowledge/corpus-counts", get(get_corpus_counts))
        .route("/knowledge/recall", get(get_recall))
        .route("/search/query", get(get_search_query))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use axum::extract::{Query, State};
    use axum::http::HeaderMap;

    use super::*;
    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer, tool_json};

    #[test]
    fn router_builds() {
        let _ = api_v2_router();
    }

    async fn nodes(gs: &GatewayState) -> Value {
        let q = Query(NodesQuery {
            node_type: None,
            page: None,
            limit: None,
        });
        body(get_knowledge_nodes(State(gs.clone()), peer(), HeaderMap::new(), q).await).await
    }

    #[tokio::test]
    async fn ingest_list_count_and_delete_round_trip() {
        let (gs, _db) = gateway_with_memory_db().await;
        let st = gs.server_state();
        let added = tool_json(
            &kb_ingest_text(
                st,
                KbIngestTextParams {
                    title: " Remote note ".into(),
                    content: "remote body".into(),
                    source_url: None,
                },
            )
            .await,
        );
        assert_eq!(added["success"], true, "{added}");
        let id = added["data"].as_str().unwrap().to_string();
        assert!(id.starts_with("doc:"));

        let listed = nodes(&gs).await;
        let rows = listed["data"].as_array().expect("rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["label"], "Remote note");

        let counts =
            body(get_corpus_counts(State(gs.clone()), peer(), HeaderMap::new()).await).await;
        assert_eq!(counts["data"]["docs"], 1, "{counts}");
        let health =
            body(get_knowledge_health(State(gs.clone()), peer(), HeaderMap::new()).await).await;
        assert_eq!(health["data"]["node_count"], 1, "{health}");

        let deleted = tool_json(&kb_node_delete(st, KbNodeDeleteParams { id }).await);
        assert_eq!(deleted["success"], true, "{deleted}");
        assert!(nodes(&gs).await["data"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn ops_validate_inputs_before_touching_the_db() {
        let (gs, _db) = gateway_with_memory_db().await;
        let st = gs.server_state();
        let blank = tool_json(
            &kb_ingest_text(
                st,
                KbIngestTextParams {
                    title: String::new(),
                    content: "  ".into(),
                    source_url: None,
                },
            )
            .await,
        );
        assert_eq!(blank["success"], false);
        for url in ["", "ftp://x"] {
            let out = tool_json(&kb_ingest_url(st, KbIngestUrlParams { url: url.into() }).await);
            assert_eq!(out["success"], false, "{url}: {out}");
        }
        let missing = tool_json(
            &kb_import_research_session(st, KbImportResearchSessionParams { session_id: 999 })
                .await,
        );
        assert_eq!(missing["success"], false);
        assert!(
            missing["error"].as_str().unwrap().contains("999"),
            "{missing}"
        );
    }

    #[tokio::test]
    async fn recall_and_search_routes_answer_on_an_empty_db() {
        let (gs, _db) = gateway_with_memory_db().await;
        let recall = body(
            get_recall(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(RecallQuery {
                    q: "anything".into(),
                    limit: Some(3),
                }),
            )
            .await,
        )
        .await;
        assert!(recall["data"].is_array(), "{recall}");
        let chats_only = body(
            get_search_query(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(SearchQuery {
                    q: "hello".into(),
                    scope: Some("chats".into()),
                    kinds: None,
                    path_glob: None,
                    limit: None,
                    offset: None,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(chats_only["data"]["corpora"], serde_json::json!(["chats"]));
        let empty = body(
            get_search_query(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(SearchQuery {
                    q: "  ".into(),
                    scope: None,
                    kinds: None,
                    path_glob: None,
                    limit: None,
                    offset: None,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(empty["error"]["code"], "search_failed");
    }

    #[tokio::test]
    async fn knowledge_routes_report_missing_db_and_are_profile_gated() {
        let gs = GatewayState::for_test().await;
        let v = body(get_corpus_counts(State(gs), peer(), HeaderMap::new()).await).await;
        assert_eq!(v["error"]["code"], "db_unavailable");

        let named = ServerState::new_test()
            .await
            .with_profile(crate::profiles::ActiveProfile::resolve(&["mesh"]).expect("mesh"));
        let gs = GatewayState::for_test_with_state(named, HashSet::new()).await;
        let v = body(get_knowledge_health(State(gs.clone()), peer(), HeaderMap::new()).await).await;
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
        let v = body(
            get_search_query(
                State(gs),
                peer(),
                HeaderMap::new(),
                Query(SearchQuery {
                    q: "x".into(),
                    scope: None,
                    kinds: None,
                    path_glob: None,
                    limit: None,
                    offset: None,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
    }

    #[test]
    fn split_list_trims_and_keeps_explicit_empty() {
        assert_eq!(
            split_list(Some("memory, repo,,".into())),
            Some(vec!["memory".to_string(), "repo".to_string()])
        );
        assert_eq!(split_list(Some(String::new())), Some(vec![]));
        assert_eq!(split_list(None), None);
    }

    #[test]
    fn test_get_robots_url_parsing() {
        assert_eq!(
            get_robots_url("https://example.com/docs/guide"),
            Some((
                "https://example.com/robots.txt".to_string(),
                "/docs/guide".to_string()
            ))
        );
        assert_eq!(
            get_robots_url("http://domain.org"),
            Some(("http://domain.org/robots.txt".to_string(), "/".to_string()))
        );
        assert_eq!(get_robots_url("invalid-url"), None);
    }

    #[test]
    fn test_minimal_artifact_deserialization() {
        let parsed: MinimalArtifact = serde_json::from_str(
            r#"{"result":{"citations":[{"url":"https://example.com/a","title":"Example A"}]}}"#,
        )
        .expect("deserialize MinimalArtifact");
        let citations = parsed.result.unwrap().citations;
        assert_eq!(citations[0].url, "https://example.com/a");
        assert_eq!(citations[0].snippet, "");
    }
}
