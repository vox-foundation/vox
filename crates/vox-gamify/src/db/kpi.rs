//! Load [`crate::kpi::LudusKpiSummary`] from Codex.

use anyhow::Result;
use serde::Serialize;
use turso::params;
use vox_db::Codex;

use crate::kpi::LudusKpiSummary;

/// One row from `gamify_policy_snapshots` for CLI audit / transparency.
#[derive(Debug, Clone, Serialize)]
pub struct PolicySnapshotRow {
    pub event_type: String,
    pub base_xp: i64,
    pub base_crystals: i64,
    pub mode_label: String,
    pub effective_multiplier: f64,
    pub awarded_xp: i64,
    pub awarded_crystals: i64,
    pub grind_capped: i64,
    pub lumens: i64,
    pub metadata: Option<String>,
    pub created_at: String,
}

/// Aggregate policy + hint telemetry for the given user.
pub async fn load_kpi_summary(db: &Codex, user_id: &str) -> Result<LudusKpiSummary> {
    let mut snap = db
        .connection()
        .query(
            "SELECT COUNT(*), COALESCE(SUM(awarded_xp), 0), COALESCE(SUM(awarded_crystals), 0),
                    COALESCE(SUM(CASE WHEN grind_capped != 0 THEN 1 ELSE 0 END), 0),
                    COALESCE(AVG(effective_multiplier), 1.0)
             FROM gamify_policy_snapshots WHERE user_id = ?1",
            params![user_id],
        )
        .await?;
    let (ev, xp, crys, capped, mult) = if let Some(row) = snap.next().await? {
        (
            row.get::<i64>(0)?,
            row.get::<i64>(1)?,
            row.get::<i64>(2)?,
            row.get::<i64>(3)?,
            row.get::<f64>(4)?,
        )
    } else {
        (0, 0, 0, 0, 1.0)
    };

    let mut hints = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_hint_telemetry WHERE user_id = ?1",
            params![user_id],
        )
        .await?;
    let hint_n = hints
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);

    let mut quests_c = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_quests WHERE user_id = ?1 AND completed != 0",
            params![user_id],
        )
        .await?;
    let quests_completed_total = quests_c
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);

    let mut unread_n = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_notifications WHERE user_id = ?1 AND read = 0",
            params![user_id],
        )
        .await?;
    let notifications_unread = unread_n
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);

    let mut hs = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_hint_telemetry WHERE user_id = ?1 AND action = 'shown'",
            params![user_id],
        )
        .await?;
    let hints_shown = hs
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);

    let mut hd = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_hint_telemetry WHERE user_id = ?1 AND action = 'dismissed'",
            params![user_id],
        )
        .await?;
    let hints_dismissed = hd
        .next()
        .await?
        .map(|r| r.get::<i64>(0).unwrap_or(0))
        .unwrap_or(0);

    Ok(LudusKpiSummary {
        events_recorded: ev,
        total_xp_awarded: xp,
        total_crystals_awarded: crys,
        grind_capped_events: capped,
        avg_effective_multiplier: mult,
        hint_events_logged: hint_n,
        quests_completed_total,
        notifications_unread,
        hints_shown,
        hints_dismissed,
    })
}

/// Recent reward-policy rows for the user (newest first).
pub async fn list_recent_policy_snapshots(
    db: &Codex,
    user_id: &str,
    limit: usize,
) -> Result<Vec<PolicySnapshotRow>> {
    let lim = limit.clamp(1, 500) as i64;
    let mut rows = db
        .connection()
        .query(
            "SELECT event_type, base_xp, base_crystals, mode_label, effective_multiplier,
                    awarded_xp, awarded_crystals, grind_capped, lumens, created_at, metadata
             FROM gamify_policy_snapshots
             WHERE user_id = ?1
             ORDER BY id DESC
             LIMIT ?2",
            params![user_id, lim],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(PolicySnapshotRow {
            event_type: row.get(0)?,
            base_xp: row.get(1)?,
            base_crystals: row.get(2)?,
            mode_label: row.get(3)?,
            effective_multiplier: row.get(4)?,
            awarded_xp: row.get(5)?,
            awarded_crystals: row.get(6)?,
            grind_capped: row.get(7)?,
            lumens: row.get(8)?,
            created_at: row.get(9)?,
            metadata: row.get(10).unwrap_or(None),
        });
    }
    Ok(out)
}

/// Policy rows since `days` ago (SQLite `datetime`), newest first.
pub async fn list_policy_snapshots_since_days(
    db: &Codex,
    user_id: &str,
    days: u32,
    limit: usize,
) -> Result<Vec<PolicySnapshotRow>> {
    let days_i = days.clamp(1, 3660);
    let rel = format!("-{days_i} days");
    let lim = limit.clamp(1, 500) as i64;
    let mut rows = db
        .connection()
        .query(
            "SELECT event_type, base_xp, base_crystals, mode_label, effective_multiplier,
                    awarded_xp, awarded_crystals, grind_capped, lumens, created_at, metadata
             FROM gamify_policy_snapshots
             WHERE user_id = ?1
               AND datetime(created_at) >= datetime('now', ?2)
             ORDER BY id DESC
             LIMIT ?3",
            params![user_id, rel.as_str(), lim],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        out.push(PolicySnapshotRow {
            event_type: row.get(0)?,
            base_xp: row.get(1)?,
            base_crystals: row.get(2)?,
            mode_label: row.get(3)?,
            effective_multiplier: row.get(4)?,
            awarded_xp: row.get(5)?,
            awarded_crystals: row.get(6)?,
            grind_capped: row.get(7)?,
            lumens: row.get(8)?,
            created_at: row.get(9)?,
            metadata: row.get(10).unwrap_or(None),
        });
    }
    Ok(out)
}
