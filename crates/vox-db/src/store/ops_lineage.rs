//! Append-only orchestration lineage (`orchestration_lineage_events`).

use turso::params;

use crate::store::types::StoreError;

impl crate::VoxDb {
    /// Append one lineage row (best-effort consumers should ignore errors).
    pub async fn append_orchestration_lineage_event(
        &self,
        repository_id: &str,
        kind: &str,
        task_id: i64,
        agent_id: Option<i64>,
        session_id: Option<&str>,
        workflow_id: Option<&str>,
        plan_session_id: Option<&str>,
        plan_node_id: Option<&str>,
        payload_json: Option<&str>,
    ) -> Result<(), StoreError> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let repository_id = repository_id.to_string();
        let kind = kind.to_string();
        let session_id = session_id.map(str::to_string);
        let workflow_id = workflow_id.map(str::to_string);
        let plan_session_id = plan_session_id.map(str::to_string);
        let plan_node_id = plan_node_id.map(str::to_string);
        let payload_json = payload_json.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| {
                let repository_id = repository_id.clone();
                let kind = kind.clone();
                let session_id = session_id.clone();
                let workflow_id = workflow_id.clone();
                let plan_session_id = plan_session_id.clone();
                let plan_node_id = plan_node_id.clone();
                let payload_json = payload_json.clone();
                async move {
                    conn.execute(
                        "INSERT INTO orchestration_lineage_events (
                    repository_id, kind, task_id, agent_id, session_id, workflow_id,
                    plan_session_id, plan_node_id, payload_json, created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        params![
                            repository_id.as_str(),
                            kind.as_str(),
                            task_id,
                            agent_id,
                            session_id.as_deref(),
                            workflow_id.as_deref(),
                            plan_session_id.as_deref(),
                            plan_node_id.as_deref(),
                            payload_json.as_deref(),
                            now_ms,
                        ],
                    )
                    .await?;
                    Ok(())
                }
            })
            .await
    }

    /// Cap growth: delete up to `max_rows` events strictly older than `cutoff_ms` (`created_at_ms`).
    pub async fn prune_orchestration_lineage_older_than_ms(
        &self,
        cutoff_ms_exclusive: i64,
        max_rows: u64,
    ) -> Result<u64, StoreError> {
        self.retention_delete_ms_older_than_chunk(
            "orchestration_lineage_events",
            "created_at_ms",
            cutoff_ms_exclusive,
            max_rows,
        )
        .await
    }

    /// Operator/debug: lineage rows for one task in repository order.
    pub async fn list_orchestration_lineage_for_task(
        &self,
        repository_id: &str,
        task_id: i64,
        limit: i64,
    ) -> Result<Vec<(i64, String, i64)>, StoreError> {
        let lim = limit.clamp(1, 500);
        let mut rows = self
            .conn
            .query(
                "SELECT id, kind, created_at_ms FROM orchestration_lineage_events
                 WHERE repository_id = ?1 AND task_id = ?2 ORDER BY id ASC LIMIT ?3",
                params![repository_id, task_id, lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push((
                r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            ));
        }
        Ok(out)
    }
}
