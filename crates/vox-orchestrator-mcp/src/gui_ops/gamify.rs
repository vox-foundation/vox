//! Dev-only gamify ops for Axis (plan 07 Task 11).
//!
//! `internal` exposure: served only to loopback callers under `dev`, so a remote server never
//! serves them and Gamify reports "not in profile" remotely. Reads are MCP ops rather than
//! `/api/v2` routes because `/api/v2` has no `internal` class.
use axum::Router;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::gui_ops::dto::gamify::{
    CompanionDto, DueActionDto, GuiEventResultDto, LeaderboardEntryDto, LudusProfileDto,
};
use crate::gui_ops::require_db;
use crate::http_gateway::GatewayState;
use crate::params::ToolResult;
use crate::server_state::ServerState;

const LEADERBOARD_DEFAULT_LIMIT: u32 = 20;

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct GamifyProfileParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GamifyDueActionsParams {
    pub limit: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GamifyRecordEventParams {
    /// Routed event JSON (`{ "type": ..., "source": "gui", ... }`).
    pub event: serde_json::Value,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct GamifyLeaderboardParams {
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct GamifyCompanionsParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GamifyDiscoveryRecordParams {
    pub action_id: String,
    pub used: bool,
    pub now_ms: i64,
    pub dwell_ms: i64,
}

pub async fn gamify_profile(state: &ServerState, _p: GamifyProfileParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<LudusProfileDto>::err(e).to_json(),
    };
    let user_id = vox_gamify::db::canonical_user_id();
    let mut profile = match vox_gamify::db::get_profile(db, &user_id).await {
        Ok(p) => p.unwrap_or_else(|| vox_gamify::profile::LudusProfile::new_default(&user_id)),
        Err(e) => return ToolResult::<LudusProfileDto>::err(e.to_string()).to_json(),
    };
    profile.regen_energy();
    if let Err(e) = vox_gamify::db::upsert_profile(db, &profile).await {
        tracing::error!("failed to upsert profile after energy regen: {}", e);
    }
    ToolResult::ok(LudusProfileDto::from_profile(&profile)).to_json()
}

pub async fn gamify_due_actions(state: &ServerState, p: GamifyDueActionsParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<Vec<DueActionDto>>::err(e).to_json(),
    };
    let user_id = vox_gamify::db::canonical_user_id();
    let now_ms = vox_gamify::util::now_unix() * 1000;
    match vox_gamify::db::due_action_ids(db, &user_id, now_ms, p.limit).await {
        Ok(ids) => ToolResult::ok(
            ids.into_iter()
                .map(|action_id| DueActionDto { action_id })
                .collect::<Vec<_>>(),
        )
        .to_json(),
        Err(e) => ToolResult::<Vec<DueActionDto>>::err(e.to_string()).to_json(),
    }
}

pub async fn gamify_record_event(state: &ServerState, p: GamifyRecordEventParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<GuiEventResultDto>::err(e).to_json(),
    };
    match vox_gamify::event_router::route_event_auto_user(db, &p.event).await {
        Ok(routed) => ToolResult::ok(GuiEventResultDto::from_route(&routed)).to_json(),
        Err(e) => ToolResult::<GuiEventResultDto>::err(e.to_string()).to_json(),
    }
}

pub async fn gamify_leaderboard(state: &ServerState, p: GamifyLeaderboardParams) -> String {
    let _db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<Vec<LeaderboardEntryDto>>::err(e).to_json(),
    };
    let limit = p.limit.unwrap_or(LEADERBOARD_DEFAULT_LIMIT) as usize;
    match vox_gamify::verified_leaderboard::verify_and_load_leaderboard_snapshot() {
        Ok(snap) => ToolResult::ok(
            snap.stewardship_standings
                .into_iter()
                .take(limit)
                .map(|r| LeaderboardEntryDto {
                    rank: r.rank,
                    user_id: r.handle,
                    level: ((r.lumens.max(0) as u64) / 200).max(1),
                    score: r.lumens,
                    rank_delta: r.rank_delta,
                    tier_title: r.tier_title,
                    lumens: r.lumens,
                    row_chain_hash_hex: r.proof.row_chain_hash_hex,
                    proof_digest_sha3_256: r.proof.payload_digest_sha3_256,
                    signature_verified: true,
                })
                .collect::<Vec<_>>(),
        )
        .to_json(),
        Err(e) => ToolResult::<Vec<LeaderboardEntryDto>>::err(e.to_string()).to_json(),
    }
}

pub async fn gamify_companions(state: &ServerState, _p: GamifyCompanionsParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<Vec<CompanionDto>>::err(e).to_json(),
    };
    let user_id = vox_gamify::db::canonical_user_id();
    match vox_gamify::db::list_companions(db, &user_id).await {
        Ok(cs) => ToolResult::ok(
            cs.iter()
                .map(CompanionDto::from_companion)
                .collect::<Vec<_>>(),
        )
        .to_json(),
        Err(e) => ToolResult::<Vec<CompanionDto>>::err(e.to_string()).to_json(),
    }
}

pub async fn gamify_discovery_record(
    state: &ServerState,
    p: GamifyDiscoveryRecordParams,
) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<()>::err(e).to_json(),
    };
    let user_id = vox_gamify::db::canonical_user_id();
    let recall = if p.used {
        vox_gamify::discovery::Recall::Used
    } else {
        vox_gamify::discovery::Recall::Seen
    };
    match vox_gamify::discovery::ledger::record(
        db,
        &user_id,
        &p.action_id,
        recall,
        p.now_ms,
        p.dwell_ms,
    )
    .await
    {
        Ok(()) => ToolResult::ok(()).to_json(),
        Err(e) => ToolResult::<()>::err(e.to_string()).to_json(),
    }
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
}

#[cfg(test)]
mod tests {
    use vox_mcp_registry::{
        ActiveProfile, BootReport, CallContext, CallerRole, ProfileDenial, Subsystem,
        authorize_tool,
    };

    use super::*;
    use crate::gui_ops::test_support::{state_with_memory_db, tool_json};

    const OPS: [&str; 6] = [
        "vox_gamify_profile",
        "vox_gamify_due_actions",
        "vox_gamify_record_event",
        "vox_gamify_leaderboard",
        "vox_gamify_companions",
        "vox_gamify_discovery_record",
    ];

    async fn gamify_state() -> ServerState {
        let (st, db) = state_with_memory_db().await;
        vox_gamify::db::apply_ludus_migrations(&db)
            .await
            .expect("ludus migrations");
        st
    }

    #[test]
    fn router_builds() {
        let _ = api_v2_router();
    }

    #[tokio::test]
    async fn recorded_event_is_reflected_by_profile() {
        let st = gamify_state().await;
        let before = tool_json(&gamify_profile(&st, GamifyProfileParams {}).await);
        assert_eq!(before["success"], true, "{before}");
        let xp_before = before["data"]["total_xp_earned"].as_u64().expect("xp");

        let event = serde_json::json!({ "type": "task_completed", "source": "gui" });
        let rec = tool_json(&gamify_record_event(&st, GamifyRecordEventParams { event }).await);
        assert_eq!(rec["success"], true, "{rec}");
        let granted = rec["data"]["xpGranted"].as_u64().expect("xpGranted");
        assert!(granted > 0, "{rec}");

        let after = tool_json(&gamify_profile(&st, GamifyProfileParams {}).await);
        let xp_after = after["data"]["total_xp_earned"].as_u64().expect("xp");
        // Routing can add streak/achievement bonuses on top of the policy reward.
        assert!(xp_after >= xp_before + granted, "{after}");
    }

    #[tokio::test]
    async fn read_ops_return_lists_on_a_fresh_db() {
        let st = gamify_state().await;
        let due = tool_json(&gamify_due_actions(&st, GamifyDueActionsParams { limit: 5 }).await);
        assert_eq!(due["success"], true, "{due}");
        assert!(due["data"].is_array());
        let board = tool_json(&gamify_leaderboard(&st, GamifyLeaderboardParams::default()).await);
        assert_eq!(board["success"], true, "{board}");
        assert!(board["data"].is_array());
        let comps = tool_json(&gamify_companions(&st, GamifyCompanionsParams {}).await);
        assert_eq!(comps["success"], true, "{comps}");
        assert!(comps["data"].is_array());
    }

    #[tokio::test]
    async fn discovery_record_marks_the_action_seen() {
        let st = gamify_state().await;
        let out = tool_json(
            &gamify_discovery_record(
                &st,
                GamifyDiscoveryRecordParams {
                    action_id: "vox.config.show".into(),
                    used: true,
                    now_ms: 1_000,
                    dwell_ms: -5,
                },
            )
            .await,
        );
        assert_eq!(out["success"], true, "{out}");
    }

    #[tokio::test]
    async fn profile_persists_energy_regen() {
        let (st, db) = state_with_memory_db().await;
        vox_gamify::db::apply_ludus_migrations(&db)
            .await
            .expect("ludus migrations");
        let user_id = vox_gamify::db::canonical_user_id();
        let mut p = vox_gamify::profile::LudusProfile::new_default(&user_id);
        p.energy = 0;
        p.last_energy_regen = vox_gamify::util::now_unix() - 7200;
        vox_gamify::db::upsert_profile(&db, &p)
            .await
            .expect("upsert");

        let out = tool_json(&gamify_profile(&st, GamifyProfileParams {}).await);
        assert_eq!(out["success"], true, "{out}");
        let reloaded = vox_gamify::db::get_profile(&db, &user_id)
            .await
            .expect("get")
            .expect("profile row");
        assert!(reloaded.energy > 0, "energy must persist after regen");
    }
    #[tokio::test]
    async fn ops_report_a_missing_db() {
        let st = ServerState::new_test().await;
        let out = tool_json(&gamify_profile(&st, GamifyProfileParams {}).await);
        assert_eq!(out["success"], false, "{out}");
    }

    #[test]
    fn gamify_ops_are_loopback_only_and_outside_named_profiles() {
        let all_up = BootReport {
            booted: Subsystem::ALL.iter().copied().collect(),
            failed: Default::default(),
        };
        let off_loopback = CallContext {
            role: CallerRole::Write,
            loopback: false,
        };
        for tool in OPS {
            assert_eq!(
                authorize_tool(&ActiveProfile::dev(), &all_up, off_loopback, tool).map(|_| ()),
                Err(ProfileDenial::ExposureDenied),
                "{tool}"
            );
            assert!(
                authorize_tool(
                    &ActiveProfile::dev(),
                    &all_up,
                    CallContext::LOCAL_WRITE,
                    tool
                )
                .is_ok(),
                "{tool}"
            );
        }
    }
}
