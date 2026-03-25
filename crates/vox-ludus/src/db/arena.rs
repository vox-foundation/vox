//! Arena (community events) persistence.

use anyhow::Result;
use turso::params;
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
