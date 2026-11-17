//! Vox Concilium & Populi Placement `/api/v2/concilium/*` HTTP gateway handlers (`CONC-03`).

use std::collections::BTreeMap;
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;
use vox_orchestrator_types::concilium::{
    ConciliumPilotActionInput, SequenceBoardEntry, SequenceBoardFilter,
};

use super::GatewayState;
use super::dashboard_api::{enforce_dashboard_read, enforce_dashboard_write};
use crate::services::routes::{err, ok};

#[path = "concilium_api/pilot.rs"]
mod pilot;
#[path = "concilium_api/placement.rs"]
mod placement;

use pilot::execute_pilot_action;
use placement::{build_enriched_snapshot, build_placement};

/// Phase 454 (CONC-03): Concilium & Populi sub-router mounted under `/api/v2`.
pub fn router() -> Router<GatewayState> {
    Router::new()
        .route("/concilium/snapshot", get(get_concilium_snapshot))
        .route("/concilium/board", get(get_concilium_board))
        .route("/concilium/placement", get(get_concilium_placement))
        .route(
            "/concilium/action",
            get(get_concilium_action).post(post_concilium_action),
        )
}

#[derive(Debug, Default, Deserialize)]
pub struct BoardQuery {
    #[serde(default)]
    pub since_seq: Option<u64>,
    #[serde(default)]
    pub seqs: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct ActionQuery {
    pub payload: String,
}

pub(super) fn to_json<T: serde::Serialize>(val: &T) -> Json<Value> {
    serde_json::to_value(val)
        .map(ok)
        .unwrap_or_else(|e| err("serialize", &e.to_string()))
}

fn reject_cross_site_get(headers: &HeaderMap) -> Result<(), Json<Value>> {
    if let Some(sfs) = headers.get("sec-fetch-site").and_then(|v| v.to_str().ok())
        && !sfs.eq_ignore_ascii_case("same-origin")
        && !sfs.eq_ignore_ascii_case("none")
    {
        return Err(err(
            "forbidden",
            "cross-site GET pilot action is not permitted",
        ));
    }
    Ok(())
}

/// Phase 454 (CONC-03): `GET /api/v2/concilium/snapshot` causal swimlane snapshot.
pub async fn get_concilium_snapshot(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_dashboard_read(&gs, &connect.0, &headers) {
        return e;
    }
    to_json(&build_enriched_snapshot(&gs))
}

/// Phase 454 (CONC-03): `GET /api/v2/concilium/board?since_seq=&seqs=&channel=&limit=`
pub async fn get_concilium_board(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<BoardQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_dashboard_read(&gs, &connect.0, &headers) {
        return e;
    }
    let seqs = q
        .seqs
        .as_deref()
        .map(|r| {
            r.split(',')
                .filter_map(|s| s.trim().parse::<u64>().ok())
                .collect::<Vec<_>>()
        })
        .filter(|v| !v.is_empty());
    let limit = q.limit.unwrap_or(50).clamp(1, 512);
    let filter = SequenceBoardFilter {
        since_seq: q.since_seq,
        seqs,
        channel: q.channel,
        limit: Some(limit),
    };
    let live = gs.server_state.orchestrator.bulletin().read_board(&filter);
    let mut by_seq: BTreeMap<u64, SequenceBoardEntry> = BTreeMap::new();
    if let Some(ref db) = gs.server_state.db
        && let Ok(durable) = db
            .read_concilium_seq_board(&gs.server_state.repository.repository_id, &filter)
            .await
    {
        for e in durable {
            by_seq.insert(e.seq, e);
        }
    }
    for e in live {
        by_seq.insert(e.seq, e);
    }
    let is_tail = filter.since_seq.is_none() && filter.seqs.as_ref().is_none_or(Vec::is_empty);
    let mut merged: Vec<SequenceBoardEntry> = by_seq.into_values().collect();
    if is_tail {
        if merged.len() > limit {
            merged.drain(..merged.len() - limit);
        }
    } else {
        merged.truncate(limit);
    }
    to_json(&merged)
}

/// Phase 454 (CONC-03): `GET /api/v2/concilium/placement` hybrid Populi node placement map.
pub async fn get_concilium_placement(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_dashboard_read(&gs, &connect.0, &headers) {
        return e;
    }
    to_json(&build_placement(&gs))
}

/// Phase 454 (CONC-03): `GET /api/v2/concilium/action?payload=<url-encoded JSON>`
pub async fn get_concilium_action(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ActionQuery>,
) -> Json<Value> {
    if let Err(e) = reject_cross_site_get(&headers) {
        return e;
    }
    if let Err(e) = enforce_dashboard_write(&gs, &connect.0, &headers) {
        return e;
    }
    match serde_json::from_str::<ConciliumPilotActionInput>(&q.payload) {
        Ok(input) => execute_pilot_action(&gs, input).await,
        Err(e) => err("bad_request", &format!("invalid action payload JSON: {e}")),
    }
}

/// Phase 454 (CONC-03): `POST /api/v2/concilium/action` pilot steering mutation handler.
pub async fn post_concilium_action(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(input): Json<ConciliumPilotActionInput>,
) -> Json<Value> {
    if let Err(e) = enforce_dashboard_write(&gs, &connect.0, &headers) {
        return e;
    }
    execute_pilot_action(&gs, input).await
}

#[cfg(test)]
#[path = "concilium_api/tests.rs"]
mod tests;
