//! Unified gamify event router.
//!
//! **Single canonical path** for all event-to-reward processing.
//! Both MCP and CLI dashboard use this function instead of calling
//! `process_event_rewards` directly. This eliminates the duplicate
//! side-effect risk and provides one place to add logging/diagnostics.
//!
//! ## Usage
//! ```ignore
//! let uid = canonical_user_id();
//! let event_json = serde_json::to_value(&event.kind)?;
//! route_event(&db, &uid, &event_json).await?;
//! ```

use anyhow::Result;
use vox_db::Codex;

use crate::companion::{Companion, Mood, Personality};
use crate::config_gate;
use crate::db::{
    canonical_user_id, get_companion, get_teaching_profile, insert_event, log_hint_event,
    process_event_rewards, try_claim_processed_event, upsert_companion, upsert_teaching_profile,
};
use crate::sprite_svg::{AgentPose, character_for_agent, generate_svg};
use crate::teaching::{MistakeKind, TeachingProfile};
use crate::util::now_unix;

/// Extract (agent_id_u64, event_type_str) from a serialised `AgentEventKind` value.
fn parse_agent_event(event_json: &serde_json::Value) -> (u64, &str) {
    let event_type = event_json
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let agent_id = event_json
        .get("agent_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    (agent_id, event_type)
}

/// Route a single orchestrator event through the full gamify pipeline:
///
/// 1. Insert into `agent_events` for audit trail.
/// 2. Auto-create/update companion records for agent lifecycle events.
/// 3. Call `process_event_rewards` (policy-driven, config-gated).
///
/// This is the **only** public entry point for event-to-gamify processing.
/// Both MCP and CLI must call this instead of inline reward logic.
pub async fn route_event(
    db: &Codex,
    user_id: &str,
    event_json: &serde_json::Value,
) -> Result<crate::reward_policy::RouteResult> {
    if !config_gate::is_enabled() {
        return Ok(Default::default());
    }

    if let Err(e) = crate::ingest::validate_event_payload(event_json) {
        tracing::warn!(error = %e, "ludus route_event: reject oversize or invalid payload");
        return Ok(Default::default());
    }

    if let Ok(raw) = std::env::var("VOX_LUDUS_ROUTE_LOG_SAMPLE") {
        if let Ok(n) = raw.parse::<u64>() {
            if n > 0 {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                user_id.hash(&mut h);
                if let Some(t) = event_json.get("type").and_then(|v| v.as_str()) {
                    t.hash(&mut h);
                }
                if h.finish() % n == 0 {
                    tracing::info!(
                        target: "vox_ludus::route_event",
                        user_id = %user_id,
                        event_type = %event_json.get("type").and_then(|v| v.as_str()).unwrap_or(""),
                        "sampled route_event"
                    );
                }
            }
        }
    }

    let (agent_id, event_type) = parse_agent_event(event_json);
    let payload = event_json.to_string();

    // 0. Idempotency for orchestrator replays (MCP adds `ludus_dedupe_id`)
    if let Some(did) = event_json.get("ludus_dedupe_id").and_then(|v| v.as_u64()) {
        let key = format!("{user_id}:orch:{did}");
        match try_claim_processed_event(db, user_id, &key).await {
            Ok(true) => {}
            Ok(false) => {
                tracing::trace!("[ludus] skip duplicate event {}", key);
                return Ok(Default::default());
            }
            Err(e) => tracing::debug!("[ludus] dedupe check failed: {e}"),
        }
    }

    // 1. Insert event record
    let _ = insert_event(db, &agent_id.to_string(), event_type, Some(&payload)).await;

    // 2. Increment daily counter for this event type (grind-persistence)
    let daily_count = crate::db_ext::increment_daily_counter(db, user_id, event_type)
        .await
        .unwrap_or(0);

    // 3. Companion lifecycle sync
    let _ = sync_companion_lifecycle(db, user_id, agent_id, event_type, event_json).await;

    // 4. Apply rewards for the primary event
    let mut route_res = process_event_rewards(db, user_id, event_json).await?;

    // 5. Build-clean special cases: phoenix bonus + 3-clean shield
    if event_type == "build_clean" {
        // Phoenix: first clean after a failure today.
        // We check if this is the FIRST clean (daily_count == 1) AND there was at least one failure.
        if daily_count == 1 {
            if let Ok(true) = crate::db_ext::has_failed_today(db, user_id).await {
                let bonus_ev = serde_json::json!({"type": "phoenix_bonus"});
                if let Ok(bonus_res) = process_event_rewards(db, user_id, &bonus_ev).await {
                    if let (Some(rw), Some(bonus_rw)) = (&mut route_res.reward, bonus_res.reward) {
                        rw.xp = rw.xp.saturating_add(bonus_rw.xp);
                        rw.crystals = rw.crystals.saturating_add(bonus_rw.crystals);
                        rw.lumens = rw.lumens.saturating_add(bonus_rw.lumens);
                    }
                    tracing::info!("[ludus] Phoenix bonus awarded for user {user_id}");
                }
            }
        }

        // 3rd clean build today → shield
        if daily_count == 3 {
            let shield_ev = serde_json::json!({"type": "build_clean_streak_3"});
            if let Ok(shield_res) = process_event_rewards(db, user_id, &shield_ev).await {
                if let (Some(rw), Some(shield_rw)) = (&mut route_res.reward, shield_res.reward) {
                    rw.grant_shield = rw.grant_shield || shield_rw.grant_shield;
                    rw.xp = rw.xp.saturating_add(shield_rw.xp);
                    rw.crystals = rw.crystals.saturating_add(shield_rw.crystals);
                }
                tracing::info!("[ludus] Build-clean streak shield awarded for user {user_id}");
            }
        }
    }

    // 6. Combo chain detection
    if let Ok(bonus_slugs) = crate::combo::process_event(db, user_id, event_type).await {
        for slug in bonus_slugs {
            let bonus_ev = serde_json::json!({"type": slug});
            if let Ok(bonus_res) = process_event_rewards(db, user_id, &bonus_ev).await {
                if let (Some(rw), Some(bonus_rw)) = (&mut route_res.reward, bonus_res.reward) {
                    rw.xp = rw.xp.saturating_add(bonus_rw.xp);
                    rw.crystals = rw.crystals.saturating_add(bonus_rw.crystals);
                    rw.lumens = rw.lumens.saturating_add(bonus_rw.lumens);
                    rw.grant_shield = rw.grant_shield || bonus_rw.grant_shield;
                }
                tracing::info!("[ludus] Combo bonus '{}' awarded for user {user_id}", slug);
            }
        }
    }

    let _ = teaching_hook(db, user_id, event_type).await;

    Ok(route_res)
}

async fn teaching_hook(db: &Codex, user_id: &str, event_type: &str) -> Result<()> {
    let kind = match event_type {
        "task_failed" | "task_expired" => Some(MistakeKind::WorkflowError),
        "build_failed" | "check_failed" | "workflow_failed" => Some(MistakeKind::WorkflowError),
        "test_fail" => Some(MistakeKind::TestFailure),
        "parse_error" | "syntax_error" => Some(MistakeKind::SyntaxError),
        "type_error" | "typecheck_error" => Some(MistakeKind::TypeCheckError),
        "scope_violation" | "injection_detected" => Some(MistakeKind::SecurityHint),
        "prompt_conflict_detected" | "stub_check_debt" | "conflict_detected" => {
            Some(MistakeKind::ArchitecturalIssue)
        }
        "fmt_failed" | "bundle_failed" => Some(MistakeKind::WorkflowError),
        _ => None,
    };
    let Some(kind) = kind else {
        return Ok(());
    };
    let mut profile = get_teaching_profile(db, user_id)
        .await
        .unwrap_or_else(|_| TeachingProfile::new(user_id));
    let freq =
        config_gate::mode().hint_frequency() * config_gate::experiment_hint_frequency_multiplier();
    let req = profile.record_mistake(kind, freq);
    let _ = upsert_teaching_profile(db, &profile).await;
    if req.is_some() {
        let _ = log_hint_event(
            db,
            user_id,
            &format!("{kind:?}"),
            "shown",
            Some("route_event"),
        )
        .await;
    }
    Ok(())
}

/// Auto-create or update companion records based on agent lifecycle events.
async fn sync_companion_lifecycle(
    db: &Codex,
    user_id: &str,
    agent_id: u64,
    event_type: &str,
    event_json: &serde_json::Value,
) -> Result<()> {
    let companion_id = format!("agent-{agent_id}");

    match event_type {
        // serde `AgentEventKind` uses #[serde(tag = "type", rename_all = "snake_case")]
        "agent_spawned" => {
            if get_companion(db, &companion_id).await?.is_none() {
                let agent_name = event_json
                    .get("name")
                    .or_else(|| event_json.get("agent_name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unnamed Agent");
                let task = event_json
                    .get("task")
                    .or_else(|| event_json.get("initial_task"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let archetype_names = [
                    "Scriptor", "Rector", "Censor", "Aedilis", "Quaestor", "Tribunus", "Praetor",
                    "Consul",
                ];
                let rank = archetype_names[(agent_id % 8) as usize];
                let sprite = generate_svg(character_for_agent(agent_id), AgentPose::Idle);
                let c = Companion {
                    id: companion_id.clone(),
                    user_id: user_id.to_string(),
                    name: format!("{rank} {agent_name}"),
                    description: if task.is_empty() {
                        None
                    } else {
                        Some(task.to_string())
                    },
                    code_hash: None,
                    language: "vox".to_string(),
                    ascii_sprite: Some(sprite.svg_body),
                    mood: Mood::Neutral,
                    health: 100,
                    max_health: 100,
                    energy: 100,
                    max_energy: 100,
                    code_quality: 50,
                    last_active: now_unix(),
                    personality: Personality::default(),
                };
                let _ = upsert_companion(db, &c).await;
            }
        }
        "activity_changed" | "task_started" => {
            if let Ok(Some(mut c)) = get_companion(db, &companion_id).await {
                let activity = event_json
                    .get("activity")
                    .or_else(|| event_json.get("task"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("working");
                c.mood = mood_from_activity(activity);
                let pose = pose_from_activity(activity);
                let sprite = generate_svg(character_for_agent(agent_id), pose);
                c.ascii_sprite = Some(sprite.svg_body);
                let _ = upsert_companion(db, &c).await;
            }
        }
        "agent_retired" | "task_failed" => {
            if let Ok(Some(mut c)) = get_companion(db, &companion_id).await {
                c.mood = if event_type == "task_failed" {
                    Mood::Sad
                } else {
                    Mood::Tired
                };
                c.energy = 0;
                let sprite = generate_svg(character_for_agent(agent_id), AgentPose::Exhausted);
                c.ascii_sprite = Some(sprite.svg_body);
                let _ = upsert_companion(db, &c).await;
            }
        }
        _ => {}
    }

    Ok(())
}

fn mood_from_activity(activity: &str) -> Mood {
    let lower = activity.to_lowercase();
    if lower.contains("error") || lower.contains("fail") || lower.contains("bug") {
        Mood::Sad
    } else if lower.contains("complete") || lower.contains("done") || lower.contains("success") {
        Mood::Excited
    } else if lower.contains("wait") || lower.contains("input") || lower.contains("block") {
        Mood::Tired
    } else if lower.contains("think") || lower.contains("plan") || lower.contains("analyz") {
        Mood::Happy
    } else {
        Mood::Neutral
    }
}

fn pose_from_activity(activity: &str) -> AgentPose {
    let lower = activity.to_lowercase();
    if lower.contains("read") || lower.contains("analyz") || lower.contains("review") {
        AgentPose::Thinking
    } else if lower.contains("error") || lower.contains("input") || lower.contains("wait") {
        AgentPose::Alert
    } else if lower.contains("complete") || lower.contains("success") || lower.contains("done") {
        AgentPose::Celebrating
    } else if lower.contains("tired") || lower.contains("fail") {
        AgentPose::Exhausted
    } else {
        AgentPose::Working
    }
}

/// Convenience wrapper that resolves the canonical user id automatically.
pub async fn route_event_auto_user(
    db: &Codex,
    event_json: &serde_json::Value,
) -> Result<crate::reward_policy::RouteResult> {
    let uid = canonical_user_id();
    route_event(db, &uid, event_json).await
}
