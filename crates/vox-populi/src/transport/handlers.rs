use std::sync::atomic::Ordering;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use tracing::{info, warn};

use crate::NodeRecord;

use super::auth::populi_control_token_from_env;
use super::{
    A2AAckRequest, A2ADeliverRequest, A2ADeliverResponse, A2AInboxRequest, A2AInboxResponse,
    A2AStoredMessage, BootstrapExchangeRequest, BootstrapExchangeResponse, LeaveRequest,
    PopuliRegistryFile, PopuliTransportState,
};
use super::store::{persist_a2a_store, scope_ok};

pub(super) async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

pub(super) async fn list_nodes(State(st): State<PopuliTransportState>) -> Json<PopuliRegistryFile> {
    let g = st.inner.read().await;
    Json(g.clone())
}

pub(super) async fn join_node(
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

pub(super) async fn heartbeat(
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

pub(super) struct ResponseErr(StatusCode, &'static str);

impl IntoResponse for ResponseErr {
    fn into_response(self) -> axum::response::Response {
        (self.0, self.1).into_response()
    }
}

pub(super) async fn leave_node(
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

pub(super) async fn bootstrap_exchange(
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
    if !super::auth::bearer_token_eq(expected.as_ref(), req.bootstrap_token.trim()) {
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

pub(super) async fn deliver_a2a(
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

pub(super) async fn a2a_inbox(
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

pub(super) async fn a2a_ack(
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
