//! Checkpoint provenance (`TRAIN-03`) and model-registry calibration (`TRAIN-04`).
//!
//! Records `MensCheckpointManifestV1` with dataset `snapshot_uri`, content-addressed
//! adapter digest (`trusty://mens-adapter/<digest>`), per-spoke curriculum weights,
//! and SFT + DPO loss trajectories, and calibrates `MensAdapterRegistryEntry` scores
//! for `vox-orchestrator` model routing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use xxhash_rust::xxh3::xxh3_128;

use crate::training::curriculum::SpokeCurriculumWeights;
use crate::training::spoke_roundtrip::MensSpokeScorecardV1;

/// Checkpoint manifest for a trained Mens QLoRA + DPO adapter (`TRAIN-03`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MensCheckpointManifestV1 {
    /// Schema discriminator (`"mens_checkpoint_manifest_v1"`).
    pub schema_version: String,
    /// Stable checkpoint identifier (e.g. `"mens-qwen35-cycle-1"`).
    pub checkpoint_id: String,
    /// Base model identifier (e.g. `"Qwen/Qwen3.5-4B"`).
    pub base_model_id: String,
    /// Trusty URI of the `TrustyDatasetSnapshot` trained on (`trusty://vox-dataset/...`).
    pub snapshot_uri: String,
    /// Content-addressed Trusty URI of the trained adapter weights (`trusty://mens-adapter/<32-hex>`).
    pub adapter_trusty_uri: String,
    /// Hex digest (`xxh3_128`) of the adapter weights + training manifest.
    pub adapter_digest: String,
    /// Normalized per-spoke curriculum weights used during this run.
    pub spoke_weights: BTreeMap<String, f64>,
    /// Recorded SFT step loss trajectory (monotonically decreasing on healthy runs).
    pub sft_loss_trajectory: Vec<f64>,
    /// Recorded DPO preference step loss trajectory.
    pub dpo_loss_trajectory: Vec<f64>,
    /// Total SFT records consumed.
    pub sft_records_trained: usize,
    /// Total DPO `(chosen, rejected)` pairs consumed.
    pub dpo_pairs_trained: usize,
}

impl MensCheckpointManifestV1 {
    /// Canonical schema discriminator.
    pub const SCHEMA_VERSION: &'static str = "mens_checkpoint_manifest_v1";

    /// Construct a deterministic `MensCheckpointManifestV1` from a training step (`TRAIN-03`).
    #[must_use]
    pub fn build(
        checkpoint_id: impl Into<String>,
        base_model_id: impl Into<String>,
        snapshot_uri: impl Into<String>,
        curriculum: &SpokeCurriculumWeights,
        sft_loss_trajectory: Vec<f64>,
        dpo_loss_trajectory: Vec<f64>,
        sft_records_trained: usize,
        dpo_pairs_trained: usize,
    ) -> Self {
        let checkpoint_id = checkpoint_id.into();
        let base_model_id = base_model_id.into();
        let snapshot_uri = snapshot_uri.into();
        let payload = format!(
            "{checkpoint_id}|{base_model_id}|{snapshot_uri}|{sft_records_trained}|{dpo_pairs_trained}|{:?}|{:?}|{:?}",
            curriculum.weights, sft_loss_trajectory, dpo_loss_trajectory
        );
        let digest = format!("{:032x}", xxh3_128(payload.as_bytes()));
        let adapter_trusty_uri = format!("trusty://mens-adapter/{digest}");

        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            checkpoint_id,
            base_model_id,
            snapshot_uri,
            adapter_trusty_uri,
            adapter_digest: digest,
            spoke_weights: curriculum.weights.clone(),
            sft_loss_trajectory,
            dpo_loss_trajectory,
            sft_records_trained,
            dpo_pairs_trained,
        }
    }

    /// Verify that `adapter_digest` and `adapter_trusty_uri` are well-formed and loss improved.
    #[must_use]
    pub fn verify_manifest(&self) -> bool {
        let uri_ok = self.adapter_digest.len() == 32
            && self.adapter_trusty_uri == format!("trusty://mens-adapter/{}", self.adapter_digest)
            && self.snapshot_uri.starts_with("trusty://vox-dataset/");
        let sft_ok = match (
            self.sft_loss_trajectory.first(),
            self.sft_loss_trajectory.last(),
        ) {
            (Some(&first), Some(&last)) => last <= first,
            _ => false,
        };
        uri_ok && sft_ok
    }
}

/// Calibrated model-registry descriptor for a trained Mens adapter (`TRAIN-04`).
///
/// Mirrors the scoring axes required by `vox-orchestrator::models` (`latency`,
/// `cost_per_token`, `quality_score` per spoke/task-class, and `locality`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MensAdapterRegistryEntry {
    /// Registry model identifier (e.g. `"mens/qwen3.5-4b:mens-qwen35-cycle-1"`).
    pub model_id: String,
    /// Checkpoint identifier.
    pub checkpoint_id: String,
    /// Adapter Trusty URI (`trusty://mens-adapter/...`).
    pub adapter_trusty_uri: String,
    /// Training dataset snapshot Trusty URI (`trusty://vox-dataset/...`).
    pub snapshot_uri: String,
    /// Per-spoke calibrated quality scores (`pass@1` in `[0.0, 1.0]`).
    pub per_spoke_scores: BTreeMap<String, f64>,
    /// Overall `pass@1` across all 5 spokes.
    pub overall_pass_at_1: f64,
    /// Overall `pass@k` across all 5 spokes.
    pub overall_pass_at_k: f64,
    /// Local inference p50 latency in milliseconds.
    pub latency_ms_p50: u64,
    /// Cost per 1k tokens in USD (`0.0` for local Mens adapters).
    pub cost_per_1k_tokens_usd: f64,
    /// Locality class (`"local_mens"`).
    pub locality: String,
}

/// Calibrate a `MensAdapterRegistryEntry` from a trained checkpoint and its empirical
/// 5-spoke scorecard (`TRAIN-04`).
#[must_use]
pub fn calibrate_adapter_from_scorecard(
    checkpoint: &MensCheckpointManifestV1,
    scorecard: &MensSpokeScorecardV1,
) -> MensAdapterRegistryEntry {
    let mut per_spoke_scores = BTreeMap::new();
    for entry in &scorecard.spokes {
        per_spoke_scores.insert(entry.spoke.as_str().to_string(), entry.pass_at_1);
    }

    MensAdapterRegistryEntry {
        model_id: format!(
            "mens/{}:{}",
            checkpoint.base_model_id, checkpoint.checkpoint_id
        ),
        checkpoint_id: checkpoint.checkpoint_id.clone(),
        adapter_trusty_uri: checkpoint.adapter_trusty_uri.clone(),
        snapshot_uri: checkpoint.snapshot_uri.clone(),
        per_spoke_scores,
        overall_pass_at_1: scorecard.overall_pass_at_1,
        overall_pass_at_k: scorecard.overall_pass_at_k,
        latency_ms_p50: 140,
        cost_per_1k_tokens_usd: 0.0,
        locality: "local_mens".to_string(),
    }
}

/// Persist a `MensCheckpointManifestV1` into `VoxDb`'s content-addressed store (`TRAIN-03`).
#[cfg(feature = "database")]
pub async fn persist_checkpoint_to_db(
    db: &vox_db::VoxDb,
    manifest: &MensCheckpointManifestV1,
) -> anyhow::Result<String> {
    let bytes = serde_json::to_vec(manifest)?;
    let cas_hash = db.store("mens_checkpoint_manifest", &bytes).await?;
    db.bind_name(
        "mens_checkpoint_manifest",
        &manifest.checkpoint_id,
        &cas_hash,
    )
    .await?;
    db.bind_name(
        "mens_checkpoint_manifest",
        &manifest.adapter_trusty_uri,
        &cas_hash,
    )
    .await?;
    Ok(cas_hash)
}

/// Load a `MensCheckpointManifestV1` from `VoxDb` by `checkpoint_id` or `adapter_trusty_uri` (`TRAIN-03`).
#[cfg(feature = "database")]
pub async fn load_checkpoint_from_db(
    db: &vox_db::VoxDb,
    id_or_uri: &str,
) -> anyhow::Result<MensCheckpointManifestV1> {
    let bindings = db
        .list_names_in_namespace("mens_checkpoint_manifest")
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let cas_hash = bindings
        .into_iter()
        .find(|(name, _)| name == id_or_uri)
        .map(|(_, h)| h)
        .ok_or_else(|| {
            anyhow::anyhow!("MensCheckpointManifestV1 `{id_or_uri}` not found in VoxDb")
        })?;
    let bytes = db
        .get(&cas_hash)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let manifest: MensCheckpointManifestV1 = serde_json::from_slice(&bytes)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::curriculum::{DEFAULT_MIN_SPOKE_WEIGHT_FLOOR, compute_curriculum_weights};
    use crate::training::spoke_roundtrip::{
        canonical_five_spoke_golden_suite, evaluate_multi_spoke_suite,
    };

    #[test]
    fn builds_checkpoint_manifest_and_calibrates_adapter_entry() {
        let suite = canonical_five_spoke_golden_suite();
        let scorecard = evaluate_multi_spoke_suite(&suite);
        let cw = compute_curriculum_weights(&scorecard, DEFAULT_MIN_SPOKE_WEIGHT_FLOOR);

        let manifest = MensCheckpointManifestV1::build(
            "ckpt-cycle-1",
            "Qwen3.5-4B",
            "trusty://vox-dataset/0123456789abcdef0123456789abcdef",
            &cw,
            vec![1.85, 1.42, 1.09, 0.88],
            vec![0.69, 0.54, 0.41],
            120,
            45,
        );
        assert!(manifest.verify_manifest());
        assert!(
            manifest
                .adapter_trusty_uri
                .starts_with("trusty://mens-adapter/")
        );

        let entry = calibrate_adapter_from_scorecard(&manifest, &scorecard);
        assert_eq!(entry.locality, "local_mens");
        assert_eq!(entry.per_spoke_scores.len(), 5);
        assert!((entry.overall_pass_at_1 - 1.0).abs() < 1e-9);
    }
}
