use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use tracing::{info, warn};

use crate::NodeRecord;

use super::auth::{
    PopuliAuthContext, auth_allows_admin_route, auth_allows_deliver, auth_allows_worker_plane,
    populi_control_token_from_env,
};
use super::result_attestation;
use super::store::{persist_a2a_store, scope_ok};
use super::{
    A2AAckRequest, A2ADeliverRequest, A2ADeliverResponse, A2AInboxRequest, A2AInboxResponse,
    A2ALeaseRenewRequest, A2AStoredMessage, AdminQuarantineRequest, BootstrapExchangeRequest,
    BootstrapExchangeResponse, LeaveRequest, PopuliRegistryFile, PopuliTransportState,
    a2a_in_memory_cap, a2a_lease_duration_ms, a2a_sweep_expired_leases, server_stale_prune_ms,
};

pub(super) async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

pub(super) async fn list_nodes(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
) -> Result<Json<PopuliRegistryFile>, ResponseErr> {
    if !auth_allows_worker_plane(ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for node list".into(),
        ));
    }
    let mut g = st.inner.read().await.clone();
    if let Some(window) = server_stale_prune_ms() {
        let now = crate::now_ms();
        g.nodes
            .retain(|n| now.saturating_sub(n.last_seen_unix_ms) <= window);
    }
    Ok(Json(g))
}

pub(super) async fn join_node(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(mut node): Json<NodeRecord>,
) -> Result<Json<NodeRecord>, ResponseErr> {
    if !auth_allows_worker_plane(ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for join".into(),
        ));
    }
    if !scope_ok(&st, &node) {
        warn!(node_id = %node.id, "join rejected: populi scope mismatch");
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi scope mismatch: set VOX_MESH_SCOPE_ID to match server".into(),
        ));
    }
    node.quarantined = None;
    node.last_seen_unix_ms = crate::now_ms();
    let mut g = st.inner.write().await;
    if let Some(i) = g.nodes.iter().position(|n| n.id == node.id) {
        let preserve_q = g.nodes[i].quarantined;
        g.nodes[i] = node.clone();
        g.nodes[i].quarantined = preserve_q;
    } else {
        g.nodes.push(node.clone());
    }
    let out = g
        .nodes
        .iter()
        .find(|n| n.id == node.id)
        .cloned()
        .expect("join upsert must leave node in registry");
    Ok(Json(out))
}

fn merge_optional_node_fields(target: &mut NodeRecord, src: &NodeRecord) {
    if src.listen_addr.is_some() {
        target.listen_addr = src.listen_addr.clone();
    }
    if src.scope_id.is_some() {
        target.scope_id = src.scope_id.clone();
    }
    if src.visibility.is_some() {
        target.visibility = src.visibility.clone();
    }
    if src.pool_id.is_some() {
        target.pool_id = src.pool_id.clone();
    }
    if src.trust_tier.is_some() {
        target.trust_tier = src.trust_tier.clone();
    }
    if src.workload_classes.is_some() {
        target.workload_classes = src.workload_classes.clone();
    }
    if src.privacy_class.is_some() {
        target.privacy_class = src.privacy_class.clone();
    }
    if src.maintenance.is_some() {
        target.maintenance = src.maintenance;
    }
    if src.provider.is_some() {
        target.provider = src.provider.clone();
    }
}

pub(super) async fn heartbeat(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(mut node): Json<NodeRecord>,
) -> Result<Json<NodeRecord>, ResponseErr> {
    if !auth_allows_worker_plane(ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for heartbeat".into(),
        ));
    }
    if !scope_ok(&st, &node) {
        warn!(node_id = %node.id, "heartbeat rejected: populi scope mismatch");
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi scope mismatch: set VOX_MESH_SCOPE_ID to match server".into(),
        ));
    }
    node.quarantined = None;
    node.last_seen_unix_ms = crate::now_ms();
    let mut g = st.inner.write().await;
    if let Some(i) = g.nodes.iter().position(|n| n.id == node.id) {
        let preserve_q = g.nodes[i].quarantined;
        g.nodes[i].last_seen_unix_ms = node.last_seen_unix_ms;
        merge_optional_node_fields(&mut g.nodes[i], &node);
        g.nodes[i].quarantined = preserve_q;
        Ok(Json(g.nodes[i].clone()))
    } else {
        g.nodes.push(node.clone());
        Ok(Json(node))
    }
}

pub(super) struct ResponseErr(StatusCode, String);

impl IntoResponse for ResponseErr {
    fn into_response(self) -> axum::response::Response {
        (self.0, self.1).into_response()
    }
}

pub(super) async fn leave_node(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<LeaveRequest>,
) -> Result<StatusCode, ResponseErr> {
    if !auth_allows_worker_plane(ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for leave".into(),
        ));
    }
    let mut g = st.inner.write().await;
    let before = g.nodes.len();
    g.nodes.retain(|n| n.id != req.id);
    if g.nodes.len() < before {
        Ok(StatusCode::NO_CONTENT)
    } else {
        warn!(node_id = %req.id, "leave requested for unknown node");
        Ok(StatusCode::NOT_FOUND)
    }
}

pub(super) async fn bootstrap_exchange(
    State(st): State<PopuliTransportState>,
    Json(req): Json<BootstrapExchangeRequest>,
) -> Result<Json<BootstrapExchangeResponse>, ResponseErr> {
    let Some(expected) = st.bootstrap_token.as_ref() else {
        return Err(ResponseErr(
            StatusCode::NOT_FOUND,
            "bootstrap exchange is not enabled".into(),
        ));
    };
    if st.bootstrap_used.swap(true, Ordering::SeqCst) {
        warn!("bootstrap exchange rejected: token already used");
        return Err(ResponseErr(
            StatusCode::GONE,
            "bootstrap token already consumed".into(),
        ));
    }
    if let Some(expires) = st.bootstrap_expires_unix_ms
        && crate::now_ms() > expires
    {
        warn!("bootstrap exchange rejected: token expired");
        return Err(ResponseErr(
            StatusCode::GONE,
            "bootstrap token expired".into(),
        ));
    }
    if !super::auth::bearer_token_eq(expected.as_ref(), req.bootstrap_token.trim()) {
        warn!("bootstrap exchange rejected: invalid token");
        return Err(ResponseErr(
            StatusCode::UNAUTHORIZED,
            "invalid bootstrap token".into(),
        ));
    }
    let mesh_token = populi_control_token_from_env().ok_or_else(|| {
        ResponseErr(
            StatusCode::SERVICE_UNAVAILABLE,
            "server missing VOX_MESH_TOKEN".into(),
        )
    })?;
    info!("bootstrap exchange granted");
    Ok(Json(BootstrapExchangeResponse {
        mesh_token,
        scope_id: crate::populi_scope_id_from_env(),
    }))
}

fn claim_policy_allows_worker(worker: &NodeRecord, msg: &A2AStoredMessage) -> bool {
    let privacy = msg
        .privacy_class
        .as_deref()
        .unwrap_or("public")
        .trim()
        .to_ascii_lowercase();
    if privacy.is_empty() || privacy == "public" {
        return true;
    }
    let vis = worker
        .visibility
        .as_deref()
        .unwrap_or("private")
        .trim()
        .to_ascii_lowercase();
    let trust = worker
        .trust_tier
        .as_deref()
        .unwrap_or("trusted")
        .trim()
        .to_ascii_lowercase();
    match privacy.as_str() {
        "private" | "trusted" => vis != "public",
        "trusted_only" => vis != "public" && trust != "new",
        _ => true,
    }
}

pub(super) async fn deliver_a2a(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<A2ADeliverRequest>,
) -> Result<Json<A2ADeliverResponse>, ResponseErr> {
    if !auth_allows_deliver(ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: submitter/mesh/admin token required for a2a/deliver".into(),
        ));
    }
    let dh = req
        .payload_blake3_hex
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let sb = req
        .worker_ed25519_sig_b64
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    if let Err(msg) = result_attestation::enforce_deliver_attestation(
        &req.message_type,
        &req.payload,
        dh,
        sb,
        st.worker_result_verify_key.as_ref(),
    ) {
        let status = if msg.contains("not configured") {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::BAD_REQUEST
        };
        return Err(ResponseErr(status, msg));
    }
    if let Some(key) = req
        .idempotency_key
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let dedupe = format!(
            "{}\x1f{}\x1f{}",
            req.sender_agent_id, req.receiver_agent_id, key
        );
        let mut maps = st.mesh_replay.maps().write().await;
        if let Some(&existing) = maps.idempotency.get(&dedupe) {
            return Ok(Json(A2ADeliverResponse {
                accepted: true,
                message_id: existing,
            }));
        }
        let id = st.a2a_id_gen.fetch_add(1, Ordering::Relaxed);
        maps.idempotency.insert(dedupe.clone(), id);
        drop(maps);
        st.mesh_replay.persist_if_configured().await;
        let msg = A2AStoredMessage {
            id,
            sender_agent_id: req.sender_agent_id,
            receiver_agent_id: req.receiver_agent_id,
            message_type: req.message_type,
            payload: req.payload,
            created_unix_ms: crate::now_ms(),
            acknowledged: false,
            lease_holder_node_id: None,
            lease_expires_unix_ms: None,
            privacy_class: req.privacy_class.clone(),
            idempotency_dedupe_key: Some(dedupe),
            payload_blake3_hex: req.payload_blake3_hex.clone(),
            worker_ed25519_sig_b64: req.worker_ed25519_sig_b64.clone(),
        };
        let mut g = st.a2a_messages.write().await;
        a2a_sweep_expired_leases(&mut g, crate::now_ms());
        let cap = a2a_in_memory_cap();
        if g.len() >= cap {
            let drop_n = g.len() - cap + 1;
            g.drain(0..drop_n);
        }
        g.push(msg);
        if let Some(path) = st.a2a_store_path.as_ref() {
            let _ = persist_a2a_store(path, &g);
        }
        return Ok(Json(A2ADeliverResponse {
            accepted: true,
            message_id: id,
        }));
    }

    let id = st.a2a_id_gen.fetch_add(1, Ordering::Relaxed);
    let msg = A2AStoredMessage {
        id,
        sender_agent_id: req.sender_agent_id,
        receiver_agent_id: req.receiver_agent_id,
        message_type: req.message_type,
        payload: req.payload,
        created_unix_ms: crate::now_ms(),
        acknowledged: false,
        lease_holder_node_id: None,
        lease_expires_unix_ms: None,
        privacy_class: req.privacy_class.clone(),
        idempotency_dedupe_key: None,
        payload_blake3_hex: req.payload_blake3_hex.clone(),
        worker_ed25519_sig_b64: req.worker_ed25519_sig_b64.clone(),
    };
    let mut g = st.a2a_messages.write().await;
    a2a_sweep_expired_leases(&mut g, crate::now_ms());
    let cap = a2a_in_memory_cap();
    if g.len() >= cap {
        let drop_n = g.len() - cap + 1;
        g.drain(0..drop_n);
    }
    g.push(msg);
    if let Some(path) = st.a2a_store_path.as_ref() {
        let _ = persist_a2a_store(path, &g);
    }
    Ok(Json(A2ADeliverResponse {
        accepted: true,
        message_id: id,
    }))
}

pub(super) async fn admin_quarantine(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<AdminQuarantineRequest>,
) -> Result<StatusCode, ResponseErr> {
    if !auth_allows_admin_route(ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: mesh/admin bearer required for quarantine".into(),
        ));
    }
    let id = req.node_id.trim();
    if id.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: node_id required".into(),
        ));
    }
    let mut g = st.inner.write().await;
    if let Some(i) = g.nodes.iter().position(|n| n.id == id) {
        g.nodes[i].quarantined = Some(req.quarantined);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

pub(super) async fn a2a_lease_renew(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<A2ALeaseRenewRequest>,
) -> Result<StatusCode, ResponseErr> {
    if !auth_allows_worker_plane(ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for lease renew".into(),
        ));
    }
    let now = crate::now_ms();
    let lease_ms = a2a_lease_duration_ms();
    let claimer = req.claimer_node_id.trim();
    if claimer.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: claimer_node_id required".into(),
        ));
    }
    let worker = {
        let reg = st.inner.read().await;
        reg.nodes.iter().find(|n| n.id == claimer).cloned()
    };
    let Some(worker) = worker else {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: unknown claimer_node_id".into(),
        ));
    };
    if worker.quarantined == Some(true) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: claimer node is quarantined".into(),
        ));
    }
    let mut g = st.a2a_messages.write().await;
    a2a_sweep_expired_leases(&mut g, now);
    let Some(msg) = g.iter_mut().find(|m| {
        m.id == req.message_id
            && m.receiver_agent_id == req.receiver_agent_id
            && !m.acknowledged
    }) else {
        return Ok(StatusCode::NOT_FOUND);
    };
    if msg.lease_holder_node_id.as_deref() != Some(claimer) {
        return Err(ResponseErr(
            StatusCode::CONFLICT,
            "populi: lease renew only for active lease holder".into(),
        ));
    }
    msg.lease_expires_unix_ms = Some(now.saturating_add(lease_ms));
    if let Some(path) = st.a2a_store_path.as_ref() {
        let _ = persist_a2a_store(path, &g);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn a2a_inbox(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<A2AInboxRequest>,
) -> Result<Json<A2AInboxResponse>, ResponseErr> {
    if !auth_allows_worker_plane(ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for a2a/inbox".into(),
        ));
    }
    let now = crate::now_ms();
    let lease_ms = a2a_lease_duration_ms();

    let claimer = req
        .claimer_node_id
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    if claimer.is_none() {
        let mut g = st.a2a_messages.write().await;
        a2a_sweep_expired_leases(&mut g, now);
        let messages = g
            .iter()
            .filter(|m| m.receiver_agent_id == req.receiver_agent_id && !m.acknowledged)
            .cloned()
            .collect();
        if let Some(path) = st.a2a_store_path.as_ref() {
            let _ = persist_a2a_store(path, &g);
        }
        return Ok(Json(A2AInboxResponse { messages }));
    }
    let claimer = claimer.expect("claimer");

    let worker = {
        let reg = st.inner.read().await;
        reg.nodes.iter().find(|n| n.id == claimer).cloned()
    };
    let Some(worker) = worker else {
        warn!(claimer, "a2a inbox claim rejected: unknown claimer node (join first)");
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: unknown claimer_node_id (join node first)".into(),
        ));
    };
    if worker.quarantined == Some(true) {
        tracing::debug!(claimer, "populi policy: quarantined worker cannot claim");
        return Ok(Json(A2AInboxResponse { messages: vec![] }));
    }

    let mut g = st.a2a_messages.write().await;
    a2a_sweep_expired_leases(&mut g, now);
    let mut picked_idx: Option<usize> = None;
    for (i, m) in g.iter_mut().enumerate() {
        if m.receiver_agent_id != req.receiver_agent_id || m.acknowledged {
            continue;
        }
        let leased_other = m
            .lease_holder_node_id
            .as_deref()
            .is_some_and(|h| h != claimer);
        let lease_alive = m
            .lease_expires_unix_ms
            .is_some_and(|exp| exp > now);
        if leased_other && lease_alive {
            continue;
        }
        if !claim_policy_allows_worker(&worker, m) {
            tracing::debug!(
                message_id = m.id,
                claimer,
                "populi policy: skipping inbox row for worker visibility/trust"
            );
            continue;
        }
        picked_idx = Some(i);
        break;
    }
    let Some(i) = picked_idx else {
        return Ok(Json(A2AInboxResponse { messages: vec![] }));
    };
    let m = &mut g[i];
    m.lease_holder_node_id = Some(claimer.to_string());
    m.lease_expires_unix_ms = Some(now.saturating_add(lease_ms));
    let one = vec![m.clone()];
    if let Some(path) = st.a2a_store_path.as_ref() {
        let _ = persist_a2a_store(path, &g);
    }
    Ok(Json(A2AInboxResponse { messages: one }))
}

pub(super) async fn a2a_ack(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<A2AAckRequest>,
) -> Result<StatusCode, ResponseErr> {
    if !auth_allows_worker_plane(ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for a2a/ack".into(),
        ));
    }
    let mut g = st.a2a_messages.write().await;
    a2a_sweep_expired_leases(&mut g, crate::now_ms());
    if let Some(msg) = g
        .iter_mut()
        .find(|m| m.id == req.message_id && m.receiver_agent_id == req.receiver_agent_id)
    {
        msg.acknowledged = true;
        msg.lease_holder_node_id = None;
        msg.lease_expires_unix_ms = None;
        if let Some(key) = msg.idempotency_dedupe_key.clone() {
            let mut maps = st.mesh_replay.maps().write().await;
            maps.idempotency.remove(&key);
        }
        msg.idempotency_dedupe_key = None;
        if let Some(path) = st.a2a_store_path.as_ref() {
            let _ = persist_a2a_store(path, &g);
        }
        st.mesh_replay.persist_if_configured().await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}
