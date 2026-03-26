//! Periodic (weekly) rewards persistence.

use anyhow::Result;
use vox_db::Codex;

use crate::periodic_reward::{PeriodicCondition, PeriodicReward};

/// Upsert a periodic reward.
pub async fn upsert_periodic_reward(db: &Codex, r: &PeriodicReward, user_id: &str) -> Result<()> {
    let condition_json = serde_json::to_string(&r.unlock_condition)
        .unwrap_or_else(|_| "\"WeeklyCheckIn\"".to_string());

    db.upsert_gamify_periodic_reward(
        r.id.as_str(),
        user_id,
        r.name.as_str(),
        r.icon.as_str(),
        r.description.as_str(),
        r.xp_bonus as i64,
        r.crystal_bonus as i64,
        r.redeemed,
        r.valid_until,
        crate::util::now_unix(),
        condition_json.as_str(),
    )
    .await
    .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Load the current weekly reward for a user if it exists in DB.
pub async fn get_reward_claim(
    db: &Codex,
    user_id: &str,
    reward_id: &str,
) -> Result<Option<PeriodicReward>> {
    let row = db
        .get_gamify_periodic_reward_row(user_id, reward_id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    if let Some((name, icon, xp_bonus, crystal_bonus, redeemed, expires_at, description, condition_str)) =
        row
    {
        let condition: PeriodicCondition =
            serde_json::from_str(&condition_str).unwrap_or(PeriodicCondition::WeeklyCheckIn);

        Ok(Some(PeriodicReward {
            id: reward_id.to_string(),
            name,
            icon,
            xp_bonus: xp_bonus as u64,
            crystal_bonus: crystal_bonus as u64,
            redeemed,
            valid_until: expires_at,
            unlock_condition: condition,
            description,
        }))
    } else {
        Ok(None)
    }
}
