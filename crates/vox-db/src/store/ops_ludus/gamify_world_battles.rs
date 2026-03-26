use turso::params;

use crate::store::types::StoreError;

impl crate::VoxDb {
    /// Get a battle by id.
    pub async fn get_gamify_battle(
        &self,
        id: &str,
    ) -> Result<Option<Vec<Option<String>>>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT id, user_id, companion_id, bug_type, bug_description, bug_code, submitted_code,
                    CAST(success AS TEXT), CAST(crystals_earned AS TEXT),
                    CAST(xp_earned AS TEXT), CAST(duration_secs AS TEXT), CAST(created_at AS TEXT)
             FROM gamify_battles WHERE id = ?1",
            params![id],
        ).await?;
        Ok(rows.next().await?.map(|row| {
            (0..12)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect()
        }))
    }

    /// Insert a new battle record.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_gamify_battle(
        &self,
        id: &str,
        user_id: &str,
        companion_id: &str,
        bug_type: &str,
        bug_description: &str,
        bug_code: Option<&str>,
        submitted_code: Option<&str>,
        success: bool,
        crystals_earned: i64,
        xp_earned: i64,
        duration_secs: i64,
        created_at: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_battles
             (id, user_id, companion_id, bug_type, bug_description, bug_code, submitted_code,
              success, crystals_earned, xp_earned, duration_secs, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    id,
                    user_id,
                    companion_id,
                    bug_type,
                    bug_description,
                    bug_code,
                    submitted_code,
                    if success { 1i64 } else { 0i64 },
                    crystals_earned,
                    xp_earned,
                    duration_secs,
                    created_at
                ],
            )
            .await?;
        Ok(())
    }

    /// Update a battle's outcome fields.
    pub async fn update_gamify_battle(
        &self,
        id: &str,
        submitted_code: Option<&str>,
        success: bool,
        crystals_earned: i64,
        xp_earned: i64,
        duration_secs: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE gamify_battles SET submitted_code=?1, success=?2, crystals_earned=?3,
             xp_earned=?4, duration_secs=?5 WHERE id=?6",
                params![
                    submitted_code,
                    if success { 1i64 } else { 0i64 },
                    crystals_earned,
                    xp_earned,
                    duration_secs,
                    id
                ],
            )
            .await?;
        Ok(())
    }

    /// Count all battles for a user.
    pub async fn count_gamify_battles(&self, user_id: &str) -> Result<i64, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM gamify_battles WHERE user_id = ?1",
                params![user_id],
            )
            .await?;
        Ok(rows
            .next()
            .await?
            .map(|r| r.get::<i64>(0).unwrap_or(0))
            .unwrap_or(0))
    }
}
