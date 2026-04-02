//! Unified trust observation + rollup persistence for multi-dimensional reliability.

use serde::{Deserialize, Serialize};
use turso::params;

use crate::store::{StoreError, TrustRollupEntry};

/// One grouped aggregate over `trust_rollups` (dashboard / MCP summary).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustRollupGroupSummary {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    pub rollup_count: i64,
    pub mean_score: f64,
    pub min_score: f64,
    pub max_score: f64,
    pub sum_sample_size: i64,
    pub max_updated_at_ms: i64,
}

/// One append-only trust observation.
#[derive(Debug, Clone)]
pub struct TrustObservationInput<'a> {
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub dimension: &'a str,
    pub domain: Option<&'a str>,
    pub task_class: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub model_id: Option<&'a str>,
    pub repository_id: Option<&'a str>,
    pub source_kind: Option<&'a str>,
    pub observation_value: f64,
    pub confidence_weight: f64,
    pub sample_size: i64,
    pub artifact_ref: Option<&'a str>,
    pub metadata_json: Option<&'a str>,
    pub ewma_alpha: f64,
}

/// One raw trust observation row from `trust_observations`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrustObservationEntry {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: String,
    pub dimension: String,
    pub domain: String,
    pub task_class: String,
    pub provider: String,
    pub model_id: String,
    pub repository_id: String,
    pub source_kind: String,
    pub observation_value: f64,
    pub confidence_weight: f64,
    pub sample_size: i64,
    pub artifact_ref: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at_ms: i64,
}

impl<'a> TrustObservationInput<'a> {
    #[must_use]
    pub fn with_defaults(entity_type: &'a str, entity_id: &'a str, dimension: &'a str) -> Self {
        Self {
            entity_type,
            entity_id,
            dimension,
            domain: None,
            task_class: None,
            provider: None,
            model_id: None,
            repository_id: None,
            source_kind: None,
            observation_value: 0.5,
            confidence_weight: 1.0,
            sample_size: 1,
            artifact_ref: None,
            metadata_json: None,
            ewma_alpha: 0.10,
        }
    }
}

impl crate::VoxDb {
    /// Append one trust observation and update its scoped EWMA rollup.
    pub async fn record_trust_observation(
        &self,
        observation: TrustObservationInput<'_>,
    ) -> Result<i64, StoreError> {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let domain = observation.domain.unwrap_or_default();
        let task_class = observation.task_class.unwrap_or_default();
        let provider = observation.provider.unwrap_or_default();
        let model_id = observation.model_id.unwrap_or_default();
        let repository_id = observation.repository_id.unwrap_or_default();
        let source_kind = observation.source_kind.unwrap_or_default();
        let confidence_weight = observation.confidence_weight.clamp(0.0, 1.0);
        let sample_size = observation.sample_size.max(1);
        let ewma_alpha = observation.ewma_alpha.clamp(0.01, 1.0);
        let score = observation.observation_value.clamp(0.0, 1.0);
        let weighted_score = (score * confidence_weight).clamp(0.0, 1.0);

        let row_id = {
            let breaker = self.breaker.clone();
            let conn = self.conn.clone();
            let entity_type = observation.entity_type.to_string();
            let entity_id = observation.entity_id.to_string();
            let dimension = observation.dimension.to_string();
            let domain = domain.to_string();
            let task_class = task_class.to_string();
            let provider = provider.to_string();
            let model_id = model_id.to_string();
            let repository_id = repository_id.to_string();
            let source_kind = source_kind.to_string();
            let artifact_ref = observation.artifact_ref.map(str::to_string);
            let metadata_json = observation.metadata_json.map(str::to_string);

            breaker
                .call(|| async move {
                    conn.execute(
                        "INSERT INTO trust_observations
                         (entity_type, entity_id, dimension, domain, task_class, provider, model_id,
                          repository_id, source_kind, observation_value, confidence_weight, sample_size,
                          artifact_ref, metadata_json, created_at_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                        params![
                            entity_type.as_str(),
                            entity_id.as_str(),
                            dimension.as_str(),
                            domain.as_str(),
                            task_class.as_str(),
                            provider.as_str(),
                            model_id.as_str(),
                            repository_id.as_str(),
                            source_kind.as_str(),
                            score,
                            confidence_weight,
                            sample_size,
                            artifact_ref.as_deref(),
                            metadata_json.as_deref(),
                            now_ms,
                        ],
                    )
                    .await?;

                    conn.execute(
                        "INSERT INTO trust_rollups
                         (entity_type, entity_id, dimension, domain, task_class, provider, model_id,
                          repository_id, score, sample_size, ewma_alpha, updated_at_ms)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                         ON CONFLICT(entity_type, entity_id, dimension, domain, task_class, provider, model_id, repository_id)
                         DO UPDATE SET
                           score = trust_rollups.score * (1.0 - ?11) + ?9 * ?11,
                           sample_size = trust_rollups.sample_size + ?10,
                           ewma_alpha = ?11,
                           updated_at_ms = ?12",
                        params![
                            entity_type.as_str(),
                            entity_id.as_str(),
                            dimension.as_str(),
                            domain.as_str(),
                            task_class.as_str(),
                            provider.as_str(),
                            model_id.as_str(),
                            repository_id.as_str(),
                            weighted_score,
                            sample_size,
                            ewma_alpha,
                            now_ms,
                        ],
                    )
                    .await?;

                    Ok::<i64, StoreError>(conn.last_insert_rowid())
                })
                .await?
        };

        Ok(row_id)
    }

    /// List trust rollups with optional scope filters.
    pub async fn list_trust_rollups(
        &self,
        entity_type: Option<&str>,
        dimension: Option<&str>,
        domain: Option<&str>,
        repository_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<TrustRollupEntry>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let mut rows = self
            .conn
            .query(
                "SELECT entity_type, entity_id, dimension, domain, task_class, provider, model_id,
                        repository_id, score, sample_size, ewma_alpha, updated_at_ms
                 FROM trust_rollups
                 WHERE (?1 IS NULL OR entity_type = ?1)
                   AND (?2 IS NULL OR dimension = ?2)
                   AND (?3 IS NULL OR domain = ?3)
                   AND (?4 IS NULL OR repository_id = ?4)
                 ORDER BY score DESC, updated_at_ms DESC
                 LIMIT ?5",
                params![entity_type, dimension, domain, repository_id, lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(TrustRollupEntry {
                entity_type: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                entity_id: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                dimension: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                domain: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                task_class: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                provider: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                model_id: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                repository_id: row.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                score: row.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                sample_size: row
                    .get::<i64>(9)
                    .map_err(|e| StoreError::Db(e.to_string()))?
                    .max(0) as u64,
                ewma_alpha: row.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
                updated_at_ms: row.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    /// Convenience query for one trust dimension by entity type, keyed by entity id.
    pub async fn list_trust_scores_for_dimension(
        &self,
        entity_type: &str,
        dimension: &str,
        domain: Option<&str>,
        limit: i64,
    ) -> Result<Vec<(String, f64)>, StoreError> {
        let rows = self
            .list_trust_rollups(Some(entity_type), Some(dimension), domain, None, limit)
            .await?;
        Ok(rows.into_iter().map(|r| (r.entity_id, r.score)).collect())
    }

    /// Aggregate `trust_rollups` rows for operator dashboards.
    ///
    /// `group_by`: `dimension` | `domain` | `entity_type` | `dimension_domain` | `entity_dimension`.
    /// Optional filters apply before grouping. `repository_id` when set matches exactly (including empty string).
    pub async fn summarize_trust_rollups(
        &self,
        entity_type: Option<&str>,
        dimension: Option<&str>,
        domain: Option<&str>,
        repository_id: Option<&str>,
        group_by: &str,
        limit_groups: i64,
    ) -> Result<Vec<TrustRollupGroupSummary>, StoreError> {
        let lim = limit_groups.clamp(1, 500);
        let g = group_by.trim().to_ascii_lowercase();
        let sql: &'static str = match g.as_str() {
            "dimension" => {
                r#"SELECT dimension,
                          COUNT(*) AS rollup_count,
                          AVG(score) AS mean_score,
                          MIN(score) AS min_score,
                          MAX(score) AS max_score,
                          COALESCE(SUM(sample_size), 0) AS sum_sample_size,
                          MAX(updated_at_ms) AS max_updated_at_ms
                   FROM trust_rollups
                   WHERE (?1 IS NULL OR entity_type = ?1)
                     AND (?2 IS NULL OR dimension = ?2)
                     AND (?3 IS NULL OR domain = ?3)
                     AND (?4 IS NULL OR repository_id = ?4)
                   GROUP BY dimension
                   ORDER BY mean_score DESC
                   LIMIT ?5"#
            }
            "domain" => {
                r#"SELECT domain,
                          COUNT(*) AS rollup_count,
                          AVG(score) AS mean_score,
                          MIN(score) AS min_score,
                          MAX(score) AS max_score,
                          COALESCE(SUM(sample_size), 0) AS sum_sample_size,
                          MAX(updated_at_ms) AS max_updated_at_ms
                   FROM trust_rollups
                   WHERE (?1 IS NULL OR entity_type = ?1)
                     AND (?2 IS NULL OR dimension = ?2)
                     AND (?3 IS NULL OR domain = ?3)
                     AND (?4 IS NULL OR repository_id = ?4)
                   GROUP BY domain
                   ORDER BY mean_score DESC
                   LIMIT ?5"#
            }
            "entity_type" => {
                r#"SELECT entity_type,
                          COUNT(*) AS rollup_count,
                          AVG(score) AS mean_score,
                          MIN(score) AS min_score,
                          MAX(score) AS max_score,
                          COALESCE(SUM(sample_size), 0) AS sum_sample_size,
                          MAX(updated_at_ms) AS max_updated_at_ms
                   FROM trust_rollups
                   WHERE (?1 IS NULL OR entity_type = ?1)
                     AND (?2 IS NULL OR dimension = ?2)
                     AND (?3 IS NULL OR domain = ?3)
                     AND (?4 IS NULL OR repository_id = ?4)
                   GROUP BY entity_type
                   ORDER BY mean_score DESC
                   LIMIT ?5"#
            }
            "dimension_domain" => {
                r#"SELECT dimension, domain,
                          COUNT(*) AS rollup_count,
                          AVG(score) AS mean_score,
                          MIN(score) AS min_score,
                          MAX(score) AS max_score,
                          COALESCE(SUM(sample_size), 0) AS sum_sample_size,
                          MAX(updated_at_ms) AS max_updated_at_ms
                   FROM trust_rollups
                   WHERE (?1 IS NULL OR entity_type = ?1)
                     AND (?2 IS NULL OR dimension = ?2)
                     AND (?3 IS NULL OR domain = ?3)
                     AND (?4 IS NULL OR repository_id = ?4)
                   GROUP BY dimension, domain
                   ORDER BY mean_score DESC
                   LIMIT ?5"#
            }
            "entity_dimension" => {
                r#"SELECT entity_type, dimension,
                          COUNT(*) AS rollup_count,
                          AVG(score) AS mean_score,
                          MIN(score) AS min_score,
                          MAX(score) AS max_score,
                          COALESCE(SUM(sample_size), 0) AS sum_sample_size,
                          MAX(updated_at_ms) AS max_updated_at_ms
                   FROM trust_rollups
                   WHERE (?1 IS NULL OR entity_type = ?1)
                     AND (?2 IS NULL OR dimension = ?2)
                     AND (?3 IS NULL OR domain = ?3)
                     AND (?4 IS NULL OR repository_id = ?4)
                   GROUP BY entity_type, dimension
                   ORDER BY mean_score DESC
                   LIMIT ?5"#
            }
            _ => {
                return Err(StoreError::Db(format!(
                    "summarize_trust_rollups: unknown group_by {group_by:?} \
                     (expected dimension|domain|entity_type|dimension_domain|entity_dimension)"
                )));
            }
        };

        let mut rows = self
            .conn
            .query(
                sql,
                params![entity_type, dimension, domain, repository_id, lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let (etype, dim, dom) = match g.as_str() {
                "dimension" => (
                    None,
                    Some(
                        row.get::<String>(0)
                            .map_err(|e| StoreError::Db(e.to_string()))?,
                    ),
                    None,
                ),
                "domain" => (
                    None,
                    None,
                    Some(
                        row.get::<String>(0)
                            .map_err(|e| StoreError::Db(e.to_string()))?,
                    ),
                ),
                "entity_type" => (
                    Some(
                        row.get::<String>(0)
                            .map_err(|e| StoreError::Db(e.to_string()))?,
                    ),
                    None,
                    None,
                ),
                "dimension_domain" => (
                    None,
                    Some(
                        row.get::<String>(0)
                            .map_err(|e| StoreError::Db(e.to_string()))?,
                    ),
                    Some(
                        row.get::<String>(1)
                            .map_err(|e| StoreError::Db(e.to_string()))?,
                    ),
                ),
                "entity_dimension" => (
                    Some(
                        row.get::<String>(0)
                            .map_err(|e| StoreError::Db(e.to_string()))?,
                    ),
                    Some(
                        row.get::<String>(1)
                            .map_err(|e| StoreError::Db(e.to_string()))?,
                    ),
                    None,
                ),
                _ => unreachable!("group_by validated above"),
            };
            let off = match g.as_str() {
                "dimension" | "domain" | "entity_type" => 1,
                "dimension_domain" | "entity_dimension" => 2,
                _ => {
                    return Err(StoreError::Db(
                        "summarize_trust_rollups: invalid group column mapping".into(),
                    ));
                }
            };
            let rollup_count = row
                .get::<i64>(off)
                .map_err(|e| StoreError::Db(e.to_string()))?;
            let mean_score = row
                .get::<f64>(off + 1)
                .map_err(|e| StoreError::Db(e.to_string()))?;
            let min_score = row
                .get::<f64>(off + 2)
                .map_err(|e| StoreError::Db(e.to_string()))?;
            let max_score = row
                .get::<f64>(off + 3)
                .map_err(|e| StoreError::Db(e.to_string()))?;
            let sum_sample_size = row
                .get::<i64>(off + 4)
                .map_err(|e| StoreError::Db(e.to_string()))?;
            let max_updated_at_ms = row
                .get::<i64>(off + 5)
                .map_err(|e| StoreError::Db(e.to_string()))?;
            out.push(TrustRollupGroupSummary {
                entity_type: etype,
                dimension: dim,
                domain: dom,
                rollup_count,
                mean_score,
                min_score,
                max_score,
                sum_sample_size,
                max_updated_at_ms,
            });
        }
        Ok(out)
    }

    /// List raw trust observations for forensic workflows.
    pub async fn list_trust_observations(
        &self,
        entity_type: Option<&str>,
        dimension: Option<&str>,
        domain: Option<&str>,
        repository_id: Option<&str>,
        artifact_ref: Option<&str>,
        since_ms: Option<i64>,
        limit: i64,
    ) -> Result<Vec<TrustObservationEntry>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let mut rows = self
            .conn
            .query(
                "SELECT id, entity_type, entity_id, dimension, domain, task_class, provider, model_id,
                        repository_id, source_kind, observation_value, confidence_weight, sample_size,
                        artifact_ref, metadata_json, created_at_ms
                 FROM trust_observations
                 WHERE (?1 IS NULL OR entity_type = ?1)
                   AND (?2 IS NULL OR dimension = ?2)
                   AND (?3 IS NULL OR domain = ?3)
                   AND (?4 IS NULL OR repository_id = ?4)
                   AND (?5 IS NULL OR artifact_ref = ?5)
                   AND (?6 IS NULL OR created_at_ms >= ?6)
                 ORDER BY created_at_ms DESC, id DESC
                 LIMIT ?7",
                params![
                    entity_type,
                    dimension,
                    domain,
                    repository_id,
                    artifact_ref,
                    since_ms,
                    lim
                ],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(TrustObservationEntry {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                entity_type: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                entity_id: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                dimension: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                domain: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                task_class: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                provider: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                model_id: row.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
                repository_id: row.get(8).map_err(|e| StoreError::Db(e.to_string()))?,
                source_kind: row.get(9).map_err(|e| StoreError::Db(e.to_string()))?,
                observation_value: row.get(10).map_err(|e| StoreError::Db(e.to_string()))?,
                confidence_weight: row.get(11).map_err(|e| StoreError::Db(e.to_string()))?,
                sample_size: row.get(12).map_err(|e| StoreError::Db(e.to_string()))?,
                artifact_ref: row.get(13).map_err(|e| StoreError::Db(e.to_string()))?,
                metadata_json: row.get(14).map_err(|e| StoreError::Db(e.to_string()))?,
                created_at_ms: row.get(15).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use crate::{DbConfig, VoxDb};

    #[tokio::test]
    async fn trust_observation_upserts_rollup() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
        db.record_trust_observation(crate::TrustObservationInput {
            entity_type: "agent",
            entity_id: "7",
            dimension: "task_completion",
            domain: Some("single_shot"),
            task_class: Some("single_shot"),
            provider: None,
            model_id: None,
            repository_id: Some("repo-1"),
            source_kind: Some("test"),
            observation_value: 1.0,
            confidence_weight: 1.0,
            sample_size: 1,
            artifact_ref: Some("task-1"),
            metadata_json: None,
            ewma_alpha: 0.5,
        })
        .await
        .expect("insert trust");
        db.record_trust_observation(crate::TrustObservationInput {
            entity_type: "agent",
            entity_id: "7",
            dimension: "task_completion",
            domain: Some("single_shot"),
            task_class: Some("single_shot"),
            provider: None,
            model_id: None,
            repository_id: Some("repo-1"),
            source_kind: Some("test"),
            observation_value: 0.0,
            confidence_weight: 1.0,
            sample_size: 1,
            artifact_ref: Some("task-2"),
            metadata_json: None,
            ewma_alpha: 0.5,
        })
        .await
        .expect("update trust");

        let rows = db
            .list_trust_rollups(
                Some("agent"),
                Some("task_completion"),
                Some("single_shot"),
                None,
                20,
            )
            .await
            .expect("list trust");
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.entity_id, "7");
        assert_eq!(row.sample_size, 2);
        assert!(row.score >= 0.0 && row.score <= 1.0);

        let groups = db
            .summarize_trust_rollups(
                Some("agent"),
                Some("task_completion"),
                Some("single_shot"),
                Some("repo-1"),
                "dimension",
                10,
            )
            .await
            .expect("summarize");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].dimension.as_deref(), Some("task_completion"));
        assert_eq!(groups[0].rollup_count, 1);
        assert!(groups[0].mean_score >= 0.0 && groups[0].mean_score <= 1.0);
    }
}
