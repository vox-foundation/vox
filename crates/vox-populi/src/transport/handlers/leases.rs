//! Remote execution lease handlers: grant, renew, release, list, admin revoke.

use std::sync::atomic::Ordering;

use axum::Json;
use axum::extract::{Extension, State};
use axum::http::StatusCode;

use super::super::auth::{PopuliAuthContext, auth_allows_admin_route, auth_allows_worker_plane};
use super::super::store::persist_exec_lease_store;
use super::super::{
    AdminExecLeaseRevokeRequest, PopuliTransportState, RemoteExecLeaseGrantRequest,
    RemoteExecLeaseGrantResponse, RemoteExecLeaseListItem, RemoteExecLeaseListResponse,
    RemoteExecLeaseReleaseRequest, RemoteExecLeaseRenewRequest, RemoteExecLeaseRow,
    a2a_lease_duration_ms, exec_lease_sweep,
};
use super::nodes::{
    ResponseErr, require_claimer_node_registered, require_claimer_worker_gate,
    store_put_exec_lease, store_revoke_exec_lease,
};

pub(crate) async fn exec_lease_grant(
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
            let updated_row = rows[idx].clone();
            if let Some(path) = st.exec_lease_store_path.as_ref() {
                let _ = persist_exec_lease_store(path, &rows);
            }
            let out = RemoteExecLeaseGrantResponse {
                lease_id: updated_row.lease_id.clone(),
                scope_key: scope_key.clone(),
                holder_node_id: claimer.to_string(),
                expires_unix_ms: updated_row.expires_unix_ms,
            };
            drop(rows);
            store_put_exec_lease(&st, updated_row);
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
    let new_row = RemoteExecLeaseRow {
        lease_id: lease_id.clone(),
        scope_key: scope_key.clone(),
        holder_node_id: claimer.to_string(),
        expires_unix_ms,
    };
    rows.push(new_row.clone());
    if let Some(path) = st.exec_lease_store_path.as_ref() {
        let _ = persist_exec_lease_store(path, &rows);
    }
    drop(rows);
    store_put_exec_lease(&st, new_row);
    Ok(Json(RemoteExecLeaseGrantResponse {
        lease_id,
        scope_key,
        holder_node_id: claimer.to_string(),
        expires_unix_ms,
    }))
}

pub(crate) async fn exec_lease_renew(
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
    let renewed_row = rows[pos].clone();
    if let Some(path) = st.exec_lease_store_path.as_ref() {
        let _ = persist_exec_lease_store(path, &rows);
    }
    drop(rows);
    store_put_exec_lease(&st, renewed_row);
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn exec_lease_release(
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
    drop(rows);
    store_revoke_exec_lease(&st, lease_id.to_string());
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn exec_lease_list(
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

pub(crate) async fn admin_exec_lease_revoke(
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
    drop(rows);
    store_revoke_exec_lease(&st, lease_id.to_string());
    Ok(StatusCode::NO_CONTENT)
}
