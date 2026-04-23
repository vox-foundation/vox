//! Compare [`trust_observations`](super::schema) across time windows (drift / calibration hints).

use serde::Serialize;
use turso::params;

use crate::VoxDb;
use crate::store::StoreError;

/// Aggregates for one `created_at_ms` window on `trust_observations`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TrustObservationWindowStats {
    pub start_ms: i64,
    pub end_ms: i64,
    pub count: i64,
    pub mean_observation: f64,
}

/// Recent vs prior window comparison (same duration).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TrustObservationDriftReport {
    pub entity_type_filter: Option<String>,
    pub dimension_filter: Option<String>,
    pub window_ms: i64,
    pub recent: TrustObservationWindowStats,
    pub prior: TrustObservationWindowStats,
    pub mean_delta: f64,
}

async fn window_agg(
    vox: &VoxDb,
    entity_type: Option<&str>,
    dimension: Option<&str>,
    start_ms: i64,
    end_ms: i64,
) -> Result<TrustObservationWindowStats, StoreError> {
    let mut rows = vox
        .conn
        .query(
            "SELECT COUNT(*), AVG(observation_value)
             FROM trust_observations
             WHERE (?1 IS NULL OR entity_type = ?1)
               AND (?2 IS NULL OR dimension = ?2)
               AND created_at_ms >= ?3 AND created_at_ms < ?4",
            params![entity_type, dimension, start_ms, end_ms],
        )
        .await?;
    let row = rows
        .next()
        .await?
        .ok_or_else(|| StoreError::Db("trust drift: empty aggregate".into()))?;
    let count = row
        .get::<i64>(0)
        .map_err(|e| StoreError::Db(e.to_string()))?;
    let mean_observation = if count > 0 {
        row.get::<f64>(1)
            .map_err(|e| StoreError::Db(e.to_string()))?
    } else {
        0.0
    };
    Ok(TrustObservationWindowStats {
        start_ms,
        end_ms,
        count,
        mean_observation,
    })
}

impl VoxDb {
    /// Compare mean `observation_value` in the last `window_ms` to the preceding window of equal length.
    pub async fn trust_observation_drift_two_window(
        &self,
        entity_type: Option<&str>,
        dimension: Option<&str>,
        window_ms: i64,
    ) -> Result<TrustObservationDriftReport, StoreError> {
        let w = window_ms.clamp(60_000, 86_400_000 * 30);
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let recent_start = now_ms.saturating_sub(w);
        let prior_start = recent_start.saturating_sub(w);
        let recent = window_agg(self, entity_type, dimension, recent_start, now_ms).await?;
        let prior = window_agg(self, entity_type, dimension, prior_start, recent_start).await?;
        let mean_delta = recent.mean_observation - prior.mean_observation;
        Ok(TrustObservationDriftReport {
            entity_type_filter: entity_type.map(str::to_string),
            dimension_filter: dimension.map(str::to_string),
            window_ms: w,
            recent,
            prior,
            mean_delta,
        })
    }
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use crate::{DbConfig, TrustObservationInput, VoxDb};

    #[tokio::test]
    async fn drift_runs_with_filters() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        db.record_trust_observation(TrustObservationInput {
            entity_type: "model",
            entity_id: "m1",
            dimension: "factuality",
            domain: Some("d"),
            task_class: None,
            provider: None,
            model_id: None,
            repository_id: Some("r"),
            source_kind: Some("test"),
            observation_value: 0.2,
            confidence_weight: 1.0,
            sample_size: 1,
            artifact_ref: None,
            metadata_json: None,
            ewma_alpha: 0.5,
        })
        .await
        .expect("obs");
        let rep = db
            .trust_observation_drift_two_window(Some("model"), Some("factuality"), 3_600_000)
            .await
            .expect("drift");
        assert_eq!(rep.entity_type_filter.as_deref(), Some("model"));
        assert_eq!(rep.dimension_filter.as_deref(), Some("factuality"));
    }
}
