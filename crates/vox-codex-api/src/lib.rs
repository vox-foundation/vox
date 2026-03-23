//! HTTP surface for **Codex** reactivity and named query snapshots (BaaS-style).
//!
//! Also exposes **`/api/audio/*`** for **Vox Oratio** (Candle Whisper, pure Rust — no whisper.cpp).

use std::convert::Infallible;
use std::path::{Path as FsPath, PathBuf};
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::extract::{Path as AxPath, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::{get, post};
use futures_util::stream::Stream;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use vox_db::{Codex, DbConfig, evaluate_codex_api_readiness};

/// Shared server state.
#[derive(Clone)]
pub struct CodexApiState {
    /// Shared **Codex** connection for all handlers.
    pub codex: Arc<Codex>,
    /// Base directory for resolving relative paths in `POST /api/audio/transcribe`.
    pub audio_workspace_root: PathBuf,
    /// Used when `POST /api/codex/research-session` omits or blanks `repository_id` (from `vox_repository` at dashboard start).
    pub default_repository_id: String,
}

/// Query parameters for the SSE route `GET /api/codex/subscribe/:topic` (resume cursor).
#[derive(Debug, Deserialize)]
pub struct AfterQuery {
    /// Return changes strictly after this `codex_change_log.id` (0 = from start).
    pub after_id: Option<i64>,
}

/// Build REST + SSE routes under `/api/codex` and Oratio audio under `/api/audio`.
pub fn codex_router(state: CodexApiState) -> Router {
    Router::new()
        .route("/health", get(codex_health))
        .route("/ready", get(codex_ready))
        .route("/api/codex/query/:name", get(query_named))
        .route("/api/codex/mutate/:name", post(mutate_named))
        .route("/api/codex/subscribe/:topic", get(subscribe_topic))
        .route("/api/search/status", get(search_status))
        .route(
            "/api/codex/research-session",
            post(post_research_session_upsert),
        )
        .route(
            "/api/codex/conversations/:conv_id/versions",
            post(post_conversation_version),
        )
        .route(
            "/api/codex/conversation-edges",
            post(post_conversation_edge),
        )
        .route(
            "/api/codex/topics/:topic_id/evolution-events",
            post(post_topic_evolution_event),
        )
        .route("/api/audio/status", get(get_audio_status))
        .route("/api/audio/transcribe", post(post_audio_transcribe))
        .with_state(state)
}

async fn codex_health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok" }))
}

async fn codex_ready(
    State(st): State<CodexApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let r = evaluate_codex_api_readiness(st.codex.as_ref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if !r.ready {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            format!(
                "not ready: schema_version={} missing_tables={:?} baseline_digest={}",
                r.schema_version, r.missing_tables, r.baseline_digest_hex
            ),
        ));
    }
    Ok(Json(json!({
        "status": "ready",
        "schema_version": r.schema_version,
        "baseline_digest": r.baseline_digest_hex,
        "capabilities": { "codex_api_surface": true },
    })))
}

async fn search_table_count(codex: &Codex, sql: &str) -> Result<i64, (StatusCode, String)> {
    let rows = codex
        .query_all(sql, ())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let row = rows.first().ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "COUNT returned no rows".into(),
        )
    })?;
    row.get::<i64>(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Row counts for search ingest tables (`search_*`).
/// `POST /api/codex/research-session` — upsert `research_sessions` by `session_key`.
#[derive(Debug, Deserialize)]
pub struct ResearchSessionUpsertBody {
    pub session_key: String,
    pub title: Option<String>,
    pub status: Option<String>,
    pub repository_id: Option<String>,
    pub config_json: Option<serde_json::Value>,
    pub summary_json: Option<serde_json::Value>,
}

async fn post_research_session_upsert(
    State(st): State<CodexApiState>,
    Json(body): Json<ResearchSessionUpsertBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let title = body.title.as_deref().unwrap_or("");
    let status = body.status.as_deref().unwrap_or("active");
    let repo = body
        .repository_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(st.default_repository_id.as_str());
    let config_s = body
        .config_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let summary_s = body
        .summary_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let id = st
        .codex
        .research_session_upsert(
            &body.session_key,
            title,
            status,
            repo,
            config_s.as_deref(),
            summary_s.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({
        "id": id,
        "session_key": body.session_key,
        "repository_id": repo,
    })))
}

/// `POST /api/codex/conversations/:conv_id/versions`
#[derive(Debug, Deserialize)]
pub struct ConversationVersionBody {
    pub version_index: i64,
    pub label: Option<String>,
    pub snapshot_json: Option<serde_json::Value>,
}

async fn post_conversation_version(
    State(st): State<CodexApiState>,
    AxPath(conv_id): AxPath<i64>,
    Json(body): Json<ConversationVersionBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let label = body.label.as_deref().unwrap_or("");
    let snap = body
        .snapshot_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let row = st
        .codex
        .conversation_version_append(conv_id, body.version_index, label, snap.as_deref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({
        "id": row,
        "conversation_id": conv_id,
        "version_index": body.version_index,
    })))
}

/// `POST /api/codex/conversation-edges`
#[derive(Debug, Deserialize)]
pub struct ConversationEdgeBody {
    pub from_conversation_id: i64,
    pub to_conversation_id: i64,
    pub edge_kind: Option<String>,
    pub weight: Option<f64>,
    pub metadata_json: Option<serde_json::Value>,
}

async fn post_conversation_edge(
    State(st): State<CodexApiState>,
    Json(body): Json<ConversationEdgeBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let kind = body.edge_kind.as_deref().unwrap_or("related");
    let w = body.weight.unwrap_or(1.0);
    let meta = body
        .metadata_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let id = st
        .codex
        .conversation_edge_insert(
            body.from_conversation_id,
            body.to_conversation_id,
            kind,
            w,
            meta.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "id": id })))
}

/// `POST /api/codex/topics/:topic_id/evolution-events`
#[derive(Debug, Deserialize)]
pub struct TopicEvolutionBody {
    pub event_kind: String,
    pub prior_label: Option<String>,
    pub new_label: Option<String>,
    pub detail_json: Option<serde_json::Value>,
}

async fn post_topic_evolution_event(
    State(st): State<CodexApiState>,
    AxPath(topic_id): AxPath<i64>,
    Json(body): Json<TopicEvolutionBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let detail = body
        .detail_json
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let id = st
        .codex
        .topic_evolution_event_append(
            topic_id,
            &body.event_kind,
            body.prior_label.as_deref(),
            body.new_label.as_deref(),
            detail.as_deref(),
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "id": id, "topic_id": topic_id })))
}

async fn search_status(
    State(st): State<CodexApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let c = st.codex.as_ref();
    let documents = search_table_count(c, "SELECT COUNT(*) FROM search_documents").await?;
    let chunks = search_table_count(c, "SELECT COUNT(*) FROM search_document_chunks").await?;
    let jobs = search_table_count(c, "SELECT COUNT(*) FROM search_indexing_jobs").await?;
    Ok(Json(json!({
        "search_documents": documents,
        "search_document_chunks": chunks,
        "search_indexing_jobs": jobs,
    })))
}

/// `GET /api/audio/status` — Oratio capability line + Candle backend JSON.
async fn get_audio_status() -> Json<serde_json::Value> {
    Json(json!({
        "summary": vox_oratio::transcript_status(),
        "candle": vox_oratio::candle_backend_status_json(),
    }))
}

/// JSON body for `POST /api/audio/transcribe`.
#[derive(Debug, Deserialize)]
pub struct AudioTranscribeBody {
    /// Workspace-relative or absolute path to an audio or transcript file.
    pub path: String,
}

/// `POST /api/audio/transcribe` — run Oratio on `path` (resolved against `CodexApiState::audio_workspace_root`).
async fn post_audio_transcribe(
    State(st): State<CodexApiState>,
    Json(body): Json<AudioTranscribeBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let full = resolve_audio_path(&st.audio_workspace_root, &body.path)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let t =
        vox_oratio::transcribe_path(&full).map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(json!({
        "path": full.display().to_string(),
        "raw_text": t.raw_text,
        "refined_text": t.refined_text,
        "text": t.display_text(),
    })))
}

fn resolve_audio_path(root: &FsPath, raw: &str) -> Result<PathBuf, String> {
    let p = FsPath::new(raw);
    let full = if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    };
    if !full.exists() {
        return Err(format!("path not found: {}", full.display()));
    }
    Ok(full)
}

async fn query_named(
    State(st): State<CodexApiState>,
    AxPath(name): AxPath<String>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let mut rows = st
        .codex
        .store()
        .conn
        .query(
            "SELECT snapshot_json, digest, created_at FROM codex_query_snapshots WHERE query_name = ?1 ORDER BY created_at DESC LIMIT 1",
            turso::params![name.as_str()],
        )
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let row = rows
        .next()
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    match row {
        Some(r) => {
            let snapshot_json: String = r
                .get::<String>(0)
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let digest: String = r
                .get::<String>(1)
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            let created_at: String = r
                .get::<String>(2)
                .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            Ok(Json(json!({
                "query_name": name,
                "snapshot_json": serde_json::from_str::<serde_json::Value>(&snapshot_json).unwrap_or(json!({})),
                "digest": digest,
                "created_at": created_at,
            })))
        }
        None => Err((
            axum::http::StatusCode::NOT_FOUND,
            format!("no snapshot for query {name}"),
        )),
    }
}

/// JSON body for `POST /api/codex/mutate/:name` (append a change-log row).
#[derive(Debug, Deserialize)]
pub struct MutateBody {
    /// Short verb, e.g. `insert`, `update`, `delete`.
    pub change_kind: String,
    /// Optional entity type for UI grouping.
    pub entity_kind: Option<String>,
    /// Optional stable entity id within `entity_kind`.
    pub entity_id: Option<String>,
    /// Arbitrary JSON payload stored as `payload_json`.
    pub payload: Option<serde_json::Value>,
}

async fn mutate_named(
    State(st): State<CodexApiState>,
    AxPath(name): AxPath<String>,
    Json(body): Json<MutateBody>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    let payload_json = body
        .payload
        .as_ref()
        .map(|p| serde_json::to_string(p).unwrap_or_else(|_| "{}".into()));
    let id = st
        .codex
        .append_codex_change(
            name.as_str(),
            body.entity_kind.as_deref(),
            body.entity_id.as_deref(),
            body.change_kind.as_str(),
            payload_json.as_deref(),
        )
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(json!({ "change_log_id": id, "topic": name })))
}

fn sse_stream(
    st: CodexApiState,
    topic: String,
    start_after: i64,
) -> impl Stream<Item = Result<Event, Infallible>> {
    let cursor = Arc::new(AtomicI64::new(start_after));
    async_stream::stream! {
        loop {
            tokio::time::sleep(Duration::from_millis(400)).await;
            let after_id = cursor.load(Ordering::SeqCst);
            let batch = match st.codex.list_codex_changes_since(Some(topic.as_str()), after_id, 256).await {
                Ok(b) => b,
                Err(e) => {
                    yield Ok(Event::default().data(format!("{{\"error\":\"{e}\"}}")));
                    continue;
                }
            };
            if batch.is_empty() {
                yield Ok(Event::default().comment("keepalive"));
                continue;
            }
            for row in batch {
                cursor.store(row.id, Ordering::SeqCst);
                let payload = json!({
                    "id": row.id,
                    "topic": row.topic,
                    "entity_kind": row.entity_kind,
                    "entity_id": row.entity_id,
                    "change_kind": row.change_kind,
                    "payload_json": row.payload_json,
                    "created_at": row.created_at,
                });
                let data = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".into());
                yield Ok(Event::default().data(data));
            }
        }
    }
}

async fn subscribe_topic(
    State(st): State<CodexApiState>,
    AxPath(topic): AxPath<String>,
    Query(q): Query<AfterQuery>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let after_id = q.after_id.unwrap_or(0);
    let stream = sse_stream(st, topic, after_id);
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Run a small dashboard + Codex API on **`VOX_DASH_HOST`:`VOX_DASH_PORT`** (default **127.0.0.1:3847**).
pub async fn run_dashboard() -> anyhow::Result<()> {
    let port: u16 = std::env::var("VOX_DASH_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3847);
    let host = std::env::var("VOX_DASH_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let ip: std::net::IpAddr = host
        .parse()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
    let addr = std::net::SocketAddr::new(ip, port);
    tracing::info!("starting Codex dashboard on http://{addr}");
    let cfg = DbConfig::resolve_standalone().map_err(|e| anyhow::anyhow!(e))?;
    let codex = Arc::new(
        Codex::connect(cfg)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?,
    );
    let audio_workspace_root = std::env::var("VOX_ORATIO_WORKSPACE")
        .map(PathBuf::from)
        .or_else(|_| std::env::current_dir())
        .unwrap_or_else(|_| PathBuf::from("."));
    let repo_hint = std::env::current_dir().unwrap_or_else(|_| audio_workspace_root.clone());
    let default_repository_id =
        vox_repository::discover_repository_or_fallback(&repo_hint).repository_id;

    let state = CodexApiState {
        codex: codex.clone(),
        audio_workspace_root,
        default_repository_id,
    };

    let api = codex_router(state);
    let app = Router::new()
        .route(
            "/",
            get(|| async {
                axum::response::Html(
                    "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>Codex</title></head>\
                     <body><h1>Codex</h1>\
                     <ul>\
                     <li><code>GET /api/codex/query/:name</code></li>\
                     <li><code>POST /api/codex/mutate/:name</code></li>\
                     <li><code>GET /api/codex/subscribe/:topic?after_id=</code> (SSE)</li>\
                     <li><code>GET /health</code> — process liveness</li>\
                     <li><code>GET /ready</code> — baseline V1 + required tables + digest</li>\
                     <li><code>GET /api/search/status</code> — search table row counts</li>\
                     <li><code>POST /api/codex/research-session</code> — upsert <code>research_sessions</code></li>\
                     <li><code>POST /api/codex/conversations/:id/versions</code></li>\
                     <li><code>POST /api/codex/conversation-edges</code></li>\
                     <li><code>POST /api/codex/topics/:id/evolution-events</code></li>\
                     <li><code>GET /api/audio/status</code> (Oratio / Candle Whisper)</li>\
                     <li><code>POST /api/audio/transcribe</code> JSON <code>{{\"path\":\"…\"}}</code></li>\
                     </ul></body></html>",
                )
            }),
        )
        .merge(api)
        .layer(tower_http::cors::CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
