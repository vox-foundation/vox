//! Extended gamification DB helpers: daily counters, event config overrides,
//! build-clean milestones, and phoenix bonus detection.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

fn current_day() -> i64 {
    crate::util::now_unix() / 86_400
}

// ── Daily Counters ────────────────────────────────────────────────────────────

/// Increment a per-user, per-event, per-day counter and return the new total.
///
/// Persists grind counts across process restarts so the tiered reward decay
/// in `reward_policy::apply_policy` applies correctly even when the terminal
/// is closed and reopened within the same calendar day.
pub async fn increment_daily_counter(db: &Codex, user_id: &str, event_type: &str) -> Result<i64> {
    let day = current_day();
    db.connection()
        .execute(
            "INSERT INTO gamify_daily_counters (user_id, event_type, day, count)
         VALUES (?1, ?2, ?3, 1)
         ON CONFLICT (user_id, event_type, day)
         DO UPDATE SET count = count + 1",
            params![user_id, event_type, day],
        )
        .await?;
    let mut rows = db
        .connection()
        .query(
            "SELECT count FROM gamify_daily_counters \
         WHERE user_id = ?1 AND event_type = ?2 AND day = ?3",
            params![user_id, event_type, day],
        )
        .await?;
    Ok(rows
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(1))
        .unwrap_or(1))
}

/// Get today's counter value without incrementing.
pub async fn get_daily_counter(db: &Codex, user_id: &str, event_type: &str) -> Result<i64> {
    let day = current_day();
    let mut rows = db
        .connection()
        .query(
            "SELECT count FROM gamify_daily_counters \
         WHERE user_id = ?1 AND event_type = ?2 AND day = ?3",
            params![user_id, event_type, day],
        )
        .await?;
    Ok(rows
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0))
}

// ── Event Config Overrides ────────────────────────────────────────────────────

/// Load all enabled event config overrides from DB into `EventConfigOverrides`.
pub async fn load_event_config_overrides(
    db: &Codex,
) -> Result<crate::reward_policy::EventConfigOverrides> {
    let mut ov = crate::reward_policy::EventConfigOverrides::default();
    let mut rows = db
        .connection()
        .query(
            "SELECT event_type, xp_override, crystals_override \
         FROM gamify_event_config WHERE enabled = 1",
            turso::params![],
        )
        .await?;
    while let Some(row) = rows.next().await? {
        let ev: String = row.get(0)?;
        let xp = row.get::<i64>(1).unwrap_or(0).max(0) as u64;
        let crystals = row.get::<i64>(2).unwrap_or(0).max(0) as u64;
        ov.set(ev, xp, crystals);
    }
    Ok(ov)
}

/// Upsert an event config override row (admin / `vox ludus config-set`).
pub async fn set_event_config_override(
    db: &Codex,
    event_type: &str,
    xp_override: Option<u64>,
    crystals_override: Option<u64>,
    enabled: bool,
) -> Result<()> {
    let now = crate::util::now_unix();
    db.connection()
        .execute(
            "INSERT INTO gamify_event_config \
             (event_type, xp_override, crystals_override, enabled, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (event_type) DO UPDATE SET
             xp_override       = excluded.xp_override,
             crystals_override = excluded.crystals_override,
             enabled           = excluded.enabled,
             updated_at        = excluded.updated_at",
            params![
                event_type,
                xp_override.map(|v| v as i64),
                crystals_override.map(|v| v as i64),
                if enabled { 1i64 } else { 0i64 },
                now,
            ],
        )
        .await?;
    Ok(())
}

// ── Special Checks ──────────────────────────────────────────────────────────

/// Returns `true` if the user has already had at least one `build_failed` today.
pub async fn has_failed_today(db: &Codex, user_id: &str) -> Result<bool> {
    let day = current_day();
    let mut rows = db
        .connection()
        .query(
            "SELECT count FROM gamify_daily_counters \
         WHERE user_id=?1 AND event_type='build_failed' AND day=?2",
            params![user_id, day],
        )
        .await?;
    Ok(rows
        .next()
        .await?
        .map(|row| row.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0)
        >= 1)
}
