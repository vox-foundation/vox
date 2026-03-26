use crate::{VoxDb, store::StoreError, store::types::*};
use super::common::now_ms;

impl VoxDb {
    pub async fn insert_external_status_snapshot(
        &self,
        p: ExternalStatusSnapshotParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO external_status_snapshots (
                    adapter, external_submission_id, publication_id, content_sha3_256, snapshot_json, fetched_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (
                    p.adapter.to_string(),
                    p.external_submission_id.to_string(),
                    p.publication_id.to_string(),
                    p.content_sha3_256.to_string(),
                    p.snapshot_json.to_string(),
                    ts,
                ),
            )
            .await?;
        Ok(())
    }

    /// Latest remote snapshot for adapter + external id.
    pub async fn get_latest_external_status_snapshot(
        &self,
        adapter: &str,
        external_submission_id: &str,
    ) -> Result<Option<ExternalStatusSnapshotRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, adapter, external_submission_id, publication_id, content_sha3_256, snapshot_json, fetched_at_ms
                 FROM external_status_snapshots
                 WHERE adapter = ?1 AND external_submission_id = ?2
                 ORDER BY fetched_at_ms DESC
                 LIMIT 1",
                (adapter.to_string(), external_submission_id.to_string()),
            )
            .await?;
        let Some(r) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(ExternalStatusSnapshotRow {
            id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
            adapter: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
            external_submission_id: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            publication_id: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            content_sha3_256: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
            snapshot_json: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            fetched_at_ms: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
        }))
    }

    /// Upsert a publication external link (DOI, URL, deposition id, note id, ...).
    pub async fn upsert_publication_external_link(
        &self,
        p: PublicationExternalLinkUpsertParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO publication_external_links (
                    publication_id, content_sha3_256, adapter, link_kind, link_value, metadata_json, created_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(publication_id, content_sha3_256, adapter, link_kind) DO UPDATE SET
                    link_value = excluded.link_value,
                    metadata_json = excluded.metadata_json,
                    created_at_ms = excluded.created_at_ms",
                (
                    p.publication_id.to_string(),
                    p.content_sha3_256.to_string(),
                    p.adapter.to_string(),
                    p.link_kind.to_string(),
                    p.link_value.to_string(),
                    p.metadata_json.map(std::string::ToString::to_string),
                    ts,
                ),
            )
            .await?;
        Ok(())
    }

    /// List external links for publication id + digest.
    pub async fn list_publication_external_links(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
    ) -> Result<Vec<PublicationExternalLinkRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, link_kind, link_value, metadata_json, created_at_ms
                 FROM publication_external_links
                 WHERE publication_id = ?1 AND content_sha3_256 = ?2
                 ORDER BY id ASC",
                (
                    publication_id.to_string(),
                    content_sha3_256.to_string(),
                ),
            )
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(PublicationExternalLinkRow {
                    id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                    publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                    content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                    adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                    link_kind: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                    link_value: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                    metadata_json: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                    created_at_ms: r.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                })
            })
            .collect()
    }

    /// Upsert the adapter revision pointer for a publication + content digest.
    pub async fn upsert_publication_external_revision(
        &self,
        p: PublicationExternalRevisionUpsertParams<'_>,
    ) -> Result<(), StoreError> {
        let ts = now_ms();
        self.conn
            .execute(
                "INSERT INTO publication_external_revisions (
                    publication_id, content_sha3_256, adapter, external_revision, metadata_json, updated_at_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(publication_id, content_sha3_256, adapter) DO UPDATE SET
                    external_revision = excluded.external_revision,
                    metadata_json = excluded.metadata_json,
                    updated_at_ms = excluded.updated_at_ms",
                (
                    p.publication_id.to_string(),
                    p.content_sha3_256.to_string(),
                    p.adapter.to_string(),
                    p.external_revision.to_string(),
                    p.metadata_json.map(std::string::ToString::to_string),
                    ts,
                ),
            )
            .await?;
        Ok(())
    }

    /// Load revision pointer for publication id + digest + adapter, if present.
    pub async fn get_publication_external_revision(
        &self,
        publication_id: &str,
        content_sha3_256: &str,
        adapter: &str,
    ) -> Result<Option<PublicationExternalRevisionRow>, StoreError> {
        let rows = self
            .query_all(
                "SELECT id, publication_id, content_sha3_256, adapter, external_revision, metadata_json, updated_at_ms
                 FROM publication_external_revisions
                 WHERE publication_id = ?1 AND content_sha3_256 = ?2 AND adapter = ?3",
                (
                    publication_id.to_string(),
                    content_sha3_256.to_string(),
                    adapter.to_string(),
                ),
            )
            .await?;
        let Some(r) = rows.first() else {
            return Ok(None);
        };
        Ok(Some(PublicationExternalRevisionRow {
            id: r.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
            publication_id: r.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
            content_sha3_256: r.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
            adapter: r.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            external_revision: r.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
            metadata_json: r.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
            updated_at_ms: r.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
        }))
    }

    /// Read-only rollup over scholarly external tables: job queue snapshot, attempts and snapshots in a time window,
    /// approximate terminal latencies from job row timestamps, and multi-channel `publication_attempts` counts.
    pub async fn summarize_scholarly_external_pipeline_metrics(
        &self,
        since_ms: i64,
    ) -> Result<serde_json::Value, StoreError> {
        fn percentile_from_sorted(sorted: &[i64], p: f64) -> Option<f64> {
            if sorted.is_empty() {
                return None;
            }
            let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
            let idx = idx.min(sorted.len() - 1);
            Some(sorted[idx] as f64)
        }

        let generated_at_ms = now_ms();
        let jobs_by_status_rows = self
            .query_all(
                "SELECT status, COUNT(*) FROM external_submission_jobs GROUP BY status ORDER BY status",
                (),
            )
            .await?;
        let mut jobs_by_status = serde_json::Map::new();
        for r in jobs_by_status_rows {
            let k: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let n: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            jobs_by_status.insert(k, serde_json::json!(n));
        }

        let jobs_by_status_win = self
            .query_all(
                "SELECT status, COUNT(*) FROM external_submission_jobs WHERE updated_at_ms >= ?1 GROUP BY status ORDER BY status",
                (since_ms,),
            )
            .await?;
        let mut by_status_in_window = serde_json::Map::new();
        for r in jobs_by_status_win {
            let k: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let n: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            by_status_in_window.insert(k, serde_json::json!(n));
        }

        let adapter_status_rows = self
            .query_all(
                "SELECT adapter, status, COUNT(*) FROM external_submission_jobs GROUP BY adapter, status ORDER BY adapter, status",
                (),
            )
            .await?;
        let mut by_adapter_status = Vec::new();
        for r in adapter_status_rows {
            by_adapter_status.push(serde_json::json!({
                "adapter": r.get::<String>(0).map_err(|e| StoreError::Db(e.to_string()))?,
                "status": r.get::<String>(1).map_err(|e| StoreError::Db(e.to_string()))?,
                "count": r.get::<i64>(2).map_err(|e| StoreError::Db(e.to_string()))?,
            }));
        }

        let adapter_status_win = self
            .query_all(
                "SELECT adapter, status, COUNT(*) FROM external_submission_jobs WHERE updated_at_ms >= ?1 GROUP BY adapter, status ORDER BY adapter, status",
                (since_ms,),
            )
            .await?;
        let mut by_adapter_status_in_window = Vec::new();
        for r in adapter_status_win {
            by_adapter_status_in_window.push(serde_json::json!({
                "adapter": r.get::<String>(0).map_err(|e| StoreError::Db(e.to_string()))?,
                "status": r.get::<String>(1).map_err(|e| StoreError::Db(e.to_string()))?,
                "count": r.get::<i64>(2).map_err(|e| StoreError::Db(e.to_string()))?,
            }));
        }

        let lat_ok_row = self
            .query_all(
                "SELECT AVG(updated_at_ms - created_at_ms) FROM external_submission_jobs WHERE status = 'succeeded' AND updated_at_ms >= ?1",
                (since_ms,),
            )
            .await?;
        let avg_latency_succeeded_ms: Option<f64> = lat_ok_row
            .first()
            .and_then(|r| r.get::<Option<f64>>(0).ok().flatten());

        let lat_fail_row = self
            .query_all(
                "SELECT AVG(updated_at_ms - created_at_ms) FROM external_submission_jobs WHERE status = 'failed' AND updated_at_ms >= ?1",
                (since_ms,),
            )
            .await?;
        let avg_latency_failed_ms: Option<f64> = lat_fail_row
            .first()
            .and_then(|r| r.get::<Option<f64>>(0).ok().flatten());

        let lat_all = self
            .query_all(
                "SELECT updated_at_ms - created_at_ms FROM external_submission_jobs WHERE status IN ('succeeded','failed') AND updated_at_ms >= ?1 AND updated_at_ms >= created_at_ms",
                (since_ms,),
            )
            .await?;
        let mut lat_ms: Vec<i64> = lat_all
            .into_iter()
            .filter_map(|r| r.get::<i64>(0).ok())
            .collect();
        lat_ms.sort_unstable();
        let p50 = percentile_from_sorted(&lat_ms, 0.50);
        let p90 = percentile_from_sorted(&lat_ms, 0.90);
        let p99 = percentile_from_sorted(&lat_ms, 0.99);

        let term_ratio_rows = self
            .query_all(
                "SELECT adapter,
                        SUM(CASE WHEN status = 'succeeded' THEN 1 ELSE 0 END) AS ok_ct,
                        SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS fail_ct
                 FROM external_submission_jobs
                 WHERE status IN ('succeeded','failed') AND updated_at_ms >= ?1
                 GROUP BY adapter ORDER BY adapter",
                (since_ms,),
            )
            .await?;
        let mut per_adapter_terminal = Vec::new();
        for r in term_ratio_rows {
            let adapter: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let ok_ct: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let fail_ct: i64 = r.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            let denom = ok_ct + fail_ct;
            let success_ratio = if denom > 0 {
                Some(ok_ct as f64 / denom as f64)
            } else {
                None
            };
            per_adapter_terminal.push(serde_json::json!({
                "adapter": adapter,
                "terminal_succeeded": ok_ct,
                "terminal_failed": fail_ct,
                "success_ratio": success_ratio,
            }));
        }

        let attempt_total_rows = self
            .query_all(
                "SELECT COUNT(*), COALESCE(SUM(retryable), 0) FROM external_submission_attempts WHERE attempted_at_ms >= ?1",
                (since_ms,),
            )
            .await?;
        let (attempts_total, attempts_retryable) = if let Some(r) = attempt_total_rows.first() {
            let c: i64 = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let s: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            (c, s)
        } else {
            (0_i64, 0_i64)
        };

        let retry_ratio_rows = self
            .query_all(
                "SELECT j.adapter AS adapter,
                        COUNT(*) AS attempts,
                        COALESCE(SUM(a.retryable), 0) AS retryable_ct
                 FROM external_submission_attempts a
                 JOIN external_submission_jobs j ON j.id = a.job_id
                 WHERE a.attempted_at_ms >= ?1
                 GROUP BY j.adapter
                 ORDER BY j.adapter",
                (since_ms,),
            )
            .await?;
        let mut per_adapter_attempt_retry_ratio = Vec::new();
        for r in retry_ratio_rows {
            let adapter: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let attempts: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let retryable_ct: i64 = r.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            let ratio = if attempts > 0 {
                Some(retryable_ct as f64 / attempts as f64)
            } else {
                None
            };
            per_adapter_attempt_retry_ratio.push(serde_json::json!({
                "adapter": adapter,
                "attempts_in_window": attempts,
                "retryable_attempts_in_window": retryable_ct,
                "retry_ratio": ratio,
            }));
        }

        let err_class_rows = self
            .query_all(
                "SELECT COALESCE(error_class, ''), COUNT(*) FROM external_submission_attempts WHERE attempted_at_ms >= ?1 GROUP BY error_class ORDER BY COUNT(*) DESC",
                (since_ms,),
            )
            .await?;
        let mut by_error_class = serde_json::Map::new();
        for r in err_class_rows {
            let k: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let n: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let key = if k.is_empty() { "(null)".to_string() } else { k };
            by_error_class.insert(key, serde_json::json!(n));
        }

        let snap_rows = self
            .query_all(
                "SELECT COUNT(*) FROM external_status_snapshots WHERE fetched_at_ms >= ?1",
                (since_ms,),
            )
            .await?;
        let snapshots_since: i64 = snap_rows
            .first()
            .map(|r| r.get(0).map_err(|e| StoreError::Db(e.to_string())))
            .transpose()?
            .unwrap_or(0);

        let sub_rows = self
            .query_all(
                "SELECT adapter, status, COUNT(*) FROM scholarly_submissions GROUP BY adapter, status ORDER BY adapter, status",
                (),
            )
            .await?;
        let mut scholarly_by_adapter_status = Vec::new();
        for r in sub_rows {
            scholarly_by_adapter_status.push(serde_json::json!({
                "adapter": r.get::<String>(0).map_err(|e| StoreError::Db(e.to_string()))?,
                "status": r.get::<String>(1).map_err(|e| StoreError::Db(e.to_string()))?,
                "count": r.get::<i64>(2).map_err(|e| StoreError::Db(e.to_string()))?,
            }));
        }

        let sub_win = self
            .query_all(
                "SELECT adapter, status, COUNT(*) FROM scholarly_submissions WHERE updated_at_ms >= ?1 GROUP BY adapter, status ORDER BY adapter, status",
                (since_ms,),
            )
            .await?;
        let mut scholarly_by_adapter_status_in_window = Vec::new();
        for r in sub_win {
            scholarly_by_adapter_status_in_window.push(serde_json::json!({
                "adapter": r.get::<String>(0).map_err(|e| StoreError::Db(e.to_string()))?,
                "status": r.get::<String>(1).map_err(|e| StoreError::Db(e.to_string()))?,
                "count": r.get::<i64>(2).map_err(|e| StoreError::Db(e.to_string()))?,
            }));
        }

        let pub_attempt_rows = self
            .query_all(
                "SELECT channel, COUNT(*) FROM publication_attempts WHERE attempted_at_ms >= ?1 GROUP BY channel ORDER BY COUNT(*) DESC",
                (since_ms,),
            )
            .await?;
        let mut publication_attempts_by_channel = serde_json::Map::new();
        for r in pub_attempt_rows {
            let ch: String = r.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let n: i64 = r.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            publication_attempts_by_channel.insert(ch, serde_json::json!(n));
        }

        Ok(serde_json::json!({
            "metrics_schema_version": 2_i64,
            "generated_at_ms": generated_at_ms,
            "since_ms": since_ms,
            "external_submission_jobs": {
                "by_status": jobs_by_status,
                "by_status_in_window": by_status_in_window,
                "by_adapter_status": by_adapter_status,
                "by_adapter_status_in_window": by_adapter_status_in_window,
                "avg_terminal_latency_ms_in_window": {
                    "succeeded": avg_latency_succeeded_ms,
                    "failed": avg_latency_failed_ms,
                },
                "terminal_latency_ms_percentiles_in_window": {
                    "p50": p50,
                    "p90": p90,
                    "p99": p99,
                },
                "per_adapter_terminal_ratio_in_window": per_adapter_terminal,
            },
            "external_submission_attempts": {
                "total_in_window": attempts_total,
                "retryable_in_window": attempts_retryable,
                "by_error_class": by_error_class,
                "per_adapter_retry_ratio_in_window": per_adapter_attempt_retry_ratio,
            },
            "external_status_snapshots_in_window": snapshots_since,
            "scholarly_submissions_by_adapter_status": scholarly_by_adapter_status,
            "scholarly_submissions_by_adapter_status_in_window": scholarly_by_adapter_status_in_window,
            "publication_attempts_in_window_by_channel": publication_attempts_by_channel,
        }))
    }
}
