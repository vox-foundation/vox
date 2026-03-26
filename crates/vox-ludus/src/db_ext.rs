//! Extended gamification DB helpers: daily counters, event config overrides,
//! build-clean milestones, and phoenix bonus detection.

use anyhow::Result;
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
    db.increment_gamify_daily_counter(user_id, event_type, day)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Get today's counter value without incrementing.
pub async fn get_daily_counter(db: &Codex, user_id: &str, event_type: &str) -> Result<i64> {
    let day = current_day();
    db.get_gamify_daily_counter(user_id, event_type, day)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

// ── Event Config Overrides ────────────────────────────────────────────────────

/// Load all enabled event config overrides from DB into `EventConfigOverrides`.
pub async fn load_event_config_overrides(
    db: &Codex,
) -> Result<crate::reward_policy::EventConfigOverrides> {
    let mut ov = crate::reward_policy::EventConfigOverrides::default();
    let rows = db
        .list_gamify_event_config_overrides()
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    for (ev, xp, crystals) in rows {
        let xp = xp.max(0) as u64;
        let crystals = crystals.max(0) as u64;
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
    db.set_gamify_event_config_override(
        event_type,
        xp_override.map(|v| v as i64),
        crystals_override.map(|v| v as i64),
        enabled,
        now,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

// ── Special Checks ──────────────────────────────────────────────────────────

/// Returns `true` if the user has already had at least one `build_failed` today.
pub async fn has_failed_today(db: &Codex, user_id: &str) -> Result<bool> {
    let day = current_day();
    let n = db
        .get_gamify_daily_counter(user_id, "build_failed", day)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(n >= 1)
}
