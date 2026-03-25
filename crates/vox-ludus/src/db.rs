//! Database persistence for gamification layer.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

use crate::companion::{Companion, Mood, Personality};
use crate::profile::LudusProfile;
use crate::quest::{Quest, QuestModifier};

// ── Helpers ──────────────────────────────────────────────

/// Canonical user-identity normalisation.
///
/// Priority: non-empty `vox_db::paths::local_user_id()` > `DEFAULT_USER_ID`.
/// All reward/event write paths MUST call this instead of constructing IDs inline.
pub fn canonical_user_id() -> String {
    let from_db = vox_db::paths::local_user_id();
    if !from_db.is_empty() && from_db != "user" {
        from_db
    } else {
        crate::util::DEFAULT_USER_ID.to_string()
    }
}

/// Parse a quest-type string from DB without losing `agent_complete` / `collaborate`.
fn parse_quest_type(s: &str) -> crate::quest::QuestType {
    use crate::quest::QuestType;
    match s {
        "create" => QuestType::Create,
        "review" => QuestType::Review,
        "battle" => QuestType::Battle,
        "improve" => QuestType::Improve,
        "agent_complete" => QuestType::AgentComplete,
        "collaborate" => QuestType::Collaborate,
        "ai_feedback" => QuestType::AiFeedback,
        "populi_contribute" => QuestType::PopuliContribute,
        "build_streak" => QuestType::BuildStreak,
        "doc_sprint" => QuestType::DocSprint,
        "toestub_fix" => QuestType::ToestubFix,
        "testing" => QuestType::Testing,
        "research" => QuestType::Research,
        "first_time" => QuestType::FirstTime,
        other => {
            tracing::warn!("unknown quest_type '{}' in DB, defaulting to Create", other);
            QuestType::Create
        }
    }
}

// ── Profile ──────────────────────────────────────────────

/// Load a gamify profile from the DB.
pub async fn get_profile(db: &Codex, user_id: &str) -> Result<Option<LudusProfile>> {
    if let Some(row) = db.get_gamify_profile_raw(user_id).await? {
        let streak = crate::streak::StreakTracker {
            current_streak: row[7] as u64,
            longest_streak: row[8] as u64,
            last_activity_ts: row[9],
            grace_periods_available: row[10] as u64,
            grace_periods_used: row[11] as u64,
        };
        Ok(Some(LudusProfile {
            user_id: user_id.to_string(),
            level: row[0] as u64,
            xp: row[1] as u64,
            crystals: row[2] as u64,
            energy: row[3] as u64,
            max_energy: row[4] as u64,
            last_energy_regen: row[5],
            last_active: row[6],
            streak,
            total_xp_earned: row[12] as u64,
            prestige_level: row[13] as u32,
            lumens: row[14],
            generosity_lumens: row[15],
            streak_shields: row[16] as i32,
        }))
    } else {
        Ok(None)
    }
}

/// Upsert a gamify profile to the DB (includes streak state).
pub async fn upsert_profile(db: &Codex, p: &LudusProfile) -> Result<()> {
    db.upsert_gamify_profile(
        &p.user_id,
        p.level as i64,
        p.xp as i64,
        p.crystals as i64,
        p.energy as i64,
        p.max_energy as i64,
        p.last_energy_regen,
        p.last_active,
        p.streak.current_streak as i64,
        p.streak.longest_streak as i64,
        p.streak.last_activity_ts,
        p.streak.grace_periods_available as i64,
        p.streak.grace_periods_used as i64,
        p.total_xp_earned as i64,
        p.prestige_level as i64,
        p.lumens,
        p.generosity_lumens,
        p.streak_shields as i64,
    )
    .await?;
    Ok(())
}

/// Record that an achievement was unlocked for a user, and credit the reward.
/// Idempotent — calling twice for the same (id, user_id) is a no-op.
pub async fn unlock_achievement(
    db: &Codex,
    user_id: &str,
    achievement_id: &str,
    xp: u32,
    crystals: u32,
) -> Result<bool> {
    let now = crate::util::now_unix();
    Ok(db
        .unlock_gamify_achievement(user_id, achievement_id, now, xp as i64, crystals as i64)
        .await?)
}

/// Record a level-up event in the level history table.
pub async fn record_level_up(
    db: &Codex,
    user_id: &str,
    level: u64,
    title: &str,
    xp_at_level: u64,
) -> Result<()> {
    let now = crate::util::now_unix();
    db.record_gamify_level_up(user_id, level as i64, title, xp_at_level as i64, now)
        .await?;
    Ok(())
}

/// Load all unlocked achievement IDs for a user.
pub async fn list_unlocked_achievements(db: &Codex, user_id: &str) -> Result<Vec<(String, i64)>> {
    Ok(db.list_gamify_achievements(user_id).await?)
}

// ── Companion ────────────────────────────────────────────

/// Load all companions for a user.
pub async fn list_companions(db: &Codex, user_id: &str) -> Result<Vec<Companion>> {
    let rows = db.list_gamify_companions(user_id).await?;
    let mut comps = Vec::new();

    for row in rows {
        let personality_str = row[14].as_deref().unwrap_or("focused");
        let personality = personality_str.parse::<Personality>().unwrap_or_default();

        comps.push(Companion {
            id: row[0].clone().unwrap_or_default(),
            user_id: row[1].clone().unwrap_or_else(|| user_id.to_string()),
            name: row[2].clone().unwrap_or_default(),
            description: row[3].clone(),
            code_hash: row[4].clone(),
            language: row[5].clone().unwrap_or_default(),
            ascii_sprite: row[6].clone(),
            mood: row[7]
                .as_deref()
                .unwrap_or("neutral")
                .parse::<Mood>()
                .unwrap_or(Mood::Neutral),
            health: row[8]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            max_health: row[9]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            energy: row[10]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            max_energy: row[11]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            code_quality: row[12]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(50) as u8,
            last_active: row[13]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or_default(),
            personality,
        });
    }

    Ok(comps)
}

/// Upsert a companion (includes personality JSON).
pub async fn upsert_companion(db: &Codex, c: &Companion) -> Result<()> {
    db.upsert_gamify_companion(
        &c.id,
        &c.user_id,
        &c.name,
        c.description.clone(),
        c.code_hash.clone(),
        &c.language,
        c.ascii_sprite.clone(),
        c.mood.as_str(),
        c.health as i64,
        c.max_health as i64,
        c.energy as i64,
        c.max_energy as i64,
        c.code_quality as i64,
        c.last_active,
        c.personality.as_str(),
    )
    .await?;
    Ok(())
}

// ── Quests ───────────────────────────────────────────────

/// Get a specific companion.
pub async fn get_companion(db: &Codex, id: &str) -> Result<Option<Companion>> {
    let row = db.get_gamify_companion(id).await?;
    if let Some(row) = row {
        let personality_str = row[13].as_deref().unwrap_or("focused");
        let personality = personality_str.parse::<Personality>().unwrap_or_default();

        Ok(Some(Companion {
            id: row[0].clone().unwrap_or_default(),
            user_id: row[1].clone().unwrap_or_default(),
            name: row[2].clone().unwrap_or_default(),
            description: row[3].clone(),
            code_hash: row[4].clone(),
            language: row[5].clone().unwrap_or_default(),
            ascii_sprite: row[6].clone(),
            mood: row[7]
                .as_deref()
                .unwrap_or("neutral")
                .parse::<Mood>()
                .unwrap_or(Mood::Neutral),
            health: row[8]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            max_health: row[9]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            energy: row[10]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            max_energy: row[11]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(100) as i32,
            code_quality: row[12]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(50) as u8,
            last_active: row[13]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or_default(),
            personality,
        }))
    } else {
        Ok(None)
    }
}

/// Delete a companion.
pub async fn delete_companion(db: &Codex, id: &str) -> Result<()> {
    db.delete_gamify_companion(id).await?;
    Ok(())
}

// ── Quests ───────────────────────────────────────────────

/// Load all active quests for a user.
pub async fn list_quests(db: &Codex, user_id: &str) -> Result<Vec<Quest>> {
    let rows = db.list_gamify_quests(user_id).await?;
    let mut quests = Vec::new();

    for row in rows {
        let completed = row[7]
            .as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(0)
            != 0;
        let modifier_str = row[10].as_deref().unwrap_or("none");
        let modifier = match modifier_str {
            "blessed" => QuestModifier::Blessed,
            "timed" => QuestModifier::Timed,
            "chains" => QuestModifier::Chains,
            "silent" => QuestModifier::Silent,
            "legendary" => QuestModifier::Legendary,
            "collaborative" => QuestModifier::Collaborative,
            "cursed" => QuestModifier::Cursed,
            "echoed" => QuestModifier::Echoed,
            "frenzy" => QuestModifier::Frenzy,
            _ => QuestModifier::None,
        };

        quests.push(Quest {
            id: row[0].clone().unwrap_or_default(),
            user_id: user_id.to_string(),
            quest_type: parse_quest_type(&row[1].as_deref().unwrap_or("build").to_string()), // Reverted to use parse_quest_type
            description: row[2].clone().unwrap_or_default(),
            target: row[3]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(1) as u32,
            progress: row[4]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0) as u32,
            crystal_reward: row[5]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(10) as u64, // Changed to u64
            xp_reward: row[6]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(15) as u64, // Changed to u64
            modifier,
            completed,
            status: row[11].clone().unwrap_or_else(|| "active".to_string()),
            expires_at: row[8]
                .as_deref()
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or_default(), // Changed to i64
            hint: row[9].clone().unwrap_or_default(),
        });
    }

    Ok(quests)
}

/// Upsert a quest.
pub async fn upsert_quest(db: &Codex, q: &Quest) -> Result<()> {
    db.upsert_gamify_quest(
        &q.id,
        &q.user_id,
        q.quest_type.as_str(),
        &q.description,
        q.xp_reward as i64,
        q.crystal_reward as i64,
        q.target as i64,
        q.progress as i64,
        &q.status,
        q.expires_at,
        q.completed,
    )
    .await?;
    Ok(())
}

// ── Battles ──────────────────────────────────────────────

/// Get a specific quest by ID.
pub async fn get_quest(db: &Codex, id: &str) -> Result<Option<Quest>> {
    let mut rows = db.connection().query(
        "SELECT id, user_id, quest_type, description, target, progress, crystal_reward, xp_reward, completed, expires_at,
                hint, modifier, status
         FROM gamify_quests WHERE id = ?1",
        params![id],
    ).await?;
    if let Some(row) = rows.next().await? {
        let completed = row.get::<i64>(8)? != 0;
        let modifier_str: String = row.get(11).unwrap_or_else(|_| "none".to_string());
        let modifier = serde_json::from_str::<QuestModifier>(&format!("\"{}\"", modifier_str))
            .unwrap_or(QuestModifier::None);

        Ok(Some(Quest {
            id: row.get::<String>(0)?,
            user_id: row.get::<String>(1)?,
            quest_type: parse_quest_type(&row.get::<String>(2)?),
            description: row.get::<String>(3)?,
            hint: row.get(10).unwrap_or_default(),
            target: row.get::<i64>(4)? as u32,
            progress: row.get::<i64>(5)? as u32,
            crystal_reward: row.get::<i64>(6)? as u64,
            xp_reward: row.get::<i64>(7)? as u64,
            modifier,
            completed,
            status: row.get(12).unwrap_or_else(|_| {
                if completed {
                    "completed".into()
                } else {
                    "active".into()
                }
            }),
            expires_at: row.get(9).unwrap_or_default(),
        }))
    } else {
        Ok(None)
    }
}

/// Update quest status: "pending" | "active" | "completed" | "abandoned".
pub async fn update_quest_status(db: &Codex, user_id: &str, id: &str, status: &str) -> Result<()> {
    let completed = status == "completed";
    db.update_gamify_quest_status(id, user_id, status, completed)
        .await?;
    Ok(())
}

/// Count active/available quests for a user.
pub async fn count_quests(db: &Codex, user_id: &str) -> Result<u32> {
    Ok(db.count_gamify_quests(user_id).await? as u32)
}

/// Delete a quest.
pub async fn delete_quest(db: &Codex, id: &str) -> Result<()> {
    db.delete_gamify_quest(id).await?;
    Ok(())
}

// ── Battles ──────────────────────────────────────────────

use crate::battle::{Battle, BugType};

/// Load recent battles for a user.
pub async fn list_battles(db: &Codex, user_id: &str, limit: i64) -> Result<Vec<Battle>> {
    let rows = db.list_gamify_battles(user_id, limit).await?;
    let mut battles = Vec::new();
    for row in rows {
        battles.push(Battle {
            id: row[0].clone().unwrap_or_default(),
            user_id: user_id.to_string(),
            companion_id: row[1].clone().unwrap_or_default(),
            bug_type: match row[2].as_deref().unwrap_or("") {
                "syntax" => BugType::Syntax,
                "logic" => BugType::Logic,
                "performance" => BugType::Performance,
                "security" => BugType::Security,
                _ => BugType::Syntax,
            },
            bug_description: row[3].clone().unwrap_or_default(),
            bug_code: row[4].clone(),
            submitted_code: row[5].clone(),
            success: row[6].as_deref().unwrap_or("0") != "0",
            crystals_earned: row[7].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            xp_earned: row[8].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            duration_secs: row[9].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
            created_at: row[10].as_deref().and_then(|s| s.parse().ok()).unwrap_or(0),
        });
    }
    Ok(battles)
}

/// Insert a new battle record.
pub async fn insert_battle(db: &Codex, b: &Battle) -> Result<()> {
    db.insert_gamify_battle(
        &b.id,
        &b.user_id,
        &b.companion_id,
        b.bug_type.as_str(),
        &b.bug_description,
        b.bug_code.as_deref(),
        b.submitted_code.as_deref(),
        b.success,
        b.crystals_earned as i64,
        b.xp_earned as i64,
        b.duration_secs as i64,
        b.created_at,
    )
    .await?;
    Ok(())
}

// ── Events ───────────────────────────────────────────────

/// Get a specific battle by ID.
pub async fn get_battle(db: &Codex, id: &str) -> Result<Option<Battle>> {
    let mut rows = db.connection().query(
        "SELECT id, user_id, companion_id, bug_type, bug_description, bug_code, submitted_code, success, crystals_earned, xp_earned, duration_secs, created_at
         FROM gamify_battles WHERE id = ?1",
        params![id],
    ).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some(Battle {
            id: row.get::<String>(0)?,
            user_id: row.get::<String>(1)?,
            companion_id: row.get::<String>(2)?,
            bug_type: match row.get::<String>(3)?.as_str() {
                "syntax" => BugType::Syntax,
                "logic" => BugType::Logic,
                "performance" => BugType::Performance,
                "security" => BugType::Security,
                _ => BugType::Syntax,
            },
            bug_description: row.get::<String>(4)?,
            bug_code: row.get::<Option<String>>(5)?,
            submitted_code: row.get::<Option<String>>(6)?,
            success: row.get::<i64>(7)? != 0,
            crystals_earned: row.get::<i64>(8)? as u64,
            xp_earned: row.get::<i64>(9)? as u64,
            duration_secs: row.get::<i64>(10)? as u64,
            created_at: row.get::<i64>(11)?,
        }))
    } else {
        Ok(None)
    }
}

/// Update a battle.
pub async fn update_battle(db: &Codex, b: &Battle) -> Result<()> {
    db.update_gamify_battle(
        &b.id,
        b.submitted_code.as_deref(),
        b.success,
        b.crystals_earned as i64,
        b.xp_earned as i64,
        b.duration_secs as i64,
    )
    .await?;
    Ok(())
}

/// Count battles played by a user.
pub async fn count_battles(db: &Codex, user_id: &str) -> Result<i64> {
    Ok(db.count_gamify_battles(user_id).await?)
}

/// A row in the player leaderboard.
#[derive(Debug, serde::Serialize)]
pub struct PlayerRankEntry {
    /// Unique user identifier.
    pub user_id: String,
    /// Player's current level.
    pub level: u64,
    /// Score (XP or Lumens) to rank by.
    pub score: i64,
}

/// Get top users by XP for the leaderboard.
pub async fn leaderboard(db: &Codex, limit: i64) -> Result<Vec<PlayerRankEntry>> {
    let rows = db.gamify_leaderboard_by_xp(limit).await?;
    let mut entries = Vec::new();
    for (user_id, level, score) in rows {
        entries.push(PlayerRankEntry {
            user_id,
            level: level as u64,
            score,
        });
    }
    Ok(entries)
}

/// Get top users by Lumens for the leaderboard.
pub async fn lumens_leaderboard(db: &Codex, limit: i64) -> Result<Vec<PlayerRankEntry>> {
    let rows = db.gamify_leaderboard_by_lumens(limit).await?;
    let mut entries = Vec::new();
    for (user_id, level, score) in rows {
        entries.push(PlayerRankEntry {
            user_id,
            level: level as u64,
            score,
        });
    }
    Ok(entries)
}

/// Get aggregate profile stats (e.g. total completed quests, total battles won, etc.).
pub async fn get_profile_stats(db: &Codex, user_id: &str) -> Result<serde_json::Value> {
    let (completed_quests, won_battles) = db.get_gamify_profile_stats(user_id).await?;
    Ok(serde_json::json!({
        "completed_quests": completed_quests,
        "won_battles": won_battles,
    }))
}

// ── Events ───────────────────────────────────────────────

/// Persistent record of an agent lifecycle or state-change event.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentEventRecord {
    /// Monotonic database row ID.
    pub id: i64,
    /// Identifier of the agent that emitted the event.
    pub agent_id: String,
    /// Discriminant string for the event kind (e.g. `"task_completed"`).
    pub event_type: String,
    /// Optional JSON payload attached to the event.
    pub payload: Option<String>,
    /// SQLite `datetime` string when the event was recorded.
    pub timestamp: String,
}

/// Load recent events for an agent.
pub async fn get_events(
    db: &Codex,
    agent_id: &str,
    limit: Option<i64>,
) -> Result<Vec<AgentEventRecord>> {
    let rows = db.list_gamify_events(agent_id, limit.unwrap_or(50)).await?;
    let mut events = Vec::new();
    for (id, agent_id, event_type, payload, timestamp) in rows {
        events.push(AgentEventRecord {
            id,
            agent_id,
            event_type,
            payload,
            timestamp,
        });
    }
    Ok(events)
}

/// Insert a new agent event.
pub async fn insert_event(
    db: &Codex,
    agent_id: &str,
    event_type: &str,
    payload: Option<&str>,
) -> Result<()> {
    db.insert_gamify_event(agent_id, event_type, payload)
        .await?;
    Ok(())
}

// ── Cost Records ─────────────────────────────────────────

/// A recorded LLM cost event for a single agent inference call.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CostRecord {
    /// Monotonic database row ID.
    pub id: i64,
    /// Identifier of the agent that incurred the cost.
    pub agent_id: String,
    /// Optional session correlation ID.
    pub session_id: Option<String>,
    /// Provider backend name (e.g. `"google"`, `"openai"`).
    pub provider: String,
    /// Model identifier used for the inference.
    pub model: Option<String>,
    /// Number of prompt (input) tokens consumed.
    pub input_tokens: i64,
    /// Number of completion (output) tokens produced.
    pub output_tokens: i64,
    /// Estimated cost in USD for this call.
    pub cost_usd: f64,
    /// SQLite `datetime` string when the record was inserted.
    pub timestamp: String,
}

impl CostRecord {
    /// Create a new cost record for in-memory tracking.
    pub fn new_ephemeral(
        agent_id: impl Into<String>,
        provider: impl Into<String>,
        model: Option<String>,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f64,
    ) -> Self {
        Self {
            id: 0,
            agent_id: agent_id.into(),
            session_id: None,
            provider: provider.into(),
            model,
            input_tokens,
            output_tokens,
            cost_usd,
            timestamp: String::new(),
        }
    }

    /// Set the session ID.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Insert a cost record.
#[allow(clippy::too_many_arguments)]
pub async fn insert_cost_record(
    db: &Codex,
    agent_id: &str,
    session_id: Option<&str>,
    provider: &str,
    model: Option<&str>,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f64,
) -> Result<()> {
    db.insert_gamify_cost_record(
        agent_id,
        session_id,
        provider,
        model,
        input_tokens,
        output_tokens,
        cost_usd,
    )
    .await?;
    Ok(())
}

/// Get total cost for an agent.
pub async fn get_agent_cost_usd(db: &Codex, agent_id: &str) -> Result<f64> {
    Ok(db.get_gamify_agent_cost_usd(agent_id).await?)
}

/// Get cost records for an agent, most recent first.
pub async fn list_cost_records(db: &Codex, agent_id: &str, limit: i64) -> Result<Vec<CostRecord>> {
    let rows = db.list_gamify_cost_records(agent_id, limit).await?;
    let mut records = Vec::new();
    for (
        id,
        agent_id,
        session_id,
        provider,
        model,
        input_tokens,
        output_tokens,
        cost_usd,
        timestamp,
    ) in rows
    {
        records.push(CostRecord {
            id,
            agent_id,
            session_id,
            provider,
            model,
            input_tokens,
            output_tokens,
            cost_usd,
            timestamp,
        });
    }
    Ok(records)
}

/// Acknowledge an A2A message by ID.
pub async fn acknowledge_message(db: &Codex, id: i64) -> Result<()> {
    db.acknowledge_a2a_message_by_id(id).await?;
    Ok(())
}

// ── Agent Sessions ────────────────────────────────────────

/// Persisted snapshot of an agent's run session for lifecycle tracking.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentSessionRecord {
    /// Unique session identifier (UUID string).
    pub id: String,
    /// Numeric or named agent identifier.
    pub agent_id: String,
    /// Human-readable name of the agent, if assigned.
    pub agent_name: Option<String>,
    /// SQLite `datetime` when the session began.
    pub started_at: String,
    /// SQLite `datetime` when the session ended, or `None` if still active.
    pub ended_at: Option<String>,
    /// Session lifecycle status: `"active"`, `"archived"`, etc.
    pub status: String,
    /// JSON snapshot of the task being processed at compaction time.
    pub task_snapshot: Option<String>,
    /// Summarized context carried across compaction boundaries.
    pub context_summary: Option<String>,
}

/// Insert a new agent session.
pub async fn insert_agent_session(
    db: &Codex,
    id: &str,
    agent_id: &str,
    agent_name: Option<&str>,
) -> Result<()> {
    db.insert_gamify_session(id, agent_id, agent_name).await?;
    Ok(())
}

/// Update session status and optional context.
pub async fn update_agent_session(
    db: &Codex,
    id: &str,
    status: &str,
    task_snapshot: Option<&str>,
    context_summary: Option<&str>,
) -> Result<()> {
    db.update_gamify_session(id, status, task_snapshot, context_summary)
        .await?;
    Ok(())
}

/// End a session by setting ended_at and status.
pub async fn end_agent_session(db: &Codex, id: &str, status: &str) -> Result<()> {
    db.end_gamify_session(id, status).await?;
    Ok(())
}

/// Get active sessions.
pub async fn list_active_sessions(db: &Codex) -> Result<Vec<AgentSessionRecord>> {
    let rows = db.list_gamify_active_sessions().await?;
    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(AgentSessionRecord {
            id: row.0,
            agent_id: row.1,
            agent_name: row.2,
            started_at: row.3,
            ended_at: row.4,
            status: row.5,
            task_snapshot: row.6,
            context_summary: row.7,
        });
    }
    Ok(sessions)
}

// ── Agent Metrics ─────────────────────────────────────────

/// Upsert an aggregated metric for an agent.
pub async fn upsert_agent_metric(
    db: &Codex,
    agent_id: &str,
    metric_name: &str,
    metric_value: f64,
    period: &str,
) -> Result<()> {
    db.upsert_gamify_agent_metric(agent_id, metric_name, metric_value, period)
        .await?;
    Ok(())
}

/// Get all metrics for an agent in a given period.
pub async fn get_agent_metrics(
    db: &Codex,
    agent_id: &str,
    period: &str,
) -> Result<std::collections::HashMap<String, f64>> {
    let metrics = db.get_gamify_agent_metrics(agent_id, period).await?;
    let mut map = std::collections::HashMap::new();
    for (name, val) in metrics {
        map.insert(name, val);
    }
    Ok(map)
}

/// Process an orchestrator event for gamification rewards (XP, crystals, companion stats).
///
/// Handles all `AgentEventKind` variants by delegating companion stat changes to
/// `Companion::interact()` (SSOT) and awarding profile XP/crystals as appropriate.
/// No-ops when gamification is disabled in config.
pub async fn process_event_rewards(
    db: &Codex,
    user_id: &str,
    event_kind: &serde_json::Value,
) -> Result<crate::reward_policy::RouteResult> {
    use crate::companion::Interaction;

    // Early exit when gamify is disabled
    if !crate::config_gate::is_enabled() {
        tracing::trace!("gamify disabled, skipping reward write");
        return Ok(Default::default());
    }

    // 1. Get/Create profile
    let mut profile = match get_profile(db, user_id).await? {
        Some(p) => p,
        None => crate::profile::LudusProfile::new_default(user_id),
    };

    // 2. Extract event type and agent info
    //    serde(tag = "type", rename_all = "snake_case") → e.g. "task_completed"
    let event_type = event_kind
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let agent_id = event_kind.get("agent_id").and_then(|v| v.as_u64());
    let agent_id_str = agent_id.map(|id| format!("agent-{}", id));

    // 3. Get/Create companion for the agent involved (if any)
    let mut companion = if let Some(aid) = &agent_id_str {
        list_companions(db, user_id)
            .await?
            .into_iter()
            .find(|c| c.id == *aid)
            .unwrap_or_else(|| Companion::new(aid.clone(), user_id, aid.clone(), "vox"))
    } else {
        Companion::new("_none", user_id, "_none", "vox")
    };

    // 4. Daily Quest Generation (check-in)
    {
        let count = count_quests(db, user_id).await.unwrap_or(0);
        if count == 0 {
            // Generate new daily quests if none active for today
            let daily_quests = crate::quest::todays_quests(user_id);
            for q in daily_quests {
                let _ = upsert_quest(db, &q).await;
            }
        }
    }

    let mut profile_changed = false;
    let mut companion_changed = false;

    // 2b. Update daily streak & detect day change for counters
    let today = crate::quest::current_day_number();
    let last_active_day = profile.last_active as u64 / 86400;
    let streak_res = profile.record_daily_activity();
    if streak_res != crate::streak::StreakResult::AlreadyActive {
        profile_changed = true;

        if today > last_active_day {
            // New day detected: Reset daily counters
            let _ = set_counter(db, user_id, "tasks_today", 0).await;
        }
    }

    // 5. Apply policy-driven rewards
    let mut policy_snapshot: Option<(u64, u64, f64, u64, u64, u32, bool, i64)> = None;
    let mut leveled_up_info = None;
    let mut final_reward = None;
    {
        use crate::reward_policy::{apply_policy, base_reward};
        use std::sync::{Mutex, OnceLock};
        static SESSION: OnceLock<Mutex<crate::reward_policy::SessionState>> = OnceLock::new();
        let session_lock = SESSION.get_or_init(|| Mutex::new(Default::default()));
        let mode_mult = crate::config_gate::reward_multiplier();
        let streak_days = profile.streak.current_streak as u32;
        let mut base_rw = None;
        let mut rw = None;
        if let Ok(mut session) = session_lock.try_lock() {
            let base = base_reward(event_type);
            let reward = apply_policy(&base, mode_mult, streak_days, event_type, &mut session);
            base_rw = Some(base);
            rw = Some(reward);
        }
        if let (Some(base), Some(reward)) = (base_rw, rw) {
            final_reward = Some(reward.clone());
            if reward.xp > 0 {
                let old_level = profile.level;
                let leveled_up = profile.add_xp(reward.xp);
                if leveled_up && profile.level > old_level {
                    leveled_up_info = Some((profile.level, profile.title(), profile.xp));
                }
                profile_changed = true;
            }
            if reward.crystals > 0 {
                profile.add_crystals(reward.crystals);
                profile_changed = true;
            }
            if reward.lumens != 0 {
                profile.add_lumens(reward.lumens);
                profile_changed = true;

                // Aggregate lumens for the player's collegium
                if let Ok(Some((cid, _, _, _))) = get_user_collegium(db, user_id).await {
                    let _ = update_collegium_lumens(db, &cid, reward.lumens).await;
                }
            }
            if reward.grant_shield {
                profile.earn_shield();
                profile_changed = true;
            }
            if reward.xp > 0 || reward.crystals > 0 || reward.lumens != 0 || reward.grant_shield {
                policy_snapshot = Some((
                    base.xp,
                    base.crystals,
                    reward.effective_multiplier,
                    reward.xp,
                    reward.crystals,
                    streak_days,
                    reward.grind_capped,
                    reward.lumens,
                ));
            }
        }
    }

    // 5b. Record level up (now safe to await outside the sync lock)
    if let Some((lvl, ref title, xp)) = leveled_up_info {
        let _ = record_level_up(db, user_id, lvl, title, xp).await;
    }
    if let Some((base_xp, base_crystals, eff_mult, rxp, rcrystals, streak, grind_capped, rlumens)) =
        policy_snapshot
    {
        let _ = insert_policy_snapshot(
            db,
            user_id,
            event_type,
            base_xp,
            base_crystals,
            &format!("{:?}", crate::config_gate::mode()),
            eff_mult,
            rxp,
            rcrystals,
            streak,
            grind_capped,
            rlumens,
        )
        .await;
    }

    // 6. Update persistent counters and check achievements
    {
        let counter_names = match event_type {
            "task_completed" => vec!["tasks_completed", "tasks_today"],
            "agent_spawned" => vec!["agents_spawned"],
            "bug_fix" => vec!["bug_fixes"],
            "test_pass" => vec!["tests_passed"],
            "doc_added" => vec!["docs_added"],
            "peer_teach_session" => vec!["peer_teach_sessions"],
            "migration_applied" => vec!["migrations_applied"],
            "seed_completed" => vec!["seeds_run"],
            "island_built" => vec!["islands_built"],
            "v0_import_complete" => vec!["v0_imports"],
            "scheduled_job_ran" => vec!["scheduled_jobs_run"],
            "turso_query_executed" => vec!["turso_queries"],
            "mcp_tool_called" => vec!["mcp_tool_calls"],
            "mcp_tool_registered" => vec!["mcp_tools_registered"],
            "pkg_published" => vec!["packages_published"],
            "workflow_completed" => vec!["workflows_completed"],
            "security_review_passed" => vec!["security_reviews_passed"],
            "perf_regression_caught" => vec!["perf_regressions_caught"],
            "unsafe_removed" => vec!["unsafe_blocks_removed"],
            "ai_thumbs_up" => vec!["ai_feedback_count", "ai_positive_feedback_given"],
            "ai_thumbs_down" => vec!["ai_feedback_count"],
            "ai_example_written" => vec!["ai_examples_written"],
            "populi_corpus_contributed" => vec!["corpus_contributions"],
            "build_clean" => vec!["green_builds"],
            "toestub_violations_fixed" => vec!["toestub_violations_fixed"],
            "finetune_epoch" => vec!["finetune_epochs"],
            "inference_run" => vec!["inference_runs"],
            "daily_quest_completed" => vec!["daily_quests_completed"],
            _ => vec![],
        };

        if !counter_names.is_empty() || profile_changed {
            let mut tracker = crate::achievement::AchievementTracker::new();
            let unlocked_ids: std::collections::HashSet<String> =
                list_unlocked_achievements(db, user_id)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(id, _)| id)
                    .collect();

            for cname in counter_names {
                let new_val = increment_counter(db, user_id, cname, 1).await.unwrap_or(0);
                let newly_unlocked = tracker.check_unlocks("_current", cname, new_val);
                for ach in newly_unlocked {
                    if !unlocked_ids.contains(&ach.id.0) {
                        let _ = unlock_achievement(
                            db,
                            user_id,
                            &ach.id.0,
                            ach.xp_reward,
                            ach.crystal_reward,
                        )
                        .await;
                    }
                }
            }

            // Level-based achievements
            let level_unlocked =
                tracker.check_unlocks("_current", "player_level", profile.level as u32);
            for ach in level_unlocked {
                if !unlocked_ids.contains(&ach.id.0) {
                    let _ = unlock_achievement(
                        db,
                        user_id,
                        &ach.id.0,
                        ach.xp_reward,
                        ach.crystal_reward,
                    )
                    .await;
                }
            }

            // Lifetime XP milestone (million)
            if profile.total_xp_earned >= 1_000_000 {
                let xp_unlocked = tracker.check_unlocks(
                    "_current",
                    "lifetime_xp_millions",
                    (profile.total_xp_earned / 1_000_000) as u32,
                );
                for ach in xp_unlocked {
                    if !unlocked_ids.contains(&ach.id.0) {
                        let _ = unlock_achievement(
                            db,
                            user_id,
                            &ach.id.0,
                            ach.xp_reward,
                            ach.crystal_reward,
                        )
                        .await;
                    }
                }
            }
        }
    }

    match event_type {
        // ── Task lifecycle ───────────────────────────────
        "task_completed" => {
            companion.interact(Interaction::TaskCompleted);
            companion.code_quality = (companion.code_quality + 1).min(100);
            companion_changed = true;
            advance_quests(db, &mut profile, user_id, crate::quest::QuestType::Improve).await;
            advance_quests(
                db,
                &mut profile,
                user_id,
                crate::quest::QuestType::AgentComplete,
            )
            .await;
            profile_changed = true;
        }
        "bug_fix" | "bug_battle_won" => {
            companion.interact(Interaction::TaskCompleted);
            companion_changed = true;
            advance_quests(db, &mut profile, user_id, crate::quest::QuestType::Battle).await;
            profile_changed = true;
        }
        "task_started" => {
            companion.interact(Interaction::TaskAssigned);
            companion_changed = true;
        }
        "task_failed" => {
            companion.interact(Interaction::TaskFailed);
            companion_changed = true;
        }

        // ── Collaboration ────────────────────────────────
        "plan_handoff" | "agent_handoff_accepted" | "peer_teach_session" => {
            advance_quests(
                db,
                &mut profile,
                user_id,
                crate::quest::QuestType::Collaborate,
            )
            .await;
            profile_changed = true;
        }

        // ── Code Quality ─────────────────────────────────
        "refactor" | "fmt_applied" | "toestub_violations_fixed" => {
            advance_quests(db, &mut profile, user_id, crate::quest::QuestType::Improve).await;
            if event_type == "toestub_violations_fixed" {
                advance_quests(
                    db,
                    &mut profile,
                    user_id,
                    crate::quest::QuestType::ToestubFix,
                )
                .await;
            }
            profile_changed = true;
        }
        "test_pass" | "test_coverage_improved" => {
            advance_quests(db, &mut profile, user_id, crate::quest::QuestType::Testing).await;
            profile_changed = true;
        }

        // ── AI & Mens ──────────────────────────────────
        "ai_thumbs_up" | "ai_thumbs_down" => {
            advance_quests(
                db,
                &mut profile,
                user_id,
                crate::quest::QuestType::AiFeedback,
            )
            .await;
            profile_changed = true;
        }
        "populi_corpus_contributed" => {
            advance_quests(
                db,
                &mut profile,
                user_id,
                crate::quest::QuestType::PopuliContribute,
            )
            .await;
            profile_changed = true;
        }

        // ── Package & Registry ──────────────────────────
        "pkg_published" | "mcp_tool_registered" => {
            advance_quests(db, &mut profile, user_id, crate::quest::QuestType::Create).await;
            profile_changed = true;
        }

        // ── Cost & Security ──────────────────────────────
        "cost_incurred" => {
            profile.spend_energy(1);
            profile_changed = true;
            companion_changed = true;
        }
        "unsafe_removed" | "security_review_passed" => {
            advance_quests(db, &mut profile, user_id, crate::quest::QuestType::Improve).await;
            profile_changed = true;
        }
        "activity_changed" => {
            if let Some(act) = event_kind.get("activity").and_then(|v| v.as_str()) {
                let interaction = match act {
                    "writing" => Interaction::Writing,
                    "idle" => Interaction::Idle,
                    _ => Interaction::Idle,
                };
                companion.interact(interaction);
                companion_changed = true;
            }
        }
        _ => {}
    }

    // 4. Persist changes
    if profile_changed {
        upsert_profile(db, &profile).await?;
    }
    if companion_changed && agent_id_str.is_some() {
        upsert_companion(db, &companion).await?;
    }

    Ok(crate::reward_policy::RouteResult {
        reward: final_reward,
        leveled_up: leveled_up_info.map(|(lvl, title, _xp)| (lvl, title)),
    })
}

// ── Teaching persistence ──────────────────────────────────

use crate::teaching::TeachingProfile;

/// Load a teaching profile. Returns a fresh default if none exists yet.
pub async fn get_teaching_profile(db: &Codex, user_id: &str) -> Result<TeachingProfile> {
    let mut rows = db
        .connection()
        .query(
            "SELECT stage, silenced, mistake_counts, cooldowns
             FROM gamify_teaching_profiles WHERE user_id = ?1",
            params![user_id.to_string()],
        )
        .await?;

    if let Some(row) = rows.next().await? {
        let stage_str: String = row.get(0)?;
        let silenced: i64 = row.get(1)?;
        let counts_json: String = row.get(2)?;
        let cooldowns_json: String = row.get(3)?;

        let stage = match stage_str.as_str() {
            "guided" => crate::teaching::TutorialStage::Guided,
            "independent" => crate::teaching::TutorialStage::Independent,
            _ => crate::teaching::TutorialStage::Onboarding,
        };
        let mistake_counts = serde_json::from_str(&counts_json).unwrap_or_default();
        let cooldowns = serde_json::from_str(&cooldowns_json).unwrap_or_default();

        Ok(TeachingProfile {
            user_id: user_id.to_string(),
            stage,
            silenced: silenced != 0,
            mistake_counts,
            cooldowns,
        })
    } else {
        Ok(TeachingProfile::new(user_id))
    }
}

/// Upsert a teaching profile.
pub async fn upsert_teaching_profile(db: &Codex, profile: &TeachingProfile) -> Result<()> {
    let stage_str = match profile.stage {
        crate::teaching::TutorialStage::Onboarding => "onboarding",
        crate::teaching::TutorialStage::Guided => "guided",
        crate::teaching::TutorialStage::Independent => "independent",
    };
    let counts_json = serde_json::to_string(&profile.mistake_counts).unwrap_or_default();
    let cooldowns_json = serde_json::to_string(&profile.cooldowns).unwrap_or_default();

    db.connection().execute(
        "INSERT INTO gamify_teaching_profiles (user_id, stage, silenced, mistake_counts, cooldowns)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(user_id) DO UPDATE SET
            stage = excluded.stage,
            silenced = excluded.silenced,
            mistake_counts = excluded.mistake_counts,
            cooldowns = excluded.cooldowns,
            updated_at = datetime('now')",
        params![
            profile.user_id.clone(),
            stage_str.to_string(),
            if profile.silenced { 1i64 } else { 0i64 },
            counts_json,
            cooldowns_json,
        ],
    ).await?;
    Ok(())
}

/// Insert a reward policy diagnostic snapshot.
#[allow(clippy::too_many_arguments)]
pub async fn insert_policy_snapshot(
    db: &Codex,
    user_id: &str,
    event_type: &str,
    base_xp: u64,
    base_crystals: u64,
    mode: &str,
    effective_multiplier: f64,
    xp_awarded: u64,
    crystals_awarded: u64,
    streak_days: u32,
    grind_capped: bool,
    lumens_awarded: i64,
) -> Result<()> {
    db.connection().execute(
        "INSERT INTO gamify_policy_snapshots
         (user_id, event_type, base_xp, base_crystals, mode, effective_multiplier, xp_awarded, crystals_awarded, streak_days, grind_capped, lumens_awarded)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            user_id.to_string(),
            event_type.to_string(),
            base_xp as i64,
            base_crystals as i64,
            mode.to_string(),
            effective_multiplier,
            xp_awarded as i64,
            crystals_awarded as i64,
            streak_days as i64,
            if grind_capped { 1i64 } else { 0i64 },
            lumens_awarded,
        ],
    ).await?;
    Ok(())
}

/// Helper: advance quests of a specific type and award bonus XP/crystals on completion.
async fn advance_quests(
    db: &Codex,
    profile: &mut crate::profile::LudusProfile,
    user_id: &str,
    quest_type: crate::quest::QuestType,
) {
    if let Ok(mut quests) = list_quests(db, user_id).await {
        for q in quests.iter_mut() {
            if q.quest_type == quest_type && !q.completed {
                if q.increment(1) {
                    profile.add_xp(q.xp_reward);
                    profile.add_crystals(q.crystal_reward);
                }
                let _ = upsert_quest(db, q).await;
            }
        }
    }
}

// ── Notifications ─────────────────────────────────────────

/// Default TTL for notifications: 7 days in seconds.
const NOTIF_TTL_SECS: i64 = 7 * 24 * 3600;

/// Persist a new notification. Expired_at is set to now + 7 days by default.
pub async fn insert_notification(
    db: &Codex,
    notif: &crate::notifications::Notification,
) -> Result<()> {
    let expires = notif.created_at + NOTIF_TTL_SECS;
    let notif_type = format!("{:?}", notif.notification_type);
    db.connection()
        .execute(
            "INSERT OR IGNORE INTO gamify_notifications
             (id, user_id, notification_type, title, message, read, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                notif.id.clone(),
                notif.user_id.clone(),
                notif_type,
                notif.title.clone(),
                notif.message.clone(),
                if notif.read { 1i64 } else { 0i64 },
                notif.created_at,
                expires,
            ],
        )
        .await?;
    Ok(())
}

/// List unread notifications for a user (up to `limit`).
pub async fn list_unread_notifications(
    db: &Codex,
    user_id: &str,
    limit: u32,
) -> Result<Vec<crate::notifications::Notification>> {
    let now = crate::util::now_unix();
    let mut rows = db
        .connection()
        .query(
            "SELECT id, notification_type, title, message, created_at
             FROM gamify_notifications
             WHERE user_id = ?1 AND read = 0 AND (expires_at = 0 OR expires_at > ?2)
             ORDER BY created_at DESC
             LIMIT ?3",
            params![user_id, now, limit as i64],
        )
        .await?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        let notif_type_str: String = row.get(1)?;
        let notif_type = parse_notification_type(&notif_type_str);
        out.push(crate::notifications::Notification {
            id: row.get(0)?,
            user_id: user_id.to_string(),
            notification_type: notif_type,
            title: row.get(2)?,
            message: row.get(3)?,
            read: false,
            created_at: row.get::<i64>(4)?,
        });
    }
    Ok(out)
}

/// Mark a notification as read by ID.
pub async fn mark_notification_read(db: &Codex, notif_id: &str) -> Result<()> {
    db.connection()
        .execute(
            "UPDATE gamify_notifications SET read = 1 WHERE id = ?1",
            params![notif_id],
        )
        .await?;
    Ok(())
}

/// Mark all unread notifications for a user as read.
pub async fn mark_all_notifications_read(db: &Codex, user_id: &str) -> Result<()> {
    db.connection()
        .execute(
            "UPDATE gamify_notifications SET read = 1 WHERE user_id = ?1 AND read = 0",
            params![user_id],
        )
        .await?;
    Ok(())
}

/// Delete notifications older than their `expires_at` timestamp (TTL cleanup).
pub async fn cleanup_expired_notifications(db: &Codex) -> Result<u64> {
    let now = crate::util::now_unix();
    let rows = db
        .connection()
        .execute(
            "DELETE FROM gamify_notifications WHERE expires_at > 0 AND expires_at < ?1",
            params![now],
        )
        .await?;
    Ok(rows)
}

fn parse_notification_type(s: &str) -> crate::notifications::NotificationType {
    use crate::notifications::NotificationType;
    match s {
        "LevelUp" => NotificationType::LevelUp,
        "AchievementUnlocked" => NotificationType::AchievementUnlocked,
        "StreakContinued" => NotificationType::StreakContinued,
        "StreakLost" => NotificationType::StreakLost,
        "ChallengeCompleted" => NotificationType::ChallengeCompleted,
        "CompanionStatus" => NotificationType::CompanionStatus,
        "QuestCompleted" => NotificationType::QuestCompleted,
        _ => NotificationType::CompanionStatus,
    }
}

// ── Feedback ─────────────────────────────────────────────

use crate::feedback::AiFeedback;

/// Insert a piece of AI feedback.
pub async fn insert_feedback(db: &Codex, fb: &AiFeedback) -> Result<()> {
    db.connection().execute(
        "INSERT INTO gamify_ai_feedback
             (id, user_id, session_id, response_id, thumbs_up, comment, tokens_generated, example_code, contributed_to_corpus, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            fb.id.clone(),
            fb.user_id.clone(),
            fb.session_id.clone(),
            fb.response_id.clone(),
            if fb.thumbs_up { 1i64 } else { 0i64 },
            fb.comment.clone(),
            fb.tokens_generated as i64,
            fb.example_code.clone(),
            if fb.contributed_to_corpus { 1i64 } else { 0i64 },
            fb.created_at,
        ],
    ).await?;
    Ok(())
}

// ── Periodic Rewards ─────────────────────────────────────

use crate::periodic_reward::{PeriodicCondition, PeriodicReward};

/// Upsert a periodic reward.
pub async fn upsert_periodic_reward(db: &Codex, r: &PeriodicReward, user_id: &str) -> Result<()> {
    let condition_json = serde_json::to_string(&r.unlock_condition)
        .unwrap_or_else(|_| "\"WeeklyCheckIn\"".to_string());

    db.connection().execute(
        "INSERT INTO gamify_periodic_rewards
             (reward_id, user_id, name, icon, description, xp_bonus, crystal_bonus, redeemed, expires_at, created_at, unlock_condition)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(reward_id, user_id) DO UPDATE SET
            redeemed = excluded.redeemed",
        params![
            r.id.clone(),
            user_id,
            r.name.clone(),
            r.icon.clone(),
            r.description.clone(),
            r.xp_bonus as i64,
            r.crystal_bonus as i64,
            if r.redeemed { 1i64 } else { 0i64 },
            r.valid_until,
            crate::util::now_unix(),
            condition_json,
        ],
    ).await?;
    Ok(())
}

/// Load the current weekly reward for a user if it exists in DB.
pub async fn get_reward_claim(
    db: &Codex,
    user_id: &str,
    reward_id: &str,
) -> Result<Option<PeriodicReward>> {
    let mut rows = db.connection().query(
        "SELECT name, icon, xp_bonus, crystal_bonus, redeemed, expires_at, description, unlock_condition
         FROM gamify_periodic_rewards WHERE user_id = ?1 AND reward_id = ?2",
        params![user_id, reward_id],
    ).await?;

    if let Some(row) = rows.next().await? {
        let condition_str: String = row.get(7)?;
        let condition: PeriodicCondition =
            serde_json::from_str(&condition_str).unwrap_or(PeriodicCondition::WeeklyCheckIn);

        Ok(Some(PeriodicReward {
            id: reward_id.to_string(),
            name: row.get(0)?,
            icon: row.get(1)?,
            xp_bonus: row.get::<i64>(2)? as u64,
            crystal_bonus: row.get::<i64>(3)? as u64,
            redeemed: row.get::<i64>(4)? != 0,
            valid_until: row.get(5)?,
            unlock_condition: condition,
            description: row.get(6)?,
        }))
    } else {
        Ok(None)
    }
}

// ── Counters ─────────────────────────────────────────────

/// Get a specific counter for a user.
pub async fn get_counter(db: &Codex, user_id: &str, name: &str) -> Result<u32> {
    let mut rows = db
        .connection()
        .query(
            "SELECT count FROM gamify_counters WHERE user_id = ?1 AND counter_name = ?2",
            params![user_id, name],
        )
        .await?;
    if let Some(row) = rows.next().await? {
        Ok(row.get::<i64>(0)? as u32)
    } else {
        Ok(0)
    }
}

/// Increment a counter and return the new value.
pub async fn increment_counter(db: &Codex, user_id: &str, name: &str, amount: u32) -> Result<u32> {
    db.connection()
        .execute(
            "INSERT INTO gamify_counters (user_id, counter_name, count)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id, counter_name) DO UPDATE SET
            count = count + excluded.count",
            params![user_id, name, amount as i64],
        )
        .await?;
    get_counter(db, user_id, name).await
}

/// Set a counter to a specific value.
pub async fn set_counter(db: &Codex, user_id: &str, name: &str, value: u32) -> Result<()> {
    db.connection()
        .execute(
            "INSERT INTO gamify_counters (user_id, counter_name, count)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(user_id, counter_name) DO UPDATE SET
            count = excluded.count",
            params![user_id, name, value as i64],
        )
        .await?;
    Ok(())
}
// ── Collegium (Teams) ───────────────────────────────────

/// Create a new collegium.
pub async fn create_collegium(
    db: &Codex,
    id: &str,
    name: &str,
    description: Option<&str>,
    leader_id: &str,
) -> Result<()> {
    let now = crate::util::now_unix();
    db.connection()
        .execute(
            "INSERT INTO gamify_collegiums (id, name, description, leader_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, name, description, leader_id, now],
        )
        .await?;

    // Auto-join leader
    join_collegium(db, id, leader_id, "pontifex").await?;
    Ok(())
}

/// Join a collegium.
pub async fn join_collegium(
    db: &Codex,
    collegium_id: &str,
    user_id: &str,
    role: &str,
) -> Result<()> {
    let now = crate::util::now_unix();
    db.connection().execute(
        "INSERT OR IGNORE INTO gamify_collegium_members (collegium_id, user_id, role, joined_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![collegium_id, user_id, role, now],
    ).await?;
    Ok(())
}

/// Increment a collegium's lumens count.
pub async fn update_collegium_lumens(db: &Codex, collegium_id: &str, delta: i64) -> Result<()> {
    db.connection()
        .execute(
            "UPDATE gamify_collegiums SET lumens = lumens + ?1 WHERE id = ?2",
            params![delta, collegium_id],
        )
        .await?;
    Ok(())
}

/// List all collegiums with their total Lumens.
pub async fn list_collegiums(db: &Codex) -> Result<Vec<(String, String, i64, i64)>> {
    let mut rows = db.connection().query(
        "SELECT id, name, lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = id) FROM gamify_collegiums ORDER BY lumens DESC",
        params![],
    ).await?;

    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
    }
    Ok(out)
}

/// Get a specific collegium.
pub async fn get_collegium(db: &Codex, id: &str) -> Result<Option<(String, String, i64, i64)>> {
    let mut rows = db.connection().query(
        "SELECT id, name, lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = id) FROM gamify_collegiums WHERE id = ?1",
        params![id],
    ).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
    } else {
        Ok(None)
    }
}

/// Get the collegium a user belongs to.
pub async fn get_user_collegium(
    db: &Codex,
    user_id: &str,
) -> Result<Option<(String, String, i64, i64)>> {
    let mut rows = db.connection().query(
        "SELECT c.id, c.name, c.lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = c.id)
         FROM gamify_collegiums c
         JOIN gamify_collegium_members m ON m.collegium_id = c.id
         WHERE m.user_id = ?1",
        params![user_id],
    ).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
    } else {
        Ok(None)
    }
}

// ── Arena (Events) ──────────────────────────────────────

/// A community event in the Arena.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ArenaEvent {
    /// Unique event identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Detailed event description.
    pub description: String,
    /// Start timestamp (Unix seconds).
    pub start_ts: i64,
    /// End timestamp (Unix seconds).
    pub end_ts: i64,
    /// Total XP target for the community.
    pub target_xp: i64,
    /// Current XP progress.
    pub current_xp: i64,
    /// Total Lumen target for the community.
    pub target_lumens: i64,
    /// Current Lumen progress.
    pub current_lumens: i64,
}

/// Get the currently active arena event.
pub async fn get_active_arena_event(db: &Codex) -> Result<Option<ArenaEvent>> {
    let now = crate::util::now_unix();
    let mut rows = db.connection().query(
        "SELECT id, name, description, start_ts, end_ts, target_xp, current_xp, target_lumens, current_lumens
         FROM gamify_arena_events
         WHERE status = 'active' AND start_ts <= ?1 AND end_ts >= ?1",
        params![now],
    ).await?;
    if let Some(row) = rows.next().await? {
        Ok(Some(ArenaEvent {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            start_ts: row.get(3)?,
            end_ts: row.get(4)?,
            target_xp: row.get(5)?,
            current_xp: row.get(6)?,
            target_lumens: row.get(7)?,
            current_lumens: row.get(8)?,
        }))
    } else {
        Ok(None)
    }
}

/// Join an arena event.
pub async fn join_arena_event(db: &Codex, event_id: &str, user_id: &str) -> Result<()> {
    let now = crate::util::now_unix();
    db.connection()
        .execute(
            "INSERT OR IGNORE INTO gamify_arena_participants (event_id, user_id, joined_at)
         VALUES (?1, ?2, ?3)",
            params![event_id, user_id, now],
        )
        .await?;
    Ok(())
}

/// Get a user's contribution to an arena event.
pub async fn get_arena_contribution(
    db: &Codex,
    event_id: &str,
    user_id: &str,
) -> Result<(i64, i64)> {
    let mut rows = db.connection().query(
        "SELECT xp_contributed, lumens_contributed FROM gamify_arena_participants WHERE event_id = ?1 AND user_id = ?2",
        params![event_id, user_id],
    ).await?;
    if let Some(row) = rows.next().await? {
        Ok((row.get(0)?, row.get(1)?))
    } else {
        Ok((0, 0))
    }
}

/// Get arena event leaderboard.
pub async fn arena_event_leaderboard(
    db: &Codex,
    event_id: &str,
    limit: i64,
) -> Result<Vec<(String, i64, i64)>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT user_id, xp_contributed, lumens_contributed
         FROM gamify_arena_participants
         WHERE event_id = ?1
         ORDER BY (xp_contributed + lumens_contributed * 10) DESC
         LIMIT ?2",
            params![event_id, limit],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push((row.get(0)?, row.get(1)?, row.get(2)?));
    }
    Ok(out)
}
