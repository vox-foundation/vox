impl crate::orchestrator::Orchestrator {
    /// Record a lineage event to the persistent Codex store asynchronously if attached.
    pub fn record_lineage_event(
        &self,
        kind: &str,
        task_id: Option<crate::TaskId>,
        agent_id: Option<crate::AgentId>,
        session_id: Option<String>,
        workflow_id: Option<String>,
        plan_session_id: Option<String>,
        plan_node_id: Option<String>,
        payload: Option<serde_json::Value>,
    ) {
        let Some(db) = self.db() else { return };
        let repo = crate::lineage::repository_id();
        let kind = kind.to_string();
        let tid = task_id.map(|t| t.0 as i64).unwrap_or(0);
        let aid = agent_id.map(|a| a.0 as i64);
        let payload_str = payload.map(|p| p.to_string());

        tokio::spawn(async move {
            if let Err(e) = db
                .append_orchestration_lineage_event(
                    &repo,
                    &kind,
                    tid,
                    aid,
                    session_id.as_deref(),
                    workflow_id.as_deref(),
                    plan_session_id.as_deref(),
                    plan_node_id.as_deref(),
                    payload_str.as_deref(),
                )
                .await
            {
                tracing::debug!(error = %e, "lineage persistence failed");
            }
        });
    }

    /// Laplace-smoothed task reliability from Codex `agent_reliability`, when DB is attached.
    pub fn lookup_agent_reliability_sync(&self, agent_id: crate::types::AgentId) -> Option<f64> {
        let db = self.db()?;
        let sid = agent_id.0.to_string();
        db.block_on(async { db.get_agent_reliability(&sid).await })
            .ok()
            .flatten()
    }
}
