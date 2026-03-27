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
        self.conn
            .execute(
                "INSERT INTO plan_sessions (
                    plan_session_id, origin_session_id, goal_text, strategy, current_version, status
                ) VALUES (?1, ?2, ?3, ?4, 1, 'pending')
                ON CONFLICT(plan_session_id) DO UPDATE SET
                    origin_session_id = excluded.origin_session_id,
                    goal_text = excluded.goal_text,
                    strategy = excluded.strategy,
                    updated_at = datetime('now')",
                params![plan_session_id, origin_session_id, goal_text, strategy],
            )
            .await?;
        Ok(())
    }

    pub async fn append_plan_version(
        &self,
        plan_session_id: &str,
        version: i64,
        parent_version: Option<i64>,
        trigger_event: Option<&str>,
        trigger_payload_json: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO plan_versions (
                    plan_session_id, version, parent_version, trigger_event, trigger_payload_json
                ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    plan_session_id,
                    version,
                    parent_version,
                    trigger_event,
                    trigger_payload_json
                ],
            )
            .await?;
        self.conn
            .execute(
                "UPDATE plan_sessions SET current_version = ?2, updated_at = datetime('now') WHERE plan_session_id = ?1",
                params![plan_session_id, version],
            )
            .await?;
        Ok(())
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
        self.conn
            .execute(
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
                    plan_session_id,
                    version,
                    node_id,
                    description,
                    dependencies_json,
                    execution_policy_json,
                    status,
                    workflow_invocation
                ],
            )
            .await?;
        Ok(())
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
        self.conn
            .execute(
                "INSERT INTO plan_node_attempts (
                    plan_session_id, version, node_id, attempt_no, task_id, outcome, error_text, latency_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    plan_session_id,
                    version,
                    node_id,
                    attempt_no,
                    task_id,
                    outcome,
                    error_text,
                    latency_ms
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn set_plan_head(
        &self,
        plan_session_id: &str,
        version: i64,
        status: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE plan_sessions
                 SET current_version = ?2,
                     status = COALESCE(?3, status),
                     updated_at = datetime('now')
                 WHERE plan_session_id = ?1",
                params![plan_session_id, version, status],
            )
            .await?;
        Ok(())
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
        self.conn
            .execute(
                "UPDATE plan_nodes SET status = ?4, updated_at = datetime('now')
                 WHERE plan_session_id = ?1 AND version = ?2 AND node_id = ?3",
                params![plan_session_id, version, node_id, status],
            )
            .await?;
        Ok(())
    }

    pub async fn update_plan_session_iterative_fields(
        &self,
        plan_session_id: &str,
        question_session_id: Option<&str>,
        loop_round: i64,
        stop_reason: Option<&str>,
        metadata_json: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE plan_sessions SET
                    question_session_id = COALESCE(?2, question_session_id),
                    iterative_loop_round = ?3,
                    iterative_stop_reason = COALESCE(?4, iterative_stop_reason),
                    iterative_loop_metadata_json = COALESCE(?5, iterative_loop_metadata_json),
                    updated_at = datetime('now')
                 WHERE plan_session_id = ?1",
                params![
                    plan_session_id,
                    question_session_id,
                    loop_round,
                    stop_reason,
                    metadata_json
                ],
            )
            .await?;
        Ok(())
    }
}
