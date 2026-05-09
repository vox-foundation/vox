//! Periodic (weekly) rewards persistence.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

use crate::periodic_reward::{PeriodicCondition, PeriodicReward};

/// Upsert a periodic reward.
#[allow(clippy::too_many_arguments)]
pub async fn upsert_periodic_reward(db: &Codex, r: &PeriodicReward, user_id: &str) -> Result<()> {
    let condition_json = serde_json::to_string(&r.unlock_condition)
        .unwrap_or_else(|_| "\"WeeklyCheckIn\"".to_string());
    let reward_id = r.id.clone();
    let user_id = user_id.to_string();
    let name = r.name.clone();
    let icon = r.icon.clone();
    let description = r.description.clone();
    let xp_bonus = r.xp_bonus as i64;
    let crystal_bonus = r.crystal_bonus as i64;
    let redeemed_flag: i64 = if r.redeemed { 1 } else { 0 };
    let expires_at = r.valid_until;
    let created_at = crate::util::now_unix();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_periodic_rewards
                 (reward_id, user_id, name, icon, description, xp_bonus, crystal_bonus, redeemed, expires_at, created_at, unlock_condition)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT(reward_id, user_id) DO UPDATE SET
                    redeemed = excluded.redeemed",
                params![
                    reward_id.as_str(), user_id.as_str(), name.as_str(), icon.as_str(),
                    description.as_str(), xp_bonus, crystal_bonus, redeemed_flag,
                    expires_at, created_at, condition_json.as_str(),
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
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
    let mut rows = db
        .connection()
        .query(
            "SELECT name, icon, xp_bonus, crystal_bonus, redeemed, expires_at, description, unlock_condition
             FROM gamify_periodic_rewards WHERE user_id = ?1 AND reward_id = ?2",
            params![user_id, reward_id],
        )
        .await?;
    if let Some(row) = rows.next().await? {
        let name: String = row.get(0)?;
        let icon: String = row.get(1)?;
        let xp_bonus: i64 = row.get(2)?;
        let crystal_bonus: i64 = row.get(3)?;
        let redeemed: bool = row.get::<i64>(4)? != 0;
        let expires_at: i64 = row.get(5)?;
        let description: String = row.get(6)?;
        let condition_str: String = row.get(7)?;
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
