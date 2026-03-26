use turso::params;

use crate::store::types::{
    EndpointReliabilityEntry, PackageSearchResult, StoreError,
};

impl crate::VoxDb {
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
                total_requests: row
                    .get::<i64>(2)
                    .map_err(|e| StoreError::Db(e.to_string()))?
                    as u64,
                hallucination_proxy_ewma: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                contradiction_ratio_ewma: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                infra_failure_ewma: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                rate_limit_hits: row
                    .get::<i64>(6)
                    .map_err(|e| StoreError::Db(e.to_string()))?
                    as u64,
                timeout_hits: row
                    .get::<i64>(7)
                    .map_err(|e| StoreError::Db(e.to_string()))?
                    as u64,
                updated_at_ms: row.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    /// `skill_reliability` rows, lowest reliability first (CLI / ops surface).
    pub async fn list_skill_reliability_worst_first(
        &self,
        limit: i64,
    ) -> Result<Vec<(String, f64, i64, i64)>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let mut rows = self
            .conn
            .query(
                "SELECT skill_id, reliability, success_count, failure_count
             FROM skill_reliability ORDER BY reliability ASC LIMIT ?1",
                params![lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            ));
        }
        Ok(out)
    }

    /// `workflow_reliability` rows, lowest reliability first.
    pub async fn list_workflow_reliability_worst_first(
        &self,
        limit: i64,
    ) -> Result<Vec<(String, f64, i64, i64)>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let mut rows = self
            .conn
            .query(
                "SELECT workflow_name, reliability, success_count, failure_count
             FROM workflow_reliability ORDER BY reliability ASC LIMIT ?1",
                params![lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            ));
        }
        Ok(out)
    }

    /// `repository_reliability` rows, lowest reliability first.
    pub async fn list_repository_reliability_worst_first(
        &self,
        limit: i64,
    ) -> Result<Vec<(String, f64, i64, i64)>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let mut rows = self
            .conn
            .query(
                "SELECT repository_id, reliability, success_count, failure_count
             FROM repository_reliability ORDER BY reliability ASC LIMIT ?1",
                params![lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push((
                row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            ));
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
                params![
                    fingerprint,
                    generator_version,
                    total_pairs,
                    pair_breakdown_json
                ],
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
}
