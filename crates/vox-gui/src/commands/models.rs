//! Tauri commands for the model registry, routing preferences and scoreboard surfaces, served by
//! the daemon (`GET /api/v2/models/*` and `vox_models_preference_set`) so they work against a
//! local or a remote Vox server.

use std::sync::Arc;

use serde_json::json;
pub use vox_orchestrator_mcp::gui_ops::dto::models::{
    ModelCardDto, ModelsPreferencesDto, ModelsSnapshotDto, NudgeDirection, ProviderStatusDto,
    RoutingIntentionDto, RoutingPriorityDto, RoutingSummaryDto, ScoreboardRowDto,
};
use vox_orchestrator_mcp::gui_ops::dto::models::{
    axis_value, nudge_axis, priority_to_csv, priority_to_intentions,
};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query, tool_data};

const PREF_TOOL: &str = "vox_models_preference_set";

/// Empty-policy JSON returned when no `selection_policy` preference is persisted.
const EMPTY_SELECTION_POLICY: &str = "{\"steps\":[]}";

fn limit_query(path: &str, limit: Option<usize>) -> String {
    path_with_query(path, &[("limit", limit.map(|l| l.to_string()))])
}

async fn preferences<B: BackendCalls>(b: &B) -> Result<ModelsPreferencesDto, String> {
    api_data(b.api_v2_get("/models/preferences").await?)
}

async fn set_preference<B: BackendCalls>(b: &B, key: &str, value: String) -> Result<(), String> {
    let env = b
        .call_tool(PREF_TOOL, json!({ "key": key, "value": value }))
        .await?;
    tool_data::<String>(PREF_TOOL, env).map(|_| ())
}

pub(crate) async fn list_model_cards_impl<B: BackendCalls>(
    b: &B,
    limit: Option<usize>,
) -> Result<Vec<ModelCardDto>, String> {
    api_data(b.api_v2_get(&limit_query("/models/cards", limit)).await?)
}

pub(crate) async fn get_models_snapshot_impl<B: BackendCalls>(
    b: &B,
    limit: Option<usize>,
) -> Result<ModelsSnapshotDto, String> {
    api_data(
        b.api_v2_get(&limit_query("/models/snapshot", limit))
            .await?,
    )
}

pub(crate) async fn get_routing_summary_impl<B: BackendCalls>(
    b: &B,
) -> Result<RoutingSummaryDto, String> {
    api_data(b.api_v2_get("/models/routing-summary").await?)
}

pub(crate) async fn get_model_scoreboard_impl<B: BackendCalls>(
    b: &B,
    window_days: Option<i64>,
) -> Result<Vec<ScoreboardRowDto>, String> {
    let path = path_with_query(
        "/models/scoreboard",
        &[("window_days", window_days.map(|w| w.to_string()))],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn inference_provider_status_impl<B: BackendCalls>(
    b: &B,
) -> Result<Vec<ProviderStatusDto>, String> {
    api_data(b.api_v2_get("/models/provider-status").await?)
}

pub(crate) async fn get_selection_policy_impl<B: BackendCalls>(b: &B) -> String {
    preferences(b)
        .await
        .ok()
        .map(|p| p.selection_policy)
        .unwrap_or_else(|| EMPTY_SELECTION_POLICY.to_string())
}

pub(crate) async fn set_selection_policy_impl<B: BackendCalls>(
    b: &B,
    json: String,
) -> Result<(), String> {
    vox_orchestrator::models::SelectionPolicy::from_json(&json)
        .map_err(|e| format!("invalid selection policy JSON: {e}"))?;
    set_preference(b, "selection_policy", json).await
}

pub(crate) async fn set_routing_priority_impl<B: BackendCalls>(
    b: &B,
    p: RoutingPriorityDto,
) -> Result<(), String> {
    set_preference(b, "routing_priority", priority_to_csv(&p)).await
}

pub(crate) async fn get_routing_intentions_impl<B: BackendCalls>(
    b: &B,
) -> Result<Vec<RoutingIntentionDto>, String> {
    Ok(priority_to_intentions(
        &preferences(b).await?.routing_priority,
    ))
}

pub(crate) async fn nudge_routing_intention_impl<B: BackendCalls>(
    b: &B,
    axis: String,
    direction: String,
) -> Result<(), String> {
    let dir = match direction.to_ascii_lowercase().as_str() {
        "promote" => NudgeDirection::Promote,
        "doubt" => NudgeDirection::Doubt,
        other => {
            return Err(format!(
                "unknown direction {other:?} (expected promote|doubt)"
            ));
        }
    };
    let base = preferences(b).await?.routing_priority;
    if axis_value(&base, &axis).is_none() {
        return Err(format!(
            "unknown routing axis {axis:?} (expected one of efficiency|precision|latency|availability|balance|mobile)"
        ));
    }
    set_routing_priority_impl(b, nudge_axis(&base, &axis, dir)).await
}

#[tauri::command]
pub async fn list_model_cards(
    limit: Option<usize>,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<Vec<ModelCardDto>, String> {
    list_model_cards_impl(&backend.current().await, limit).await
}

/// One-call refresh of the Models surface (cards, routing summary, active model, providers).
#[tauri::command]
pub async fn get_models_snapshot(
    limit: Option<usize>,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<ModelsSnapshotDto, String> {
    get_models_snapshot_impl(&backend.current().await, limit).await
}

/// Live routing summary as the server sees it.
#[tauri::command]
pub async fn get_routing_summary_live(
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<RoutingSummaryDto, String> {
    get_routing_summary_impl(&backend.current().await).await
}

#[tauri::command]
pub async fn set_routing_priority(
    efficiency: u8,
    precision: u8,
    latency: u8,
    availability: u8,
    balance: u8,
    mobile: u8,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<(), String> {
    let p = RoutingPriorityDto {
        efficiency,
        precision,
        latency,
        availability,
        balance,
        mobile,
    };
    set_routing_priority_impl(&backend.current().await, p).await
}

#[tauri::command]
pub async fn get_model_scoreboard(
    window_days: Option<i64>,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<Vec<ScoreboardRowDto>, String> {
    get_model_scoreboard_impl(&backend.current().await, window_days).await
}

/// Per-backend availability (B9) as seen by the server: credential presence for every
/// candidate provider plus live local-server health.
#[tauri::command]
pub async fn inference_provider_status(
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<Vec<ProviderStatusDto>, String> {
    inference_provider_status_impl(&backend.current().await).await
}

/// The persisted `selection_policy` preference (JSON for a
/// [`vox_orchestrator::models::SelectionPolicy`]), or `{"steps":[]}` when absent or unreachable.
#[tauri::command]
pub async fn get_selection_policy(
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<String, String> {
    Ok(get_selection_policy_impl(&backend.current().await).await)
}

/// Persist a `selection_policy` preference after checking it parses as a `SelectionPolicy`.
/// The daemon installs the policy at startup, so it takes effect on the next (re)start.
#[tauri::command]
pub async fn set_selection_policy(
    json: String,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<(), String> {
    set_selection_policy_impl(&backend.current().await, json).await
}

/// Live "intentions" for the Matrix surface: the real routing-priority axes, derived from the
/// server's effective (preference → env → default) priority.
#[tauri::command]
pub async fn get_routing_intentions(
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<Vec<RoutingIntentionDto>, String> {
    get_routing_intentions_impl(&backend.current().await).await
}

/// Promote or Doubt a single routing axis from the Matrix surface and persist it as the
/// `routing_priority` preference, which the next orchestrator decision reads.
#[tauri::command]
pub async fn nudge_routing_intention(
    axis: String,
    direction: String,
    backend: tauri::State<'_, Arc<BackendState>>,
) -> Result<(), String> {
    nudge_routing_intention_impl(&backend.current().await, axis, direction).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    fn prefs_env() -> serde_json::Value {
        json!({"v": 1, "data": {"active_model": null, "selection_policy": "{\"steps\":[]}",
            "routing_priority": {"efficiency": 20, "precision": 20, "latency": 20,
                "availability": 20, "balance": 10, "mobile": 10}}})
    }

    fn pref_ok() -> FakeBackend {
        FakeBackend::new()
            .on_get("/models/preferences", Ok(prefs_env()))
            .on_tool(
                PREF_TOOL,
                Ok(json!({"success": true, "data": "routing_priority"})),
            )
    }

    #[tokio::test]
    async fn nudge_reads_preferences_then_writes_csv_through_op() {
        let b = pref_ok();
        nudge_routing_intention_impl(&b, "latency".into(), "promote".into())
            .await
            .unwrap();
        let calls = b.calls();
        assert_eq!(calls[0].0, "get:/models/preferences");
        assert_eq!(calls[1].0, format!("tool:{PREF_TOOL}"));
        assert_eq!(calls[1].1["key"], "routing_priority");
        assert!(calls[1].1["value"].as_str().unwrap().contains("latency=25"));
    }

    #[tokio::test]
    async fn nudge_rejects_unknown_axis_and_direction_without_writing() {
        let b = pref_ok();
        assert!(
            nudge_routing_intention_impl(&b, "latency".into(), "up".into())
                .await
                .is_err()
        );
        assert!(
            nudge_routing_intention_impl(&b, "nonsense".into(), "promote".into())
                .await
                .is_err()
        );
        assert!(b.calls().iter().all(|(k, _)| !k.starts_with("tool:")));
    }

    #[tokio::test]
    async fn set_routing_priority_writes_full_csv() {
        let b = pref_ok();
        let p = RoutingPriorityDto {
            efficiency: 40,
            precision: 30,
            latency: 10,
            availability: 10,
            balance: 5,
            mobile: 5,
        };
        set_routing_priority_impl(&b, p).await.unwrap();
        assert_eq!(
            b.calls()[0].1,
            json!({"key": "routing_priority", "value": "efficiency=40,precision=30,latency=10,availability=10,balance=5,mobile=5"})
        );
    }

    #[tokio::test]
    async fn intentions_project_server_priority() {
        let b = pref_ok();
        let cells = get_routing_intentions_impl(&b).await.unwrap();
        assert_eq!(cells.len(), 6);
        assert!((cells[0].conf - 0.20).abs() < 1e-6);
    }

    #[tokio::test]
    async fn selection_policy_validates_locally_and_degrades_to_empty() {
        let b = pref_ok();
        assert!(
            set_selection_policy_impl(&b, "not json".into())
                .await
                .is_err()
        );
        assert!(b.calls().is_empty());
        set_selection_policy_impl(&b, "{\"steps\":[]}".into())
            .await
            .unwrap();
        assert_eq!(b.calls()[0].1["key"], "selection_policy");

        assert_eq!(
            get_selection_policy_impl(&FakeBackend::new()).await,
            EMPTY_SELECTION_POLICY
        );
    }

    #[tokio::test]
    async fn reads_hit_the_expected_routes() {
        let empty = Ok(json!({"v": 1, "data": []}));
        let b = FakeBackend::new()
            .on_get("/models/cards?limit=5", empty.clone())
            .on_get("/models/scoreboard?window_days=3", empty.clone())
            .on_get("/models/provider-status", empty);
        assert!(list_model_cards_impl(&b, Some(5)).await.unwrap().is_empty());
        assert!(
            get_model_scoreboard_impl(&b, Some(3))
                .await
                .unwrap()
                .is_empty()
        );
        assert!(inference_provider_status_impl(&b).await.unwrap().is_empty());
        let keys: Vec<_> = b.calls().into_iter().map(|c| c.0).collect();
        assert_eq!(
            keys,
            [
                "get:/models/cards?limit=5",
                "get:/models/scoreboard?window_days=3",
                "get:/models/provider-status"
            ]
        );
    }

    #[tokio::test]
    async fn snapshot_and_summary_surface_server_errors() {
        let denied = Ok(json!({"v": 1, "error": {"code": "tool_not_in_profile", "message": "no"}}));
        let b = FakeBackend::new()
            .on_get("/models/snapshot", denied.clone())
            .on_get("/models/routing-summary", denied);
        assert!(get_models_snapshot_impl(&b, None).await.is_err());
        assert!(get_routing_summary_impl(&b).await.is_err());
    }

    #[test]
    fn models_module_has_no_direct_db_access_or_env_mutation() {
        let src = include_str!("models.rs");
        assert!(!src.contains(concat!("vox", "_db")));
        assert!(
            !src.contains(concat!("set_", "var(")),
            "routing priority must not mutate the GUI process env"
        );
    }
}
