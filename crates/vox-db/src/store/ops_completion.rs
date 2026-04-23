//! CI completion audit persistence (`ci_completion_*` tables).
//!
//! **S2 workspace-adjacent** telemetry: paths, fingerprints, and detector ids can reveal repository shape.
//! Retention / prune: `contracts/db/retention-policy.yaml`; rationale: `docs/src/architecture/telemetry-retention-sensitivity-ssot.md`.

use turso::params;

use crate::store::types::StoreError;

impl crate::VoxDb {
    /// Insert a completion audit run row; returns new `id`.
    pub async fn insert_ci_completion_run(
        &self,
        repository_id: &str,
        branch: Option<&str>,
        commit_sha: Option<&str>,
        workflow: &str,
        run_kind: &str,
        tool_versions_json: Option<&str>,
    ) -> Result<i64, StoreError> {
        let repository_id = repository_id.to_string();
        let branch = branch.map(str::to_string);
        let commit_sha = commit_sha.map(str::to_string);
        let workflow = workflow.to_string();
        let run_kind = run_kind.to_string();
        let tool_versions_json = tool_versions_json.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO ci_completion_run (repository_id, branch, commit_sha, workflow, run_kind, tool_versions_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        repository_id.as_str(),
                        branch.as_deref(),
                        commit_sha.as_deref(),
                        workflow.as_str(),
                        run_kind.as_str(),
                        tool_versions_json.as_deref(),
                    ],
                )
                .await?;
                let mut rows = conn.query("SELECT last_insert_rowid()", ()).await?;
                let id: i64 = rows.next().await?.and_then(|r| r.get(0).ok()).unwrap_or(0);
                Ok::<i64, StoreError>(id)
            })
            .await
    }

    /// Insert one finding; skips duplicate fingerprint for same run (ignored).
    pub async fn insert_ci_completion_finding(
        &self,
        run_id: i64,
        detector_id: &str,
        tier: &str,
        severity: &str,
        confidence: Option<&str>,
        file_path: Option<&str>,
        symbol: Option<&str>,
        line_start: Option<i64>,
        line_end: Option<i64>,
        fingerprint: &str,
        meta_json: Option<&str>,
    ) -> Result<(), StoreError> {
        let detector_id = detector_id.to_string();
        let tier = tier.to_string();
        let severity = severity.to_string();
        let confidence = confidence.map(str::to_string);
        let file_path = file_path.map(str::to_string);
        let symbol = symbol.map(str::to_string);
        let fingerprint = fingerprint.to_string();
        let meta_json = meta_json.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT OR IGNORE INTO ci_completion_finding
                    (run_id, detector_id, tier, severity, confidence, file_path, symbol, line_start, line_end, fingerprint, meta_json)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        run_id,
                        detector_id.as_str(),
                        tier.as_str(),
                        severity.as_str(),
                        confidence.as_deref(),
                        file_path.as_deref(),
                        symbol.as_deref(),
                        line_start,
                        line_end,
                        fingerprint.as_str(),
                        meta_json.as_deref(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Upsert per-detector snapshot row for a run.
    pub async fn upsert_ci_completion_detector_snapshot(
        &self,
        run_id: i64,
        detector_id: &str,
        tier: &str,
        finding_count: i64,
        new_count: i64,
        resolved_count: i64,
        block_state: Option<&str>,
    ) -> Result<(), StoreError> {
        let detector_id = detector_id.to_string();
        let tier = tier.to_string();
        let block_state = block_state.map(str::to_string);
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO ci_completion_detector_snapshot
                    (run_id, detector_id, tier, finding_count, new_count, resolved_count, block_state)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                    ON CONFLICT(run_id, detector_id) DO UPDATE SET
                    finding_count = excluded.finding_count,
                    new_count = excluded.new_count,
                    resolved_count = excluded.resolved_count,
                    tier = excluded.tier,
                    block_state = excluded.block_state",
                    params![
                        run_id,
                        detector_id.as_str(),
                        tier.as_str(),
                        finding_count,
                        new_count,
                        resolved_count,
                        block_state.as_deref(),
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Latest completion run for a repository (by largest `id`).
    pub async fn latest_ci_completion_run_id(
        &self,
        repository_id: &str,
    ) -> Result<Option<i64>, StoreError> {
        let repository_id = repository_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = conn
                    .query(
                        "SELECT id FROM ci_completion_run WHERE repository_id = ?1 ORDER BY id DESC LIMIT 1",
                        params![repository_id.as_str()],
                    )
                    .await?;
                let id = match rows.next().await? {
                    Some(row) => Some(row.get::<i64>(0)?),
                    None => None,
                };
                Ok::<Option<i64>, StoreError>(id)
            })
            .await
    }

    /// Fingerprints grouped by `detector_id` for a completed run.
    pub async fn ci_completion_fingerprints_by_detector(
        &self,
        run_id: i64,
    ) -> Result<std::collections::HashMap<String, std::collections::HashSet<String>>, StoreError>
    {
        use std::collections::{HashMap, HashSet};

        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = conn
                    .query(
                        "SELECT detector_id, fingerprint FROM ci_completion_finding WHERE run_id = ?1",
                        params![run_id],
                    )
                    .await?;
                let mut out: HashMap<String, HashSet<String>> = HashMap::new();
                while let Some(row) = rows.next().await? {
                    let det: String = row.get(0)?;
                    let fp: String = row.get(1)?;
                    out.entry(det).or_default().insert(fp);
                }
                Ok::<HashMap<String, HashSet<String>>, StoreError>(out)
            })
            .await
    }
}
