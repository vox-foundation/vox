use turso::params;

use crate::store::types::{AgentEventRow, StoreError};

impl crate::VoxDb {
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
    ) -> Result<Vec<AgentEventRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, agent_id, event_type, payload_json, cli_version, timestamp
             FROM agent_events WHERE agent_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
                params![agent_id, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(AgentEventRow {
                id: row.get(0)?,
                agent_id: row.get(1)?,
                event_type: row.get(2)?,
                payload_json: row.get(3)?,
                cli_version: row.get(4)?,
                timestamp: row.get(5)?,
            });
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
}
