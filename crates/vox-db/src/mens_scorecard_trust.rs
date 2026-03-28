//! Ingest MENS scorecard `summary.json` artifacts into [`super::TrustObservationInput`].

use serde::{Deserialize, Serialize};

use crate::store::StoreError;
use crate::{TrustObservationInput, VoxDb};

#[derive(Debug, Deserialize)]
struct MensScorecardSummaryWire {
    schema: String,
    conditions: Vec<MensScorecardConditionWire>,
}

#[derive(Debug, Deserialize, Serialize)]
struct MensScorecardConditionWire {
    id: String,
    #[serde(default)]
    label: String,
    backend: String,
    model_id: Option<String>,
    #[serde(default)]
    adapter_revision: Option<String>,
    total_tasks: usize,
    compile_pass_at_1: f64,
    compile_pass_at_n: f64,
    canonical_pass: f64,
    #[serde(default)]
    voxelized_strictness: f64,
    task_success: f64,
    semantic_task_success: f64,
    repair_depth_mean: f64,
    repair_stall_rate: f64,
    #[serde(default)]
    time_to_first_valid_p50_ms: u64,
    #[serde(default)]
    latency_p50_ms: u64,
    #[serde(default)]
    latency_p95_ms: u64,
    #[serde(default)]
    tokens_in_total: usize,
    #[serde(default)]
    tokens_out_total: usize,
    #[serde(default)]
    placeholder_hits_total: usize,
    #[serde(default)]
    placeholder_event_rate: f64,
    #[serde(default)]
    trivial_placeholder_rate: f64,
    construct_richness_mean: f64,
    anti_stub_pass_rate: f64,
}

impl VoxDb {
    /// Record trust observations from a validated `vox_mens_scorecard_summary_v1` JSON payload.
    ///
    /// Writes per-condition metrics under entity type `model`, domain `mens_{backend}`,
    /// task class `mens_scorecard`, and `artifact_ref` set to `artifact_ref` (typically path or URI).
    pub async fn ingest_mens_scorecard_summary_json(
        &self,
        summary_json: &str,
        repository_id: &str,
        artifact_ref: &str,
    ) -> Result<usize, StoreError> {
        let s: MensScorecardSummaryWire = serde_json::from_str(summary_json)
            .map_err(|e| StoreError::Serialization(format!("mens scorecard summary JSON: {e}")))?;
        if s.schema != "vox_mens_scorecard_summary_v1" {
            return Err(StoreError::Db(format!(
                "unexpected scorecard schema {:?}; expected vox_mens_scorecard_summary_v1",
                s.schema
            )));
        }
        let mut count = 0_usize;
        for c in &s.conditions {
            let sample = c.total_tasks.max(1) as i64;
            let entity_id = c.model_id.as_deref().unwrap_or(c.id.as_str());
            let domain = format!("mens_{}", c.backend);
            let meta =
                serde_json::to_string(c).map_err(|e| StoreError::Serialization(e.to_string()))?;
            let model_id_field = c.model_id.as_deref().unwrap_or("");
            let dims: [(&str, f64); 7] = [
                ("mens_task_success_rate", c.task_success),
                ("mens_compile_pass_at_1", c.compile_pass_at_1),
                ("mens_compile_pass_at_n", c.compile_pass_at_n),
                ("mens_canonical_pass_rate", c.canonical_pass),
                ("mens_semantic_task_success_rate", c.semantic_task_success),
                ("mens_construct_richness_mean", c.construct_richness_mean),
                ("mens_anti_stub_pass_rate", c.anti_stub_pass_rate),
            ];
            for (dimension, v) in dims {
                self.record_trust_observation(TrustObservationInput {
                    entity_type: "model",
                    entity_id,
                    dimension,
                    domain: Some(domain.as_str()),
                    task_class: Some("mens_scorecard"),
                    provider: None,
                    model_id: Some(model_id_field),
                    repository_id: Some(repository_id),
                    source_kind: Some("mens_scorecard_summary"),
                    observation_value: v.clamp(0.0, 1.0),
                    confidence_weight: 1.0,
                    sample_size: sample,
                    artifact_ref: Some(artifact_ref),
                    metadata_json: Some(meta.as_str()),
                    ewma_alpha: 0.12,
                })
                .await?;
                count += 1;
            }
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use crate::{DbConfig, VoxDb};

    #[tokio::test]
    async fn ingest_summary_records_seven_dimensions_per_condition() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        let json = r#"{"schema":"vox_mens_scorecard_summary_v1","conditions":[{"id":"c1","label":"l","backend":"qlora","model_id":"m1","adapter_revision":null,"total_tasks":3,"compile_pass_at_1":0.9,"compile_pass_at_n":1.0,"canonical_pass":1.0,"voxelized_strictness":1.0,"task_success":1.0,"semantic_task_success":1.0,"repair_depth_mean":0.0,"repair_stall_rate":0.0,"time_to_first_valid_p50_ms":1,"latency_p50_ms":1,"latency_p95_ms":2,"tokens_in_total":1,"tokens_out_total":1,"placeholder_hits_total":0,"placeholder_event_rate":0.0,"trivial_placeholder_rate":0.0,"construct_richness_mean":0.5,"anti_stub_pass_rate":1.0}]}"#;
        let n = db
            .ingest_mens_scorecard_summary_json(json, "repo_test", "artifact")
            .await
            .expect("ingest");
        assert_eq!(n, 7);
    }
}
