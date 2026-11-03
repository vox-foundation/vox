//! Attention-profile routes for Axis (plan 07 Task 9). D1: the `attention` profile is DB-free,
//! so these routes read `state.db` when a DB is attached and degrade to zeros otherwise.
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;

use crate::gui_ops::dto::attention::LlmSpendDto;
use crate::gui_ops::enforce_route;
use crate::http_gateway::GatewayState;
use crate::server_state::ServerState;
use crate::services::routes::ok;

const SPEND_SURFACES: &[&str] = &["settings"];

/// Recorded spend from the attached DB (zeros when none) plus the server's budget caps.
pub async fn llm_spend(state: &ServerState, session_id: Option<&str>) -> LlmSpendDto {
    let cfg = vox_config::VoxConfig::load();
    let spend = match &state.db {
        Some(db) => db.llm_spend_summary(session_id).await.unwrap_or_default(),
        None => Default::default(),
    };
    LlmSpendDto {
        session_usd: spend.session_usd,
        day_usd: spend.day_usd,
        total_usd: spend.total_usd,
        daily_budget_usd: cfg.daily_budget_usd,
        per_session_budget_usd: cfg.per_session_budget_usd,
    }
}

#[derive(Debug, Deserialize)]
pub struct SpendQuery {
    pub session_id: Option<String>,
}

/// GET /api/v2/llm/spend?session_id=
pub async fn get_llm_spend(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<SpendQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, SPEND_SURFACES) {
        return e;
    }
    let session_id = q.session_id.as_deref().filter(|s| !s.trim().is_empty());
    ok(llm_spend(gs.server_state(), session_id).await)
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new().route("/llm/spend", get(get_llm_spend))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer};

    #[test]
    fn router_builds() {
        let _ = api_v2_router();
    }

    fn query(session_id: Option<&str>) -> Query<SpendQuery> {
        Query(SpendQuery {
            session_id: session_id.map(str::to_string),
        })
    }

    #[tokio::test]
    async fn no_db_returns_zero_spend_with_server_caps() {
        let gs =
            GatewayState::for_test_with_state(ServerState::new_test().await, HashSet::new()).await;
        assert!(gs.server_state().db.is_none());
        let v =
            body(get_llm_spend(State(gs), peer(), HeaderMap::new(), query(Some("s1"))).await).await;
        let dto: LlmSpendDto = serde_json::from_value(v["data"].clone()).expect("{v}");
        let cfg = vox_config::VoxConfig::load();
        assert_eq!(
            dto,
            LlmSpendDto {
                daily_budget_usd: cfg.daily_budget_usd,
                per_session_budget_usd: cfg.per_session_budget_usd,
                ..Default::default()
            }
        );
    }

    #[tokio::test]
    async fn memory_db_returns_numeric_envelope() {
        let (gs, _db) = gateway_with_memory_db().await;
        let v = body(get_llm_spend(State(gs), peer(), HeaderMap::new(), query(None)).await).await;
        for key in [
            "sessionUsd",
            "dayUsd",
            "totalUsd",
            "dailyBudgetUsd",
            "perSessionBudgetUsd",
        ] {
            assert!(v["data"][key].is_number(), "{key}: {v}");
        }
    }

    #[tokio::test]
    async fn attention_profile_serves_spend_and_mesh_does_not() {
        for (profile, allowed) in [("attention", true), ("mesh", false)] {
            let state = ServerState::new_test().await.with_profile(
                crate::profiles::ActiveProfile::resolve(&[profile]).expect("profile"),
            );
            let gs = GatewayState::for_test_with_state(state, HashSet::new()).await;
            let v =
                body(get_llm_spend(State(gs), peer(), HeaderMap::new(), query(None)).await).await;
            if allowed {
                assert!(v["data"]["dailyBudgetUsd"].is_number(), "{profile}: {v}");
            } else {
                assert_eq!(v["error"]["code"], "tool_not_in_profile", "{profile}: {v}");
            }
        }
    }
}
