//! Minimal HTTP control plane for populi join / list / heartbeat / leave (loopback-first).
//!
//! When **`VOX_MESH_TOKEN`** is set and non-empty, all routes except **`GET /health`** require
//! `Authorization: Bearer <token>` (value is never logged). Bearer comparison uses
//! [`subtle::ConstantTimeEq`] on UTF-8 bytes when lengths match.
//!
//! When **`VOX_MESH_SCOPE_ID`** is set on the server process, **`POST /v1/populi/join`** and
//! **`POST /v1/populi/heartbeat`** require the JSON [`crate::NodeRecord::scope_id`] to match.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tokio::sync::RwLock;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use crate::{NodeRecord, PopuliRegistryError, PopuliRegistryFile};

/// Body for [`leave_node`]: remove a node id from the in-memory registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveRequest {
    /// Node id to remove (same as [`NodeRecord::id`]).
    pub id: String,
}

/// Request to deliver an A2A message to a local agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ADeliverRequest {
    /// Sender agent ID.
    pub sender_agent_id: String,
    /// Receiver agent ID.
    pub receiver_agent_id: String,
    /// The message type/schema name.
    pub message_type: String,
    /// The JSON or raw payload.
    pub payload: String,
}

/// Persisted A2A delivery envelope in the control plane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AStoredMessage {
    /// Monotonic message row id.
    pub id: u64,
    /// Sender agent ID.
    pub sender_agent_id: String,
    /// Receiver agent ID.
    pub receiver_agent_id: String,
    /// Message type / schema name.
    pub message_type: String,
    /// JSON or raw payload.
    pub payload: String,
    /// Control-plane wall time when stored (unix ms).
    pub created_unix_ms: u64,
    /// Whether the receiver has acked delivery.
    pub acknowledged: bool,
}

/// Reply from the control plane after an A2A deliver attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ADeliverResponse {
    /// Whether the message was accepted for storage/delivery.
    pub accepted: bool,
    /// Assigned [`A2AStoredMessage::id`] when accepted.
    pub message_id: u64,
}

/// Inbox poll: identify the receiving agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AInboxRequest {
    /// Receiver agent ID.
    pub receiver_agent_id: String,
}

/// Inbox poll: queued messages for the receiver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AInboxResponse {
    /// Pending messages (order is transport-defined).
    pub messages: Vec<A2AStoredMessage>,
}

/// Ack a delivered inbox message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AAckRequest {
    /// Receiver agent ID.
    pub receiver_agent_id: String,
    /// [`A2AStoredMessage::id`] to acknowledge.
    pub message_id: u64,
}

/// Request a one-time bootstrap exchange for mesh join.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapExchangeRequest {
    /// Ephemeral bootstrap token provisioned by `vox populi up`.
    pub bootstrap_token: String,
}

/// Response payload for bootstrap exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapExchangeResponse {
    /// Long-lived mesh bearer token (same as `VOX_MESH_TOKEN`).
    pub mesh_token: String,
    /// Optional scope id to join.
    pub scope_id: Option<String>,
}

/// Shared registry state for the HTTP server (in-memory; optionally persisted by callers).
#[derive(Clone)]
pub struct PopuliTransportState {
    inner: Arc<RwLock<PopuliRegistryFile>>,
    a2a_messages: Arc<RwLock<Vec<A2AStoredMessage>>>,
    a2a_id_gen: Arc<AtomicU64>,
    a2a_store_path: Option<PathBuf>,
    bootstrap_token: Option<Arc<str>>,
    bootstrap_expires_unix_ms: Option<u64>,
    bootstrap_used: Arc<AtomicBool>,
    /// When set, join/heartbeat must send the same [`NodeRecord::scope_id`].
    pub required_scope: Option<Arc<str>>,
}

impl PopuliTransportState {
    /// New empty in-memory registry; does **not** read `VOX_MESH_SCOPE_ID` (for tests).
    #[must_use]
    pub fn new() -> Self {
        Self::with_required_scope(None)
    }

    /// New empty registry and optional required scope (trimmed; empty string becomes `None`).
    #[must_use]
    pub fn with_required_scope(scope: Option<String>) -> Self {
        let required_scope = scope
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|s| Arc::from(s.into_boxed_str()));
        Self {
            inner: Arc::new(RwLock::new(PopuliRegistryFile {
                schema_version: 1,
                nodes: Vec::new(),
            })),
            a2a_messages: Arc::new(RwLock::new(Vec::new())),
            a2a_id_gen: Arc::new(AtomicU64::new(1)),
            a2a_store_path: None,
            bootstrap_token: None,
            bootstrap_expires_unix_ms: None,
            bootstrap_used: Arc::new(AtomicBool::new(false)),
            required_scope,
        }
    }

    /// Same as [`Self::new`] but sets [`Self::required_scope`] from **`VOX_MESH_SCOPE_ID`** when set.
    #[must_use]
    pub fn new_for_serve() -> Self {
        let mut s = Self::with_required_scope(crate::populi_scope_id_from_env());
        let store_path = a2a_store_path_from_env();
        if let Some(path) = &store_path
            && let Ok(existing) = load_a2a_store(path)
        {
            let next_id = existing
                .iter()
                .map(|m| m.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1);
            s.a2a_messages = Arc::new(RwLock::new(existing));
            s.a2a_id_gen = Arc::new(AtomicU64::new(next_id));
        }
        s.a2a_store_path = store_path;
        s.bootstrap_token = std::env::var("VOX_MESH_BOOTSTRAP_TOKEN")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .map(Arc::from);
        s.bootstrap_expires_unix_ms = std::env::var("VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS")
            .ok()
            .and_then(|v| v.trim().parse::<u64>().ok())
            .filter(|ms| *ms > crate::now_ms());
        s
    }

    /// Load initial snapshot from disk (best-effort) and apply scope from **`VOX_MESH_SCOPE_ID`**.
    pub async fn load_from_path(path: &std::path::Path) -> Result<Self, PopuliRegistryError> {
        let reg = if path.is_file() {
            let raw = std::fs::read_to_string(path).map_err(PopuliRegistryError::Io)?;
            serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))?
        } else {
            PopuliRegistryFile {
                schema_version: 1,
                nodes: Vec::new(),
            }
        };
        let store_path = a2a_store_path_from_env();
        let rows = if let Some(sp) = &store_path {
            load_a2a_store(sp).unwrap_or_default()
        } else {
            Vec::new()
        };
        let next_id = rows
            .iter()
            .map(|m| m.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        Ok(Self {
            inner: Arc::new(RwLock::new(reg)),
            a2a_messages: Arc::new(RwLock::new(rows)),
            a2a_id_gen: Arc::new(AtomicU64::new(next_id)),
            a2a_store_path: store_path,
            bootstrap_token: None,
            bootstrap_expires_unix_ms: None,
            bootstrap_used: Arc::new(AtomicBool::new(false)),
            required_scope: crate::populi_scope_id_from_env().map(|s| Arc::from(s.into_boxed_str())),
        })
    }
}

fn a2a_store_path_from_env() -> Option<PathBuf> {
    if let Ok(v) = std::env::var("VOX_MESH_A2A_STORE_PATH") {
        let trimmed = v.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    let mut p = crate::local_registry_path();
    p.set_file_name("a2a-store.json");
    Some(p)
}

fn load_a2a_store(path: &std::path::Path) -> Result<Vec<A2AStoredMessage>, PopuliRegistryError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(path).map_err(PopuliRegistryError::Io)?;
    serde_json::from_str(&raw).map_err(|e| PopuliRegistryError::Json(e.to_string()))
}

fn persist_a2a_store(
    path: &std::path::Path,
    rows: &[A2AStoredMessage],
) -> Result<(), PopuliRegistryError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(PopuliRegistryError::Io)?;
    }
    let payload =
        serde_json::to_string_pretty(rows).map_err(|e| PopuliRegistryError::Json(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, payload.as_bytes()).map_err(PopuliRegistryError::Io)?;
    std::fs::rename(&tmp, path).map_err(PopuliRegistryError::Io)?;
    Ok(())
}

impl Default for PopuliTransportState {
    fn default() -> Self {
        Self::new()
    }
}

fn scope_ok(state: &PopuliTransportState, node: &NodeRecord) -> bool {
    match &state.required_scope {
        None => true,
        Some(req) => node.scope_id.as_deref().is_some_and(|s| s == req.as_ref()),
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

async fn list_nodes(State(st): State<PopuliTransportState>) -> Json<PopuliRegistryFile> {
    let g = st.inner.read().await;
    Json(g.clone())
}

async fn join_node(
    State(st): State<PopuliTransportState>,
    Json(mut node): Json<NodeRecord>,
) -> Result<Json<NodeRecord>, ResponseErr> {
    if !scope_ok(&st, &node) {
        warn!(node_id = %node.id, "join rejected: populi scope mismatch");
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi scope mismatch: set VOX_MESH_SCOPE_ID to match server",
        ));
    }
    node.last_seen_unix_ms = crate::now_ms();
    let mut g = st.inner.write().await;
    if let Some(i) = g.nodes.iter().position(|n| n.id == node.id) {
        g.nodes[i] = node.clone();
    } else {
        g.nodes.push(node.clone());
    }
    Ok(Json(node))
}

async fn heartbeat(
    State(st): State<PopuliTransportState>,
    Json(mut node): Json<NodeRecord>,
) -> Result<Json<NodeRecord>, ResponseErr> {
    if !scope_ok(&st, &node) {
        warn!(node_id = %node.id, "heartbeat rejected: populi scope mismatch");
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi scope mismatch: set VOX_MESH_SCOPE_ID to match server",
        ));
    }
    node.last_seen_unix_ms = crate::now_ms();
    let mut g = st.inner.write().await;
    if let Some(i) = g.nodes.iter().position(|n| n.id == node.id) {
        g.nodes[i].last_seen_unix_ms = node.last_seen_unix_ms;
        if node.listen_addr.is_some() {
            g.nodes[i].listen_addr = node.listen_addr.clone();
        }
        if node.scope_id.is_some() {
            g.nodes[i].scope_id = node.scope_id.clone();
        }
        Ok(Json(g.nodes[i].clone()))
    } else {
        g.nodes.push(node.clone());
        Ok(Json(node))
    }
}

struct ResponseErr(StatusCode, &'static str);

impl IntoResponse for ResponseErr {
    fn into_response(self) -> axum::response::Response {
        (self.0, self.1).into_response()
    }
}

async fn leave_node(
    State(st): State<PopuliTransportState>,
    Json(req): Json<LeaveRequest>,
) -> StatusCode {
    let mut g = st.inner.write().await;
    let before = g.nodes.len();
    g.nodes.retain(|n| n.id != req.id);
    if g.nodes.len() < before {
        StatusCode::NO_CONTENT
    } else {
        warn!(node_id = %req.id, "leave requested for unknown node");
        StatusCode::NOT_FOUND
    }
}

async fn bootstrap_exchange(
    State(st): State<PopuliTransportState>,
    Json(req): Json<BootstrapExchangeRequest>,
) -> Result<Json<BootstrapExchangeResponse>, ResponseErr> {
    let Some(expected) = st.bootstrap_token.as_ref() else {
        return Err(ResponseErr(
            StatusCode::NOT_FOUND,
            "bootstrap exchange is not enabled",
        ));
    };
    if st.bootstrap_used.swap(true, Ordering::SeqCst) {
        warn!("bootstrap exchange rejected: token already used");
        return Err(ResponseErr(
            StatusCode::GONE,
            "bootstrap token already consumed",
        ));
    }
    if let Some(expires) = st.bootstrap_expires_unix_ms
        && crate::now_ms() > expires
    {
        warn!("bootstrap exchange rejected: token expired");
        return Err(ResponseErr(StatusCode::GONE, "bootstrap token expired"));
    }
    if !bearer_token_eq(expected.as_ref(), req.bootstrap_token.trim()) {
        warn!("bootstrap exchange rejected: invalid token");
        return Err(ResponseErr(
            StatusCode::UNAUTHORIZED,
            "invalid bootstrap token",
        ));
    }
    let mesh_token = populi_control_token_from_env().ok_or(ResponseErr(
        StatusCode::SERVICE_UNAVAILABLE,
        "server missing VOX_MESH_TOKEN",
    ))?;
    info!("bootstrap exchange granted");
    Ok(Json(BootstrapExchangeResponse {
        mesh_token,
        scope_id: crate::populi_scope_id_from_env(),
    }))
}

async fn deliver_a2a(
    State(st): State<PopuliTransportState>,
    Json(req): Json<A2ADeliverRequest>,
) -> Json<A2ADeliverResponse> {
    let id = st.a2a_id_gen.fetch_add(1, Ordering::Relaxed);
    let msg = A2AStoredMessage {
        id,
        sender_agent_id: req.sender_agent_id,
        receiver_agent_id: req.receiver_agent_id,
        message_type: req.message_type,
        payload: req.payload,
        created_unix_ms: crate::now_ms(),
        acknowledged: false,
    };
    let mut g = st.a2a_messages.write().await;
    g.push(msg);
    if let Some(path) = st.a2a_store_path.as_ref() {
        let _ = persist_a2a_store(path, &g);
    }
    Json(A2ADeliverResponse {
        accepted: true,
        message_id: id,
    })
}

async fn a2a_inbox(
    State(st): State<PopuliTransportState>,
    Json(req): Json<A2AInboxRequest>,
) -> Json<A2AInboxResponse> {
    let g = st.a2a_messages.read().await;
    let messages = g
        .iter()
        .filter(|m| m.receiver_agent_id == req.receiver_agent_id && !m.acknowledged)
        .cloned()
        .collect();
    Json(A2AInboxResponse { messages })
}

async fn a2a_ack(
    State(st): State<PopuliTransportState>,
    Json(req): Json<A2AAckRequest>,
) -> StatusCode {
    let mut g = st.a2a_messages.write().await;
    if let Some(msg) = g
        .iter_mut()
        .find(|m| m.id == req.message_id && m.receiver_agent_id == req.receiver_agent_id)
    {
        msg.acknowledged = true;
        if let Some(path) = st.a2a_store_path.as_ref() {
            let _ = persist_a2a_store(path, &g);
        }
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

fn populi_control_token_from_env() -> Option<String> {
    std::env::var("VOX_MESH_TOKEN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Constant-time comparison when lengths match (avoids early return on length for the equal-length case).
fn bearer_token_eq(expected: &str, presented: &str) -> bool {
    let a = expected.as_bytes();
    let b = presented.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

/// Bearer authentication mode for [`populi_http_app_with_auth`].
#[derive(Clone, Debug)]
pub enum PopuliHttpAuth {
    /// Read `VOX_MESH_TOKEN` once when building the router (used by [`populi_http_app`] / [`serve`]).
    FromEnv,
    /// No bearer check (e.g. integration tests; explicit open control plane).
    Open,
    /// Require this bearer value; **ignores** the environment (tests or embedded callers).
    Bearer(String),
}

/// Inner control-plane router (no auth layer). Prefer [`populi_http_app`] for serving.
pub fn router(state: PopuliTransportState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/populi/nodes", get(list_nodes))
        .route("/v1/populi/join", post(join_node))
        .route("/v1/populi/heartbeat", post(heartbeat))
        .route("/v1/populi/leave", post(leave_node))
        .route("/v1/populi/bootstrap/exchange", post(bootstrap_exchange))
        .route("/v1/populi/a2a/deliver", post(deliver_a2a))
        .route("/v1/populi/a2a/inbox", post(a2a_inbox))
        .route("/v1/populi/a2a/ack", post(a2a_ack))
        .with_state(state)
}

/// Same as [`populi_http_app`] but with an explicit auth mode (avoids process-global env in tests).
///
/// The expected bearer value is **captured at build time** (not re-read on every request).
pub fn populi_http_app_with_auth(state: PopuliTransportState, auth: PopuliHttpAuth) -> Router {
    let r = router(state);
    let expected: Option<Arc<str>> = match auth {
        PopuliHttpAuth::FromEnv => populi_control_token_from_env().map(Arc::from),
        PopuliHttpAuth::Open => None,
        PopuliHttpAuth::Bearer(t) => {
            let t = t.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(Arc::from(t))
            }
        }
    };
    let r = if let Some(expected) = expected {
        r.layer(middleware::from_fn(
            move |req: Request<Body>, next: Next| {
                let expected = Arc::clone(&expected);
                async move {
                    if req.uri().path() == "/health" {
                        return next.run(req).await;
                    }
                    let ok = req
                        .headers()
                        .get(header::AUTHORIZATION)
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.strip_prefix("Bearer "))
                        .is_some_and(|t| bearer_token_eq(expected.as_ref(), t));
                    if !ok {
                        warn!(path = %req.uri().path(), "populi bearer auth rejected request");
                        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
                    }
                    next.run(req).await
                }
            },
        ))
    } else {
        r
    };

    r.layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
}

/// Full app: same routes as [`router`], plus optional `VOX_MESH_TOKEN` bearer check (except `/health`).
pub fn populi_http_app(state: PopuliTransportState) -> Router {
    populi_http_app_with_auth(state, PopuliHttpAuth::FromEnv)
}

/// Bind and serve until error (Ctrl+C stops the process).
pub async fn serve(addr: SocketAddr, state: PopuliTransportState) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "vox-populi HTTP control plane listening");
    let app = populi_http_app(state);
    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn populi_routes_exist_and_legacy_mens_routes_are_absent() {
        let app = router(PopuliTransportState::new());
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let client = reqwest::Client::new();
        let ok = client
            .get(format!("http://{addr}/v1/populi/nodes"))
            .send()
            .await
            .expect("GET populi nodes");
        assert_eq!(ok.status(), StatusCode::OK);

        let missing = client
            .get(format!("http://{addr}/v1/mens/nodes"))
            .send()
            .await
            .expect("GET legacy mens nodes");
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);

        server.abort();
    }
}
