//! Leaderboards and aggregate stats.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

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
    let mut rows = db
        .connection()
        .query(
            "SELECT user_id, level, xp FROM gamify_profiles ORDER BY xp DESC LIMIT ?1",
            params![limit],
        )
        .await?;
    let mut entries = Vec::new();
    while let Some(row) = rows.next().await? {
        entries.push(PlayerRankEntry {
            user_id: row.get::<String>(0)?,
            level: row.get::<i64>(1)? as u64,
            score: row.get::<i64>(2)?,
        });
    }
    Ok(entries)
}

/// Get top users by Lumens for the leaderboard.
pub async fn lumens_leaderboard(db: &Codex, limit: i64) -> Result<Vec<PlayerRankEntry>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT user_id, level, COALESCE(lumens, 0) FROM gamify_profiles ORDER BY 3 DESC LIMIT ?1",
            params![limit],
        )
        .await?;
    let mut entries = Vec::new();
    while let Some(row) = rows.next().await? {
        entries.push(PlayerRankEntry {
            user_id: row.get::<String>(0)?,
            level: row.get::<i64>(1)? as u64,
            score: row.get::<i64>(2)?,
        });
    }
    Ok(entries)
}

/// Get aggregate profile stats (e.g. total completed quests, total battles won, etc.).
pub async fn get_profile_stats(db: &Codex, user_id: &str) -> Result<serde_json::Value> {
    let mut r1 = db
        .connection()
        .query(
            "SELECT COUNT(id) FROM gamify_quests WHERE user_id=?1 AND completed=1",
            params![user_id],
        )
        .await?;
    let completed_quests = r1
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);
    let mut r2 = db
        .connection()
        .query(
            "SELECT COUNT(id) FROM gamify_battles WHERE user_id=?1 AND success=1",
            params![user_id],
        )
        .await?;
    let won_battles = r2
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);
    Ok(serde_json::json!({
        "completed_quests": completed_quests,
        "won_battles": won_battles,
    }))
}
