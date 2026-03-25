//! Periodic (weekly) rewards persistence.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

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
