//! Load [`crate::kpi::LudusKpiSummary`] from Codex.

use anyhow::Result;
use serde::Serialize;
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
    pub created_at: String,
}

impl From<vox_db::GamifyPolicySnapshotListRow> for PolicySnapshotRow {
    fn from(r: vox_db::GamifyPolicySnapshotListRow) -> Self {
        PolicySnapshotRow {
            event_type: r.event_type,
            base_xp: r.base_xp,
            base_crystals: r.base_crystals,
            mode_label: r.mode_label,
            effective_multiplier: r.effective_multiplier,
            awarded_xp: r.awarded_xp,
            awarded_crystals: r.awarded_crystals,
            grind_capped: r.grind_capped,
            lumens: r.lumens,
            created_at: r.created_at,
        }
    }
}

/// Aggregate policy + hint telemetry for the given user.
pub async fn load_kpi_summary(db: &Codex, user_id: &str) -> Result<LudusKpiSummary> {
    let r = db
        .load_gamify_ludus_kpi_rollup(user_id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(LudusKpiSummary {
        events_recorded: r.events_recorded,
        total_xp_awarded: r.total_xp_awarded,
        total_crystals_awarded: r.total_crystals_awarded,
        grind_capped_events: r.grind_capped_events,
        avg_effective_multiplier: r.avg_effective_multiplier,
        hint_events_logged: r.hint_events_logged,
        quests_completed_total: r.quests_completed_total,
        notifications_unread: r.notifications_unread,
        hints_shown: r.hints_shown,
        hints_dismissed: r.hints_dismissed,
    })
}

/// Recent reward-policy rows for the user (newest first).
pub async fn list_recent_policy_snapshots(
    db: &Codex,
    user_id: &str,
    limit: usize,
) -> Result<Vec<PolicySnapshotRow>> {
    let lim = limit.clamp(1, 500) as i64;
    let rows = db
        .list_gamify_policy_snapshots_recent(user_id, lim)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(rows.into_iter().map(PolicySnapshotRow::from).collect())
}

/// Policy rows since `days` ago (SQLite `datetime`), newest first.
pub async fn list_policy_snapshots_since_days(
    db: &Codex,
    user_id: &str,
    days: u32,
    limit: usize,
) -> Result<Vec<PolicySnapshotRow>> {
    let days_i = days.max(1).min(3660);
    let rel = format!("-{days_i} days");
    let lim = limit.clamp(1, 500) as i64;
    let rows = db
        .list_gamify_policy_snapshots_since_days(user_id, rel.as_str(), lim)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(rows.into_iter().map(PolicySnapshotRow::from).collect())
}
