//! Models-profile routes and ops for Axis (plan 07 Tasks 2–3).
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use vox_orchestrator::catalog::{MensCatalog, ModelCatalog};
use vox_orchestrator::models::{
    ModelRegistry, ModelSelectionRequest, ModelSpec, SelectionIntent, TaskCategory, decide,
};

use crate::gui_ops::dto::models::{
    DecisionPreviewDto, ModelCardDto, ModelsPreferencesDto, ModelsSnapshotDto, ProviderStatusDto,
    RoutingPriorityDto, RoutingSummaryDto, ScoreboardRowDto, apply_routing_csv, model_spec_to_card,
};
use crate::gui_ops::dto::routing_explain;
use crate::gui_ops::{GUI_USER_ID, api_db, enforce_dashboard_surface, require_db};
use crate::http_gateway::GatewayState;
use crate::http_gateway::dashboard_api::enforce_dashboard_read;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::services::routes::{err, ok};

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

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct WindowQuery {
    pub window_days: Option<i64>,
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
        .route("/models/preferences", get(get_models_preferences))
        .route("/models/cards", get(get_model_cards))
        .route("/models/routing-summary", get(get_routing_summary_gui))
        .route("/models/scoreboard", get(get_model_scoreboard_route))
        .route("/models/provider-status", get(get_provider_status))
        .route("/models/snapshot", get(get_models_snapshot))
        .route("/models/routing-explain", get(get_routing_explanation))
        .route("/models/routing-health", get(get_routing_health))
}

fn guard_models(
    gs: &GatewayState,
    peer: &SocketAddr,
    headers: &HeaderMap,
) -> Result<(), Json<Value>> {
    enforce_dashboard_read(gs, peer, headers)?;
    enforce_dashboard_surface(gs, &["models", "settings", "runs"])
}

/// Cached registry plus the persisted scoreboard and pricing catalog, the way `vox model explain`
/// builds it. Degrades to the bare cached registry without a DB: listing models must not fail
/// because telemetry is unavailable.
pub async fn registry_with_scoreboard(db: Option<&Arc<vox_db::VoxDb>>) -> ModelRegistry {
    let mut reg = ModelRegistry::from_cache();
    let Some(db) = db else {
        return reg;
    };
    if let Ok(rows) = db.get_model_scoreboard(7).await {
        reg.inject_scoreboard_latency(&rows);
        reg.inject_scoreboard(
            rows.into_iter()
                .map(|row| {
                    (
                        row.model_id.clone(),
                        vox_orchestrator::models::ModelScore::from(row),
                    )
                })
                .collect(),
        );
    }
    if let Ok(pricing) = db.get_pricing_catalog().await {
        reg.inject_pricing_catalog(pricing);
    }
    reg.apply_routing_reference();
    reg
}

/// Union live Mens runs into `reg` (local FS scan only — no OpenRouter fetch).
/// Live rows overwrite cache/bootstrap entries for the same id via [`ModelRegistry::register`].
fn merge_live_mens_specs(reg: &mut ModelRegistry, live: impl IntoIterator<Item = ModelSpec>) {
    for spec in live {
        reg.register(spec);
    }
}

/// Cheap local catalog refresh: scan `mens/runs/` under the workspace root.
async fn refresh_live_mens_catalog(reg: &mut ModelRegistry) {
    let repo_root = vox_repository::resolve_repo_root_for_ci();
    if let Ok(models) = MensCatalog::new(&repo_root).refresh().await {
        merge_live_mens_specs(reg, models);
    }
}

pub async fn model_cards(st: &ServerState, limit: usize) -> Vec<ModelCardDto> {
    let mut reg = registry_with_scoreboard(st.db.as_ref()).await;
    refresh_live_mens_catalog(&mut reg).await;
    // Warm OpenRouter live TTL cache in the background so autocomplete is hot.
    tokio::spawn(async {
        vox_orchestrator::catalog_live::warm_openrouter_live_cache_if_keyed().await;
    });
    let mut models = reg.list_models();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models
        .into_iter()
        .take(limit)
        .map(|m| {
            let sb = reg.scoreboard_snapshot().get(&m.id);
            model_spec_to_card(&m, sb.map(|s| s.success_rate))
        })
        .collect()
}

/// Family, provenance and reason of a routing pick, for [`RoutingSummaryDto`].
fn routing_provenance(
    spec: &ModelSpec,
    reason: &vox_orchestrator::models::select::SelectionReason,
) -> (String, &'static str, String) {
    (
        vox_orchestrator::models::family::family_key(&spec.id),
        vox_orchestrator::models::provenance::resolved_from(spec).as_str(),
        reason.to_string(),
    )
}

pub async fn routing_summary(st: &ServerState) -> RoutingSummaryDto {
    let reg = ModelRegistry::from_cache();
    let cfg = vox_config::load_model_routing_config();
    let prefs = models_preferences(st).await;
    let decision = decide(
        &ModelSelectionRequest::from_intent(SelectionIntent::for_task(TaskCategory::CodeGen)),
        &reg,
    );
    let (family, resolved_from, reason) = match &decision {
        Some(d) => {
            let (family, from, reason) =
                routing_provenance(&d.outcome.model_spec, &d.outcome.reason);
            (Some(family), Some(from.to_string()), Some(reason))
        }
        None => (None, None, None),
    };
    let decision_preview = decision.map(|d| DecisionPreviewDto {
        selected_model: d.selected_model,
        discovery_state: d.discovery_state.as_str().to_string(),
        alternatives: d.alternatives,
        rejection_reasons: d.rejection_reasons,
        intelligence_score: d.score_breakdown.intelligence_score,
        efficiency_score: d.score_breakdown.efficiency_score,
        latency_score: d.score_breakdown.latency_score,
    });
    RoutingSummaryDto {
        active_model: prefs.active_model,
        exploration_spent_usd: st.orchestrator.status().global_exploration_cost_usd,
        exploration_budget_usd: cfg.exploration.budget_usd_per_day,
        routing_priority: prefs.routing_priority,
        arm_count: reg.arm_stats_snapshot().len(),
        model_count: reg.list_models().len(),
        decision_preview,
        family,
        resolved_from,
        reason,
    }
}

/// Ollama / PopuliMesh share the `:11434` probe. VoxLocal is a different
/// server (`VOX_LOCAL_ENDPOINT`) — do not mark it unreachable when Ollama is down.
fn local_availability(
    p: vox_orchestrator::models::ProviderType,
    ollama_reachable: bool,
    ollama_models: &[String],
    vox_local_reachable: Option<bool>,
    vox_local_models: &[String],
) -> (bool, Option<bool>, Vec<String>) {
    use vox_orchestrator::models::ProviderType;
    match p {
        ProviderType::Ollama | ProviderType::PopuliMesh => {
            (true, Some(ollama_reachable), ollama_models.to_vec())
        }
        ProviderType::VoxLocal => (true, vox_local_reachable, vox_local_models.to_vec()),
        _ => (false, None, Vec::new()),
    }
}

fn parse_vox_local_model_ids(body: &serde_json::Value) -> Vec<String> {
    body.get("data")
        .and_then(|d| d.as_array())
        .map(|rows| {
            rows.iter()
                .filter_map(|row| row.get("id").and_then(|id| id.as_str()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Readiness probe for `vox-ml-cli serve` over the given base URLs; also used by Axis
/// `mens_serve` against the one port it just spawned.
pub async fn probe_vox_local_bases(candidates: &[String]) -> (Option<bool>, Vec<String>) {
    if candidates.is_empty() {
        return (Some(false), Vec::new());
    }
    let client = vox_http_client::client();
    let timeout = std::time::Duration::from_secs(2);
    for base in candidates {
        let mut identifies_serve = false;
        for path in ["/health", "/ready"] {
            let url = format!("{base}{path}");
            match client.get(url).timeout(timeout).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(body) = resp.text().await
                        && vox_config::inference::vox_local_health_identifies_serve(&body)
                    {
                        identifies_serve = true;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !identifies_serve {
            continue;
        }
        let models = match client
            .get(format!("{base}/v1/models"))
            .timeout(timeout)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => resp
                .json::<serde_json::Value>()
                .await
                .ok()
                .map(|body| parse_vox_local_model_ids(&body))
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        return (Some(true), models);
    }
    (Some(false), Vec::new())
}

/// Per-backend availability (B9): credential presence for every candidate provider plus
/// live local-server health from the shared TTL-cached probe, as seen by the server.
pub async fn provider_status() -> Vec<ProviderStatusDto> {
    let statuses = vox_orchestrator::models::key_guard::inference_provider_statuses();
    let base = vox_config::inference::local_ollama_populi_base_url();
    let probe = vox_actor_runtime::inference_env::probe_populi_capabilities_cached(
        &base,
        vox_config::timeouts::D_15S,
    )
    .await;
    let candidates = vox_config::inference::vox_local_endpoint_probe_candidates();
    let (vox_local_reachable, vox_local_models) = probe_vox_local_bases(&candidates).await;
    statuses
        .into_iter()
        .map(|(p, key_present)| {
            let provider = format!("{p:?}");
            let (is_local, local_reachable, local_models) = local_availability(
                p,
                probe.reachable,
                &probe.model_names,
                vox_local_reachable,
                &vox_local_models,
            );
            ProviderStatusDto {
                provider,
                key_present,
                is_local,
                local_reachable,
                local_models,
            }
        })
        .collect()
}

/// GET /api/v2/models/cards?limit=
pub async fn get_model_cards(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<LimitQuery>,
) -> Json<Value> {
    if let Err(e) = guard_models(&gs, &connect.0, &headers) {
        return e;
    }
    ok(model_cards(gs.server_state(), q.limit.unwrap_or(2000)).await)
}

/// GET /api/v2/models/routing-summary
pub async fn get_routing_summary_gui(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = guard_models(&gs, &connect.0, &headers) {
        return e;
    }
    ok(routing_summary(gs.server_state()).await)
}

/// GET /api/v2/models/scoreboard?window_days=
pub async fn get_model_scoreboard_route(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<WindowQuery>,
) -> Json<Value> {
    if let Err(e) = guard_models(&gs, &connect.0, &headers) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db.get_model_scoreboard(q.window_days.unwrap_or(7)).await {
        Ok(rows) => ok(rows
            .into_iter()
            .map(ScoreboardRowDto::from)
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/models/provider-status
pub async fn get_provider_status(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = guard_models(&gs, &connect.0, &headers) {
        return e;
    }
    ok(provider_status().await)
}

/// GET /api/v2/models/snapshot?limit=
pub async fn get_models_snapshot(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<LimitQuery>,
) -> Json<Value> {
    if let Err(e) = guard_models(&gs, &connect.0, &headers) {
        return e;
    }
    let st = gs.server_state();
    ok(ModelsSnapshotDto {
        cards: model_cards(st, q.limit.unwrap_or(2000)).await,
        routing: routing_summary(st).await,
        active_model: models_preferences(st).await.active_model,
        providers: provider_status().await,
    })
}

#[derive(Debug, Deserialize)]
pub struct RoutingExplainQuery {
    pub task: String,
    pub mode: String,
    pub complexity: u8,
}

/// GET /api/v2/models/routing-explain?task=&mode=&complexity= — how task dispatch would choose now.
pub async fn get_routing_explanation(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<RoutingExplainQuery>,
) -> Json<Value> {
    if let Err(e) = guard_models(&gs, &connect.0, &headers) {
        return e;
    }
    let Some(clutch) = vox_orchestrator::mode::ClutchProfile::from_label(&q.mode) else {
        return err("bad_request", "unknown mode");
    };
    let Some(task_category) = routing_explain::task_category_from_label(&q.task) else {
        return err("bad_request", "unknown task");
    };
    let registry = registry_with_scoreboard(gs.server_state().db.as_ref()).await;
    ok(routing_explain::explain(
        &registry,
        clutch,
        task_category,
        q.complexity.clamp(1, 10),
    ))
}

/// GET /api/v2/models/routing-health — the registry's routing health, checked now.
pub async fn get_routing_health(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = guard_models(&gs, &connect.0, &headers) {
        return e;
    }
    let registry = registry_with_scoreboard(gs.server_state().db.as_ref()).await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    ok(vox_orchestrator::models::health::check_routing_health(
        &registry, now,
    ))
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

    use axum::extract::Query;

    #[tokio::test]
    async fn scoreboard_route_returns_rows_array() {
        let (gs, _db) = gateway_with_memory_db().await;
        let q = Query(super::WindowQuery {
            window_days: Some(7),
        });
        let v =
            body(super::get_model_scoreboard_route(State(gs), peer(), HeaderMap::new(), q).await)
                .await;
        assert!(v["data"].is_array(), "{v}");
    }

    #[tokio::test]
    async fn scoreboard_route_reports_missing_db() {
        let gs = crate::http_gateway::GatewayState::for_test().await;
        let q = Query(super::WindowQuery { window_days: None });
        let v =
            body(super::get_model_scoreboard_route(State(gs), peer(), HeaderMap::new(), q).await)
                .await;
        assert_eq!(v["error"]["code"], "db_unavailable");
    }

    #[tokio::test]
    async fn snapshot_bundles_cards_routing_active_and_providers() {
        let (gs, db) = gateway_with_memory_db().await;
        db.set_user_preference("local_user", "active_model", "openai/gpt-x")
            .await
            .unwrap();
        let q = Query(super::LimitQuery { limit: Some(5) });
        let v =
            body(super::get_models_snapshot(State(gs), peer(), HeaderMap::new(), q).await).await;
        assert!(v["data"]["cards"].as_array().unwrap().len() <= 5);
        assert_eq!(v["data"]["active_model"], "openai/gpt-x");
        assert_eq!(v["data"]["routing"]["active_model"], "openai/gpt-x");
        assert!(v["data"]["routing"]["routing_priority"].is_object());
        assert!(v["data"]["providers"].is_array());
    }

    #[tokio::test]
    async fn models_routes_are_profile_gated() {
        let (state, _db) = state_with_memory_db().await;
        let state = state.with_profile(
            crate::profiles::ActiveProfile::resolve(&["knowledge"]).expect("knowledge"),
        );
        let gs =
            crate::http_gateway::GatewayState::for_test_with_state(state, Default::default()).await;
        let v =
            body(super::get_models_preferences(State(gs.clone()), peer(), HeaderMap::new()).await)
                .await;
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
        let v = body(
            super::get_models_snapshot(
                State(gs),
                peer(),
                HeaderMap::new(),
                Query(super::LimitQuery { limit: None }),
            )
            .await,
        )
        .await;
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
    }

    fn sample_mens_spec(id: &str, max_tokens: u64) -> super::ModelSpec {
        use vox_orchestrator::models::spec::{ModelCapabilities, PricingSource, ProviderType};
        use vox_orchestrator::models::{ModelTier, StrengthTag};
        super::ModelSpec {
            id: id.to_string(),
            canonical_slug: id.to_string(),
            provider: "populi_local".to_string(),
            provider_type: ProviderType::VoxLocal,
            max_tokens,
            cost_per_1k: 0.0,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            is_free: true,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                tier: ModelTier::Local,
                writes_vox: true,
                ..Default::default()
            },
            supported_parameters: vec![],
            observed_cost_per_1k: None,
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
        }
    }

    #[test]
    fn merge_live_mens_specs_adds_new_runs_and_prefers_live_for_same_id() {
        let mut reg = super::ModelRegistry::new();
        reg.clear();
        reg.register(sample_mens_spec("mens/stale-run", 4096));
        super::merge_live_mens_specs(
            &mut reg,
            [
                sample_mens_spec("mens/stale-run", 8192),
                sample_mens_spec("mens/fresh-run", 8192),
            ],
        );
        let ids: Vec<_> = reg.list_models().into_iter().map(|m| m.id).collect();
        assert!(ids.contains(&"mens/fresh-run".to_string()));
        assert_eq!(
            reg.get("mens/stale-run").expect("stale-run").max_tokens,
            8192
        );
    }

    fn provenance_spec(
        pricing_source: vox_orchestrator::models::spec::PricingSource,
    ) -> super::ModelSpec {
        super::ModelSpec {
            id: "acme/widget-5.5".into(),
            canonical_slug: "acme/widget-5.5".into(),
            provider: "test".into(),
            provider_type: vox_orchestrator::models::ProviderType::OpenRouter,
            max_tokens: 32_000,
            cost_per_1k: 0.001,
            cost_per_1k_input: 0.001,
            cost_per_1k_output: 0.001,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![vox_orchestrator::models::StrengthTag::Generalist],
            capabilities: vox_orchestrator::models::ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn routing_provenance_reports_family_catalog_source_and_reason() {
        let (family, from, reason) = super::routing_provenance(
            &provenance_spec(vox_orchestrator::models::spec::PricingSource::OpenRouter),
            &vox_orchestrator::models::select::SelectionReason::Scored,
        );
        assert_eq!(family, "acme/widget");
        assert_eq!(from, "catalog");
        assert_eq!(
            reason,
            "Chosen by the model scorer as the best match for your request"
        );
    }

    #[test]
    fn routing_provenance_marks_a_bootstrap_pick_as_bootstrap() {
        let (_, from, _) = super::routing_provenance(
            &provenance_spec(vox_orchestrator::models::spec::PricingSource::Bootstrap),
            &vox_orchestrator::models::select::SelectionReason::Scored,
        );
        assert_eq!(from, "bootstrap");
    }

    #[test]
    fn vox_local_is_not_gated_on_the_ollama_probe() {
        use vox_orchestrator::models::ProviderType;
        let (is_local, reachable, models) = super::local_availability(
            ProviderType::VoxLocal,
            false,
            &["llama3".into()],
            Some(true),
            &["e2e-smoke-metal".into()],
        );
        assert!(is_local);
        assert_eq!(reachable, Some(true));
        assert_eq!(models, vec!["e2e-smoke-metal".to_string()]);

        let (down_local, down_reach, down_models) = super::local_availability(
            ProviderType::VoxLocal,
            true,
            &["llama3".into()],
            Some(false),
            &[],
        );
        assert!(down_local);
        assert_eq!(down_reach, Some(false));
        assert!(down_models.is_empty());

        let (ollama_local, ollama_reach, ollama_models) = super::local_availability(
            ProviderType::Ollama,
            false,
            &["llama3".into()],
            Some(true),
            &["e2e-smoke-metal".into()],
        );
        assert!(ollama_local);
        assert_eq!(ollama_reach, Some(false));
        assert_eq!(ollama_models, vec!["llama3".to_string()]);
    }

    #[test]
    fn parse_vox_local_model_ids_reads_openai_list_shape() {
        let body = serde_json::json!({
            "object": "list",
            "data": [
                {"id": "e2e-smoke-metal", "object": "model"},
                {"id": "run-b", "object": "model"}
            ]
        });
        assert_eq!(
            super::parse_vox_local_model_ids(&body),
            vec!["e2e-smoke-metal".to_string(), "run-b".to_string()]
        );
    }

    #[tokio::test]
    async fn probe_vox_local_bases_reports_unreachable_for_no_candidates() {
        assert_eq!(
            super::probe_vox_local_bases(&[]).await,
            (Some(false), vec![])
        );
    }

    #[tokio::test]
    async fn routing_explain_route_explains_and_rejects_unknown_labels() {
        let (gs, _db) = gateway_with_memory_db().await;
        let q = |task: &str, mode: &str| {
            Query(super::RoutingExplainQuery {
                task: task.into(),
                mode: mode.into(),
                complexity: 42,
            })
        };
        let v = body(
            super::get_routing_explanation(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                q("codegen", "balanced"),
            )
            .await,
        )
        .await;
        assert_eq!(v["data"]["complexity"], 10, "{v}");
        let v = body(
            super::get_routing_explanation(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                q("codegen", "warp-speed"),
            )
            .await,
        )
        .await;
        assert_eq!(v["error"]["message"], "unknown mode");
        let v = body(super::get_routing_health(State(gs), peer(), HeaderMap::new()).await).await;
        assert!(v["data"].is_object(), "{v}");
    }
}
