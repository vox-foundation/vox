use turso::params;

use crate::store::types::{A2AMessageRow, StoreError};

impl crate::VoxDb {
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
    ) -> Result<Vec<A2AMessageRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, message_uuid, sender_agent, receiver_agent, msg_type, payload,
                    priority, thread_id, acknowledged, created_at, repository_id
             FROM a2a_messages
             WHERE receiver_agent=?1 AND acknowledged=0 AND repository_id=?2
             ORDER BY priority DESC, created_at ASC",
                params![agent_id, repository_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let ack: i64 = row.get(8).unwrap_or(0);
            out.push(A2AMessageRow {
                id: row.get(0)?,
                message_uuid: row.get(1)?,
                sender_agent: row.get(2)?,
                receiver_agent: row.get(3)?,
                msg_type: row.get(4)?,
                payload: row.get(5)?,
                priority: row.get(6)?,
                thread_id: row.get(7)?,
                acknowledged: ack != 0,
                created_at: row.get(9)?,
                repository_id: row.get(10)?,
            });
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

    // ── Teaching profiles (gamify_teaching_profiles) ─────────────────────────

    /// Read teaching profile row (`stage`, `silenced`, `mistake_counts` JSON, `cooldowns` JSON).
    pub async fn get_gamify_teaching_profile_row(
        &self,
        user_id: &str,
    ) -> Result<Option<(String, i64, String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT stage, silenced, mistake_counts, cooldowns
             FROM gamify_teaching_profiles WHERE user_id = ?1",
                params![user_id],
            )
            .await?;
        Ok(if let Some(row) = rows.next().await? {
            Some((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        } else {
            None
        })
    }

    /// Upsert teaching profile (same semantics as Ludus `upsert_teaching_profile`).
    pub async fn upsert_gamify_teaching_profile(
        &self,
        user_id: &str,
        stage: &str,
        silenced: bool,
        mistake_counts_json: &str,
        cooldowns_json: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO gamify_teaching_profiles (user_id, stage, silenced, mistake_counts, cooldowns)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(user_id) DO UPDATE SET
            stage = excluded.stage,
            silenced = excluded.silenced,
            mistake_counts = excluded.mistake_counts,
            cooldowns = excluded.cooldowns,
            updated_at = datetime('now')",
                params![
                    user_id,
                    stage,
                    if silenced { 1i64 } else { 0i64 },
                    mistake_counts_json,
                    cooldowns_json,
                ],
            )
            .await?;
        Ok(())
    }
}
