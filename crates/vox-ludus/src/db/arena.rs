//! Arena (community events) persistence.

use anyhow::Result;
use vox_db::Codex;

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
    let row = db
        .get_active_gamify_arena_event(now)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(row.map(
        |(id, name, description, start_ts, end_ts, target_xp, current_xp, target_lumens, current_lumens)| {
            ArenaEvent {
                id,
                name,
                description,
                start_ts,
                end_ts,
                target_xp,
                current_xp,
                target_lumens,
                current_lumens,
            }
        },
    ))
}

/// Join an arena event.
pub async fn join_arena_event(db: &Codex, event_id: &str, user_id: &str) -> Result<()> {
    let now = crate::util::now_unix();
    db.insert_gamify_arena_participant_or_ignore(event_id, user_id, now)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Get a user's contribution to an arena event.
pub async fn get_arena_contribution(
    db: &Codex,
    event_id: &str,
    user_id: &str,
) -> Result<(i64, i64)> {
    db.get_gamify_arena_contribution(event_id, user_id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Get arena event leaderboard.
pub async fn arena_event_leaderboard(
    db: &Codex,
    event_id: &str,
    limit: i64,
) -> Result<Vec<(String, i64, i64)>> {
    db.list_gamify_arena_leaderboard(event_id, limit)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}
