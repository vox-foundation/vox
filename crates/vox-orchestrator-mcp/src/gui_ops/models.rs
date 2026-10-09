//! Models-profile routes and ops for Axis (plan 07 Tasks 2–3).
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::gui_ops::dto::models::{ModelsPreferencesDto, RoutingPriorityDto, apply_routing_csv};
use crate::gui_ops::{GUI_USER_ID, enforce_dashboard_surface, require_db};
use crate::http_gateway::GatewayState;
use crate::http_gateway::dashboard_api::enforce_dashboard_read;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::services::routes::ok;

pub const EMPTY_SELECTION_POLICY: &str = "{\"steps\":[]}";
const REM_MODELS_KEY: &str = "Only active_model, routing_priority and selection_policy are models preferences; use the owning surface's tool for other keys.";
const REM_MODELS_PREF_DB: &str =
    "Verify Turso connectivity and vox-db migrations for user_preferences.";

/// Keys the Models/Settings surfaces persist. Anything else (e.g. `socrates_gate_enforced`,
/// which `vox_preference_set` also applies to live orchestrator config) is refused.
pub const MODELS_PREFERENCE_KEYS: &[&str] =
    &["active_model", "routing_priority", "selection_policy"];
const ROUTING_AXES: &[&str] = &[
    "efficiency",
    "cost",
    "precision",
    "quality",
    "latency",
    "speed",
    "availability",
    "balance",
    "mobile",
];

async fn pref(state: &ServerState, key: &str) -> Option<String> {
    let db = state.db.as_ref()?;
    db.get_user_preference(GUI_USER_ID, key)
        .await
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
}

pub(crate) async fn effective_routing_priority(state: &ServerState) -> RoutingPriorityDto {
    let base: RoutingPriorityDto = vox_config::AutoRoutingPriority::from_env().into();
    match pref(state, "routing_priority").await {
        Some(csv) => apply_routing_csv(base, &csv),
        None => base,
    }
}

pub(crate) async fn models_preferences(state: &ServerState) -> ModelsPreferencesDto {
    ModelsPreferencesDto {
        active_model: pref(state, "active_model").await,
        routing_priority: effective_routing_priority(state).await,
        selection_policy: pref(state, "selection_policy")
            .await
            .unwrap_or_else(|| EMPTY_SELECTION_POLICY.to_string()),
    }
}

/// GET /api/v2/models/preferences
pub async fn get_models_preferences(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_dashboard_read(&gs, &connect.0, &headers) {
        return e;
    }
    if let Err(e) = enforce_dashboard_surface(&gs, &["models", "settings"]) {
        return e;
    }
    ok(models_preferences(gs.server_state()).await)
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ModelsPreferenceSetParams {
    /// One of `active_model`, `routing_priority`, `selection_policy`.
    pub key: String,
    /// Model id, `axis=u8,...` CSV, or `SelectionPolicy` JSON respectively.
    pub value: String,
}

pub fn validate_routing_priority_csv(csv: &str) -> Result<(), String> {
    for part in csv.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let (k, v) = part
            .split_once('=')
            .ok_or_else(|| format!("expected axis=value, got {part:?}"))?;
        let k = k.trim().to_ascii_lowercase();
        if !ROUTING_AXES.contains(&k.as_str()) {
            return Err(format!("unknown routing axis {k:?}"));
        }
        v.trim()
            .parse::<u8>()
            .map_err(|_| format!("axis {k} must be 0-255, got {v:?}"))?;
    }
    Ok(())
}

fn validate_models_preference(state: &ServerState, key: &str, value: &str) -> Result<(), String> {
    match key {
        "active_model" => {
            let id = value.trim();
            if id.is_empty() {
                return Err("model_id must not be empty".into());
            }
            let handle = state.orchestrator.models_handle();
            if vox_orchestrator::sync_lock::rw_read(&*handle)
                .get(id)
                .is_none()
            {
                return Err(format!("model {id} not found in registry"));
            }
            Ok(())
        }
        "routing_priority" => validate_routing_priority_csv(value),
        "selection_policy" => vox_orchestrator::models::SelectionPolicy::from_json(value)
            .map(|_| ())
            .map_err(|e| format!("invalid selection policy JSON: {e}")),
        other => Err(format!("key '{other}' is not a models preference")),
    }
}

pub async fn models_preference_set(state: &ServerState, p: ModelsPreferenceSetParams) -> String {
    if !MODELS_PREFERENCE_KEYS.contains(&p.key.as_str()) {
        return ToolResult::<String>::err_with_remediation(
            format!("key '{}' is not a models preference", p.key),
            REM_MODELS_KEY,
        )
        .to_json();
    }
    if let Err(e) = validate_models_preference(state, &p.key, &p.value) {
        return ToolResult::<String>::err_with_remediation(e, REM_MODELS_KEY).to_json();
    }
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(e, REM_MODELS_PREF_DB).to_json();
        }
    };
    match db
        .set_user_preference(GUI_USER_ID, &p.key, p.value.trim())
        .await
    {
        Ok(()) => ToolResult::ok(p.key).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(e.to_string(), REM_MODELS_PREF_DB).to_json()
        }
    }
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new().route("/models/preferences", get(get_models_preferences))
}

#[cfg(test)]
mod tests {
    use axum::extract::State;
    use axum::http::HeaderMap;

    use crate::gui_ops::test_support::{
        body, gateway_with_memory_db, peer, state_with_memory_db, tool_json,
    };

    #[test]
    fn router_builds() {
        let _ = super::api_v2_router();
    }

    #[tokio::test]
    async fn preferences_route_reads_persisted_rows_with_defaults() {
        let (gs, db) = gateway_with_memory_db().await;
        db.set_user_preference("local_user", "active_model", "openai/gpt-x")
            .await
            .unwrap();
        db.set_user_preference("local_user", "routing_priority", "latency=77")
            .await
            .unwrap();
        let v =
            body(super::get_models_preferences(State(gs), peer(), HeaderMap::new()).await).await;
        assert_eq!(v["data"]["active_model"], "openai/gpt-x");
        assert_eq!(v["data"]["routing_priority"]["latency"], 77);
        assert_eq!(v["data"]["selection_policy"], "{\"steps\":[]}");
    }

    #[tokio::test]
    async fn preferences_route_degrades_without_db() {
        let gs = crate::http_gateway::GatewayState::for_test().await;
        let v =
            body(super::get_models_preferences(State(gs), peer(), HeaderMap::new()).await).await;
        assert!(v["data"]["active_model"].is_null());
        assert!(v["data"]["routing_priority"].is_object());
    }

    #[tokio::test]
    async fn models_preference_set_rejects_keys_outside_allowlist() {
        let (state, db) = state_with_memory_db().await;
        let out = super::models_preference_set(
            &state,
            super::ModelsPreferenceSetParams {
                key: "socrates_gate_enforced".into(),
                value: "false".into(),
            },
        )
        .await;
        assert_eq!(tool_json(&out)["success"], false);
        assert_eq!(
            db.get_user_preference("local_user", "socrates_gate_enforced")
                .await
                .unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn models_preference_set_validates_selection_policy_json() {
        let (state, _db) = state_with_memory_db().await;
        let out = super::models_preference_set(
            &state,
            super::ModelsPreferenceSetParams {
                key: "selection_policy".into(),
                value: "{not json".into(),
            },
        )
        .await;
        assert!(
            tool_json(&out)["error"]
                .as_str()
                .unwrap()
                .contains("invalid selection policy JSON")
        );
    }

    #[tokio::test]
    async fn models_preference_set_persists_routing_priority() {
        let (state, db) = state_with_memory_db().await;
        let csv = "efficiency=25,precision=30,latency=15,availability=10,balance=10,mobile=10";
        let out = super::models_preference_set(
            &state,
            super::ModelsPreferenceSetParams {
                key: "routing_priority".into(),
                value: csv.into(),
            },
        )
        .await;
        assert_eq!(tool_json(&out)["success"], true);
        assert_eq!(
            db.get_user_preference("local_user", "routing_priority")
                .await
                .unwrap()
                .as_deref(),
            Some(csv)
        );
    }

    #[test]
    fn routing_priority_csv_rejects_unknown_axis_and_overflow() {
        assert!(super::validate_routing_priority_csv("efficiency=25,precision=30").is_ok());
        assert!(super::validate_routing_priority_csv("bogus=1").is_err());
        assert!(super::validate_routing_priority_csv("latency=300").is_err());
    }
}
