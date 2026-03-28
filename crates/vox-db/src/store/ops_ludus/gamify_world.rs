//! Gamification CRUD for [`crate::store::VoxDb`] (Arca / Turso).
//!
//! All gamification tables live in the schema under the `gamify_*` prefix. This module provides
//! the typed CRUD methods that `vox-ludus` consumes; SQL is owned by `vox-db` (`store/ops_*.rs`).
//!
//! **Do not** use `db.connection().execute(...)` from `vox-ludus` or other crates. Add a method
//! here and call it via `VoxDb::<method>`.

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
        let user_id = user_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
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
                    params![
                        user_id.as_str(),
                        level,
                        xp,
                        crystals,
                        energy,
                        max_energy,
                        last_energy_regen,
                        last_active,
                        streak_days,
                        longest_streak,
                        streak_last_ts,
                        grace_available,
                        grace_used,
                        total_xp_earned,
                        prestige_level,
                        lumens,
                        generosity_lumens,
                        streak_shields
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Update only lumens for a user's profile row.
    pub async fn update_gamify_profile_lumens(
        &self,
        user_id: &str,
        lumens_delta: i64,
    ) -> Result<(), StoreError> {
        let user_id = user_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE gamify_profiles SET lumens = COALESCE(lumens, 0) + ?1 WHERE user_id = ?2",
                    params![lumens_delta, user_id.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
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
        let user_id = user_id.to_string();
        let achievement_id = achievement_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let affected = conn
                    .execute(
                        "INSERT OR IGNORE INTO gamify_achievements
             (id, user_id, unlocked_at, xp_rewarded, crystals_rewarded)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            achievement_id.as_str(),
                            user_id.as_str(),
                            unlocked_at,
                            xp_rewarded,
                            crystals_rewarded
                        ],
                    )
                    .await?;
                Ok::<_, StoreError>(affected > 0)
            })
            .await
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
        let id = id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "DELETE FROM gamify_companions WHERE id = ?1",
                    params![id.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
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
        let user_id = user_id.to_string();
        let title = title.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO gamify_level_history (user_id, level, title, xp_at_level, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![user_id.as_str(), level, title.as_str(), xp_at_level, created_at],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
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
        let id = id.to_string();
        let user_id = user_id.to_string();
        let name = name.to_string();
        let language = language.to_string();
        let mood = mood.to_string();
        let personality = personality.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
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
                    params![
                        id.as_str(),
                        user_id.as_str(),
                        name.as_str(),
                        description,
                        code_hash,
                        language.as_str(),
                        ascii_sprite,
                        mood.as_str(),
                        health,
                        max_health,
                        energy,
                        max_energy,
                        code_quality,
                        last_active,
                        personality.as_str(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
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
        let id = id.to_string();
        let user_id = user_id.to_string();
        let quest_type = quest_type.to_string();
        let description = description.to_string();
        let status = status.to_string();
        let completed_flag = if completed { 1i64 } else { 0i64 };
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO gamify_quests
             (id, user_id, quest_type, description, target, progress, crystal_reward, xp_reward, completed, expires_at, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(id) DO UPDATE SET
               quest_type=excluded.quest_type,
               description=excluded.description, xp_reward=excluded.xp_reward,
               crystal_reward=excluded.crystal_reward, target=excluded.target,
               progress=excluded.progress, status=excluded.status,
               expires_at=excluded.expires_at, completed=excluded.completed",
                    params![
                        id.as_str(),
                        user_id.as_str(),
                        quest_type.as_str(),
                        description.as_str(),
                        target,
                        progress,
                        crystal_reward,
                        xp_reward,
                        completed_flag,
                        expires_at,
                        status.as_str(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Update quest status + completed flag.
    pub async fn update_gamify_quest_status(
        &self,
        id: &str,
        user_id: &str,
        status: &str,
        completed: bool,
    ) -> Result<(), StoreError> {
        let id = id.to_string();
        let user_id = user_id.to_string();
        let status = status.to_string();
        let completed_flag = if completed { 1i64 } else { 0i64 };
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE gamify_quests SET status = ?1, completed = ?2 WHERE id = ?3 AND user_id = ?4",
                    params![
                        status.as_str(),
                        completed_flag,
                        id.as_str(),
                        user_id.as_str(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Delete a quest by id.
    pub async fn delete_gamify_quest(&self, id: &str) -> Result<(), StoreError> {
        let id = id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "DELETE FROM gamify_quests WHERE id = ?1",
                    params![id.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Load one quest row by id (Ludus `Quest` mapping).
    pub async fn get_gamify_quest_by_id(
        &self,
        id: &str,
    ) -> Result<
        Option<(
            String,
            String,
            String,
            String,
            i64,
            i64,
            i64,
            i64,
            i64,
            i64,
            String,
            String,
            String,
        )>,
        StoreError,
    > {
        let mut rows = self.conn.query(
            "SELECT id, user_id, quest_type, description, target, progress, crystal_reward, xp_reward, completed, expires_at,
                    hint, modifier, status
             FROM gamify_quests WHERE id = ?1",
            params![id],
        ).await?;
        if let Some(row) = rows.next().await? {
            Ok(Some((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get::<i64>(4)?,
                row.get::<i64>(5)?,
                row.get::<i64>(6)?,
                row.get::<i64>(7)?,
                row.get::<i64>(8)?,
                row.get::<i64>(9)?,
                row.get::<String>(10).unwrap_or_default(),
                row.get::<String>(11).unwrap_or_else(|_| "none".to_string()),
                row.get::<String>(12).unwrap_or_default(),
            )))
        } else {
            Ok(None)
        }
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
}
