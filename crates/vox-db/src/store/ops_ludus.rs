//! Gamification CRUD for [`crate::store::VoxDb`] (Arca / Turso).
//!
//! All gamification tables live in the schema under the `gamify_*` prefix. This module provides
//! the typed CRUD methods that `vox-ludus` consumes, keeping raw SQL inside `vox-pm` where it
//! belongs (SSOT rule from §5.1 AGENTS.md — all SQL routes through `vox-pm`).
//!
//! **Do not** use `db.conn.execute(...)` from `vox-ludus` or other crates. Add a method
//! here and call it via `db.<method>` or a `VoxDb` wrapper in `vox-db/src/ludus.rs`.

use turso::params;

use crate::store::types::StoreError;

impl crate::VoxDb {
    // ── Profiles (gamify_profiles) ────────────────────────────────────────────

    /// Get the raw profile row for a user (all numeric columns as i64).
    ///
    /// Returns columns in the order:
    /// level, xp, crystals, energy, max_energy, last_energy_regen, last_active,
    /// streak_days, longest_streak, streak_last_ts, grace_available, grace_used,
    /// total_xp_earned, prestige_level, lumens, generosity_lumens, streak_shields.
    pub async fn get_gamify_profile_raw(
        &self,
        user_id: &str,
    ) -> Result<Option<Vec<i64>>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT level, xp, crystals, energy, max_energy,
                    CAST(COALESCE(last_energy_regen, 0) AS INTEGER),
                    CAST(COALESCE(last_active, 0) AS INTEGER),
                    COALESCE(streak_days, 0), COALESCE(longest_streak, 0),
                    COALESCE(streak_last_ts, 0), COALESCE(grace_available, 1), COALESCE(grace_used, 0),
                    COALESCE(total_xp_earned, 0), COALESCE(prestige_level, 0),
                    COALESCE(lumens, 0), COALESCE(generosity_lumens, 0), COALESCE(streak_shields, 0)
             FROM gamify_profiles WHERE user_id = ?1",
            params![user_id],
        ).await?;
        if let Some(row) = rows.next().await? {
            let vals: Vec<i64> = (0..17).map(|i| row.get::<i64>(i).unwrap_or(0)).collect();
            Ok(Some(vals))
        } else {
            Ok(None)
        }
    }

    /// Upsert a gamify profile row (all fields).
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_gamify_profile(
        &self,
        user_id: &str,
        level: i64,
        xp: i64,
        crystals: i64,
        energy: i64,
        max_energy: i64,
        last_energy_regen: i64,
        last_active: i64,
        streak_days: i64,
        longest_streak: i64,
        streak_last_ts: i64,
        grace_available: i64,
        grace_used: i64,
        total_xp_earned: i64,
        prestige_level: i64,
        lumens: i64,
        generosity_lumens: i64,
        streak_shields: i64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO gamify_profiles
             (user_id, level, xp, crystals, energy, max_energy, last_energy_regen, last_active,
              streak_days, longest_streak, streak_last_ts, grace_available, grace_used,
              total_xp_earned, prestige_level, lumens, generosity_lumens, streak_shields)
          VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
          ON CONFLICT(user_id) DO UPDATE SET
            level = excluded.level, xp = excluded.xp, crystals = excluded.crystals,
            energy = excluded.energy, max_energy = excluded.max_energy,
            last_energy_regen = excluded.last_energy_regen, last_active = excluded.last_active,
            streak_days = excluded.streak_days, longest_streak = excluded.longest_streak,
            streak_last_ts = excluded.streak_last_ts, grace_available = excluded.grace_available,
            grace_used = excluded.grace_used, total_xp_earned = excluded.total_xp_earned,
            prestige_level = excluded.prestige_level, lumens = excluded.lumens,
            generosity_lumens = excluded.generosity_lumens, streak_shields = excluded.streak_shields",
            params![user_id, level, xp, crystals, energy, max_energy, last_energy_regen, last_active,
                    streak_days, longest_streak, streak_last_ts, grace_available, grace_used,
                    total_xp_earned, prestige_level, lumens, generosity_lumens, streak_shields],
        ).await?;
        Ok(())
    }

    /// Update only lumens for a user's profile row.
    pub async fn update_gamify_profile_lumens(
        &self,
        user_id: &str,
        lumens_delta: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE gamify_profiles SET lumens = COALESCE(lumens, 0) + ?1 WHERE user_id = ?2",
                params![lumens_delta, user_id],
            )
            .await?;
        Ok(())
    }

    // ── Achievements (gamify_achievements) ────────────────────────────────────

    /// Insert an achievement unlock, ignoring conflicts (idempotent).
    /// Returns `true` if newly inserted, `false` if already unlocked.
    pub async fn unlock_gamify_achievement(
        &self,
        user_id: &str,
        achievement_id: &str,
        unlocked_at: i64,
        xp_rewarded: i64,
        crystals_rewarded: i64,
    ) -> Result<bool, StoreError> {
        let affected = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO gamify_achievements
             (id, user_id, unlocked_at, xp_rewarded, crystals_rewarded)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    achievement_id,
                    user_id,
                    unlocked_at,
                    xp_rewarded,
                    crystals_rewarded
                ],
            )
            .await?;
        Ok(affected > 0)
    }

    /// List all unlocked achievement IDs + timestamps for a user.
    pub async fn list_gamify_achievements(
        &self,
        user_id: &str,
    ) -> Result<Vec<(String, i64)>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT id, unlocked_at FROM gamify_achievements WHERE user_id = ?1 ORDER BY unlocked_at ASC",
            params![user_id],
        ).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<i64>(1)?));
        }
        Ok(out)
    }

    /// Get a single companion by ID.
    pub async fn get_gamify_companion(
        &self,
        id: &str,
    ) -> Result<Option<Vec<Option<String>>>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT id, user_id, name, description, code_hash, language, ascii_sprite, mood,
                    CAST(health AS TEXT), CAST(max_health AS TEXT), CAST(energy AS TEXT), CAST(max_energy AS TEXT),
                    CAST(code_quality AS TEXT), CAST(last_active AS TEXT), personality
             FROM gamify_companions WHERE id = ?1",
            params![id],
        ).await?;
        if let Some(row) = rows.next().await? {
            let cols: Vec<Option<String>> = (0..15)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            Ok(Some(cols))
        } else {
            Ok(None)
        }
    }

    /// Delete a companion record by ID.
    pub async fn delete_gamify_companion(&self, id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM gamify_companions WHERE id = ?1", params![id])
            .await?;
        Ok(())
    }

    /// Append a level-up history row.
    pub async fn record_gamify_level_up(
        &self,
        user_id: &str,
        level: i64,
        title: &str,
        xp_at_level: i64,
        created_at: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_level_history (user_id, level, title, xp_at_level, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![user_id, level, title, xp_at_level, created_at],
            )
            .await?;
        Ok(())
    }

    // ── Companions (gamify_companions) ────────────────────────────────────────

    /// List all companions for a user.
    pub async fn list_gamify_companions(
        &self,
        user_id: &str,
    ) -> Result<Vec<Vec<Option<String>>>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT id, user_id, name, description, code_hash, language, ascii_sprite, mood,
                    CAST(health AS TEXT), CAST(max_health AS TEXT), CAST(energy AS TEXT), CAST(max_energy AS TEXT),
                    CAST(code_quality AS TEXT), CAST(last_active AS TEXT), personality
             FROM gamify_companions WHERE user_id = ?1",
            params![user_id],
        ).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let cols: Vec<Option<String>> = (0..15)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            out.push(cols);
        }
        Ok(out)
    }

    /// Upsert a companion row; all numeric values as i64.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_gamify_companion(
        &self,
        id: &str,
        user_id: &str,
        name: &str,
        description: Option<String>,
        code_hash: Option<String>,
        language: &str,
        ascii_sprite: Option<String>,
        mood: &str,
        health: i64,
        max_health: i64,
        energy: i64,
        max_energy: i64,
        code_quality: i64,
        last_active: i64,
        personality: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO gamify_companions
             (id, user_id, name, description, code_hash, language, ascii_sprite, mood, health, max_health, energy, max_energy, code_quality, last_active, personality)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, user_id=excluded.user_id,
               description=excluded.description, code_hash=excluded.code_hash,
               language=excluded.language, ascii_sprite=excluded.ascii_sprite,
               mood=excluded.mood, health=excluded.health, max_health=excluded.max_health,
               energy=excluded.energy, max_energy=excluded.max_energy,
               code_quality=excluded.code_quality, last_active=excluded.last_active,
               personality=excluded.personality",
            params![id, user_id, name, description, code_hash, language, ascii_sprite, mood,
                    health, max_health, energy, max_energy, code_quality, last_active, personality],
        ).await?;
        Ok(())
    }

    // ── Quests (gamify_quests) ────────────────────────────────────────────────

    pub async fn list_gamify_quests(
        &self,
        user_id: &str,
    ) -> Result<Vec<Vec<Option<String>>>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, quest_type, description, CAST(target AS TEXT), CAST(progress AS TEXT),
                    CAST(crystal_reward AS TEXT), CAST(xp_reward AS TEXT), CAST(completed AS TEXT),
                    CAST(expires_at AS TEXT), hint, modifier, status
             FROM gamify_quests WHERE user_id = ?1",
                params![user_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let cols: Vec<Option<String>> = (0..12)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            out.push(cols);
        }
        Ok(out)
    }

    /// Upsert a quest row.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_gamify_quest(
        &self,
        id: &str,
        user_id: &str,
        quest_type: &str,
        description: &str,
        xp_reward: i64,
        crystal_reward: i64,
        target: i64,
        progress: i64,
        status: &str,
        expires_at: i64,
        completed: bool,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO gamify_quests
             (id, user_id, quest_type, description, target, progress, crystal_reward, xp_reward, completed, expires_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
               quest_type=excluded.quest_type,
               description=excluded.description, xp_reward=excluded.xp_reward,
               crystal_reward=excluded.crystal_reward, target=excluded.target,
               progress=excluded.progress, status=excluded.status,
               expires_at=excluded.expires_at, completed=excluded.completed",
            params![id, user_id, quest_type, description, target, progress, crystal_reward,
                    xp_reward, if completed { 1i64 } else { 0i64 }, expires_at, status],
        ).await?;
        Ok(())
    }

    /// Update quest status + completed flag.
    pub async fn update_gamify_quest_status(
        &self,
        id: &str,
        user_id: &str,
        status: &str,
        completed: bool,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE gamify_quests SET status = ?1, completed = ?2 WHERE id = ?3 AND user_id = ?4",
            params![status, if completed { 1i64 } else { 0i64 }, id, user_id],
        ).await?;
        Ok(())
    }

    /// Delete a quest by id.
    pub async fn delete_gamify_quest(&self, id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM gamify_quests WHERE id = ?1", params![id])
            .await?;
        Ok(())
    }

    /// Count active/non-expired quests for a user.
    pub async fn count_gamify_quests(&self, user_id: &str) -> Result<i64, StoreError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM gamify_quests
             WHERE user_id = ?1 AND status = 'active' AND (expires_at = 0 OR expires_at > ?2)",
                params![user_id, now],
            )
            .await?;
        Ok(rows
            .next()
            .await?
            .map(|r| r.get::<i64>(0).unwrap_or(0))
            .unwrap_or(0))
    }

    // ── Battles (gamify_battles) ──────────────────────────────────────────────

    /// List battles for a user, newest first.
    pub async fn list_gamify_battles(
        &self,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<Vec<Option<String>>>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, companion_id, bug_type, bug_description, bug_code, submitted_code,
                    CAST(success AS TEXT), CAST(crystals_earned AS TEXT),
                    CAST(xp_earned AS TEXT), CAST(duration_secs AS TEXT), CAST(created_at AS TEXT)
             FROM gamify_battles WHERE user_id = ?1 ORDER BY created_at DESC LIMIT ?2",
                params![user_id, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let cols: Vec<Option<String>> = (0..11)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            out.push(cols);
        }
        Ok(out)
    }

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

    // ── Agent Events (agent_events — ludus path) ─────────────────────────────

    /// Insert an agent event from the gamification layer (wraps `record_agent_event`
    /// with automatic CLI version tagging).
    pub async fn insert_gamify_event(
        &self,
        agent_id: &str,
        event_type: &str,
        payload: Option<&str>,
    ) -> Result<(), StoreError> {
        self.record_agent_event(
            agent_id,
            event_type,
            payload.unwrap_or("{}"),
            env!("CARGO_PKG_VERSION"),
        )
        .await?;
        Ok(())
    }

    /// Get recent agent events (agent_events table) for a given agent.
    pub async fn list_gamify_events(
        &self,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<(i64, String, String, Option<String>, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, agent_id, event_type, payload, timestamp
             FROM agent_events WHERE agent_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
                params![agent_id, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<i64>(0)?,
                row.get::<String>(1)?,
                row.get::<String>(2)?,
                row.get::<Option<String>>(3)?,
                row.get::<String>(4)?,
            ));
        }
        Ok(out)
    }

    // ── Cost Records (cost_records) ───────────────────────────────────────────

    /// Insert a cost record for an AI inference call.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_gamify_cost_record(
        &self,
        agent_id: &str,
        session_id: Option<&str>,
        provider: &str,
        model: Option<&str>,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO cost_records (agent_id, session_id, provider, model,
             input_tokens, output_tokens, cost_usd)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    agent_id,
                    session_id,
                    provider,
                    model,
                    input_tokens,
                    output_tokens,
                    cost_usd
                ],
            )
            .await?;
        Ok(())
    }

    /// Get total cost in USD for an agent.
    pub async fn get_gamify_agent_cost_usd(&self, agent_id: &str) -> Result<f64, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT COALESCE(SUM(cost_usd), 0.0) FROM cost_records WHERE agent_id = ?1",
                params![agent_id],
            )
            .await?;
        Ok(rows
            .next()
            .await?
            .map(|r| r.get::<f64>(0).unwrap_or(0.0))
            .unwrap_or(0.0))
    }

    /// List cost records for an agent, newest first.
    pub async fn list_gamify_cost_records(
        &self,
        agent_id: &str,
        limit: i64,
    ) -> Result<
        Vec<(
            i64,
            String,
            Option<String>,
            String,
            Option<String>,
            i64,
            i64,
            f64,
            String,
        )>,
        StoreError,
    > {
        let mut rows = self.conn.query(
            "SELECT id, agent_id, session_id, provider, model, input_tokens, output_tokens, cost_usd, timestamp
             FROM cost_records WHERE agent_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
            params![agent_id, limit],
        ).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<i64>(0)?,
                row.get::<String>(1)?,
                row.get::<Option<String>>(2)?,
                row.get::<String>(3)?,
                row.get::<Option<String>>(4)?,
                row.get::<i64>(5)?,
                row.get::<i64>(6)?,
                row.get::<f64>(7)?,
                row.get::<String>(8)?,
            ));
        }
        Ok(out)
    }

    // ── Agent Sessions (gamification path) ───────────────────────────────────

    /// Insert a new agent session (gamify-specific, maps to `agent_sessions`).
    pub async fn insert_gamify_session(
        &self,
        id: &str,
        agent_id: &str,
        agent_name: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT OR IGNORE INTO agent_sessions (id, agent_id, agent_name) VALUES (?1, ?2, ?3)",
            params![id, agent_id, agent_name],
        ).await?;
        Ok(())
    }

    /// Update an agent session's status and optional context.
    pub async fn update_gamify_session(
        &self,
        id: &str,
        status: &str,
        task_snapshot: Option<&str>,
        context_summary: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE agent_sessions SET status=?1, task_snapshot=?2, context_summary=?3 WHERE id=?4",
            params![status, task_snapshot, context_summary, id],
        ).await?;
        Ok(())
    }

    /// End a session with a status and set ended_at.
    pub async fn end_gamify_session(&self, id: &str, status: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE agent_sessions SET status=?1, ended_at=datetime('now') WHERE id=?2",
                params![status, id],
            )
            .await?;
        Ok(())
    }

    /// List active sessions.
    pub async fn list_gamify_active_sessions(
        &self,
    ) -> Result<
        Vec<(
            String,
            String,
            Option<String>,
            String,
            Option<String>,
            String,
            Option<String>,
            Option<String>,
        )>,
        StoreError,
    > {
        let mut rows = self.conn.query(
            "SELECT id, agent_id, agent_name, started_at, ended_at, status, task_snapshot, context_summary
             FROM agent_sessions WHERE status='active' ORDER BY started_at DESC",
            (),
        ).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<String>(1)?,
                row.get::<Option<String>>(2)?,
                row.get::<String>(3)?,
                row.get::<Option<String>>(4)?,
                row.get::<String>(5)?,
                row.get::<Option<String>>(6)?,
                row.get::<Option<String>>(7)?,
            ));
        }
        Ok(out)
    }

    // ── Agent Metrics (agent_metrics) ─────────────────────────────────────────

    /// Upsert an aggregated metric for an agent.
    pub async fn upsert_gamify_agent_metric(
        &self,
        agent_id: &str,
        metric_name: &str,
        metric_value: f64,
        period: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO agent_metrics (agent_id, metric_name, metric_value, period)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(agent_id, metric_name, period) DO UPDATE SET
               metric_value=excluded.metric_value, timestamp=datetime('now')",
                params![agent_id, metric_name, metric_value, period],
            )
            .await?;
        Ok(())
    }

    /// Get all metrics for an agent in a period.
    pub async fn get_gamify_agent_metrics(
        &self,
        agent_id: &str,
        period: &str,
    ) -> Result<Vec<(String, f64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT metric_name, metric_value FROM agent_metrics
             WHERE agent_id=?1 AND period=?2",
                params![agent_id, period],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((row.get::<String>(0)?, row.get::<f64>(1).unwrap_or(0.0)));
        }
        Ok(out)
    }

    // ── Counters (gamify_counters / gamify_daily_counters) ────────────────────

    /// Get a persistent counter value for a user.
    pub async fn get_gamify_counter(&self, user_id: &str, name: &str) -> Result<i64, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT count FROM gamify_counters WHERE user_id=?1 AND name=?2",
                params![user_id, name],
            )
            .await?;
        Ok(rows
            .next()
            .await?
            .map(|r| r.get::<i64>(0).unwrap_or(0))
            .unwrap_or(0))
    }

    /// Set a persistent counter to an explicit value.
    pub async fn set_gamify_counter(
        &self,
        user_id: &str,
        name: &str,
        value: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_counters (user_id, name, count) VALUES (?1, ?2, ?3)
             ON CONFLICT(user_id, name) DO UPDATE SET count=excluded.count",
                params![user_id, name, value],
            )
            .await?;
        Ok(())
    }

    /// Increment a persistent counter and return the new value.
    pub async fn increment_gamify_counter(
        &self,
        user_id: &str,
        name: &str,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_counters (user_id, name, count) VALUES (?1, ?2, 1)
             ON CONFLICT(user_id, name) DO UPDATE SET count=count+1",
                params![user_id, name],
            )
            .await?;
        let mut rows = self
            .conn
            .query(
                "SELECT count FROM gamify_counters WHERE user_id=?1 AND name=?2",
                params![user_id, name],
            )
            .await?;
        Ok(rows
            .next()
            .await?
            .map(|r| r.get::<i64>(0).unwrap_or(1))
            .unwrap_or(1))
    }

    /// Increment a daily counter (gamify_daily_counters); returns new value.
    pub async fn increment_gamify_daily_counter(
        &self,
        user_id: &str,
        event_type: &str,
        day: i64,
    ) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO gamify_daily_counters (user_id, event_type, day, count) VALUES (?1, ?2, ?3, 1)
             ON CONFLICT(user_id, event_type, day) DO UPDATE SET count=count+1",
            params![user_id, event_type, day],
        ).await?;
        let mut rows = self.conn.query(
            "SELECT count FROM gamify_daily_counters WHERE user_id=?1 AND event_type=?2 AND day=?3",
            params![user_id, event_type, day],
        ).await?;
        Ok(rows
            .next()
            .await?
            .map(|r| r.get::<i64>(0).unwrap_or(1))
            .unwrap_or(1))
    }

    /// Get a daily counter value without incrementing.
    pub async fn get_gamify_daily_counter(
        &self,
        user_id: &str,
        event_type: &str,
        day: i64,
    ) -> Result<i64, StoreError> {
        let mut rows = self.conn.query(
            "SELECT count FROM gamify_daily_counters WHERE user_id=?1 AND event_type=?2 AND day=?3",
            params![user_id, event_type, day],
        ).await?;
        Ok(rows
            .next()
            .await?
            .map(|r| r.get::<i64>(0).unwrap_or(0))
            .unwrap_or(0))
    }

    // ── Event Config (gamify_event_config) ────────────────────────────────────

    /// Load all enabled event config overrides (event_type, xp_override, crystals_override).
    pub async fn list_gamify_event_config_overrides(
        &self,
    ) -> Result<Vec<(String, i64, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT event_type, COALESCE(xp_override, 0), COALESCE(crystals_override, 0)
             FROM gamify_event_config WHERE enabled=1",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<i64>(1).unwrap_or(0),
                row.get::<i64>(2).unwrap_or(0),
            ));
        }
        Ok(out)
    }

    /// Upsert an event config override row.
    pub async fn set_gamify_event_config_override(
        &self,
        event_type: &str,
        xp_override: Option<i64>,
        crystals_override: Option<i64>,
        enabled: bool,
        updated_at: i64,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO gamify_event_config (event_type, xp_override, crystals_override, enabled, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(event_type) DO UPDATE SET
               xp_override=excluded.xp_override, crystals_override=excluded.crystals_override,
               enabled=excluded.enabled, updated_at=excluded.updated_at",
            params![event_type, xp_override, crystals_override, if enabled { 1i64 } else { 0i64 }, updated_at],
        ).await?;
        Ok(())
    }

    // ── Leaderboard (gamify_profiles fast path) ───────────────────────────────

    /// Get top users by XP.
    pub async fn gamify_leaderboard_by_xp(
        &self,
        limit: i64,
    ) -> Result<Vec<(String, i64, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT user_id, level, xp FROM gamify_profiles ORDER BY xp DESC LIMIT ?1",
                params![limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<i64>(1)?,
                row.get::<i64>(2)?,
            ));
        }
        Ok(out)
    }

    /// Get top users by Lumens.
    pub async fn gamify_leaderboard_by_lumens(
        &self,
        limit: i64,
    ) -> Result<Vec<(String, i64, i64)>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT user_id, level, COALESCE(lumens, 0) FROM gamify_profiles ORDER BY 3 DESC LIMIT ?1",
            params![limit],
        ).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<i64>(1)?,
                row.get::<i64>(2)?,
            ));
        }
        Ok(out)
    }

    /// Get aggregate profile stats (completed quests, won battles).
    pub async fn get_gamify_profile_stats(&self, user_id: &str) -> Result<(i64, i64), StoreError> {
        let mut r1 = self
            .conn
            .query(
                "SELECT COUNT(id) FROM gamify_quests WHERE user_id=?1 AND completed=1",
                params![user_id],
            )
            .await?;
        let quests = r1
            .next()
            .await?
            .map(|r| r.get::<i64>(0).unwrap_or(0))
            .unwrap_or(0);
        let mut r2 = self
            .conn
            .query(
                "SELECT COUNT(id) FROM gamify_battles WHERE user_id=?1 AND success=1",
                params![user_id],
            )
            .await?;
        let battles = r2
            .next()
            .await?
            .map(|r| r.get::<i64>(0).unwrap_or(0))
            .unwrap_or(0);
        Ok((quests, battles))
    }

    // ── A2A Messages (a2a_messages) ───────────────────────────────────────────

    /// Acknowledge an A2A message by row id.
    pub async fn acknowledge_a2a_message_by_id(&self, id: i64) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE a2a_messages SET acknowledged=1 WHERE id=?1",
                params![id],
            )
            .await?;
        Ok(())
    }

    /// Send an A2A message and return the UUID.
    pub async fn send_a2a_message(
        &self,
        message_uuid: &str,
        sender_agent: &str,
        receiver_agent: &str,
        msg_type: &str,
        payload: &str,
        priority: i64,
        thread_id: Option<&str>,
        repository_id: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO a2a_messages
             (message_uuid, sender_agent, receiver_agent, msg_type, payload, priority, thread_id, repository_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![message_uuid, sender_agent, receiver_agent, msg_type, payload,
                    priority, thread_id, repository_id],
        ).await?;
        Ok(())
    }

    /// Poll unacknowledged messages for an agent in a repository.
    pub async fn poll_a2a_inbox(
        &self,
        agent_id: &str,
        repository_id: &str,
    ) -> Result<Vec<Vec<Option<String>>>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT CAST(id AS TEXT), message_uuid, sender_agent, receiver_agent, msg_type, payload,
                    CAST(priority AS TEXT), thread_id, CAST(acknowledged AS TEXT), created_at, repository_id
             FROM a2a_messages
             WHERE receiver_agent=?1 AND acknowledged=0 AND repository_id=?2
             ORDER BY priority DESC, created_at ASC",
            params![agent_id, repository_id],
        ).await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let cols: Vec<Option<String>> = (0..11)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            out.push(cols);
        }
        Ok(out)
    }

    /// Acknowledge an A2A message by UUID.
    pub async fn acknowledge_a2a_message_by_uuid(&self, uuid: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE a2a_messages SET acknowledged=1 WHERE message_uuid=?1",
                params![uuid],
            )
            .await?;
        Ok(())
    }

    /// Prune old acknowledged A2A messages older than N days.
    pub async fn prune_a2a_messages(&self, older_than_days: u32) -> Result<u64, StoreError> {
        let sql = format!(
            "DELETE FROM a2a_messages WHERE acknowledged=1 AND created_at < datetime('now', '-{} days')",
            older_than_days
        );
        let affected = self.conn.execute(&sql, ()).await?;
        Ok(affected as u64)
    }

    // ── OpLog (agent_oplog) ───────────────────────────────────────────────────

    /// Append an operation log entry.
    #[allow(clippy::too_many_arguments)]
    pub async fn append_oplog_entry(
        &self,
        agent_id: &str,
        operation_id: &str,
        kind_json: &str,
        description: &str,
        predecessor_hash: Option<&str>,
        model_id: Option<&str>,
        change_id: Option<i64>,
        timestamp_ms: i64,
        repository_id: &str,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO agent_oplog
             (agent_id, operation_id, kind, description, predecessor_hash, model_id, change_id, timestamp_ms, repository_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![agent_id, operation_id, kind_json, description, predecessor_hash,
                    model_id, change_id, timestamp_ms, repository_id],
        ).await?;
        Ok(())
    }

    /// List oplog entries for a repository (optionally filtered by agent), newest first.
    pub async fn list_oplog_entries(
        &self,
        agent_id: Option<&str>,
        repository_id: &str,
        limit: u32,
    ) -> Result<Vec<Vec<Option<String>>>, StoreError> {
        let (sql, rows) = if let Some(aid) = agent_id {
            let sql = "SELECT operation_id, agent_id, kind, description, predecessor_hash, model_id,
                              CAST(change_id AS TEXT), CAST(timestamp_ms AS TEXT), CAST(undone AS TEXT)
                       FROM agent_oplog WHERE repository_id=?1 AND agent_id=?2
                       ORDER BY timestamp_ms DESC LIMIT ?3";
            let rows = self
                .conn
                .query(sql, params![repository_id, aid, limit as i64])
                .await?;
            (sql, rows)
        } else {
            let sql = "SELECT operation_id, agent_id, kind, description, predecessor_hash, model_id,
                              CAST(change_id AS TEXT), CAST(timestamp_ms AS TEXT), CAST(undone AS TEXT)
                       FROM agent_oplog WHERE repository_id=?1
                       ORDER BY timestamp_ms DESC LIMIT ?2";
            let rows = self
                .conn
                .query(sql, params![repository_id, limit as i64])
                .await?;
            (sql, rows)
        };
        let _ = sql;
        let mut cursor = rows;
        let mut out = Vec::new();
        while let Some(row) = cursor.next().await? {
            let cols: Vec<Option<String>> = (0..9)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            out.push(cols);
        }
        Ok(out)
    }

    /// Mark an oplog entry as undone (or re-done).
    pub async fn set_oplog_undone(
        &self,
        operation_id: &str,
        undone: bool,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE agent_oplog SET undone=?1 WHERE operation_id=?2",
                params![if undone { 1i64 } else { 0i64 }, operation_id],
            )
            .await?;
        Ok(())
    }

    // ── Actor State (actor_state) — V21 ──────────────────────────────────────

    /// Save a JSON-serialized actor state value under a key (upsert).
    pub async fn save_actor_state(&self, key: &str, value_json: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO actor_state (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=datetime('now')",
                params![key, value_json],
            )
            .await?;
        Ok(())
    }

    /// Load a JSON-serialized actor state value by key.
    pub async fn load_actor_state(&self, key: &str) -> Result<Option<String>, StoreError> {
        let mut rows = self
            .conn
            .query("SELECT value FROM actor_state WHERE key=?1", params![key])
            .await?;
        Ok(rows
            .next()
            .await?
            .and_then(|r| r.get::<Option<String>>(0).unwrap_or(None)))
    }

    /// Delete an actor state entry.
    pub async fn delete_actor_state(&self, key: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM actor_state WHERE key=?1", params![key])
            .await?;
        Ok(())
    }

    // ── Oplog Locks (agent_locks via ops_orchestrator) ────────────────────────

    /// Try to acquire a file lock for an agent. Returns true if acquired.
    pub async fn acquire_file_lock(
        &self,
        path: &str,
        agent_id: &str,
        repository_id: &str,
    ) -> Result<bool, StoreError> {
        let affected = self
            .conn
            .execute(
                "INSERT OR IGNORE INTO agent_locks (path, agent_id, repository_id, acquired_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
                params![path, agent_id, repository_id],
            )
            .await?;
        Ok(affected > 0)
    }

    /// Release a file lock held by an agent.
    pub async fn release_file_lock(&self, path: &str, agent_id: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "DELETE FROM agent_locks WHERE path=?1 AND agent_id=?2",
                params![path, agent_id],
            )
            .await?;
        Ok(())
    }

    /// List current file locks for a repository.
    pub async fn list_file_locks(
        &self,
        repository_id: &str,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT path, agent_id, acquired_at FROM agent_locks WHERE repository_id=?1",
                params![repository_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<String>(1)?,
                row.get::<String>(2)?,
            ));
        }
        Ok(out)
    }

    // ── Heartbeats (agent_heartbeats) ─────────────────────────────────────────

    /// Upsert a heartbeat record for an agent.
    pub async fn upsert_heartbeat(
        &self,
        agent_id: &str,
        repository_id: &str,
        status: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO agent_heartbeats (agent_id, repository_id, status, last_seen)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(agent_id, repository_id) DO UPDATE SET
               status=excluded.status, last_seen=datetime('now')",
                params![agent_id, repository_id, status],
            )
            .await?;
        Ok(())
    }

    /// List heartbeats for a repository.
    pub async fn list_heartbeats(
        &self,
        repository_id: &str,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT agent_id, status, last_seen FROM agent_heartbeats WHERE repository_id=?1",
                params![repository_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get::<String>(0)?,
                row.get::<String>(1)?,
                row.get::<String>(2)?,
            ));
        }
        Ok(out)
    }

    /// Delete heartbeats not updated within `timeout_secs` seconds.
    pub async fn prune_stale_heartbeats(&self, timeout_secs: i64) -> Result<u64, StoreError> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM agent_heartbeats
             WHERE last_seen < datetime('now', '-' || ?1 || ' seconds')",
                params![timeout_secs],
            )
            .await?;
        Ok(affected as u64)
    }

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
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_policy_snapshots
             (user_id, event_type, base_xp, base_crystals, mode_label, effective_multiplier,
              awarded_xp, awarded_crystals, streak_days, grind_capped, lumens)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    user_id,
                    event_type,
                    base_xp,
                    base_crystals,
                    mode_label,
                    effective_multiplier,
                    awarded_xp,
                    awarded_crystals,
                    streak_days,
                    if grind_capped { 1i64 } else { 0i64 },
                    lumens
                ],
            )
            .await?;
        Ok(())
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
        self.conn
            .execute(
                "UPDATE gamify_collegium SET lumens=COALESCE(lumens, 0)+?1 WHERE id=?2",
                params![lumens_delta, collegium_id],
            )
            .await?;
        Ok(())
    }
}
