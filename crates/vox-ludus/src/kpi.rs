//! Local KPI schema for Ludus (fun / quality / grind signals).

use serde::Serialize;

/// Aggregates derivable from `gamify_policy_snapshots` and counters.
#[derive(Debug, Clone, Serialize)]
pub struct LudusKpiSummary {
    pub events_recorded: i64,
    pub total_xp_awarded: i64,
    pub total_crystals_awarded: i64,
    pub grind_capped_events: i64,
    pub avg_effective_multiplier: f64,
    pub hint_events_logged: i64,
    /// Rows in `gamify_quests` with `completed` set for this user.
    pub quests_completed_total: i64,
    /// Count of unread rows in `gamify_notifications`.
    pub notifications_unread: i64,
    /// Rows in `gamify_hint_telemetry` with `action = 'shown'`.
    pub hints_shown: i64,
    /// Rows with `action = 'dismissed'`.
    pub hints_dismissed: i64,
}
