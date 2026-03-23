//! Agent session management and LLM interaction logging for [`CodeStore`].
//!
//! Covers two V3/agents-domain table groups:
//! - **`agent_sessions`** — lifecycle tracking: create, close, query.
//! - **`llm_interactions`** + **`llm_feedback`** — RLHF data pipeline used by `vox-pm/feedback.rs`.

use turso::params;

use crate::store::CodeStore;
use crate::store::types::StoreError;

impl CodeStore {
    // ── Agent Sessions (agent_sessions) ──────────────────────────────────────

    /// Insert or activate an `agent_sessions` row.
    ///
    /// On conflict the row's `status` is set back to `'active'` and `task_snapshot` updated.
    /// Called from `vox-orchestrator/src/session.rs`.
    pub async fn create_session(
        &self,
        session_id: &str,
        agent_id: &str,
        task_snapshot: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO agent_sessions (id, agent_id, task_snapshot, status, started_at)
                 VALUES (?1, ?2, ?3, 'active', datetime('now'))
                 ON CONFLICT(id) DO UPDATE SET
                     task_snapshot = excluded.task_snapshot,
                     status        = 'active'",
                params![session_id, agent_id, task_snapshot],
            )
            .await?;
        Ok(())
    }

    /// Mark an `agent_sessions` row as the given `status` and set `ended_at`.
    pub async fn close_session(&self, session_id: &str, status: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE agent_sessions
                 SET status = ?2, ended_at = datetime('now')
                 WHERE id = ?1",
                params![session_id, status],
            )
            .await?;
        Ok(())
    }

    // ── LLM Interactions (llm_interactions) ──────────────────────────────────

    /// Append a row to `llm_interactions`. Returns the inserted `rowid`.
    ///
    /// Called from `vox-pm/src/feedback.rs` `FeedbackCollector::persist_to_store`.
    pub async fn log_interaction(
        &self,
        session_id: &str,
        user_id: Option<&str>,
        prompt: &str,
        response: &str,
        model_version: &str,
        latency_ms: Option<i64>,
        token_count: Option<i64>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO llm_interactions
                     (session_id, user_id, prompt, response, model_version, latency_ms, token_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session_id,
                    user_id,
                    prompt,
                    response,
                    model_version,
                    latency_ms,
                    token_count
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── LLM Feedback (llm_feedback) ───────────────────────────────────────────

    /// Append a `llm_feedback` row linked to an `llm_interactions` rowid.
    ///
    /// Called from `vox-pm/src/feedback.rs` `FeedbackCollector::persist_to_store`.
    pub async fn submit_feedback(
        &self,
        interaction_id: i64,
        user_id: Option<&str>,
        rating: Option<i64>,
        feedback_type: &str,
        correction_text: Option<&str>,
        preferred_response: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO llm_feedback
                     (interaction_id, user_id, rating, feedback_type, correction_text, preferred_response)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    interaction_id,
                    user_id,
                    rating,
                    feedback_type,
                    correction_text,
                    preferred_response
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── Agent Reliability (agent_reliability) ─────────────────────────────────

    /// Return all `(agent_id, reliability)` pairs from `agent_reliability`, highest first.
    ///
    /// Used by `vox-orchestrator` `RoutingService::route` when Socrates reputation routing
    /// is enabled (`OrchestratorConfig::socrates_reputation_routing = true`).
    pub async fn list_agent_reliability(&self) -> Result<Vec<(String, f64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT agent_id, reliability FROM agent_reliability ORDER BY reliability DESC",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let r: f64 = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((id, r));
        }
        Ok(out)
    }

    /// Upsert a Laplace-smoothed reliability score for `agent_id` in `agent_reliability`.
    ///
    /// On first insert the row starts at `(success=1, failure=0)` or `(success=0, failure=1)`.
    /// Subsequent calls increment the relevant counter and recompute
    /// `reliability = (success_count + 1) / (success_count + failure_count + 2)`.
    ///
    /// Called from `vox-orchestrator` `Orchestrator::complete_task` and `fail_task`.
    pub async fn record_task_reliability_observation(
        &self,
        agent_id: &str,
        success: bool,
    ) -> Result<(), StoreError> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if success {
            self.conn
                .execute(
                    "INSERT INTO agent_reliability (agent_id, success_count, failure_count,
                         reliability, updated_at_ms)
                     VALUES (?1, 1, 0,
                         CAST(2 AS REAL) / CAST(3 AS REAL),
                         ?2)
                     ON CONFLICT(agent_id) DO UPDATE SET
                         success_count  = success_count + 1,
                         reliability    = CAST(success_count + 2 AS REAL)
                                        / CAST(success_count + failure_count + 3 AS REAL),
                         updated_at_ms  = ?2",
                    params![agent_id, now_ms],
                )
                .await?;
        } else {
            self.conn
                .execute(
                    "INSERT INTO agent_reliability (agent_id, success_count, failure_count,
                         reliability, updated_at_ms)
                     VALUES (?1, 0, 1,
                         CAST(1 AS REAL) / CAST(3 AS REAL),
                         ?2)
                     ON CONFLICT(agent_id) DO UPDATE SET
                         failure_count  = failure_count + 1,
                         reliability    = CAST(success_count + 1 AS REAL)
                                        / CAST(success_count + failure_count + 3 AS REAL),
                         updated_at_ms  = ?2",
                    params![agent_id, now_ms],
                )
                .await?;
        }
        Ok(())
    }

    // ── Object / Workspace Metadata (user_preferences) ───────────────────────

    /// Read a metadata value keyed by `namespace` and `key` from `user_preferences`.
    ///
    /// The look-up key is `"{namespace}.{key}"` and returns the `value` column,
    /// or `StoreError::NotFound` when absent. Used by `vox doctor` to detect
    /// registered project workspaces.
    pub async fn get_object_metadata(&self, namespace: &str, key: &str) -> Result<String, StoreError> {
        let lookup = format!("{namespace}.{key}");
        let mut rows = self
            .conn
            .query(
                "SELECT value FROM user_preferences WHERE key = ?1 LIMIT 1",
                params![lookup],
            )
            .await?;
        match rows.next().await? {
            Some(row) => {
                let val: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
                Ok(val)
            }
            None => Err(StoreError::NotFound(format!("{namespace}.{key}"))),
        }
    }
}
