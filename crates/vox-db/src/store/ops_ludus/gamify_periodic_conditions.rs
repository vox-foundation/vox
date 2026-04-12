//! Periodic reward condition probes for [`crate::VoxDb`] (Ludus / gamify tables).
//!
//! SQL lives here so `vox-ludus` does not call [`crate::VoxDb::query_all`](crate::VoxDb::query_all).

use turso::params;

use crate::store::types::StoreError;

impl crate::VoxDb {
    /// `last_active` from `gamify_profiles` as stored (TEXT or numeric), for daily / weekly checks.
    pub async fn gamify_periodic_profile_last_active(
        &self,
        user_id: &str,
    ) -> Result<Option<String>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT last_active FROM gamify_profiles WHERE user_id = ?1",
                params![user_id],
            )
            .await?;
        let Some(row) = rows.next().await? else {
            return Ok(None);
        };
        if let Ok(s) = row.get::<String>(0) {
            return Ok(Some(s));
        }
        if let Ok(n) = row.get::<i64>(0) {
            return Ok(Some(n.to_string()));
        }
        Ok(None)
    }

    /// Count of `gamify_quests` completed today (SQLite `date('now', 'start of day')`).
    pub async fn gamify_periodic_daily_quests_completed_today_count(
        &self,
        user_id: &str,
    ) -> Result<i64, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM gamify_quests WHERE user_id = ?1 AND status = 'completed' AND created_at >= date('now', 'start of day')",
                params![user_id],
            )
            .await?;
        let Some(row) = rows.next().await? else {
            return Ok(0);
        };
        Ok(row.get::<i64>(0).unwrap_or(0))
    }

    /// Whether the user has unlocked the given achievement id.
    pub async fn gamify_periodic_has_achievement(
        &self,
        user_id: &str,
        achievement_id: &str,
    ) -> Result<bool, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT 1 FROM gamify_achievements WHERE user_id = ?1 AND id = ?2 LIMIT 1",
                params![user_id, achievement_id],
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }

    /// `streak_days` from `gamify_profiles`.
    pub async fn gamify_periodic_profile_streak_days(
        &self,
        user_id: &str,
    ) -> Result<Option<i64>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT streak_days FROM gamify_profiles WHERE user_id = ?1",
                params![user_id],
            )
            .await?;
        let Some(row) = rows.next().await? else {
            return Ok(None);
        };
        Ok(Some(row.get::<i64>(0).unwrap_or(0)))
    }

    /// Count of `gamify_policy_snapshots` rows with `event_type = 'doc_item'` this calendar month.
    pub async fn gamify_periodic_doc_item_count_this_month(
        &self,
        user_id: &str,
    ) -> Result<i64, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM gamify_policy_snapshots WHERE user_id = ?1 AND event_type = 'doc_item' AND created_at >= date('now', 'start of month')",
                params![user_id],
            )
            .await?;
        let Some(row) = rows.next().await? else {
            return Ok(0);
        };
        Ok(row.get::<i64>(0).unwrap_or(0))
    }

    /// Whether a quest row exists with `status = 'completed'` for the given quest id.
    pub async fn gamify_periodic_has_completed_quest(
        &self,
        user_id: &str,
        quest_id: &str,
    ) -> Result<bool, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT 1 FROM gamify_quests WHERE user_id = ?1 AND id = ?2 AND status = 'completed' LIMIT 1",
                params![user_id, quest_id],
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }

    /// Count of completed daily quests in the rolling 7-day window.
    pub async fn gamify_periodic_perfect_week_completed_count(
        &self,
        user_id: &str,
    ) -> Result<i64, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM gamify_quests WHERE user_id = ?1 AND status = 'completed' AND created_at >= date('now', '-7 days')",
                params![user_id],
            )
            .await?;
        let Some(row) = rows.next().await? else {
            return Ok(0);
        };
        Ok(row.get::<i64>(0).unwrap_or(0))
    }
}
