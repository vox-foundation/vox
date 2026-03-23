//! Codex reactivity, research graph, endpoint reliability, trusted evidence, eval
//! runs, and corpus snapshots for [`CodeStore`].
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

use crate::store::CodeStore;
use crate::store::types::{
    CodexChangeLogEntry, EndpointReliabilityEntry, PackageSearchResult, SkillManifestEntry, SkillExecutionParams,
    StoreError,
};

impl CodeStore {
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
                    p.skill_id, p.version, p.session_id, p.workflow_id, p.agent_id,
                    p.status, p.duration_ms, p.input_hash, p.output_size,
                    p.error_kind, p.reflection_score
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

    // ── Workflow Execution Telemetry (workflow_executions) ────────────────────

    /// Mark a `workflow_executions` row as finished (sets `ended_at`, `status`, `output_size`,
    /// and optionally `error_message`). A no-op when the row does not exist.
    ///
    /// Called from `vox-orchestrator` `Orchestrator::complete_task` / `fail_task`.
    pub async fn finish_workflow_execution(
        &self,
        workflow_id: &str,
        status: &str,
        output_size: i64,
        error_message: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "UPDATE workflow_executions
                 SET status = ?2, output_size = ?3, error_message = ?4,
                     ended_at = datetime('now')
                 WHERE workflow_id = ?1 AND ended_at IS NULL",
                params![workflow_id, status, output_size, error_message],
            )
            .await?;
        Ok(())
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

    // ── Research Metrics (research_metrics) ───────────────────────────────────

    /// Append a `research_metrics` row. Returns its `rowid`.
    ///
    /// Called from `vox-db/src/codex_conversation_graph.rs` and
    /// `vox-db/src/socrates_telemetry.rs`.
    pub async fn append_research_metric(
        &self,
        session_id: &str,
        metric_type: &str,
        metric_value: Option<f64>,
        metadata_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO research_metrics (session_id, metric_type, metric_value, metadata_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![session_id, metric_type, metric_value, metadata_json],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Fetch the newest `research_metrics` rows of `metric_type` where `session_id` starts with
    /// `session_prefix` (prefix match via `LIKE`). Returns `(session_id, metric_value, metadata_json)`.
    ///
    /// Called from `vox-db/src/socrates_telemetry.rs` `VoxDb::list_socrates_surface_events`.
    pub async fn list_research_metrics_by_type(
        &self,
        metric_type: &str,
        session_prefix: &str,
        limit: i64,
    ) -> Result<Vec<(String, f64, Option<String>)>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let pattern = format!("{session_prefix}%");
        let mut rows = self
            .conn
            .query(
                "SELECT session_id, COALESCE(metric_value, 0.0), metadata_json
                 FROM research_metrics
                 WHERE metric_type = ?1
                   AND (?2 = '%' OR session_id LIKE ?2)
                 ORDER BY id DESC LIMIT ?3",
                params![metric_type, pattern, lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let sid: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let mv: f64 = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let meta: Option<String> = row.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((sid, mv, meta));
        }
        Ok(out)
    }

    // ── Trusted Evidence Bundles (trusted_evidence_bundles) ───────────────────

    /// Upsert a `trusted_evidence_bundles` row. Returns its `rowid`.
    ///
    /// Called from `vox-db/src/rag_evidence.rs` `VoxDb::store_trusted_evidence`.
    pub async fn upsert_trusted_evidence_bundle(
        &self,
        bundle_key: &str,
        repository_id: &str,
        session_key: &str,
        evidence_json: &str,
        contradiction_count: i64,
        expires_at: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO trusted_evidence_bundles
                     (bundle_key, repository_id, session_key, evidence_json,
                      contradiction_count, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(bundle_key) DO UPDATE SET
                     evidence_json       = excluded.evidence_json,
                     contradiction_count = excluded.contradiction_count,
                     expires_at          = excluded.expires_at,
                     created_at          = datetime('now')",
                params![
                    bundle_key,
                    repository_id,
                    session_key,
                    evidence_json,
                    contradiction_count,
                    expires_at
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Fetch `evidence_json` for a `bundle_key`, or `None` if absent or expired.
    ///
    /// Called from `vox-db/src/rag_evidence.rs` `VoxDb::get_trusted_evidence`.
    pub async fn get_trusted_evidence_bundle(
        &self,
        bundle_key: &str,
    ) -> Result<Option<String>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT evidence_json FROM trusted_evidence_bundles
                 WHERE bundle_key = ?1
                   AND (expires_at IS NULL OR expires_at > datetime('now'))
                 LIMIT 1",
                params![bundle_key],
            )
            .await?;
        match rows.next().await? {
            Some(row) => {
                let j: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
                Ok(Some(j))
            }
            None => Ok(None),
        }
    }

    /// List `(bundle_key, evidence_json)` for a `(repository_id, session_key)` pair, newest first.
    ///
    /// Called from `vox-db/src/rag_evidence.rs` `VoxDb::list_trusted_evidence`.
    pub async fn list_trusted_evidence_bundles(
        &self,
        repository_id: &str,
        session_key: &str,
        limit: i64,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let lim = limit.clamp(1, 1_000);
        let mut rows = self
            .conn
            .query(
                "SELECT bundle_key, evidence_json
                 FROM trusted_evidence_bundles
                 WHERE repository_id = ?1 AND session_key = ?2
                   AND (expires_at IS NULL OR expires_at > datetime('now'))
                 ORDER BY id DESC LIMIT ?3",
                params![repository_id, session_key, lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let key: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let json: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((key, json));
        }
        Ok(out)
    }

    // ── Endpoint Reliability (endpoint_reliability) ───────────────────────────

    /// Update EWMA stats for `(endpoint_url, model_id)` in `endpoint_reliability`.
    ///
    /// Uses a 5 % smoothing factor (`alpha = 0.05`) for all three EWMA columns.
    /// `infra_failure` is 1.0 for rate-limit / timeout events, 0.0 otherwise.
    ///
    /// Called from `vox-db/src/rag_evidence.rs` `VoxDb::record_endpoint_infra_failure`
    /// and Socrates telemetry paths.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_endpoint_observation(
        &self,
        endpoint_url: &str,
        model_id: &str,
        hallucination_signal: f64,
        contradiction_signal: f64,
        infra_failure: f64,
        is_rate_limit: bool,
        is_timeout: bool,
    ) -> Result<(), StoreError> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        self.conn
            .execute(
                "INSERT INTO endpoint_reliability
                     (endpoint_url, model_id, total_requests,
                      hallucination_proxy_ewma, contradiction_ratio_ewma,
                      infra_failure_ewma, rate_limit_hits, timeout_hits, updated_at_ms)
                 VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, ?7, ?8)
                 ON CONFLICT(endpoint_url, model_id) DO UPDATE SET
                     total_requests            = total_requests + 1,
                     hallucination_proxy_ewma  = hallucination_proxy_ewma  * 0.95 + ?3 * 0.05,
                     contradiction_ratio_ewma  = contradiction_ratio_ewma  * 0.95 + ?4 * 0.05,
                     infra_failure_ewma        = infra_failure_ewma        * 0.95 + ?5 * 0.05,
                     rate_limit_hits           = rate_limit_hits + ?6,
                     timeout_hits              = timeout_hits    + ?7,
                     updated_at_ms             = ?8",
                params![
                    endpoint_url,
                    model_id,
                    hallucination_signal,
                    contradiction_signal,
                    infra_failure,
                    i64::from(is_rate_limit),
                    i64::from(is_timeout),
                    now_ms
                ],
            )
            .await?;
        Ok(())
    }

    /// Fetch all `endpoint_reliability` rows sorted by composite degradation score (worst first).
    ///
    /// Composite = `0.5 * hallucination + 0.3 * contradiction + 0.2 * infra`.
    /// Called from `vox-db/src/rag_evidence.rs`.
    pub async fn list_endpoint_reliability(
        &self,
        limit: i64,
    ) -> Result<Vec<EndpointReliabilityEntry>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let mut rows = self
            .conn
            .query(
                "SELECT endpoint_url, model_id, total_requests,
                        hallucination_proxy_ewma, contradiction_ratio_ewma,
                        infra_failure_ewma, rate_limit_hits, timeout_hits, updated_at_ms
                 FROM endpoint_reliability
                 ORDER BY (0.5 * hallucination_proxy_ewma
                         + 0.3 * contradiction_ratio_ewma
                         + 0.2 * infra_failure_ewma) DESC
                 LIMIT ?1",
                params![lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(EndpointReliabilityEntry {
                endpoint_url: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                model_id: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                total_requests: row.get::<i64>(2).map_err(|e| StoreError::Db(e.to_string()))? as u64,
                hallucination_proxy_ewma: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                contradiction_ratio_ewma: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                infra_failure_ewma: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                rate_limit_hits: row.get::<i64>(6).map_err(|e| StoreError::Db(e.to_string()))? as u64,
                timeout_hits: row.get::<i64>(7).map_err(|e| StoreError::Db(e.to_string()))? as u64,
                updated_at_ms: row.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    // ── Eval Runs (eval_runs) ─────────────────────────────────────────────────

    /// Insert or replace an `eval_runs` row. Returns its `rowid`.
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::record_eval_run`.
    #[allow(clippy::too_many_arguments)]
    pub async fn record_eval_run(
        &self,
        run_id: &str,
        model_path: Option<&str>,
        format_validity: Option<f64>,
        safety_rejection_rate: Option<f64>,
        quality_proxy: Option<f64>,
        skills_discovered: Option<i64>,
        workflows_discovered: Option<i64>,
        metadata_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO eval_runs
                     (run_id, model_path, format_validity, safety_rejection_rate,
                      quality_proxy, skills_discovered, workflows_discovered, metadata_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    run_id,
                    model_path,
                    format_validity,
                    safety_rejection_rate,
                    quality_proxy,
                    skills_discovered,
                    workflows_discovered,
                    metadata_json
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    // ── Corpus Snapshots (corpus_snapshots) ───────────────────────────────────

    /// Insert a `corpus_snapshots` row (idempotent — `INSERT OR IGNORE`). Returns `rowid`.
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::record_corpus_snapshot`.
    pub async fn record_corpus_snapshot(
        &self,
        fingerprint: &str,
        generator_version: &str,
        total_pairs: i64,
        pair_breakdown_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO corpus_snapshots
                     (fingerprint, generator_version, total_pairs, pair_breakdown_json)
                 VALUES (?1, ?2, ?3, ?4)",
                params![fingerprint, generator_version, total_pairs, pair_breakdown_json],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Return `true` if `fingerprint` is already recorded in `corpus_snapshots`.
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::is_corpus_fresh`.
    pub async fn is_corpus_fresh(&self, fingerprint: &str) -> Result<bool, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT 1 FROM corpus_snapshots WHERE fingerprint = ?1 LIMIT 1",
                params![fingerprint],
            )
            .await?;
        Ok(rows.next().await?.is_some())
    }

    // ── Packages (packages) ───────────────────────────────────────────────────

    /// Full-text search of the `packages` table.
    ///
    /// Returns rows where `name`, `description`, or `author` match `%query%`.
    /// An empty or `%` query returns all packages up to `limit`. Used by
    /// `vox share search` and `vox share list`.
    pub async fn search_packages(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<PackageSearchResult>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let pattern = if query.is_empty() || query == "%" {
            "%".to_string()
        } else {
            format!("%{query}%")
        };
        let mut rows = self
            .conn
            .query(
                "SELECT name, version, description, author, license
                 FROM packages
                 WHERE name LIKE ?1 OR COALESCE(description,'') LIKE ?1 OR COALESCE(author,'') LIKE ?1
                 ORDER BY name ASC LIMIT ?2",
                params![pattern, lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(PackageSearchResult {
                name: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                version: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                description: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                author: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                license: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    /// List all `(version, content_hash)` pairs for a package `name`.
    ///
    /// Returns rows newest-by-rowid first. Used by `vox info` local fallback.
    pub async fn get_package_versions(
        &self,
        name: &str,
    ) -> Result<Vec<(String, String)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT version, COALESCE(content_hash,'') FROM packages
                 WHERE name = ?1
                 ORDER BY rowid DESC",
                params![name],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let ver: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let hash: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((ver, hash));
        }
        Ok(out)
    }

    // ── Skill Manifests (skill_manifests) ─────────────────────────────────────

    /// Upsert a row in `skill_manifests`. Returns `()` on success.
    ///
    /// Called from `vox-skills/src/registry.rs` `SkillRegistry::install`.
    pub async fn publish_skill(
        &self,
        id: &str,
        version: &str,
        manifest_json: &str,
        skill_md: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO skill_manifests (id, version, manifest_json, skill_md, created_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                params![id, version, manifest_json, skill_md],
            )
            .await?;
        Ok(())
    }

    /// Delete the `skill_manifests` row for `id`. No-op if absent.
    ///
    /// Called from `vox-skills/src/registry.rs` `SkillRegistry::uninstall`.
    pub async fn unpublish_skill(&self, id: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "DELETE FROM skill_manifests WHERE id = ?1",
                params![id],
            )
            .await?;
        Ok(())
    }

    /// Return all rows from `skill_manifests`, ordered by `id`.
    ///
    /// Called from `vox-skills/src/registry.rs` `SkillRegistry::hydrate_from_db`.
    pub async fn list_skill_manifests(&self) -> Result<Vec<SkillManifestEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, version, manifest_json, COALESCE(skill_md,'') FROM skill_manifests ORDER BY id ASC",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(SkillManifestEntry {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                version: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                manifest_json: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                skill_md: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }
}
