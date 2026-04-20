use turso::params;

use crate::store::types::StoreError;

impl crate::VoxDb {
    // ── Policy Snapshots (gamify_policy_snapshots) ────────────────────────────

    /// Insert a reward policy snapshot row.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_gamify_policy_snapshot(
        &self,
        user_id: &str,
        event_type: &str,
        base_xp: i64,
        base_crystals: i64,
        mode_label: &str,
        effective_multiplier: f64,
        awarded_xp: i64,
        awarded_crystals: i64,
        streak_days: i64,
        grind_capped: bool,
        lumens: i64,
        metadata: Option<&str>,
    ) -> Result<(), StoreError> {
        let user_id = user_id.to_string();
        let event_type = event_type.to_string();
        let mode_label = mode_label.to_string();
        let metadata = metadata.map(|s| s.to_string());
        let grind = if grind_capped { 1i64 } else { 0i64 };
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO gamify_policy_snapshots
             (user_id, event_type, base_xp, base_crystals, mode_label, effective_multiplier,
              awarded_xp, awarded_crystals, streak_days, grind_capped, lumens, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        user_id.as_str(),
                        event_type.as_str(),
                        base_xp,
                        base_crystals,
                        mode_label.as_str(),
                        effective_multiplier,
                        awarded_xp,
                        awarded_crystals,
                        streak_days,
                        grind,
                        lumens,
                        metadata
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    // ── Collegium (gamify_collegium) ──────────────────────────────────────────

    /// Get collegium membership for a user: returns (collegium_id, name, role, lumens).
    pub async fn get_gamify_user_collegium(
        &self,
        user_id: &str,
    ) -> Result<Option<(String, String, String, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT c.id, c.name, m.role, COALESCE(c.lumens, 0)
             FROM gamify_collegium c
             JOIN gamify_collegium_members m ON c.id=m.collegium_id
             WHERE m.user_id=?1",
                params![user_id],
            )
            .await?;
        Ok(rows.next().await?.map(|r| {
            (
                r.get::<String>(0).unwrap_or_default(),
                r.get::<String>(1).unwrap_or_default(),
                r.get::<String>(2).unwrap_or_default(),
                r.get::<i64>(3).unwrap_or(0),
            )
        }))
    }

    /// Add lumens to a collegium.
    pub async fn update_gamify_collegium_lumens(
        &self,
        collegium_id: &str,
        lumens_delta: i64,
    ) -> Result<(), StoreError> {
        let collegium_id = collegium_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE gamify_collegium SET lumens=COALESCE(lumens, 0)+?1 WHERE id=?2",
                    params![lumens_delta, collegium_id.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }
}
