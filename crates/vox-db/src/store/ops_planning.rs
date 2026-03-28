use turso::params;

use crate::store::types::{PlanNodeRow, StoreError};

impl crate::VoxDb {
    pub async fn create_plan_session(
        &self,
        plan_session_id: &str,
        origin_session_id: Option<&str>,
        goal_text: &str,
        strategy: &str,
    ) -> Result<(), StoreError> {
        let plan_session_id = plan_session_id.to_string();
        let origin_session_id = origin_session_id.map(str::to_string);
        let goal_text = goal_text.to_string();
        let strategy = strategy.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO plan_sessions (
                    plan_session_id, origin_session_id, goal_text, strategy, current_version, status
                ) VALUES (?1, ?2, ?3, ?4, 1, 'pending')
                ON CONFLICT(plan_session_id) DO UPDATE SET
                    origin_session_id = excluded.origin_session_id,
                    goal_text = excluded.goal_text,
                    strategy = excluded.strategy,
                    updated_at = datetime('now')",
                    params![
                        plan_session_id.as_str(),
                        origin_session_id.as_deref(),
                        goal_text.as_str(),
                        strategy.as_str(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    pub async fn append_plan_version(
        &self,
        plan_session_id: &str,
        version: i64,
        parent_version: Option<i64>,
        trigger_event: Option<&str>,
        trigger_payload_json: Option<&str>,
    ) -> Result<(), StoreError> {
        let plan_session_id = plan_session_id.to_string();
        let trigger_event = trigger_event.map(str::to_string);
        let trigger_payload_json = trigger_payload_json.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT OR REPLACE INTO plan_versions (
                    plan_session_id, version, parent_version, trigger_event, trigger_payload_json
                ) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        plan_session_id.as_str(),
                        version,
                        parent_version,
                        trigger_event.as_deref(),
                        trigger_payload_json.as_deref(),
                    ],
                )
                .await?;
                conn.execute(
                    "UPDATE plan_sessions SET current_version = ?2, updated_at = datetime('now') WHERE plan_session_id = ?1",
                    params![plan_session_id.as_str(), version],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_plan_node(
        &self,
        plan_session_id: &str,
        version: i64,
        node_id: &str,
        description: &str,
        dependencies_json: &str,
        execution_policy_json: &str,
        status: &str,
        workflow_invocation: Option<&str>,
    ) -> Result<(), StoreError> {
        let plan_session_id = plan_session_id.to_string();
        let node_id = node_id.to_string();
        let description = description.to_string();
        let dependencies_json = dependencies_json.to_string();
        let execution_policy_json = execution_policy_json.to_string();
        let status = status.to_string();
        let workflow_invocation = workflow_invocation.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO plan_nodes (
                    plan_session_id, version, node_id, description, dependencies_json, execution_policy_json, status, workflow_invocation
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                ON CONFLICT(plan_session_id, version, node_id) DO UPDATE SET
                    description = excluded.description,
                    dependencies_json = excluded.dependencies_json,
                    execution_policy_json = excluded.execution_policy_json,
                    status = excluded.status,
                    workflow_invocation = excluded.workflow_invocation,
                    updated_at = datetime('now')",
                    params![
                        plan_session_id.as_str(),
                        version,
                        node_id.as_str(),
                        description.as_str(),
                        dependencies_json.as_str(),
                        execution_policy_json.as_str(),
                        status.as_str(),
                        workflow_invocation.as_deref(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    pub async fn record_plan_node_attempt(
        &self,
        plan_session_id: &str,
        version: i64,
        node_id: &str,
        attempt_no: i64,
        task_id: Option<&str>,
        outcome: &str,
        error_text: Option<&str>,
        latency_ms: Option<i64>,
    ) -> Result<(), StoreError> {
        let plan_session_id = plan_session_id.to_string();
        let node_id = node_id.to_string();
        let task_id = task_id.map(str::to_string);
        let outcome = outcome.to_string();
        let error_text = error_text.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO plan_node_attempts (
                    plan_session_id, version, node_id, attempt_no, task_id, outcome, error_text, latency_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        plan_session_id.as_str(),
                        version,
                        node_id.as_str(),
                        attempt_no,
                        task_id.as_deref(),
                        outcome.as_str(),
                        error_text.as_deref(),
                        latency_ms,
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    pub async fn set_plan_head(
        &self,
        plan_session_id: &str,
        version: i64,
        status: Option<&str>,
    ) -> Result<(), StoreError> {
        let plan_session_id = plan_session_id.to_string();
        let status = status.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE plan_sessions
                 SET current_version = ?2,
                     status = COALESCE(?3, status),
                     updated_at = datetime('now')
                 WHERE plan_session_id = ?1",
                    params![plan_session_id.as_str(), version, status.as_deref(),],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    pub async fn load_plan_head(&self, plan_session_id: &str) -> Result<Option<i64>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT current_version FROM plan_sessions WHERE plan_session_id = ?1",
                params![plan_session_id],
            )
            .await?;
        if let Some(r) = rows.next().await? {
            return Ok(Some(r.get::<i64>(0)?));
        }
        Ok(None)
    }

    pub async fn list_runnable_nodes(
        &self,
        plan_session_id: &str,
        version: i64,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT node_id, description
                 FROM plan_nodes
                 WHERE plan_session_id = ?1
                   AND version = ?2
                   AND status IN ('pending','queued')",
                params![plan_session_id, version],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push((r.get::<String>(0)?, r.get::<String>(1)?));
        }
        Ok(out)
    }

    /// Full node rows for DAG scheduling (`plan_nodes.status` drives readiness).
    pub async fn load_plan_nodes_with_status(
        &self,
        plan_session_id: &str,
        version: i64,
    ) -> Result<Vec<PlanNodeRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT node_id, description, dependencies_json, execution_policy_json, status, workflow_invocation
                 FROM plan_nodes
                 WHERE plan_session_id = ?1 AND version = ?2
                 ORDER BY node_id",
                params![plan_session_id, version],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(PlanNodeRow {
                plan_session_id: plan_session_id.to_string(),
                version,
                node_id: r.get::<String>(0)?,
                description: r.get::<String>(1)?,
                dependencies_json: r.get::<String>(2)?,
                execution_policy_json: r.get::<String>(3)?,
                status: r.get::<String>(4)?,
                workflow_invocation: r.get::<Option<String>>(5)?,
            });
        }
        Ok(out)
    }

    pub async fn set_plan_node_status(
        &self,
        plan_session_id: &str,
        version: i64,
        node_id: &str,
        status: &str,
    ) -> Result<(), StoreError> {
        let plan_session_id = plan_session_id.to_string();
        let node_id = node_id.to_string();
        let status = status.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE plan_nodes SET status = ?4, updated_at = datetime('now')
                 WHERE plan_session_id = ?1 AND version = ?2 AND node_id = ?3",
                    params![
                        plan_session_id.as_str(),
                        version,
                        node_id.as_str(),
                        status.as_str(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    pub async fn update_plan_session_iterative_fields(
        &self,
        plan_session_id: &str,
        question_session_id: Option<&str>,
        loop_round: i64,
        stop_reason: Option<&str>,
        metadata_json: Option<&str>,
    ) -> Result<(), StoreError> {
        let plan_session_id = plan_session_id.to_string();
        let question_session_id = question_session_id.map(str::to_string);
        let stop_reason = stop_reason.map(str::to_string);
        let metadata_json = metadata_json.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE plan_sessions SET
                    question_session_id = COALESCE(?2, question_session_id),
                    iterative_loop_round = ?3,
                    iterative_stop_reason = COALESCE(?4, iterative_stop_reason),
                    iterative_loop_metadata_json = COALESCE(?5, iterative_loop_metadata_json),
                    updated_at = datetime('now')
                 WHERE plan_session_id = ?1",
                    params![
                        plan_session_id.as_str(),
                        question_session_id.as_deref(),
                        loop_round,
                        stop_reason.as_deref(),
                        metadata_json.as_deref(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }
}
