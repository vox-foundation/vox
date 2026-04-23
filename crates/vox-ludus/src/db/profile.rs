//! Profile and achievement persistence.

use anyhow::Result;
use vox_db::Codex;

use crate::profile::LudusProfile;

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
            trust_tier: match row[17] {
                1 => crate::profile::TrustTier::Linked,
                2 => crate::profile::TrustTier::Proven,
                3 => crate::profile::TrustTier::Master,
                _ => crate::profile::TrustTier::Novice,
            },
            reward_suppressed: row[18] != 0,
            suppressed_until_ts: row[19],
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
        p.trust_tier as i64,
        if p.reward_suppressed { 1 } else { 0 },
        p.suppressed_until_ts,
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

/// If `target_user_id` has no profile but the synthetic `default` profile exists, copy it over.
///
/// Helps MCP (`default`) vs CLI (`local_user_id`) progression split.
pub async fn merge_default_profile_into_user(db: &Codex, target_user_id: &str) -> Result<bool> {
    let default_id = crate::util::DEFAULT_USER_ID;
    if target_user_id == default_id {
        return Ok(false);
    }
    if get_profile(db, target_user_id).await?.is_some() {
        return Ok(false);
    }
    let Some(src) = get_profile(db, default_id).await? else {
        return Ok(false);
    };
    let mut merged = src;
    merged.user_id = target_user_id.to_string();
    upsert_profile(db, &merged).await?;
    Ok(true)
}
