//! Minimal HTTP control plane for populi join / list / heartbeat / leave (loopback-first).
//!
//! When **`VOX_MESH_TOKEN`** is set and non-empty, all routes except **`GET /health`** require
//! `Authorization: Bearer <token>` (value is never logged). Bearer comparison uses
//! [`subtle::ConstantTimeEq`] on UTF-8 bytes when lengths match.
//!
//! When **`VOX_MESH_SCOPE_ID`** is set on the server process, **`POST /v1/populi/join`** and
//! **`POST /v1/populi/heartbeat`** require the JSON [`crate::NodeRecord::scope_id`] to match.

use std::net::SocketAddr;
use std::sync::Arc;

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
use tracing::info;

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

/// Shared registry state for the HTTP server (in-memory; optionally persisted by callers).
#[derive(Clone)]
pub struct MeshTransportState {
    inner: Arc<RwLock<PopuliRegistryFile>>,
    /// When set, join/heartbeat must send the same [`NodeRecord::scope_id`].
    pub required_scope: Option<Arc<str>>,
}

impl MeshTransportState {
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
            required_scope,
        }
    }

    /// Same as [`Self::new`] but sets [`Self::required_scope`] from **`VOX_MESH_SCOPE_ID`** when set.
    #[must_use]
    pub fn new_for_serve() -> Self {
        Self::with_required_scope(crate::mesh_scope_id_from_env())
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
        Ok(Self {
            inner: Arc::new(RwLock::new(reg)),
            required_scope: crate::mesh_scope_id_from_env().map(|s| Arc::from(s.into_boxed_str())),
        })
    }
}

impl Default for MeshTransportState {
    fn default() -> Self {
        Self::new()
    }
}

fn scope_ok(state: &MeshTransportState, node: &NodeRecord) -> bool {
    match &state.required_scope {
        None => true,
        Some(req) => node.scope_id.as_deref().is_some_and(|s| s == req.as_ref()),
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

async fn list_nodes(State(st): State<MeshTransportState>) -> Json<PopuliRegistryFile> {
    let g = st.inner.read().await;
    Json(g.clone())
}

async fn join_node(
    State(st): State<MeshTransportState>,
    Json(mut node): Json<NodeRecord>,
) -> Result<Json<NodeRecord>, ResponseErr> {
    if !scope_ok(&st, &node) {
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
    State(st): State<MeshTransportState>,
    Json(mut node): Json<NodeRecord>,
) -> Result<Json<NodeRecord>, ResponseErr> {
    if !scope_ok(&st, &node) {
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
    State(st): State<MeshTransportState>,
    Json(req): Json<LeaveRequest>,
) -> StatusCode {
    let mut g = st.inner.write().await;
    let before = g.nodes.len();
    g.nodes.retain(|n| n.id != req.id);
    if g.nodes.len() < before {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn deliver_a2a(
    State(_st): State<MeshTransportState>,
    Json(_req): Json<A2ADeliverRequest>,
) -> StatusCode {
    // This route is a stub in the control plane itself;
    // real delivery happens in the local node's proxy or orchestrator.
    StatusCode::ACCEPTED
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

/// Bearer authentication mode for [`mesh_http_app_with_auth`].
#[derive(Clone, Debug)]
pub enum MeshHttpAuth {
    /// Read `VOX_MESH_TOKEN` once when building the router (used by [`mesh_http_app`] / [`serve`]).
    FromEnv,
    /// No bearer check (e.g. integration tests; explicit open control plane).
    Open,
    /// Require this bearer value; **ignores** the environment (tests or embedded callers).
    Bearer(String),
}

/// Inner control-plane router (no auth layer). Prefer [`mesh_http_app`] for serving.
pub fn router(state: MeshTransportState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/populi/nodes", get(list_nodes))
        .route("/v1/populi/join", post(join_node))
        .route("/v1/populi/heartbeat", post(heartbeat))
        .route("/v1/populi/leave", post(leave_node))
        .route("/v1/populi/a2a/deliver", post(deliver_a2a))
        .with_state(state)
}

/// Same as [`mesh_http_app`] but with an explicit auth mode (avoids process-global env in tests).
///
/// The expected bearer value is **captured at build time** (not re-read on every request).
pub fn mesh_http_app_with_auth(state: MeshTransportState, auth: MeshHttpAuth) -> Router {
    let r = router(state);
    let expected: Option<Arc<str>> = match auth {
        MeshHttpAuth::FromEnv => populi_control_token_from_env().map(Arc::from),
        MeshHttpAuth::Open => None,
        MeshHttpAuth::Bearer(t) => {
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
pub fn mesh_http_app(state: MeshTransportState) -> Router {
    mesh_http_app_with_auth(state, MeshHttpAuth::FromEnv)
}

/// Bind and serve until error (Ctrl+C stops the process).
pub async fn serve(addr: SocketAddr, state: MeshTransportState) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "vox-populi HTTP control plane listening");
    let app = mesh_http_app(state);
    axum::serve(listener, app).await
}
