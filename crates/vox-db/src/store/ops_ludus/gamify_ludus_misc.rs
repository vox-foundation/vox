//! Ludus-facing gamify tables moved out of consumer crates (feedback, dedupe, arena, …).

use turso::params;

use crate::store::types::{GamifyLudusKpiRollup, GamifyPolicySnapshotListRow, StoreError};

impl crate::VoxDb {
    // ── AI feedback (gamify_ai_feedback) ─────────────────────────────────────

    /// Insert one AI feedback row.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_gamify_ai_feedback(
        &self,
        id: &str,
        user_id: &str,
        session_id: &str,
        response_id: &str,
        thumbs_up: bool,
        comment: &str,
        tokens_generated: i64,
        example_code: &str,
        contributed_to_corpus: bool,
        created_at: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_ai_feedback
             (id, user_id, session_id, response_id, thumbs_up, comment, tokens_generated, example_code, contributed_to_corpus, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    id,
                    user_id,
                    session_id,
                    response_id,
                    if thumbs_up { 1i64 } else { 0i64 },
                    comment,
                    tokens_generated,
                    example_code,
                    if contributed_to_corpus { 1i64 } else { 0i64 },
                    created_at,
                ],
            )
            .await?;
        Ok(())
    }

    // ── Dedupe (gamify_processed_events) ──────────────────────────────────────

    /// Returns `true` when the row was newly inserted.
    pub async fn try_claim_gamify_processed_event(
        &self,
        user_id: &str,
        dedupe_key: &str,
    ) -> Result<bool, StoreError> {
        let n = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO gamify_processed_events (user_id, dedupe_key) VALUES (?1, ?2)",
                params![user_id, dedupe_key],
            )
            .await?;
        Ok(n > 0)
    }

    // ── Hint telemetry (gamify_hint_telemetry) ────────────────────────────────

    pub async fn insert_gamify_hint_telemetry(
        &self,
        user_id: &str,
        kind: &str,
        action: &str,
        reason: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_hint_telemetry (user_id, kind, action, reason)
             VALUES (?1, ?2, ?3, ?4)",
                params![user_id, kind, action, reason.unwrap_or("")],
            )
            .await?;
        Ok(())
    }

    // ── Periodic rewards (gamify_periodic_rewards) ────────────────────────────

    /// Upsert a periodic reward row (matches Ludus `upsert_periodic_reward`).
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_gamify_periodic_reward(
        &self,
        reward_id: &str,
        user_id: &str,
        name: &str,
        icon: &str,
        description: &str,
        xp_bonus: i64,
        crystal_bonus: i64,
        redeemed: bool,
        expires_at: i64,
        created_at: i64,
        unlock_condition_json: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_periodic_rewards
             (reward_id, user_id, name, icon, description, xp_bonus, crystal_bonus, redeemed, expires_at, created_at, unlock_condition)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(reward_id, user_id) DO UPDATE SET
            redeemed = excluded.redeemed",
                params![
                    reward_id,
                    user_id,
                    name,
                    icon,
                    description,
                    xp_bonus,
                    crystal_bonus,
                    if redeemed { 1i64 } else { 0i64 },
                    expires_at,
                    created_at,
                    unlock_condition_json,
                ],
            )
            .await?;
        Ok(())
    }

    /// Load one periodic reward claim row.
    pub async fn get_gamify_periodic_reward_row(
        &self,
        user_id: &str,
        reward_id: &str,
    ) -> Result<
        Option<(
            String,
            String,
            i64,
            i64,
            bool,
            i64,
            String,
            String,
        )>,
        StoreError,
    > {
        let mut rows = self
            .conn
            .query(
                "SELECT name, icon, xp_bonus, crystal_bonus, redeemed, expires_at, description, unlock_condition
         FROM gamify_periodic_rewards WHERE user_id = ?1 AND reward_id = ?2",
                params![user_id, reward_id],
            )
            .await?;
        if let Some(row) = rows.next().await? {
            Ok(Some((
                row.get(0)?,
                row.get(1)?,
                row.get::<i64>(2)?,
                row.get::<i64>(3)?,
                row.get::<i64>(4)? != 0,
                row.get::<i64>(5)?,
                row.get(6)?,
                row.get(7)?,
            )))
        } else {
            Ok(None)
        }
    }

    // ── Collegium (gamify_collegium) ─────────────────────────────────────────

    pub async fn insert_gamify_collegium(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
        leader_id: &str,
        created_at: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_collegium (id, name, description, leader_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, name, description, leader_id, created_at],
            )
            .await?;
        Ok(())
    }

    pub async fn insert_gamify_collegium_member_or_ignore(
        &self,
        collegium_id: &str,
        user_id: &str,
        role: &str,
        joined_at: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO gamify_collegium_members (collegium_id, user_id, role, joined_at)
         VALUES (?1, ?2, ?3, ?4)",
                params![collegium_id, user_id, role, joined_at],
            )
            .await?;
        Ok(())
    }

    pub async fn list_gamify_collegiums_with_counts(
        &self,
    ) -> Result<Vec<(String, String, i64, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, name, lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = id) FROM gamify_collegium ORDER BY lumens DESC",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get(0)?,
                row.get(1)?,
                row.get::<i64>(2).unwrap_or(0),
                row.get::<i64>(3).unwrap_or(0),
            ));
        }
        Ok(out)
    }

    pub async fn get_gamify_collegium_with_count(
        &self,
        id: &str,
    ) -> Result<Option<(String, String, i64, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, name, lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = id) FROM gamify_collegium WHERE id = ?1",
                params![id],
            )
            .await?;
        Ok(rows.next().await?.map(|row| {
            (
                row.get(0).unwrap_or_default(),
                row.get(1).unwrap_or_default(),
                row.get::<i64>(2).unwrap_or(0),
                row.get::<i64>(3).unwrap_or(0),
            )
        }))
    }

    pub async fn get_gamify_user_collegium_summary(
        &self,
        user_id: &str,
    ) -> Result<Option<(String, String, i64, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT c.id, c.name, c.lumens, (SELECT COUNT(*) FROM gamify_collegium_members WHERE collegium_id = c.id)
         FROM gamify_collegium c
         JOIN gamify_collegium_members m ON m.collegium_id = c.id
         WHERE m.user_id = ?1",
                params![user_id],
            )
            .await?;
        Ok(rows.next().await?.map(|row| {
            (
                row.get(0).unwrap_or_default(),
                row.get(1).unwrap_or_default(),
                row.get::<i64>(2).unwrap_or(0),
                row.get::<i64>(3).unwrap_or(0),
            )
        }))
    }

    // ── Arena ────────────────────────────────────────────────────────────────

    pub async fn get_active_gamify_arena_event(
        &self,
        now_ts: i64,
    ) -> Result<
        Option<(
            String,
            String,
            String,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
        )>,
        StoreError,
    > {
        let mut rows = self
            .conn
            .query(
                "SELECT id, name, description, start_ts, end_ts, target_xp, current_xp, target_lumens, current_lumens
         FROM gamify_arena_events
         WHERE status = 'active' AND start_ts <= ?1 AND end_ts >= ?1",
                params![now_ts],
            )
            .await?;
        Ok(rows.next().await?.map(|row| {
            (
                row.get(0).unwrap_or_default(),
                row.get(1).unwrap_or_default(),
                row.get(2).unwrap_or_default(),
                row.get::<i64>(3).unwrap_or(0),
                row.get::<i64>(4).unwrap_or(0),
                row.get::<i64>(5).unwrap_or(0),
                row.get::<i64>(6).unwrap_or(0),
                row.get::<i64>(7).unwrap_or(0),
                row.get::<i64>(8).unwrap_or(0),
            )
        }))
    }

    pub async fn insert_gamify_arena_participant_or_ignore(
        &self,
        event_id: &str,
        user_id: &str,
        joined_at: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO gamify_arena_participants (event_id, user_id, joined_at)
         VALUES (?1, ?2, ?3)",
                params![event_id, user_id, joined_at],
            )
            .await?;
        Ok(())
    }

    pub async fn get_gamify_arena_contribution(
        &self,
        event_id: &str,
        user_id: &str,
    ) -> Result<(i64, i64), StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT xp_contributed, lumens_contributed FROM gamify_arena_participants WHERE event_id = ?1 AND user_id = ?2",
                params![event_id, user_id],
            )
            .await?;
        Ok(
            if let Some(row) = rows.next().await? {
                (row.get::<i64>(0).unwrap_or(0), row.get::<i64>(1).unwrap_or(0))
            } else {
                (0, 0)
            },
        )
    }

    pub async fn list_gamify_arena_leaderboard(
        &self,
        event_id: &str,
        limit: i64,
    ) -> Result<Vec<(String, i64, i64)>, StoreError> {
        let mut rows = self
            .conn
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
            out.push((
                row.get(0)?,
                row.get::<i64>(1).unwrap_or(0),
                row.get::<i64>(2).unwrap_or(0),
            ));
        }
        Ok(out)
    }

    // ── Notifications (gamify_notifications) ─────────────────────────────────

    pub async fn insert_gamify_notification_ignore(
        &self,
        id: &str,
        user_id: &str,
        notification_type: &str,
        title: &str,
        message: &str,
        read: bool,
        created_at: i64,
        expires_at: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO gamify_notifications
             (id, user_id, notification_type, title, message, read, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    id,
                    user_id,
                    notification_type,
                    title,
                    message,
                    if read { 1i64 } else { 0i64 },
                    created_at,
                    expires_at,
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn list_gamify_unread_notifications(
        &self,
        user_id: &str,
        now_ts: i64,
        limit: i64,
    ) -> Result<Vec<(String, String, String, String, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, notification_type, title, message, created_at
             FROM gamify_notifications
             WHERE user_id = ?1 AND read = 0 AND (expires_at = 0 OR expires_at > ?2)
             ORDER BY created_at DESC
             LIMIT ?3",
                params![user_id, now_ts, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get::<i64>(4)?,
            ));
        }
        Ok(out)
    }

    pub async fn mark_gamify_notification_read(&self, notif_id: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE gamify_notifications SET read = 1 WHERE id = ?1",
                params![notif_id],
            )
            .await?;
        Ok(())
    }

    pub async fn mark_gamify_notification_read_for_user(
        &self,
        user_id: &str,
        notif_id: &str,
    ) -> Result<u64, StoreError> {
        let n = self
            .conn
            .execute(
                "UPDATE gamify_notifications SET read = 1 WHERE id = ?1 AND user_id = ?2",
                params![notif_id, user_id],
            )
            .await?;
        Ok(n)
    }

    pub async fn mark_all_gamify_notifications_read(&self, user_id: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE gamify_notifications SET read = 1 WHERE user_id = ?1 AND read = 0",
                params![user_id],
            )
            .await?;
        Ok(())
    }

    pub async fn delete_expired_gamify_notifications(&self, now_ts: i64) -> Result<u64, StoreError> {
        let n = self
            .conn
            .execute(
                "DELETE FROM gamify_notifications WHERE expires_at > 0 AND expires_at < ?1",
                params![now_ts],
            )
            .await?;
        Ok(n)
    }

    // ── KPI / policy audit ───────────────────────────────────────────────────

    pub async fn load_gamify_ludus_kpi_rollup(
        &self,
        user_id: &str,
    ) -> Result<GamifyLudusKpiRollup, StoreError> {
        let mut snap = self
            .conn
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

        let mut hints = self
            .conn
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

        let mut quests_c = self
            .conn
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

        let mut unread_n = self
            .conn
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

        let mut hs = self
            .conn
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

        let mut hd = self
            .conn
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

        Ok(GamifyLudusKpiRollup {
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

    pub async fn list_gamify_policy_snapshots_recent(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<GamifyPolicySnapshotListRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT event_type, base_xp, base_crystals, mode_label, effective_multiplier,
                    awarded_xp, awarded_crystals, grind_capped, lumens, created_at
             FROM gamify_policy_snapshots
             WHERE user_id = ?1
             ORDER BY id DESC
             LIMIT ?2",
                params![user_id, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(GamifyPolicySnapshotListRow {
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
            });
        }
        Ok(out)
    }

    pub async fn list_gamify_policy_snapshots_since_days(
        &self,
        user_id: &str,
        days_rel: &str,
        limit: i64,
    ) -> Result<Vec<GamifyPolicySnapshotListRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT event_type, base_xp, base_crystals, mode_label, effective_multiplier,
                    awarded_xp, awarded_crystals, grind_capped, lumens, created_at
             FROM gamify_policy_snapshots
             WHERE user_id = ?1
               AND datetime(created_at) >= datetime('now', ?2)
             ORDER BY id DESC
             LIMIT ?3",
                params![user_id, days_rel, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(GamifyPolicySnapshotListRow {
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
            });
        }
        Ok(out)
    }
}
