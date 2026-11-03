//! Typed gamification (Ludus) Tauri commands. Served by the daemon's dev-only `vox_gamify_*`
//! MCP ops (`internal` exposure: loopback under `dev` only), so a remote server reports Gamify
//! as not in profile.

use std::sync::Arc;

use serde_json::json;
use tauri::State;
use vox_gamify::notifications::{Notification, NotificationType};
pub use vox_orchestrator_mcp::gui_ops::dto::gamify::{
    CompanionDto, DueActionDto, GuiEventResultDto, LeaderboardEntryDto, LudusProfileDto,
};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, tool_data};

const PROFILE_TOOL: &str = "vox_gamify_profile";
const NOTIFICATIONS_TOOL: &str = "vox_gamify_notifications_list";
const ACK_TOOL: &str = "vox_gamify_notification_ack";
const DUE_ACTIONS_TOOL: &str = "vox_gamify_due_actions";
const PROGRESS_TOOL: &str = "vox_gamify_progress_snapshot";
const RECORD_EVENT_TOOL: &str = "vox_gamify_record_event";
const LEADERBOARD_TOOL: &str = "vox_gamify_leaderboard";
const COMPANIONS_TOOL: &str = "vox_gamify_companions";
const QUESTS_TOOL: &str = "vox_gamify_quest_list";
const NOTIFICATIONS_DEFAULT_LIMIT: u32 = 20;
/// `vox_gamify_quest_list` clamps to 200; the surface has always shown every active quest.
const QUESTS_LIMIT: u32 = 200;

#[derive(Debug, serde::Serialize)]
pub struct KpiSummaryDto {
    pub events_recorded: i64,
    pub grind_ratio: f64,
    pub avg_multiplier: f64,
    pub quests_completed: i64,
    pub total_xp: i64,
}

impl KpiSummaryDto {
    fn from_summary(s: &vox_gamify::kpi::LudusKpiSummary) -> Self {
        let grind_ratio = if s.events_recorded > 0 {
            (s.grind_capped_events as f64 / s.events_recorded as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Self {
            events_recorded: s.events_recorded,
            grind_ratio,
            avg_multiplier: s.avg_effective_multiplier,
            quests_completed: s.quests_completed_total,
            total_xp: s.total_xp_awarded,
        }
    }
}

/// Map a notification kind to a banner/toast severity (`ok`/`warn`/`info`).
pub(crate) fn notification_level(t: &NotificationType) -> &'static str {
    match t {
        NotificationType::LevelUp
        | NotificationType::AchievementUnlocked
        | NotificationType::QuestCompleted
        | NotificationType::ChallengeCompleted
        | NotificationType::BattleWon
        | NotificationType::StreakContinued => "ok",
        NotificationType::StreakLost | NotificationType::BattleLost => "warn",
        _ => "info",
    }
}

#[derive(Debug, serde::Serialize)]
pub struct LudusNotificationDto {
    pub id: String,
    pub level: String,
    pub title: String,
    pub message: String,
    pub created_at: i64,
    pub kind: String,
}

impl LudusNotificationDto {
    fn from_notification(n: &Notification) -> Self {
        Self {
            id: n.id.clone(),
            level: notification_level(&n.notification_type).to_string(),
            title: n.title.clone(),
            message: n.message.clone(),
            created_at: n.created_at,
            kind: format!("{:?}", n.notification_type),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct QuestDto {
    pub id: String,
    pub quest_type: String,
    pub description: String,
    pub hint: String,
    pub target: u32,
    pub progress: u32,
    pub xp_reward: u64,
    pub crystal_reward: u64,
    pub completed: bool,
    pub status: String,
    pub expires_at: i64,
}

impl QuestDto {
    fn from_quest(q: &vox_gamify::quest::Quest) -> Self {
        Self {
            id: q.id.clone(),
            quest_type: format!("{:?}", q.quest_type),
            description: q.description.clone(),
            hint: q.hint.clone(),
            target: q.target,
            progress: q.progress,
            xp_reward: q.xp_reward,
            crystal_reward: q.crystal_reward,
            completed: q.completed,
            status: q.status.clone(),
            expires_at: q.expires_at,
        }
    }
}

#[derive(serde::Deserialize)]
struct NotificationsData {
    notifications: Vec<Notification>,
}

#[derive(serde::Deserialize)]
struct ProgressData {
    kpi: vox_gamify::kpi::LudusKpiSummary,
}

#[derive(serde::Deserialize)]
struct QuestsData {
    quests: Vec<vox_gamify::quest::Quest>,
}

/// Map GUI hook names from the TypeScript layer to reward_policy event types.
pub(crate) fn map_gui_hook_event_type(hook: &str) -> &str {
    match hook {
        "chat_message_sent" => "message_sent",
        "task_submitted" => "task_submitted",
        "search_query_executed" => "gui_search_query",
        "policy_rule_viewed" => "gui_policy_viewed",
        "palette_navigation" => "gui_palette_nav",
        "console_command_success" => "gui_console_command",
        "discovery_action_used" => "gui_discovery_action",
        "model_activated" => "gui_model_activated",
        "approval_decision" => "gui_approval_decision",
        "browser_preview_loaded" => "gui_browser_preview",
        "mesh_dispatch_success" => "gui_mesh_dispatch",
        "isolation_strategy_set" => "gui_isolation_strategy",
        "isolation_scan_complete" => "gui_isolation_scan",
        "harness_redirect_viewed" => "gui_harness_redirect",
        "breadcrumb_navigation" => "gui_breadcrumb_nav",
        "claim_approved" => "gui_claim_approved",
        "nanopub_built" => "gui_nanopub_built",
        "secret_rotated" => "gui_secret_rotated",
        "signing_key_rotated" => "gui_signing_key_rotated",
        "orchestrator_first_connect" => "gui_orchestrator_connect",
        other => other,
    }
}

fn merge_gui_event_json(hook: &str, metadata: Option<serde_json::Value>) -> serde_json::Value {
    let routed = map_gui_hook_event_type(hook);
    let mut event_json = serde_json::json!({ "type": routed, "source": "gui" });
    if let Some(serde_json::Value::Object(meta)) = metadata
        && let Some(obj) = event_json.as_object_mut()
    {
        for (k, v) in meta {
            if k != "type" {
                obj.insert(k, v);
            }
        }
    }
    event_json
}

async fn call<T: serde::de::DeserializeOwned, B: BackendCalls>(
    b: &B,
    tool: &str,
    args: serde_json::Value,
) -> Result<T, String> {
    tool_data(tool, b.call_tool(tool, args).await?)
}

pub(crate) async fn get_ludus_profile_impl<B: BackendCalls>(
    b: &B,
) -> Result<LudusProfileDto, String> {
    call(b, PROFILE_TOOL, json!({})).await
}

pub(crate) async fn list_ludus_notifications_impl<B: BackendCalls>(
    b: &B,
    limit: Option<u32>,
) -> Result<Vec<LudusNotificationDto>, String> {
    let limit = limit.unwrap_or(NOTIFICATIONS_DEFAULT_LIMIT);
    let data: NotificationsData = call(b, NOTIFICATIONS_TOOL, json!({ "limit": limit })).await?;
    Ok(data
        .notifications
        .iter()
        .map(LudusNotificationDto::from_notification)
        .collect())
}

pub(crate) async fn ack_ludus_notification_impl<B: BackendCalls>(
    b: &B,
    notification_id: String,
) -> Result<(), String> {
    call::<serde_json::Value, _>(b, ACK_TOOL, json!({ "notification_id": notification_id }))
        .await
        .map(|_| ())
}

pub(crate) async fn gamify_due_actions_impl<B: BackendCalls>(
    b: &B,
    limit: u32,
) -> Result<Vec<DueActionDto>, String> {
    call(b, DUE_ACTIONS_TOOL, json!({ "limit": limit })).await
}

pub(crate) async fn gamify_kpi_summary_impl<B: BackendCalls>(
    b: &B,
) -> Result<KpiSummaryDto, String> {
    let data: ProgressData = call(b, PROGRESS_TOOL, json!({})).await?;
    Ok(KpiSummaryDto::from_summary(&data.kpi))
}

pub(crate) async fn record_gui_event_impl<B: BackendCalls>(
    b: &B,
    event_type: String,
    metadata: Option<serde_json::Value>,
) -> Result<GuiEventResultDto, String> {
    let event = merge_gui_event_json(&event_type, metadata);
    call(b, RECORD_EVENT_TOOL, json!({ "event": event })).await
}

pub(crate) async fn list_gamify_leaderboard_impl<B: BackendCalls>(
    b: &B,
    limit: Option<u32>,
) -> Result<Vec<LeaderboardEntryDto>, String> {
    call(b, LEADERBOARD_TOOL, json!({ "limit": limit })).await
}

pub(crate) async fn list_gamify_companions_impl<B: BackendCalls>(
    b: &B,
) -> Result<Vec<CompanionDto>, String> {
    call(b, COMPANIONS_TOOL, json!({})).await
}

pub(crate) async fn list_gamify_quests_impl<B: BackendCalls>(
    b: &B,
) -> Result<Vec<QuestDto>, String> {
    let data: QuestsData = call(b, QUESTS_TOOL, json!({ "limit": QUESTS_LIMIT })).await?;
    Ok(data.quests.iter().map(QuestDto::from_quest).collect())
}

#[tauri::command]
pub async fn get_ludus_profile(
    backend: State<'_, Arc<BackendState>>,
) -> Result<LudusProfileDto, String> {
    get_ludus_profile_impl(&backend.current().await).await
}

#[tauri::command]
pub async fn list_ludus_notifications(
    backend: State<'_, Arc<BackendState>>,
    limit: Option<u32>,
) -> Result<Vec<LudusNotificationDto>, String> {
    list_ludus_notifications_impl(&backend.current().await, limit).await
}

#[tauri::command]
pub async fn ack_ludus_notification(
    backend: State<'_, Arc<BackendState>>,
    notification_id: String,
) -> Result<(), String> {
    ack_ludus_notification_impl(&backend.current().await, notification_id).await
}

#[tauri::command]
pub async fn gamify_due_actions(
    backend: State<'_, Arc<BackendState>>,
    limit: u32,
) -> Result<Vec<DueActionDto>, String> {
    gamify_due_actions_impl(&backend.current().await, limit).await
}

#[tauri::command]
pub async fn gamify_kpi_summary(
    backend: State<'_, Arc<BackendState>>,
) -> Result<KpiSummaryDto, String> {
    gamify_kpi_summary_impl(&backend.current().await).await
}

/// Thin Tauri bridge: GUI hooks → `vox_gamify::event_router` on the daemon (no XP math in TS).
#[tauri::command]
pub async fn record_gui_event(
    backend: State<'_, Arc<BackendState>>,
    event_type: String,
    metadata: Option<serde_json::Value>,
) -> Result<GuiEventResultDto, String> {
    record_gui_event_impl(&backend.current().await, event_type, metadata).await
}

/// Top players by XP. Mirrors `vox ludus leaderboard`.
#[tauri::command]
pub async fn list_gamify_leaderboard(
    backend: State<'_, Arc<BackendState>>,
    limit: Option<u32>,
) -> Result<Vec<LeaderboardEntryDto>, String> {
    list_gamify_leaderboard_impl(&backend.current().await, limit).await
}

/// The user's companions, each with a mood SVG rendered by the daemon.
#[tauri::command]
pub async fn list_gamify_companions(
    backend: State<'_, Arc<BackendState>>,
) -> Result<Vec<CompanionDto>, String> {
    list_gamify_companions_impl(&backend.current().await).await
}

/// The user's active quests.
#[tauri::command]
pub async fn list_gamify_quests(
    backend: State<'_, Arc<BackendState>>,
) -> Result<Vec<QuestDto>, String> {
    list_gamify_quests_impl(&backend.current().await).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    fn ok(data: serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(json!({ "success": true, "data": data }))
    }

    #[test]
    fn notification_level_maps_severity() {
        assert_eq!(notification_level(&NotificationType::LevelUp), "ok");
        assert_eq!(notification_level(&NotificationType::StreakLost), "warn");
        assert_eq!(
            notification_level(&NotificationType::CompanionStatus),
            "info"
        );
    }

    #[tokio::test]
    async fn commands_call_the_gamify_ops_with_the_tauri_arguments() {
        let b = FakeBackend::new()
            .on_tool(ACK_TOOL, ok(json!({ "marked_read": 1 })))
            .on_tool(
                DUE_ACTIONS_TOOL,
                ok(json!([{ "action_id": "vox.config.show" }])),
            )
            .on_tool(
                RECORD_EVENT_TOOL,
                ok(json!({ "xpGranted": 8, "lumensGranted": 0, "achievementTitle": "XP" })),
            )
            .on_tool(LEADERBOARD_TOOL, ok(json!([])))
            .on_tool(COMPANIONS_TOOL, ok(json!([])));
        ack_ludus_notification_impl(&b, "n1".into()).await.unwrap();
        let due = gamify_due_actions_impl(&b, 3).await.unwrap();
        assert_eq!(due[0].action_id, "vox.config.show");
        let rec = record_gui_event_impl(
            &b,
            "task_submitted".into(),
            Some(json!({ "description": "x" })),
        )
        .await
        .unwrap();
        assert_eq!(rec.xp_granted, 8);
        list_gamify_leaderboard_impl(&b, None).await.unwrap();
        list_gamify_companions_impl(&b).await.unwrap();
        let calls = b.calls();
        assert_eq!(
            calls[0],
            (
                format!("tool:{ACK_TOOL}"),
                json!({ "notification_id": "n1" })
            )
        );
        assert_eq!(calls[1].1, json!({ "limit": 3 }));
        assert_eq!(
            calls[2].1,
            json!({ "event": { "type": "task_submitted", "source": "gui", "description": "x" } })
        );
        assert_eq!(calls[3].1, json!({ "limit": null }));
        assert_eq!(calls[4].0, format!("tool:{COMPANIONS_TOOL}"));
    }

    #[tokio::test]
    async fn kpi_summary_extracts_data_kpi() {
        let b = FakeBackend::new().on_tool(
            PROGRESS_TOOL,
            ok(json!({
                "kpi": {
                    "events_recorded": 10, "total_xp_awarded": 50, "total_crystals_awarded": 5,
                    "grind_capped_events": 2, "avg_effective_multiplier": 1.5,
                    "hint_events_logged": 0, "quests_completed_total": 3,
                    "notifications_unread": 0, "hints_shown": 0, "hints_dismissed": 0
                },
                "notifications": [],
                "policy_recent": []
            })),
        );
        let kpi = gamify_kpi_summary_impl(&b).await.unwrap();
        assert_eq!(kpi.events_recorded, 10);
        assert_eq!(kpi.grind_ratio, 0.2);
        assert_eq!(kpi.total_xp, 50);
    }

    #[tokio::test]
    async fn list_reads_unwrap_their_envelopes_and_propagate_denial() {
        let b = FakeBackend::new()
            .on_tool(NOTIFICATIONS_TOOL, ok(json!({ "notifications": [] })))
            .on_tool(QUESTS_TOOL, ok(json!({ "quests": [] })))
            .on_tool(
                PROFILE_TOOL,
                Ok(json!({ "error": { "code": "exposure_denied", "message": "dev-only" } })),
            );
        assert!(
            list_ludus_notifications_impl(&b, None)
                .await
                .unwrap()
                .is_empty()
        );
        assert!(list_gamify_quests_impl(&b).await.unwrap().is_empty());
        let err = get_ludus_profile_impl(&b).await.unwrap_err();
        assert!(err.contains("exposure_denied"), "{err}");
        let calls = b.calls();
        assert_eq!(calls[0].1, json!({ "limit": NOTIFICATIONS_DEFAULT_LIMIT }));
        assert_eq!(calls[1].1, json!({ "limit": QUESTS_LIMIT }));
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("gamify.rs");
        assert!(!src.contains(concat!("vox", "_db")));
        assert!(!src.contains(concat!("fetch_gamify", "_alerts")));
    }

    #[test]
    fn map_gui_hook_event_type_routes_known_hooks() {
        assert_eq!(map_gui_hook_event_type("chat_message_sent"), "message_sent");
        assert_eq!(map_gui_hook_event_type("task_submitted"), "task_submitted");
        assert_eq!(
            map_gui_hook_event_type("search_query_executed"),
            "gui_search_query"
        );
        assert_eq!(
            map_gui_hook_event_type("policy_rule_viewed"),
            "gui_policy_viewed"
        );
        assert_eq!(
            map_gui_hook_event_type("palette_navigation"),
            "gui_palette_nav"
        );
        assert_eq!(
            map_gui_hook_event_type("console_command_success"),
            "gui_console_command"
        );
        assert_eq!(
            map_gui_hook_event_type("discovery_action_used"),
            "gui_discovery_action"
        );
        assert_eq!(
            map_gui_hook_event_type("model_activated"),
            "gui_model_activated"
        );
        assert_eq!(
            map_gui_hook_event_type("approval_decision"),
            "gui_approval_decision"
        );
        assert_eq!(
            map_gui_hook_event_type("browser_preview_loaded"),
            "gui_browser_preview"
        );
        assert_eq!(
            map_gui_hook_event_type("claim_approved"),
            "gui_claim_approved"
        );
        assert_eq!(
            map_gui_hook_event_type("nanopub_built"),
            "gui_nanopub_built"
        );
        assert_eq!(
            map_gui_hook_event_type("secret_rotated"),
            "gui_secret_rotated"
        );
        assert_eq!(
            map_gui_hook_event_type("signing_key_rotated"),
            "gui_signing_key_rotated"
        );
        assert_eq!(
            map_gui_hook_event_type("orchestrator_first_connect"),
            "gui_orchestrator_connect"
        );
        assert_eq!(
            map_gui_hook_event_type("isolation_scan_complete"),
            "gui_isolation_scan"
        );
    }

    #[test]
    fn merge_gui_event_json_includes_metadata_and_routed_type() {
        let json = merge_gui_event_json(
            "chat_message_sent",
            Some(serde_json::json!({ "session_id": "abc" })),
        );
        assert_eq!(json["type"], "message_sent");
        assert_eq!(json["source"], "gui");
        assert_eq!(json["session_id"], "abc");
    }

    #[test]
    fn kpi_to_dto() {
        let summary = vox_gamify::kpi::LudusKpiSummary {
            events_recorded: 10,
            total_xp_awarded: 50,
            total_crystals_awarded: 5,
            grind_capped_events: 2,
            avg_effective_multiplier: 1.5,
            hint_events_logged: 0,
            quests_completed_total: 3,
            notifications_unread: 0,
            hints_shown: 0,
            hints_dismissed: 0,
        };
        let dto = KpiSummaryDto::from_summary(&summary);
        assert_eq!(dto.events_recorded, 10);
        assert_eq!(dto.grind_ratio, 0.2);
        assert_eq!(dto.avg_multiplier, 1.5);
        assert_eq!(dto.quests_completed, 3);
        assert_eq!(dto.total_xp, 50);
    }
}
