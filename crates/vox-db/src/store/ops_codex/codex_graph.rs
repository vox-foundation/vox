//! Codex reactivity, research graph, endpoint reliability, trusted evidence, eval
//! runs, and corpus snapshots for [`VoxDb`].
//!
//! Tables covered:
//! - **`codex_change_log`** + **`codex_schema_lineage`** — Codex SSE reactivity (V8 schema / codex.rs domain).
//! - **`research_sessions`** + **`conversation_versions`** + **`conversation_edges`** + **`topic_evolution_events`** — research graph (V17).
//! - **`research_metrics`** — Socrates telemetry + arbitrary session metrics (agents.rs domain).
//! - **`trusted_evidence_bundles`** — RAG evidence cache (agents.rs domain).
//! - **`endpoint_reliability`** — exponential-moving-average endpoint health (agents.rs domain).
//! - **`eval_runs`** — regression / RLHF eval snapshots (agents.rs domain).
//! - **`corpus_snapshots`** — corpus fingerprint deduplication (V18 schema).

use turso::params;

use crate::store::types::{
    CodexChangeLogEntry, SkillExecutionParams, SkillExecutionRow, SkillManifestEntry, StoreError,
    WorkflowExecutionRow,
};

impl crate::VoxDb {
    // ── Skill Manifests (skill_manifests) ─────────────────────────────────────

    /// Fetch the latest version of a skill manifest by `skill_id`.
    ///
    /// Returns `None` when the skill is not installed. Called from
    /// `vox-mcp/src/tools/mod.rs` skill macro dispatch.
    pub async fn get_skill_manifest(
        &self,
        skill_id: &str,
    ) -> Result<Option<SkillManifestEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, version, manifest_json, skill_md
                 FROM skill_manifests
                 WHERE id = ?1
                 ORDER BY version DESC LIMIT 1",
                params![skill_id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(SkillManifestEntry {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                version: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                manifest_json: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                skill_md: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            })),
            None => Ok(None),
        }
    }

    // ── Skill Execution Telemetry (skill_executions) ──────────────────────────

    /// Record one skill execution; atomically updates `skill_manifests` counters and
    /// returns the inserted `rowid`. This is the **mandatory** call site after every
    /// tool/skill invocation via `handle_tool_call`.
    pub async fn record_skill_execution(
        &self,
        p: SkillExecutionParams<'_>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO skill_executions
                 (skill_id, version, session_id, workflow_id, agent_id, status, duration_ms,
                  input_hash, output_size, error_kind, reflection_score)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    p.skill_id,
                    p.version,
                    p.session_id,
                    p.workflow_id,
                    p.agent_id,
                    p.status,
                    p.duration_ms,
                    p.input_hash,
                    p.output_size,
                    p.error_kind,
                    p.reflection_score
                ],
            )
            .await?;
        let exec_id = self.conn.last_insert_rowid();
        // Update skill_manifests counters (best-effort — ignore if skill not registered)
        let ok = p.status == "ok";
        let (sc_delta, inv_delta): (i64, i64) = if ok { (1, 1) } else { (0, 1) };
        let _ = self
            .conn
            .execute(
                "UPDATE skill_manifests SET
                   invocation_count = invocation_count + ?1,
                   success_count    = success_count + ?2,
                   last_used_at     = datetime('now')
                 WHERE id = ?3
                   AND version = (SELECT MAX(version) FROM skill_manifests WHERE id = ?3)",
                params![inv_delta, sc_delta, p.skill_id],
            )
            .await;
        Ok(exec_id)
    }

    /// List the most recent executions for a given skill, newest first.
    /// Returns at most `limit` rows (clamped to 1..=1000).
    pub async fn list_skill_executions_by_skill(
        &self,
        skill_id: &str,
        limit: i64,
    ) -> Result<Vec<SkillExecutionRow>, StoreError> {
        let lim = limit.clamp(1, 1_000);
        let mut rows = self
            .conn
            .query(
                "SELECT id, skill_id, version, session_id, workflow_id, agent_id,
                        status, duration_ms, input_hash, output_size, error_kind,
                        reflection_score, created_at
                 FROM skill_executions
                 WHERE skill_id = ?1
                 ORDER BY id DESC LIMIT ?2",
                params![skill_id, lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(SkillExecutionRow {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                skill_id: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                version: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                session_id: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                workflow_id: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                agent_id: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                status: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                duration_ms: row.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                input_hash: row.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                output_size: row.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
                error_kind: row.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
                reflection_score: row.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
                created_at: row.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    // ── Workflow Execution Telemetry (workflow_executions) ────────────────────

    /// Start or resume a `workflow_executions` row.
    pub async fn start_workflow_execution(
        &self,
        workflow_id: &str,
        step_count: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO workflow_executions (workflow_id, status, step_count)
                 VALUES (?1, 'running', ?2)
                 ON CONFLICT(workflow_id) DO UPDATE SET status = 'running', step_count = ?2",
                params![workflow_id, step_count],
            )
            .await?;
        Ok(())
    }

    /// Check if an activity was already completed successfully in a previous run.
    pub async fn is_activity_completed(
        &self,
        workflow_id: &str,
        activity_name: &str,
    ) -> Result<bool, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM execution_log
                 WHERE workflow_id = ?1 AND activity_name = ?2 AND status = 'ok'",
                params![workflow_id, activity_name],
            )
            .await?;
        let count: i64 = if let Some(row) = rows.next().await? {
            row.get(0).unwrap_or(0)
        } else {
            0
        };
        Ok(count > 0)
    }

    /// Insert a record for an activity execution into `execution_log`.
    pub async fn log_execution(
        &self,
        p: &crate::store::types::LogExecutionParams<'_>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO execution_log (
                    workflow_id, agent_id, skill_id, activity_name, status,
                    attempt, duration_ms, output_size, input, output, error, options
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    p.workflow_id,
                    p.agent_id,
                    p.skill_id,
                    p.activity_name,
                    p.status,
                    p.attempt,
                    p.duration_ms,
                    p.output_size,
                    p.input,
                    p.output,
                    p.error,
                    p.options
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Mark a `workflow_executions` row as finished (sets `finished_at`, `status`).
    /// A no-op when the row does not exist.
    ///
    /// Called from `vox-orchestrator` `Orchestrator::complete_task` / `fail_task`.
    pub async fn finish_workflow_execution(
        &self,
        workflow_id: &str,
        status: &str,
        error_count: i64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE workflow_executions
                 SET status = ?2, error_count = ?3,
                     finished_at = datetime('now')
                 WHERE workflow_id = ?1 AND finished_at IS NULL",
                params![workflow_id, status, error_count],
            )
            .await?;
        Ok(())
    }

    /// Fetch the current `workflow_executions` row for `workflow_id`.
    /// Returns `None` when no matching row exists.
    pub async fn get_workflow_execution(
        &self,
        workflow_id: &str,
    ) -> Result<Option<WorkflowExecutionRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT workflow_id, status, step_count, error_count,
                        started_at, finished_at
                 FROM workflow_executions
                 WHERE workflow_id = ?1 LIMIT 1",
                params![workflow_id],
            )
            .await?;
        match rows.next().await? {
            Some(row) => Ok(Some(WorkflowExecutionRow {
                workflow_id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                status: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                step_count: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                error_count: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                started_at: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                finished_at: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            })),
            None => Ok(None),
        }
    }

    // ── Codex Change Log (codex_change_log) ──────────────────────────────────

    /// Append a `codex_change_log` row. Returns its `rowid`.
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::append_codex_change`.
    pub async fn append_codex_change(
        &self,
        topic: &str,
        entity_kind: Option<&str>,
        entity_id: Option<&str>,
        change_kind: &str,
        payload_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO codex_change_log (topic, entity_kind, entity_id, change_kind, payload_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![topic, entity_kind, entity_id, change_kind, payload_json],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Read `codex_change_log` rows with `id > after_id`, optionally filtered by `topic`.
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::list_codex_changes_since`.
    pub async fn list_codex_changes_since(
        &self,
        topic: Option<&str>,
        after_id: i64,
        limit: i64,
    ) -> Result<Vec<CodexChangeLogEntry>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let mut rows = match topic {
            Some(t) => {
                self.conn
                    .query(
                        "SELECT id, topic, entity_kind, entity_id, change_kind, payload_json, created_at
                         FROM codex_change_log
                         WHERE id > ?1 AND topic = ?2
                         ORDER BY id ASC LIMIT ?3",
                        params![after_id, t, lim],
                    )
                    .await?
            }
            None => {
                self.conn
                    .query(
                        "SELECT id, topic, entity_kind, entity_id, change_kind, payload_json, created_at
                         FROM codex_change_log
                         WHERE id > ?1
                         ORDER BY id ASC LIMIT ?2",
                        params![after_id, lim],
                    )
                    .await?
            }
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(CodexChangeLogEntry {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                topic: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                entity_kind: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                entity_id: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                change_kind: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                payload_json: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                created_at: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    /// Insert a `codex_schema_lineage` row. Returns its `rowid`.
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::record_codex_schema_lineage`.
    pub async fn record_codex_schema_lineage(
        &self,
        baseline_id: &str,
        schema_digest: &str,
        provenance: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO codex_schema_lineage (baseline_id, schema_digest, provenance)
                 VALUES (?1, ?2, ?3)",
                params![baseline_id, schema_digest, provenance],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── Research Graph (research_sessions, conversation_versions, …) ──────────

    /// Upsert a `research_sessions` row by the stable `session_key`. Returns the row `id`.
    ///
    /// Called from `vox-db/src/codex_conversation_graph.rs`.
    pub async fn upsert_research_session(
        &self,
        session_key: &str,
        title: &str,
        status: &str,
        repository_id: &str,
        config_json: Option<&str>,
        summary_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO research_sessions
                     (session_key, title, status, repository_id, config_json, summary_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(session_key) DO UPDATE SET
                     title        = CASE WHEN excluded.title = '' THEN research_sessions.title ELSE excluded.title END,
                     status       = excluded.status,
                     config_json  = COALESCE(excluded.config_json,  research_sessions.config_json),
                     summary_json = COALESCE(excluded.summary_json, research_sessions.summary_json),
                     updated_at   = datetime('now')",
                params![session_key, title, status, repository_id, config_json, summary_json],
            )
            .await?;
        let mut rows = self
            .conn
            .query(
                "SELECT id FROM research_sessions WHERE session_key = ?1 LIMIT 1",
                params![session_key],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("upsert_research_session: readback failed".into()))?;
        let id: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(id)
    }

    /// Upsert a `conversation_versions` row. Returns its `rowid`.
    pub async fn append_conversation_version(
        &self,
        conversation_id: i64,
        version_index: i64,
        label: &str,
        snapshot_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO conversation_versions
                     (conversation_id, version_index, label, snapshot_json)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(conversation_id, version_index) DO UPDATE SET
                     label         = excluded.label,
                     snapshot_json = COALESCE(excluded.snapshot_json,
                                              conversation_versions.snapshot_json)",
                params![conversation_id, version_index, label, snapshot_json],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Insert a `conversation_edges` row. Returns its `rowid`.
    pub async fn insert_conversation_edge(
        &self,
        from_conversation_id: i64,
        to_conversation_id: i64,
        edge_kind: &str,
        weight: f64,
        metadata_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO conversation_edges
                     (from_conversation_id, to_conversation_id, edge_kind, weight, metadata_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    from_conversation_id,
                    to_conversation_id,
                    edge_kind,
                    weight,
                    metadata_json
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Append a `topic_evolution_events` row. Returns its `rowid`.
    pub async fn append_topic_evolution_event(
        &self,
        topic_id: i64,
        event_kind: &str,
        prior_label: Option<&str>,
        new_label: Option<&str>,
        detail_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO topic_evolution_events
                     (topic_id, event_kind, prior_label, new_label, detail_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![topic_id, event_kind, prior_label, new_label, detail_json],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }
}
