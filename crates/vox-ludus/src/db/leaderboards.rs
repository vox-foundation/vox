//! Leaderboards and aggregate stats.

use anyhow::Result;
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
