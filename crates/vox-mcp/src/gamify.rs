//! Gamify companion MCP tools: mood, status markdown, continuation tick, assessment, handoff payload.
//!
//! When [`ServerState::db`] is present, companion rows are read/written via Codex; otherwise
//! in-memory companions are synthesized per agent id.

use std::path::PathBuf;

use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::Deserialize;
use vox_ludus::companion::{Companion, Interaction};
use vox_ludus::db::{list_companions, upsert_companion};

/// MCP arguments: load or bootstrap the gamify companion row for one orchestrator agent.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckMoodParams {
    /// Orchestrator agent id backing the companion row.
    pub agent_id: u64,
}

/// Return JSON companion record (persisted when DB is wired).
pub async fn check_mood(state: &ServerState, params: CheckMoodParams) -> String {
    let id = format!("agent-{}", params.agent_id);
    let user_id = vox_ludus::db::canonical_user_id();

    if let Some(db) = &state.db {
        match list_companions(db, &user_id).await {
            Ok(comps) => {
                if let Some(c) = comps.into_iter().find(|c: &Companion| c.id == id) {
                    return ToolResult::ok(c).to_json();
                }
            }
            Err(e) => tracing::warn!("failed to list companions from DB: {}", e),
        }
    }

    // Fallback/Initial create
    let companion = Companion::new(&id, &user_id, format!("Agent {}", params.agent_id), "vox");

    // Auto-save if DB exists
    if let Some(db) = &state.db {
        let _ = upsert_companion(db, &companion).await;
    }

    ToolResult::ok(companion).to_json()
}

/// MCP arguments: render queue-aware status markdown for one agent.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentStatusParams {
    /// Agent to describe in markdown.
    pub agent_id: u64,
}

/// Return markdown summarizing queue depth, completed tasks, and companion HP bar.
pub async fn agent_status(state: &ServerState, params: AgentStatusParams) -> String {
    let id = format!("agent-{}", params.agent_id);
    let user_id = vox_ludus::db::canonical_user_id();
    let mut companion = if let Some(db) = &state.db {
        match list_companions(db, &user_id).await {
            Ok(comps) => comps.into_iter().find(|c: &Companion| c.id == id),
            Err(_) => None,
        }
    } else {
        None
    }
    .unwrap_or_else(|| Companion::new(&id, &user_id, format!("Agent {}", params.agent_id), "vox"));

    let orch = &state.orchestrator;

    if let Some(queue_arc) = orch.agent_queue(vox_orchestrator::AgentId(params.agent_id)) {
        let hp_bar = companion.render_status_bar(10);
        let (q_len, q_done, q_empty) = {
            let q = match crate::sync_poison::poison_rw_read(queue_arc.read(), "agent queue") {
                Ok(g) => g,
                Err(e) => {
                    tracing::warn!(error = %e, "gamify status: queue poisoned");
                    return ToolResult::<String>::err(e.to_string()).to_json();
                }
            };
            (q.len(), q.completed_count(), q.is_empty())
        };
        let markdown = format!(
            "### 🤖 Agent {} Status\n\n**{}**\n\n**Stats:**\n- Queue Depth: `{}`\n- Tasks Done: `{}`\n\n**Activity:** {}",
            params.agent_id,
            hp_bar,
            q_len,
            q_done,
            if !q_empty {
                "Processing tasks... ⚙️"
            } else {
                "Idle 💤"
            }
        );
        ToolResult::ok(markdown).to_json()
    } else {
        ToolResult::<String>::err("Agent not found").to_json()
    }
}

/// MCP arguments: nudge orchestrator auto-continuations (idle agent wake-up path).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentContinueParams {
    /// Agent mentioned in the confirmation string (tick is global).
    pub agent_id: u64,
}

/// Run one orchestrator tick then return a short confirmation string (JSON `ToolResult`).
pub async fn agent_continue(state: &ServerState, params: AgentContinueParams) -> String {
    let orch = &state.orchestrator;
    orch.tick().await; // Triggers auto-continuations for idle agents
    ToolResult::ok(format!(
        "Agent {} triggered for continuation",
        params.agent_id
    ))
    .to_json()
}

/// MCP arguments: estimate remaining wall time from queue depth and user preference `task.estimate_ms`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentAssessParams {
    /// Agent whose queue depth is estimated.
    pub agent_id: u64,
}

/// Return human-readable pending/completed counts and rough ETA string.
pub async fn agent_assess(state: &ServerState, params: AgentAssessParams) -> String {
    let mut ms_per_task: usize = 45_000;
    let user_id = vox_ludus::db::canonical_user_id();

    if let Some(db) = &state.db {
        if let Ok(Some(pref)) = db.get_user_preference(&user_id, "task.estimate_ms").await {
            if let Ok(val) = pref.parse::<usize>() {
                ms_per_task = val;
            }
        }
    }

    let orch = &state.orchestrator;

    if let Some(queue_arc) = orch.agent_queue(vox_orchestrator::AgentId(params.agent_id)) {
        let (active, completed) = {
            let q = match crate::sync_poison::poison_rw_read(queue_arc.read(), "agent queue") {
                Ok(g) => g,
                Err(e) => {
                    tracing::warn!(error = %e, "gamify assess: queue poisoned");
                    return ToolResult::<String>::err(e.to_string()).to_json();
                }
            };
            (q.len(), q.completed_count())
        };

        let estimate_s = (active * ms_per_task) / 1000;
        ToolResult::ok(format!(
            "Agent {} has {} pending and {} completed tasks. Est remaining time: {}s",
            params.agent_id, active, completed, estimate_s
        ))
        .to_json()
    } else {
        ToolResult::<String>::err("Agent not found").to_json()
    }
}

/// MCP arguments: structured plan handoff published on the orchestrator event bus.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentHandoffParams {
    /// Handoff source agent.
    pub from_agent_id: u64,
    /// Handoff destination agent.
    pub to_agent_id: u64,
    /// High-level narrative inserted into [`vox_orchestrator::handoff::HandoffPayload`].
    pub plan_summary: String,
    #[serde(default)]
    /// Open work items the receiver should address.
    pub unresolved_objectives: Vec<String>,
    #[serde(default)]
    /// Checklist the receiver can use to validate completion.
    pub verification_criteria: Vec<String>,
}

/// Emit a [`vox_orchestrator::handoff::HandoffPayload`] (side effect: event bus + downstream listeners).
pub async fn agent_handoff(state: &ServerState, params: AgentHandoffParams) -> String {
    let orch = &state.orchestrator;
    let mut payload = vox_orchestrator::handoff::HandoffPayload::new(
        vox_orchestrator::AgentId(params.from_agent_id),
        Some(vox_orchestrator::AgentId(params.to_agent_id)),
        &params.plan_summary,
    );
    payload.unresolved_objectives = params.unresolved_objectives;
    payload.verification_criteria = params.verification_criteria;
    if let Err(e) = vox_orchestrator::handoff::execute_handoff(&payload, orch.event_bus()) {
        return ToolResult::<String>::err(e.to_string()).to_json();
    }

    ToolResult::ok(format!(
        "Handoff initiated from agent {} to agent {}: {}",
        params.from_agent_id, params.to_agent_id, params.plan_summary
    ))
    .to_json()
}

/// MCP arguments: unread Ludus notification feed (Codex `gamify_notifications`).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LudusNotificationsParams {
    /// Max rows (1–100).
    #[serde(default = "ludus_notif_limit_default")]
    pub limit: u32,
}

fn ludus_notif_limit_default() -> u32 {
    20
}

/// List unread notifications for the canonical local Ludus user.
pub async fn ludus_notifications_list(
    state: &ServerState,
    params: LudusNotificationsParams,
) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<serde_json::Value>::err("Codex database not connected").to_json();
    };
    let uid = vox_ludus::db::canonical_user_id();
    let lim = params.limit.clamp(1, 100);
    match vox_ludus::db::list_unread_notifications(db, &uid, lim).await {
        Ok(n) => ToolResult::ok(serde_json::json!({ "notifications": n })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// MCP arguments: mark one Ludus notification read (user-scoped).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LudusNotificationAckParams {
    /// Notification row id from `gamify_notifications`.
    pub notification_id: String,
}

/// Mark one notification as read for the canonical user.
pub async fn ludus_notification_ack(
    state: &ServerState,
    params: LudusNotificationAckParams,
) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<serde_json::Value>::err("Codex database not connected").to_json();
    };
    let uid = vox_ludus::db::canonical_user_id();
    let id = params.notification_id.trim();
    if id.is_empty() {
        return ToolResult::<serde_json::Value>::err("notification_id is required").to_json();
    }
    match vox_ludus::db::mark_notification_read_for_user(db, &uid, id).await {
        Ok(0) => ToolResult::<serde_json::Value>::err(
            "notification not found or already read for this user",
        )
        .to_json(),
        Ok(n) => ToolResult::ok(serde_json::json!({
            "marked_read": n,
            "notification_id": id,
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// Mark all unread Ludus notifications read for the canonical user.
pub async fn ludus_notifications_ack_all(state: &ServerState) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<serde_json::Value>::err("Codex database not connected").to_json();
    };
    let uid = vox_ludus::db::canonical_user_id();
    match vox_ludus::db::mark_all_notifications_read(db, &uid).await {
        Ok(()) => ToolResult::ok(serde_json::json!({ "ack_all": true })).to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// MCP arguments: weekly-style Ludus digest (KPI + recent policy + notifications).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LudusProgressSnapshotParams {
    #[serde(default = "ludus_snap_notif_lim")]
    notification_limit: u32,
    #[serde(default = "ludus_snap_policy_lim")]
    policy_limit: usize,
    /// Rolling window for policy snapshots (days).
    #[serde(default = "ludus_snap_policy_days")]
    policy_days: u32,
}

fn ludus_snap_notif_lim() -> u32 {
    12
}

fn ludus_snap_policy_lim() -> usize {
    32
}

fn ludus_snap_policy_days() -> u32 {
    7
}

/// Aggregate Ludus KPI, unread notifications, and recent policy awards (for agents / dashboards).
pub async fn ludus_progress_snapshot(
    state: &ServerState,
    params: LudusProgressSnapshotParams,
) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<serde_json::Value>::err("Codex database not connected").to_json();
    };
    let uid = vox_ludus::db::canonical_user_id();
    let notif_lim = params.notification_limit.clamp(1, 100);
    let policy_lim = params.policy_limit.clamp(1, 500);
    let days = params.policy_days.clamp(1, 3660);

    let kpi = match vox_ludus::db::load_kpi_summary(db, &uid).await {
        Ok(k) => serde_json::to_value(k).unwrap_or(serde_json::Value::Null),
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!("kpi: {e}")).to_json();
        }
    };
    let notifications = match vox_ludus::db::list_unread_notifications(db, &uid, notif_lim).await {
        Ok(n) => serde_json::to_value(n).unwrap_or(serde_json::Value::Array(vec![])),
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!("notifications: {e}")).to_json();
        }
    };
    let policy_recent = match vox_ludus::db::list_policy_snapshots_since_days(
        db,
        &uid,
        days,
        policy_lim,
    )
    .await
    {
        Ok(rows) => serde_json::to_value(rows).unwrap_or(serde_json::Value::Array(vec![])),
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!("policy: {e}")).to_json();
        }
    };

    ToolResult::ok(serde_json::json!({
        "ludus_enabled": vox_ludus::config_gate::is_enabled(),
        "ludus_channel": format!("{:?}", vox_ludus::config_gate::ludus_channel()),
        "user_id": uid,
        "experiment": std::env::var("VOX_LUDUS_EXPERIMENT").unwrap_or_default(),
        "experiment_hint_multiplier": vox_ludus::config_gate::experiment_hint_frequency_multiplier(),
        "experiment_reward_multiplier": vox_ludus::config_gate::experiment_reward_multiplier(),
        "kpi": kpi,
        "notifications": notifications,
        "policy_snapshots_recent": policy_recent,
        "policy_window_days": days,
    }))
    .to_json()
}

// ── Quest / shop / collegium / battle (read + buy + join) ───────────────────

/// MCP: list quests for the canonical user (read-only).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LudusQuestListParams {
    #[serde(default = "ludus_quest_list_limit")]
    pub limit: u32,
}

fn ludus_quest_list_limit() -> u32 {
    50
}

pub async fn ludus_quest_list(state: &ServerState, params: LudusQuestListParams) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<serde_json::Value>::err("Codex database not connected").to_json();
    };
    let uid = vox_ludus::db::canonical_user_id();
    let lim = params.limit.clamp(1, 200) as usize;
    match vox_ludus::db::list_quests(db, &uid).await {
        Ok(mut qs) => {
            qs.truncate(lim);
            ToolResult::ok(serde_json::json!({ "quests": qs })).to_json()
        }
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// MCP: crystal shop catalog (prices scale with current reward mode multiplier).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LudusShopCatalogParams {}

pub async fn ludus_shop_catalog(_state: &ServerState, _params: LudusShopCatalogParams) -> String {
    let items = vox_ludus::shop::default_shop_items();
    let mode_mult = vox_ludus::config_gate::reward_multiplier();
    let rows: Vec<serde_json::Value> = items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            serde_json::json!({
                "item_index": i + 1,
                "name": it.name(),
                "cost_crystals": it.effective_cost(mode_mult),
            })
        })
        .collect();
    ToolResult::ok(serde_json::json!({ "items": rows })).to_json()
}

/// MCP: purchase one shop item by 1-based index from [`ludus_shop_catalog`].
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LudusShopBuyParams {
    pub item_index: u32,
    pub idempotency_key: Option<String>,
}

pub async fn ludus_shop_buy(state: &ServerState, params: LudusShopBuyParams) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<serde_json::Value>::err("Codex database not connected").to_json();
    };
    let uid = vox_ludus::db::canonical_user_id();
    if let Some(ref key) = params.idempotency_key {
        if !key.trim().is_empty() {
            let dedupe = format!("ludus_shop:{}", key.trim());
            match vox_ludus::db::try_claim_processed_event(db, &uid, &dedupe).await {
                Ok(true) => {}
                Ok(false) => {
                    return ToolResult::ok(serde_json::json!({
                        "duplicate": true,
                        "message": "idempotency_key already applied"
                    }))
                    .to_json();
                }
                Err(e) => return ToolResult::<serde_json::Value>::err(format!("dedupe: {e}")).to_json(),
            }
        }
    }
    let mut profile = match vox_ludus::db::get_profile(db, &uid).await {
        Ok(Some(p)) => p,
        Ok(None) => return ToolResult::<serde_json::Value>::err("profile not found").to_json(),
        Err(e) => return ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    };
    let items = vox_ludus::shop::default_shop_items();
    let idx = params.item_index.saturating_sub(1) as usize;
    let Some(item) = items.get(idx) else {
        return ToolResult::<serde_json::Value>::err("invalid item_index").to_json();
    };
    let mode_mult = vox_ludus::config_gate::reward_multiplier();
    let mut abilities = Vec::new();
    let result = vox_ludus::shop::purchase(&mut profile, item, mode_mult, &mut abilities);
    if result.success {
        let _ = vox_ludus::db::upsert_profile(db, &profile).await;
    }
    ToolResult::ok(serde_json::to_value(result).unwrap_or(serde_json::Value::Null)).to_json()
}

/// MCP: join a collegium and route a `collegium_joined` Ludus event.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LudusCollegiumJoinParams {
    pub collegium_id: String,
}

pub async fn ludus_collegium_join(state: &ServerState, params: LudusCollegiumJoinParams) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<serde_json::Value>::err("Codex database not connected").to_json();
    };
    let uid = vox_ludus::db::canonical_user_id();
    let cid = params.collegium_id.trim();
    if cid.is_empty() {
        return ToolResult::<serde_json::Value>::err("collegium_id required").to_json();
    }
    if let Err(e) = vox_ludus::db::join_collegium(db, cid, &uid, "legionnaire").await {
        return ToolResult::<serde_json::Value>::err(e.to_string()).to_json();
    }
    let ev = serde_json::json!({
        "type": "collegium_joined",
        "agent_id": 0u64,
    });
    match vox_ludus::event_router::route_event(db, &uid, &ev).await {
        Ok(res) => ToolResult::ok(serde_json::json!({
            "joined": true,
            "route": serde_json::to_value(res).unwrap_or_default()
        }))
        .to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

/// MCP: start a bug battle (synthetic finding; for agent-driven play / demos).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct LudusBattleStartParams {
    pub companion_name: String,
    pub rule_id: String,
    pub message: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub line: Option<usize>,
    pub context: Option<String>,
}

pub async fn ludus_battle_start(state: &ServerState, params: LudusBattleStartParams) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<serde_json::Value>::err("Codex database not connected").to_json();
    };
    let uid = vox_ludus::db::canonical_user_id();
    let finding = vox_ludus::BattleFinding {
        rule_id: params.rule_id,
        message: params.message,
        file_path: PathBuf::from(params.file_path.as_deref().unwrap_or(".")),
        line: params.line.unwrap_or(1).max(1),
        context: params.context,
    };
    match vox_ludus::run_battle_start(db, &uid, &params.companion_name, &finding).await {
        Ok(Some(o)) => ToolResult::ok(serde_json::json!({
            "battle_id": o.battle.id,
            "companion": o.companion_name,
        }))
        .to_json(),
        Ok(None) => ToolResult::<serde_json::Value>::err("companion not found or battle could not start").to_json(),
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LudusBattleSubmitParams {
    pub companion_name: String,
    pub code: String,
    #[serde(default)]
    pub success: bool,
}

pub async fn ludus_battle_submit(state: &ServerState, params: LudusBattleSubmitParams) -> String {
    let Some(db) = &state.db else {
        return ToolResult::<serde_json::Value>::err("Codex database not connected").to_json();
    };
    let uid = vox_ludus::db::canonical_user_id();
    let r = vox_ludus::run_battle_submit(db, &uid, &params.companion_name, params.code, params.success).await;
    match r {
        Ok(vox_ludus::BattleSubmitResult::Tired) => {
            ToolResult::<serde_json::Value>::err("companion out of battle energy").to_json()
        }
        Ok(vox_ludus::BattleSubmitResult::NotFound) => {
            ToolResult::<serde_json::Value>::err("no active battle for companion").to_json()
        }
        Ok(vox_ludus::BattleSubmitResult::Outcome(o)) => {
            ToolResult::ok(serde_json::json!({ "success": o.success, "battle_id": o.battle.id }))
                .to_json()
        }
        Err(e) => ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    }
}
