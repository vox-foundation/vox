//! HTTP handlers for Populi join / heartbeat / A2A / exec leases.
//!
//! ## `privacy_class` on mesh A2A (`A2ADeliverRequest`, stored on [`A2AStoredMessage`](super::A2AStoredMessage))
//! Optional string carried with the message; default **`public`** when unset. [`claim_policy_allows_worker`]
//! gates which workers may **claim** a message for delivery:
//! - **`public`** (or empty): any eligible worker.
//! - **`private`** / **`trusted`**: worker must not be `visibility=public`.
//! - **`trusted_only`**: worker must not be `visibility=public` and must not be `trust_tier=new`.
//! - **Other values**: treated as permissive (`true`) today — define new classes in mesh docs before relying on them.
//! This is a **routing / data-plane policy hint**, not Codex `research_metrics`; see trust/taxonomy SSOT for sensitivity classes.

use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use base64::Engine as _;
use tracing::{info, warn};
use super::MeshQueueStats;

use crate::{
    MAX_MAINTENANCE_FOR_MS, NodeRecord, node_maintenance_blocks_new_work,
    sweep_expired_maintenance_on_nodes,
};

use super::auth::{
    PopuliAuthContext, auth_allows_admin_route, auth_allows_deliver, auth_allows_worker_plane,
    populi_control_token_from_env,
};
#[cfg(feature = "transport")]
use super::dispatch_results_sweep;
use super::result_attestation;
use super::store::{persist_a2a_store, persist_exec_lease_store, scope_ok};
use super::{
    A2AAckRequest, A2ADeliverRequest, A2ADeliverResponse, A2AInboxRequest, A2AInboxResponse,
    A2ALeaseRenewRequest, A2AStoredMessage, AdminExecLeaseRevokeRequest, AdminMaintenanceRequest,
    AdminQuarantineRequest, BootstrapExchangeRequest, BootstrapExchangeResponse, DispatchRequest,
    DispatchResponse, LeaveRequest, PopuliRegistryFile, PopuliTransportState,
    RemoteExecLeaseGrantRequest, RemoteExecLeaseGrantResponse, RemoteExecLeaseListItem,
    RemoteExecLeaseListResponse, RemoteExecLeaseReleaseRequest, RemoteExecLeaseRenewRequest,
    RemoteExecLeaseRow, a2a_in_memory_cap, a2a_lease_duration_ms, a2a_sweep_expired_leases,
    exec_lease_sweep, server_stale_prune_ms,
};

pub(super) async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok\n")
}

async fn registry_sweep_maintenance(st: &PopuliTransportState) {
    let now = crate::now_ms();
    let mut inner = st.inner.write().await;
    sweep_expired_maintenance_on_nodes(&mut inner.nodes, now);

    #[cfg(feature = "transport")]
    dispatch_results_sweep(&st.dispatch_results, now);
}

pub(super) async fn list_nodes(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
) -> Result<Json<PopuliRegistryFile>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for node list".into(),
        ));
    }
    registry_sweep_maintenance(&st).await;
    let mut g = st.inner.read().await.clone();
    if let Some(window) = server_stale_prune_ms() {
        let now = crate::now_ms();
        g.nodes
            .retain(|n| now.saturating_sub(n.last_seen_unix_ms) <= window);
    }

    let a2a = st.a2a_messages.read().await;
    let pending = a2a
        .iter()
        .filter(|m| !m.acknowledged && m.lease_holder_node_id.is_none())
        .count();
    g.queue_depth = Some(pending);

    Ok(Json(g))
}

pub(super) async fn join_node(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(mut node): Json<NodeRecord>,
) -> Result<Json<NodeRecord>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
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
    let now = crate::now_ms();
    sweep_expired_maintenance_on_nodes(&mut g.nodes, now);
    if let Some(i) = g.nodes.iter().position(|n| n.id == node.id) {
        let preserve_q = g.nodes[i].quarantined;
        let preserve_m = g.nodes[i].maintenance;
        let preserve_mu = g.nodes[i].maintenance_until_unix_ms;
        g.nodes[i] = node.clone();
        g.nodes[i].quarantined = preserve_q;
        g.nodes[i].maintenance = preserve_m;
        g.nodes[i].maintenance_until_unix_ms = preserve_mu;
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
    if src.maintenance_until_unix_ms.is_some() {
        target.maintenance_until_unix_ms = src.maintenance_until_unix_ms;
    }
    if src.maintenance.is_some() {
        target.maintenance = src.maintenance;
        if target.maintenance != Some(true) {
            target.maintenance_until_unix_ms = None;
        }
    }
    if src.provider.is_some() {
        target.provider = src.provider.clone();
    }
    if src.advertised_models.is_some() {
        target.advertised_models = src.advertised_models.clone();
    }
    if src.donation_policy.is_some() {
        target.donation_policy = src.donation_policy.clone();
    }
    if src.owner_vox_user_id.is_some() {
        target.owner_vox_user_id = src.owner_vox_user_id.clone();
    }
    if src.ed25519_pub_key_b64.is_some() {
        target.ed25519_pub_key_b64 = src.ed25519_pub_key_b64.clone();
    }
}

pub(super) async fn heartbeat(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(mut node): Json<NodeRecord>,
) -> Result<Json<NodeRecord>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
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
    let now = crate::now_ms();
    sweep_expired_maintenance_on_nodes(&mut g.nodes, now);
    if let Some(i) = g.nodes.iter().position(|n| n.id == node.id) {
        let preserve_q = g.nodes[i].quarantined;
        let preserve_m = g.nodes[i].maintenance;
        let preserve_mu = g.nodes[i].maintenance_until_unix_ms;
        g.nodes[i].last_seen_unix_ms = node.last_seen_unix_ms;
        merge_optional_node_fields(&mut g.nodes[i], &node);
        g.nodes[i].quarantined = preserve_q;
        g.nodes[i].maintenance = preserve_m;
        g.nodes[i].maintenance_until_unix_ms = preserve_mu;
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
    if !auth_allows_worker_plane(&ctx) {
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

/// Mesh A2A wire ids: trim, non-empty, ASCII decimal digits only (orchestrator agent id JSON form).
fn parse_a2a_mesh_agent_id(label: &str, raw: &str) -> Result<String, ResponseErr> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            format!("populi: {label} required (non-empty decimal digit string)"),
        ));
    }
    if !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            format!(
                "populi: {label} must be a non-empty decimal digit string (orchestrator agent id)"
            ),
        ));
    }
    Ok(s.to_string())
}

fn a2a_inbox_limit(requested: Option<usize>) -> usize {
    requested.unwrap_or(64).clamp(1, 256)
}

async fn require_claimer_worker_gate(
    st: &PopuliTransportState,
    claimer: &str,
) -> Result<(), ResponseErr> {
    if claimer.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: claimer_node_id required".into(),
        ));
    }
    registry_sweep_maintenance(st).await;
    let now = crate::now_ms();
    let worker = {
        let reg = st.inner.read().await;
        reg.nodes.iter().find(|n| n.id == claimer).cloned()
    };
    let Some(worker) = worker else {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: unknown claimer_node_id (join node first)".into(),
        ));
    };
    if worker.quarantined == Some(true) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: claimer node is quarantined".into(),
        ));
    }
    if node_maintenance_blocks_new_work(now, &worker) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: claimer node is in maintenance mode".into(),
        ));
    }
    Ok(())
}

/// Like [`require_claimer_worker_gate`] but only verifies the node is registered (join).
/// Used for **exec lease release** so holders can clear `scope_key` while in maintenance/quarantine.
async fn require_claimer_node_registered(
    st: &PopuliTransportState,
    claimer: &str,
) -> Result<(), ResponseErr> {
    if claimer.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: claimer_node_id required".into(),
        ));
    }
    let known = {
        let reg = st.inner.read().await;
        reg.nodes.iter().any(|n| n.id == claimer)
    };
    if !known {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: unknown claimer_node_id (join node first)".into(),
        ));
    }
    Ok(())
}

pub(super) async fn exec_lease_grant(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<RemoteExecLeaseGrantRequest>,
) -> Result<Json<RemoteExecLeaseGrantResponse>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for exec lease grant".into(),
        ));
    }
    let claimer = req.claimer_node_id.trim();
    let scope_key = req.scope_key.trim().to_string();
    if scope_key.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: scope_key required (non-empty)".into(),
        ));
    }
    if scope_key.len() > 2048 {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: scope_key exceeds max length (2048)".into(),
        ));
    }
    require_claimer_worker_gate(&st, claimer).await?;
    let now = crate::now_ms();
    let lease_ms = a2a_lease_duration_ms();
    let mut rows = st.exec_leases.write().await;
    exec_lease_sweep(&mut rows, now);
    if let Some(idx) = rows.iter().position(|r| r.scope_key == scope_key) {
        let existing = &rows[idx];
        if existing.holder_node_id == claimer {
            rows[idx].expires_unix_ms = now.saturating_add(lease_ms);
            if let Some(path) = st.exec_lease_store_path.as_ref() {
                let _ = persist_exec_lease_store(path, &rows);
            }
            let out = RemoteExecLeaseGrantResponse {
                lease_id: rows[idx].lease_id.clone(),
                scope_key: scope_key.clone(),
                holder_node_id: claimer.to_string(),
                expires_unix_ms: rows[idx].expires_unix_ms,
            };
            return Ok(Json(out));
        }
        return Err(ResponseErr(
            StatusCode::CONFLICT,
            "populi: scope_key already leased to another node".into(),
        ));
    }
    let id = st.exec_lease_id_gen.fetch_add(1, Ordering::Relaxed);
    let lease_id = id.to_string();
    let expires_unix_ms = now.saturating_add(lease_ms);
    rows.push(RemoteExecLeaseRow {
        lease_id: lease_id.clone(),
        scope_key: scope_key.clone(),
        holder_node_id: claimer.to_string(),
        expires_unix_ms,
    });
    if let Some(path) = st.exec_lease_store_path.as_ref() {
        let _ = persist_exec_lease_store(path, &rows);
    }
    Ok(Json(RemoteExecLeaseGrantResponse {
        lease_id,
        scope_key,
        holder_node_id: claimer.to_string(),
        expires_unix_ms,
    }))
}

pub(super) async fn exec_lease_renew(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<RemoteExecLeaseRenewRequest>,
) -> Result<StatusCode, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for exec lease renew".into(),
        ));
    }
    let claimer = req.claimer_node_id.trim();
    let lease_id = req.lease_id.trim();
    if lease_id.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: lease_id required".into(),
        ));
    }
    require_claimer_worker_gate(&st, claimer).await?;
    let now = crate::now_ms();
    let lease_ms = a2a_lease_duration_ms();
    let mut rows = st.exec_leases.write().await;
    exec_lease_sweep(&mut rows, now);
    let Some(pos) = rows.iter().position(|r| r.lease_id == lease_id) else {
        return Ok(StatusCode::NOT_FOUND);
    };
    if rows[pos].holder_node_id != claimer {
        return Err(ResponseErr(
            StatusCode::CONFLICT,
            "populi: exec lease renew only for active lease holder".into(),
        ));
    }
    rows[pos].expires_unix_ms = now.saturating_add(lease_ms);
    if let Some(path) = st.exec_lease_store_path.as_ref() {
        let _ = persist_exec_lease_store(path, &rows);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn exec_lease_release(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<RemoteExecLeaseReleaseRequest>,
) -> Result<StatusCode, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for exec lease release".into(),
        ));
    }
    let claimer = req.claimer_node_id.trim();
    let lease_id = req.lease_id.trim();
    if lease_id.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: lease_id required".into(),
        ));
    }
    require_claimer_node_registered(&st, claimer).await?;
    let now = crate::now_ms();
    let mut rows = st.exec_leases.write().await;
    exec_lease_sweep(&mut rows, now);
    let Some(pos) = rows.iter().position(|r| r.lease_id == lease_id) else {
        return Ok(StatusCode::NOT_FOUND);
    };
    if rows[pos].holder_node_id != claimer {
        return Err(ResponseErr(
            StatusCode::CONFLICT,
            "populi: exec lease release only for active lease holder".into(),
        ));
    }
    rows.remove(pos);
    if let Some(path) = st.exec_lease_store_path.as_ref() {
        let _ = persist_exec_lease_store(path, &rows);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn exec_lease_list(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
) -> Result<Json<RemoteExecLeaseListResponse>, ResponseErr> {
    if !auth_allows_admin_route(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: mesh/admin bearer required for exec lease list".into(),
        ));
    }
    let now = crate::now_ms();
    let mut rows = st.exec_leases.write().await;
    exec_lease_sweep(&mut rows, now);
    let leases: Vec<RemoteExecLeaseListItem> = rows
        .iter()
        .map(|r| RemoteExecLeaseListItem {
            lease_id: r.lease_id.clone(),
            scope_key: r.scope_key.clone(),
            holder_node_id: r.holder_node_id.clone(),
            expires_unix_ms: r.expires_unix_ms,
        })
        .collect();
    if let Some(path) = st.exec_lease_store_path.as_ref() {
        let _ = persist_exec_lease_store(path, &rows);
    }
    Ok(Json(RemoteExecLeaseListResponse { leases }))
}

pub(super) async fn admin_exec_lease_revoke(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<AdminExecLeaseRevokeRequest>,
) -> Result<StatusCode, ResponseErr> {
    if !auth_allows_admin_route(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: mesh/admin bearer required for exec lease revoke".into(),
        ));
    }
    let lease_id = req.lease_id.trim();
    if lease_id.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: lease_id required".into(),
        ));
    }
    let now = crate::now_ms();
    let mut rows = st.exec_leases.write().await;
    exec_lease_sweep(&mut rows, now);
    let Some(pos) = rows.iter().position(|r| r.lease_id == lease_id) else {
        return Ok(StatusCode::NOT_FOUND);
    };
    rows.remove(pos);
    if let Some(path) = st.exec_lease_store_path.as_ref() {
        let _ = persist_exec_lease_store(path, &rows);
    }
    Ok(StatusCode::NO_CONTENT)
}

fn claim_policy_allows_worker(worker: &NodeRecord, msg: &A2AStoredMessage, sender_owner_id: Option<&str>) -> bool {
    let privacy = msg
        .privacy_class
        .as_deref()
        .unwrap_or("public")
        .trim()
        .to_ascii_lowercase();

    let is_public = privacy.is_empty() || privacy == "public";

    let vis = worker
        .visibility
        .as_deref()
        .unwrap_or("private")
        .trim()
        .to_ascii_lowercase();

    // 1. Mandatory Visibility Check: Private nodes never take public tasks.
    if is_public && vis == "private" {
        return false;
    }

    // 2. Donation Policy Check (for public mesh).
    if is_public {
        if let Some(policy) = &worker.donation_policy {
            // Opt-out check.
            if !policy.public_mesh_opt_in {
                return false;
            }

            // Priority check.
            if msg.priority < policy.min_priority {
                return false;
            }

            // Task Kind check.
            if let Some(msg_kind) = &msg.task_kind {
                let allowed = policy.slots.iter().any(|s| {
                    let s_kind = format!("{:?}", s.task_kind).to_lowercase();
                    s_kind == msg_kind.to_lowercase()
                });
                if !allowed {
                    return false;
                }
            }

            // Identity checks.
            if let Some(denied) = &policy.denied_users {
                if let Some(owner) = sender_owner_id {
                    if denied.contains(&owner.to_string()) {
                        return false;
                    }
                }
            }

            if let Some(allowed) = &policy.allowed_users {
                match sender_owner_id {
                    Some(owner) => {
                        if !allowed.contains(&owner.to_string()) {
                            return false;
                        }
                    }
                    None => return false, // If allowed_users is set, anonymous tasks are rejected.
                }
            }

            // Allowed Scopes check.
            if let Some(_allowed_scopes) = &policy.allowed_scopes {
                // If we don't know the sender's scope, we can't verify, so we skip if policy is strict.
                // (In a real federation, we'd have the sender's scope in the message envelope).
                // For now, if sender_node_id is present, we check it.
                // Since we don't have easy access to the whole registry here without passing it,
                // we'll assume for Phase 1 that allowed_scopes: None means "all public".
            }
        }
    }

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
    if !auth_allows_deliver(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: submitter/mesh/admin token required for a2a/deliver".into(),
        ));
    }
    let sender_agent_id = parse_a2a_mesh_agent_id("sender_agent_id", &req.sender_agent_id)?;
    let receiver_agent_id = parse_a2a_mesh_agent_id("receiver_agent_id", &req.receiver_agent_id)?;
    let sender_node_id = if let PopuliAuthContext::NodeSignature { node_id, .. } = &ctx {
        Some(node_id.clone())
    } else {
        None
    };
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
        // Punish node for failed attestation.
        if let Some(node_id) = &sender_node_id {
            let mut g = st.inner.write().await;
            if let Some(i) = g.nodes.iter().position(|n| n.id == *node_id) {
                g.nodes[i].trust_tier = Some("degraded".to_string());
                tracing::warn!(
                    node_id = node_id,
                    error = %msg,
                    "populi: node trust degraded due to attestation failure"
                );
            }
        }

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
        let dedupe = format!("{}\x1f{}\x1f{}", sender_agent_id, receiver_agent_id, key);
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
            sender_agent_id,
            receiver_agent_id,
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
            jwe_payload: req.jwe_payload.clone(),
            priority: req.priority,
            task_kind: req.task_kind.clone(),
            model_id: req.model_id.clone(),
            sender_node_id: sender_node_id.clone(),
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
        sender_agent_id,
        receiver_agent_id,
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
        jwe_payload: req.jwe_payload.clone(),
        priority: req.priority,
        task_kind: req.task_kind.clone(),
        model_id: req.model_id.clone(),
        sender_node_id,
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
    if !auth_allows_admin_route(&ctx) {
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

pub(super) async fn admin_maintenance(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<AdminMaintenanceRequest>,
) -> Result<StatusCode, ResponseErr> {
    if !auth_allows_admin_route(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: mesh/admin bearer required for maintenance".into(),
        ));
    }
    let id = req.node_id.trim();
    if id.is_empty() {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            "populi: node_id required".into(),
        ));
    }
    let now = crate::now_ms();
    let mut g = st.inner.write().await;
    sweep_expired_maintenance_on_nodes(&mut g.nodes, now);
    if let Some(i) = g.nodes.iter().position(|n| n.id == id) {
        if !req.maintenance {
            g.nodes[i].maintenance = Some(false);
            g.nodes[i].maintenance_until_unix_ms = None;
            Ok(StatusCode::NO_CONTENT)
        } else {
            let abs = req.maintenance_until_unix_ms.filter(|&u| u > now);
            let until = if abs.is_some() {
                abs
            } else if let Some(rel) = req.maintenance_for_ms.filter(|&m| m > 0) {
                let capped = rel.min(MAX_MAINTENANCE_FOR_MS);
                Some(now.saturating_add(capped))
            } else {
                None
            };
            g.nodes[i].maintenance = Some(true);
            g.nodes[i].maintenance_until_unix_ms = until;
            Ok(StatusCode::NO_CONTENT)
        }
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}

pub(super) async fn a2a_lease_renew(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<A2ALeaseRenewRequest>,
) -> Result<StatusCode, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
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
    registry_sweep_maintenance(&st).await;
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
    if node_maintenance_blocks_new_work(now, &worker) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: claimer node is in maintenance mode".into(),
        ));
    }
    let mut g = st.a2a_messages.write().await;
    a2a_sweep_expired_leases(&mut g, now);
    let Some(msg) = g.iter_mut().find(|m| {
        m.id == req.message_id && m.receiver_agent_id == req.receiver_agent_id && !m.acknowledged
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
    if !auth_allows_worker_plane(&ctx) {
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
        let max_messages = a2a_inbox_limit(req.max_messages);
        let before = req.before_message_id;
        let messages = g
            .iter()
            .rev()
            .filter(|m| {
                m.receiver_agent_id == req.receiver_agent_id
                    && !m.acknowledged
                    && before.is_none_or(|cursor| m.id < cursor)
            })
            .take(max_messages)
            .cloned()
            .collect();
        if let Some(path) = st.a2a_store_path.as_ref() {
            let _ = persist_a2a_store(path, &g);
        }
        return Ok(Json(A2AInboxResponse { messages }));
    }
    let claimer = claimer.expect("claimer");

    registry_sweep_maintenance(&st).await;
    let worker = {
        let reg = st.inner.read().await;
        reg.nodes.iter().find(|n| n.id == claimer).cloned()
    };
    let Some(worker) = worker else {
        warn!(
            claimer,
            "a2a inbox claim rejected: unknown claimer node (join first)"
        );
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: unknown claimer_node_id (join node first)".into(),
        ));
    };
    if worker.quarantined == Some(true) {
        tracing::debug!(claimer, "populi policy: quarantined worker cannot claim");
        return Ok(Json(A2AInboxResponse { messages: vec![] }));
    }
    if node_maintenance_blocks_new_work(now, &worker) {
        tracing::debug!(claimer, "populi policy: maintenance worker cannot claim");
        return Ok(Json(A2AInboxResponse { messages: vec![] }));
    }

    let nodes = {
        let reg = st.inner.read().await;
        reg.nodes.clone()
    };

    let mut g = st.a2a_messages.write().await;
    a2a_sweep_expired_leases(&mut g, now);
    let mut picked_idx: Option<usize> = None;
    let mut best_priority: u8 = 0;
    for (i, m) in g.iter_mut().enumerate() {
        if m.receiver_agent_id != req.receiver_agent_id || m.acknowledged {
            continue;
        }
        let leased_other = m
            .lease_holder_node_id
            .as_deref()
            .is_some_and(|h| h != claimer);
        let lease_alive = m.lease_expires_unix_ms.is_some_and(|exp| exp > now);
        if leased_other && lease_alive {
            continue;
        }
        
        let sender_owner_id = m.sender_node_id.as_ref().and_then(|id| {
            nodes.iter().find(|n| n.id == *id).and_then(|n| n.owner_vox_user_id.as_deref())
        });

        if !claim_policy_allows_worker(&worker, m, sender_owner_id) {
            tracing::debug!(
                message_id = m.id,
                claimer,
                "populi policy: skipping inbox row for worker visibility/trust"
            );
            continue;
        }
        if picked_idx.is_none() || m.priority > best_priority {
            picked_idx = Some(i);
            best_priority = m.priority;
        }
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
    if !auth_allows_worker_plane(&ctx) {
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

        // Wave 2: Kudos crediting for job results.
        if msg.message_type == "job_result" {
            if let (Ok(result), Some(db), Some(node_id)) = (
                serde_json::from_str::<vox_mesh_types::TaskResult>(&msg.payload),
                &st.db,
                &msg.sender_node_id,
            ) {
                if result.success {
                    let owner_id = {
                        let reg = st.inner.read().await;
                        reg.nodes
                            .iter()
                            .find(|n| n.id == *node_id)
                            .and_then(|n| n.owner_vox_user_id.clone())
                    };

                    if let Some(vox_user_id) = owner_id {
                        let credit = vox_mesh_types::kudos::CreditJobRequest {
                            vox_user_id,
                            node_id: node_id.clone(),
                            primitive: vox_mesh_types::kudos::RewardPrimitive::GpuComputeMs,
                            amount: result.duration_ms,
                            task_id: Some(msg.id.to_string()),
                            metadata_json: None,
                        };
                        let db = db.clone();
                        tokio::spawn(async move {
                            if let Err(e) = db.credit_kudos(&credit).await {
                                tracing::error!("failed to credit kudos: {:?}", e);
                            }
                        });
                    }
                }
            }
        }

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
pub(super) async fn dispatch_script(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<DispatchRequest>,
) -> Result<Json<DispatchResponse>, ResponseErr> {
    if !auth_allows_deliver(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: submitter/mesh/admin token required for dispatch".into(),
        ));
    }

    let nodes = st.inner.read().await;
    let target = if let Some(id) = &req.node_id {
        nodes.nodes.iter().find(|n| n.id == *id).cloned()
    } else {
        select_best_node(&nodes.nodes, &req).cloned()
    };
    drop(nodes);

    let Some(target) = target else {
        return Err(ResponseErr(
            StatusCode::NOT_FOUND,
            "populi: no suitable worker node found for dispatch".into(),
        ));
    };

    let Some(addr) = &target.listen_addr else {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            format!("populi: target node {} has no listen_addr", target.id),
        ));
    };

    // Forward to worker
    let client = crate::http_client::PopuliHttpClient::new(addr).with_env_token();

    if req.is_detached {
        use vox_primitives::id::simple_hex_id;
        let dispatch_id = simple_hex_id();
        let st_cl = st.clone();
        let dispatch_id_cl = dispatch_id.clone();
        let target_node_id = target.id.clone();

        tokio::spawn(async move {
            let res = client.worker_execute(&req).await;
            match res {
                Ok(mut resp) => {
                    resp.expires_unix_ms = Some(crate::now_ms() + 3_600_000); // 1 hour TTL
                    st_cl.dispatch_results.insert(dispatch_id_cl, resp);
                }
                Err(e) => {
                    st_cl.dispatch_results.insert(
                        dispatch_id_cl,
                        DispatchResponse {
                            success: false,
                            output: String::new(),
                            error: Some(format!(
                                "populi: detached execution failed to forward: {}",
                                e
                            )),
                            node_id: target_node_id,
                            duration_ms: 0,
                            exit_code: None,
                            is_truncated: false,
                            expires_unix_ms: Some(crate::now_ms() + 3_600_000),
                        },
                    );
                }
            }
            if let Some(path) = &st_cl.dispatch_results_store_path {
                let _ = super::store::persist_dispatch_results_store(path, &st_cl.dispatch_results);
            }
        });

        Ok(Json(DispatchResponse {
            success: true,
            output: format!(
                "populi: detached dispatch accepted. poll for results with id: {}",
                dispatch_id
            ),
            error: None,
            node_id: target.id,
            duration_ms: 0,
            exit_code: None,
            is_truncated: false,
            expires_unix_ms: None,
        }))
    } else {
        let mut resp = client.worker_execute(&req).await.map_err(|e| {
            ResponseErr(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("populi: failed to forward dispatch to worker: {}", e),
            )
        })?;
        resp.expires_unix_ms = None;
        Ok(Json(resp))
    }
}

pub(super) async fn dispatch_results_poll(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    axum::extract::Path(dispatch_id): axum::extract::Path<String>,
) -> Result<Json<DispatchResponse>, ResponseErr> {
    if !auth_allows_deliver(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: submitter/mesh/admin token required for dispatch polls".into(),
        ));
    }

    #[cfg(feature = "transport")]
    dispatch_results_sweep(&st.dispatch_results, crate::now_ms());

    if let Some(res) = st.dispatch_results.get(&dispatch_id) {
        Ok(Json(res.clone()))
    } else {
        Err(ResponseErr(
            StatusCode::NOT_FOUND,
            format!(
                "populi: dispatch result for id {} not found or still in-flight",
                dispatch_id
            ),
        ))
    }
}

fn select_best_node<'a>(nodes: &'a [NodeRecord], req: &DispatchRequest) -> Option<&'a NodeRecord> {
    let mut candidates: Vec<_> = nodes
        .iter()
        .filter(|n| {
            n.quarantined != Some(true) && !node_maintenance_blocks_new_work(crate::now_ms(), n)
        })
        .filter(|n| {
            // Label matching
            if let Some(required) = &req.required_labels {
                if !required.is_empty()
                    && !required
                        .iter()
                        .all(|req_lab| n.capabilities.labels.contains(req_lab))
                {
                    return false;
                }
            }
            // VRAM matching
            if let Some(min_vram) = req.min_vram_mb {
                let node_vram = n.capabilities.min_vram_mb.unwrap_or(0);
                if node_vram < min_vram {
                    return false;
                }
            }
            // Donation policy matching
            if let (Some(task_kind_str), Some(policy)) = (&req.task_kind, &n.donation_policy) {
                let allowed = policy.slots.iter().any(|slot| {
                    format!("{:?}", slot.task_kind).to_lowercase() == task_kind_str.to_lowercase()
                });
                if !allowed {
                    return false;
                }
            }
            true
        })
        .collect();

    // Load balancing: Sort by CPU usage ascending
    candidates.sort_by(|a, b| {
        let a_usage = a.cpu_usage_pct.unwrap_or(100.0);
        let b_usage = b.cpu_usage_pct.unwrap_or(100.0);
        a_usage
            .partial_cmp(&b_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    candidates.first().copied()
}


pub(super) async fn queue_stats(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
) -> Result<Json<MeshQueueStats>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for queue stats".into(),
        ));
    }
    let msgs = st.a2a_messages.read().await;
    let mut stats = MeshQueueStats {
        pending_count: 0,
        pending_by_kind: std::collections::HashMap::new(),
        pending_by_priority: std::collections::HashMap::new(),
    };

    for m in msgs.iter() {
        if m.acknowledged || m.lease_holder_node_id.is_some() {
            continue;
        }
        stats.pending_count += 1;
        if let Some(kind) = &m.task_kind {
            *stats.pending_by_kind.entry(kind.clone()).or_insert(0) += 1;
        }
        *stats.pending_by_priority.entry(m.priority).or_insert(0) += 1;
    }

    Ok(Json(stats))
}

pub(super) async fn execute_on_worker(
    State(_st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<DispatchRequest>,
) -> Result<Json<DispatchResponse>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for worker execution".into(),
        ));
    }

    // Phase 4: Policy Gating
    let secret = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshExecPolicy);
    let policy = secret.expose().unwrap_or("permissive");
    if req.is_bundle && policy == "source-only" {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi policy: this node only allows source-based dispatch (binary execution disabled)".into(),
        ));
    }

    if let Some(req_labels) = &req.required_labels {
        let local_record = crate::node_record_for_current_process("".into(), None);
        for req_label in req_labels {
            if !local_record.capabilities.labels.contains(req_label) {
                return Err(ResponseErr(
                    StatusCode::FORBIDDEN,
                    format!(
                        "populi capacity constraints: this node lacks the required capability label '{}'",
                        req_label
                    ),
                ));
            }
        }
    }

    let source_bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.source)
        .map_err(|e| {
            ResponseErr(
                StatusCode::BAD_REQUEST,
                format!("populi: invalid base64 source: {}", e),
            )
        })?;

    // Phase 2: Integrity Verification
    if let Some(expected_hex) = &req.source_blake3_hex {
        let actual_hash = blake3::hash(&source_bytes);
        let actual_hex = actual_hash.to_hex().to_string();
        if &actual_hex != expected_hex {
            return Err(ResponseErr(
                StatusCode::BAD_REQUEST,
                format!(
                    "populi integrity error: bundle hash mismatch (expected {}, got {})",
                    expected_hex, actual_hex
                ),
            ));
        }
    }

    let tmp_dir = std::env::temp_dir();
    let bin_path = if req.is_bundle {
        // Source is actually a pre-compiled binary.
        // Identify .wasm vs native.
        let is_wasm = source_bytes.starts_with(b"\0asm");
        let ext = if is_wasm { ".wasm" } else { "" };
        let file_name = format!("vox-bundle-{}{}", vox_primitives::id::simple_hex_id(), ext);
        let tmp_file = tmp_dir.join(file_name);
        std::fs::write(&tmp_file, &source_bytes).map_err(|e| {
            ResponseErr(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("populi: failed to write bundle: {}", e),
            )
        })?;

        // Ensure executable on Unix
        #[cfg(unix)]
        if !is_wasm {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&tmp_file)
                .map(|m| m.permissions())
                .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o755));
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&tmp_file, perms);
        }

        tmp_file
    } else {
        let file_name = format!("vox-dispatch-{}.vox", vox_primitives::id::simple_hex_id());
        let tmp_file = tmp_dir.join(file_name);
        std::fs::write(&tmp_file, &source_bytes).map_err(|e| {
            ResponseErr(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("populi: failed to write tmp file: {}", e),
            )
        })?;
        tmp_file
    };

    let start_time = std::time::Instant::now();

    let output = if req.is_bundle {
        if bin_path.extension().map_or(false, |ext| ext == "wasm") {
            std::process::Command::new("vox")
                .arg("run")
                .arg("--mode")
                .arg("script")
                .arg("--isolation")
                .arg("wasm")
                .arg(&bin_path)
                .output()
        } else {
            std::process::Command::new(&bin_path).output()
        }
    } else {
        std::process::Command::new("vox")
            .arg("run")
            .arg("--mode")
            .arg("script")
            .arg(&bin_path)
            .output()
    };

    let duration_ms = start_time.elapsed().as_millis() as u64;

    // Cleanup early
    let _ = std::fs::remove_file(&bin_path);

    match output {
        Ok(out) => {
            // Phase 3: Output Truncation (10MB Limit)
            const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;
            let mut combined_stdout = out.stdout;
            let mut combined_stderr = out.stderr;

            let total_len = combined_stdout.len() + combined_stderr.len();
            let is_truncated = total_len > MAX_OUTPUT_BYTES;

            if is_truncated {
                // Keep the first 10MB of stderr then stdout or split evenly
                if combined_stderr.len() > MAX_OUTPUT_BYTES / 2 {
                    combined_stderr.truncate(MAX_OUTPUT_BYTES / 2);
                }
                let remaining = MAX_OUTPUT_BYTES.saturating_sub(combined_stderr.len());
                if combined_stdout.len() > remaining {
                    combined_stdout.truncate(remaining);
                }
            }

            let output_str = String::from_utf8_lossy(&combined_stdout).to_string()
                + &String::from_utf8_lossy(&combined_stderr);

            Ok(Json(DispatchResponse {
                success: out.status.success(),
                output: output_str,
                is_truncated,
                duration_ms,
                exit_code: out.status.code(),
                error: if out.status.success() {
                    None
                } else {
                    Some(format!("Exit code: {:?}", out.status.code()))
                },
                node_id: vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshNodeId)
                    .expose()
                    .unwrap_or("unknown")
                    .to_string(),
                expires_unix_ms: None,
            }))
        }
        Err(e) => Ok(Json(DispatchResponse {
            success: false,
            output: String::new(),
            is_truncated: false,
            duration_ms,
            exit_code: None,
            error: Some(format!("Failed to execute vox: {}", e)),
            node_id: vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshNodeId)
                .expose()
                .unwrap_or("unknown")
                .to_string(),
            expires_unix_ms: None,
        })),
    }
}

pub(super) async fn federation_directory(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
) -> Result<Json<super::FederationDirectoryResponse>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: token required for federation/directory".into(),
        ));
    }
    
    let entries = {
        let g = st.federated_meshes.read().await;
        g.clone()
    };

    Ok(Json(super::FederationDirectoryResponse { entries }))
}

pub(super) async fn federation_announce(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<super::FederationAnnounceRequest>,
) -> Result<Json<super::FederationDirectoryResponse>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: token required for federation/announce".into(),
        ));
    }

    // Optional Security: Verify entry signature if provided
    if let (Some(sig), Some(pk)) = (&req.entry.signature, &req.entry.public_key) {
        let msg = req.entry.canonical_bytes();
        let mut sig_arr = [0u8; 64];
        if sig.len() == 64 {
            sig_arr.copy_from_slice(sig);
            if let Ok(vk) = vox_crypto::facades::verifying_key_from_bytes(pk) {
                if !vox_crypto::facades::verify(&vk, &msg, &sig_arr) {
                    return Err(ResponseErr(
                        StatusCode::BAD_REQUEST,
                        "populi: invalid federation announcement signature".into(),
                    ));
                }
            } else {
                return Err(ResponseErr(
                    StatusCode::BAD_REQUEST,
                    "populi: invalid federation public key".into(),
                ));
            }
        } else {
            return Err(ResponseErr(
                StatusCode::BAD_REQUEST,
                "populi: invalid signature length (expected 64 bytes)".into(),
            ));
        }
    } else if req.entry.public {
        // Policy: Public meshes MUST sign their announcements in production
        // For now, we log a warning but allow it for backward compatibility or simple LAN use.
        tracing::warn!(scope_id = %req.entry.scope_id, "Received unsigned public mesh announcement");
    }

    let entries = {
        let mut g = st.federated_meshes.write().await;
        
        if let Some(i) = g.iter().position(|e| e.scope_id == req.entry.scope_id) {
            g[i] = req.entry;
        } else {
            g.push(req.entry);
        }
        g.clone()
    };

    Ok(Json(super::FederationDirectoryResponse { entries }))
}
