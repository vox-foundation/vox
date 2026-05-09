//! Profile and achievement persistence.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

use crate::profile::LudusProfile;

/// Load a gamify profile from the DB.
pub async fn get_profile(db: &Codex, user_id: &str) -> Result<Option<LudusProfile>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT level, xp, crystals, energy, max_energy,
                    CAST(COALESCE(last_energy_regen, 0) AS INTEGER),
                    CAST(COALESCE(last_active, 0) AS INTEGER),
                    COALESCE(streak_days, 0), COALESCE(longest_streak, 0),
                    COALESCE(streak_last_ts, 0), COALESCE(grace_available, 1), COALESCE(grace_used, 0),
                    COALESCE(total_xp_earned, 0), COALESCE(prestige_level, 0),
                    COALESCE(lumens, 0), COALESCE(generosity_lumens, 0), COALESCE(streak_shields, 0),
                    COALESCE(trust_tier, 0), COALESCE(reward_suppressed, 0), COALESCE(suppressed_until_ts, 0)
             FROM gamify_profiles WHERE user_id = ?1",
            params![user_id],
        )
        .await?;
    if let Some(row) = rows.next().await? {
        let vals: Vec<i64> = (0..20).map(|i| row.get::<i64>(i).unwrap_or(0)).collect();
        let streak = crate::streak::StreakTracker {
            current_streak: vals[7] as u64,
            longest_streak: vals[8] as u64,
            last_activity_ts: vals[9],
            grace_periods_available: vals[10] as u64,
            grace_periods_used: vals[11] as u64,
        };
        Ok(Some(LudusProfile {
            user_id: user_id.to_string(),
            level: vals[0] as u64,
            xp: vals[1] as u64,
            crystals: vals[2] as u64,
            energy: vals[3] as u64,
            max_energy: vals[4] as u64,
            last_energy_regen: vals[5],
            last_active: vals[6],
            streak,
            total_xp_earned: vals[12] as u64,
            prestige_level: vals[13] as u32,
            lumens: vals[14],
            generosity_lumens: vals[15],
            streak_shields: vals[16] as i32,
            trust_tier: match vals[17] {
                1 => crate::profile::TrustTier::Linked,
                2 => crate::profile::TrustTier::Proven,
                3 => crate::profile::TrustTier::Master,
                _ => crate::profile::TrustTier::Novice,
            },
            reward_suppressed: vals[18] != 0,
            suppressed_until_ts: vals[19],
        }))
    } else {
        Ok(None)
    }
}

/// Upsert a gamify profile to the DB (includes streak state).
#[allow(clippy::too_many_arguments)]
pub async fn upsert_profile(db: &Codex, p: &LudusProfile) -> Result<()> {
    let user_id = p.user_id.clone();
    let level = p.level as i64;
    let xp = p.xp as i64;
    let crystals = p.crystals as i64;
    let energy = p.energy as i64;
    let max_energy = p.max_energy as i64;
    let last_energy_regen = p.last_energy_regen;
    let last_active = p.last_active;
    let streak_days = p.streak.current_streak as i64;
    let longest_streak = p.streak.longest_streak as i64;
    let streak_last_ts = p.streak.last_activity_ts;
    let grace_available = p.streak.grace_periods_available as i64;
    let grace_used = p.streak.grace_periods_used as i64;
    let total_xp_earned = p.total_xp_earned as i64;
    let prestige_level = p.prestige_level as i64;
    let lumens = p.lumens;
    let generosity_lumens = p.generosity_lumens;
    let streak_shields = p.streak_shields as i64;
    let trust_tier = p.trust_tier as i64;
    let reward_suppressed: i64 = if p.reward_suppressed { 1 } else { 0 };
    let suppressed_until_ts = p.suppressed_until_ts;
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_profiles
                 (user_id, level, xp, crystals, energy, max_energy, last_energy_regen, last_active,
                  streak_days, longest_streak, streak_last_ts, grace_available, grace_used,
                  total_xp_earned, prestige_level, lumens, generosity_lumens, streak_shields, trust_tier,
                  reward_suppressed, suppressed_until_ts)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
                 ON CONFLICT(user_id) DO UPDATE SET
                   level = excluded.level, xp = excluded.xp, crystals = excluded.crystals,
                   energy = excluded.energy, max_energy = excluded.max_energy,
                   last_energy_regen = excluded.last_energy_regen, last_active = excluded.last_active,
                   streak_days = excluded.streak_days, longest_streak = excluded.longest_streak,
                   streak_last_ts = excluded.streak_last_ts, grace_available = excluded.grace_available,
                   grace_used = excluded.grace_used, total_xp_earned = excluded.total_xp_earned,
                   prestige_level = excluded.prestige_level, lumens = excluded.lumens,
                   generosity_lumens = excluded.generosity_lumens, streak_shields = excluded.streak_shields,
                   trust_tier = excluded.trust_tier, reward_suppressed = excluded.reward_suppressed,
                   suppressed_until_ts = excluded.suppressed_until_ts",
                params![
                    user_id.as_str(),
                    level, xp, crystals, energy, max_energy,
                    last_energy_regen, last_active,
                    streak_days, longest_streak, streak_last_ts,
                    grace_available, grace_used, total_xp_earned, prestige_level,
                    lumens, generosity_lumens, streak_shields, trust_tier,
                    reward_suppressed, suppressed_until_ts
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
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
    let user_id = user_id.to_string();
    let achievement_id = achievement_id.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    let inserted = breaker
        .call(|| async move {
            let affected = conn
                .execute(
                    "INSERT OR IGNORE INTO gamify_achievements
                     (id, user_id, unlocked_at, xp_rewarded, crystals_rewarded)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        achievement_id.as_str(),
                        user_id.as_str(),
                        now,
                        xp as i64,
                        crystals as i64
                    ],
                )
                .await?;
            Ok::<_, vox_db::StoreError>(affected > 0)
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(inserted)
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
    let user_id = user_id.to_string();
    let title = title.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_level_history (user_id, level, title, xp_at_level, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    user_id.as_str(),
                    level as i64,
                    title.as_str(),
                    xp_at_level as i64,
                    now
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Load all unlocked achievement IDs for a user.
pub async fn list_unlocked_achievements(db: &Codex, user_id: &str) -> Result<Vec<(String, i64)>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT id, unlocked_at FROM gamify_achievements WHERE user_id = ?1 ORDER BY unlocked_at ASC",
            params![user_id],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push((row.get::<String>(0)?, row.get::<i64>(1)?));
    }
    Ok(out)
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
